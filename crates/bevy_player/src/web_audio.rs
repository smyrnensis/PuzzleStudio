use std::{
    cell::RefCell,
    rc::{Rc, Weak},
    sync::Arc,
};

use bevy::{
    prelude::*,
    winit::{EventLoopProxyWrapper, WinitUserEvent},
};
use puzzle_audio::AudioAssetCatalog;
use puzzle_web_audio::{
    BrowserAudioBackend, BrowserAudioError, BrowserAudioUnlockCompletion, BrowserAudioUnlockStart,
    BrowserAudioWorkletCompletion, BrowserAudioWorkletStart,
};
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::{Event, EventTarget, VisibilityState};

use super::{AudioDeviceFeedback, PuzzleBevyPlayerHost};

enum BrowserAudioEvent {
    UnlockCompleted(BrowserAudioUnlockCompletion),
    WorkletCompleted(BrowserAudioWorkletCompletion),
    DeviceFailure(String),
}

struct DomListener {
    target: EventTarget,
    event_name: &'static str,
    callback: Closure<dyn FnMut(Event)>,
}

impl DomListener {
    fn install(
        target: &EventTarget,
        event_name: &'static str,
        callback: Closure<dyn FnMut(Event)>,
    ) -> Result<Self, String> {
        target
            .add_event_listener_with_callback(event_name, callback.as_ref().unchecked_ref())
            .map_err(|error| {
                format!("browser audio could not install the `{event_name}` listener: {error:?}")
            })?;
        Ok(Self {
            target: target.clone(),
            event_name,
            callback,
        })
    }
}

impl Drop for DomListener {
    fn drop(&mut self) {
        let _ = self.target.remove_event_listener_with_callback(
            self.event_name,
            self.callback.as_ref().unchecked_ref(),
        );
    }
}

struct BrowserAudioDevice {
    backend: Rc<RefCell<BrowserAudioBackend>>,
    events: Rc<RefCell<Vec<BrowserAudioEvent>>>,
    listeners: Vec<DomListener>,
    listeners_installed: bool,
    initialization_reported: bool,
}

impl BrowserAudioDevice {
    fn new(catalog: Arc<AudioAssetCatalog>) -> Self {
        Self {
            backend: Rc::new(RefCell::new(BrowserAudioBackend::new(catalog))),
            events: Rc::new(RefCell::new(Vec::new())),
            listeners: Vec::new(),
            listeners_installed: false,
            initialization_reported: false,
        }
    }
}

pub(crate) struct PuzzleBevyWebAudioPlugin {
    catalog: Arc<AudioAssetCatalog>,
}

impl PuzzleBevyWebAudioPlugin {
    pub(crate) fn new(catalog: Arc<AudioAssetCatalog>) -> Self {
        Self { catalog }
    }
}

impl Plugin for PuzzleBevyWebAudioPlugin {
    fn build(&self, app: &mut App) {
        app.insert_non_send(BrowserAudioDevice::new(Arc::clone(&self.catalog)))
            .add_systems(
                Update,
                (install_browser_audio_listeners, process_browser_audio).chain(),
            );
    }
}

fn install_browser_audio_listeners(
    event_loop: Option<Res<EventLoopProxyWrapper>>,
    mut device: NonSendMut<BrowserAudioDevice>,
) {
    if device.listeners_installed {
        return;
    }
    let Some(event_loop) = event_loop else {
        return;
    };
    let Some(window) = web_sys::window() else {
        device
            .events
            .borrow_mut()
            .push(BrowserAudioEvent::DeviceFailure(
                "browser audio requires a Window".to_string(),
            ));
        device.listeners_installed = true;
        return;
    };
    let Some(document) = window.document() else {
        device
            .events
            .borrow_mut()
            .push(BrowserAudioEvent::DeviceFailure(
                "browser audio requires a Document".to_string(),
            ));
        device.listeners_installed = true;
        return;
    };
    let proxy = (**event_loop).clone();
    let wake: Rc<dyn Fn()> = Rc::new(move || {
        let _ = proxy.send_event(WinitUserEvent::WakeUp);
    });
    device
        .backend
        .borrow()
        .set_feedback_wakeup(Rc::clone(&wake));
    device
        .backend
        .borrow()
        .set_context_state_wakeup(Rc::clone(&wake));
    begin_worklet_load(&device.backend, &device.events, &wake);

    let document_target: EventTarget = document.clone().unchecked_into();
    let weak_backend = Rc::downgrade(&device.backend);
    let events = Rc::clone(&device.events);
    let unlock_wake = Rc::clone(&wake);
    let unlock = Closure::wrap(Box::new(move |_event: Event| {
        begin_unlock_from_gesture(&weak_backend, &events, &unlock_wake);
    }) as Box<dyn FnMut(Event)>);
    install_listener(&mut device, &document_target, "pointerdown", unlock, &wake);

    let weak_backend = Rc::downgrade(&device.backend);
    let events = Rc::clone(&device.events);
    let unlock_wake = Rc::clone(&wake);
    let unlock = Closure::wrap(Box::new(move |_event: Event| {
        begin_unlock_from_gesture(&weak_backend, &events, &unlock_wake);
    }) as Box<dyn FnMut(Event)>);
    install_listener(&mut device, &document_target, "keydown", unlock, &wake);

    let weak_backend = Rc::downgrade(&device.backend);
    let visibility_document = document.clone();
    let visibility_wake = Rc::clone(&wake);
    let visibility = Closure::wrap(Box::new(move |_event: Event| {
        if let Some(backend) = weak_backend.upgrade() {
            let visible = visibility_document.visibility_state() != VisibilityState::Hidden;
            backend.borrow_mut().set_visible(visible);
            visibility_wake();
        }
    }) as Box<dyn FnMut(Event)>);
    install_listener(
        &mut device,
        &document_target,
        "visibilitychange",
        visibility,
        &wake,
    );

    let visible = document.visibility_state() != VisibilityState::Hidden;
    device.backend.borrow_mut().set_visible(visible);
    device.listeners_installed = true;
    wake();
}

fn begin_worklet_load(
    backend: &Rc<RefCell<BrowserAudioBackend>>,
    events: &Rc<RefCell<Vec<BrowserAudioEvent>>>,
    wake: &Rc<dyn Fn()>,
) {
    match backend.borrow_mut().begin_worklet_load() {
        Ok(BrowserAudioWorkletStart::Started(task)) => {
            let events = Rc::clone(events);
            let wake = Rc::clone(wake);
            wasm_bindgen_futures::spawn_local(async move {
                let completion = task.await;
                events
                    .borrow_mut()
                    .push(BrowserAudioEvent::WorkletCompleted(completion));
                wake();
            });
        }
        Ok(BrowserAudioWorkletStart::InFlight | BrowserAudioWorkletStart::Ready) => {}
        Err(error) => {
            warn!("music worklet initialization diagnostic: {error}");
        }
    }
}

fn install_listener(
    device: &mut BrowserAudioDevice,
    target: &EventTarget,
    event_name: &'static str,
    callback: Closure<dyn FnMut(Event)>,
    wake: &Rc<dyn Fn()>,
) {
    match DomListener::install(target, event_name, callback) {
        Ok(listener) => device.listeners.push(listener),
        Err(error) => {
            device
                .events
                .borrow_mut()
                .push(BrowserAudioEvent::DeviceFailure(error));
            wake();
        }
    }
}

fn begin_unlock_from_gesture(
    weak_backend: &Weak<RefCell<BrowserAudioBackend>>,
    events: &Rc<RefCell<Vec<BrowserAudioEvent>>>,
    wake: &Rc<dyn Fn()>,
) {
    let Some(backend) = weak_backend.upgrade() else {
        return;
    };
    let start = backend.borrow_mut().begin_unlock();
    match start {
        Ok(BrowserAudioUnlockStart::Started(task)) => {
            let events = Rc::clone(events);
            let wake = Rc::clone(wake);
            wasm_bindgen_futures::spawn_local(async move {
                let completion = task.await;
                events
                    .borrow_mut()
                    .push(BrowserAudioEvent::UnlockCompleted(completion));
                wake();
            });
        }
        Ok(BrowserAudioUnlockStart::InFlight) => {}
        Ok(BrowserAudioUnlockStart::Ready(_)) => wake(),
        Err(error) => {
            events
                .borrow_mut()
                .push(BrowserAudioEvent::DeviceFailure(error.to_string()));
            wake();
        }
    }
}

fn process_browser_audio(
    time: Res<Time>,
    mut host: NonSendMut<PuzzleBevyPlayerHost>,
    mut device: NonSendMut<BrowserAudioDevice>,
) {
    let now_seconds = time.elapsed_secs_f64();
    let queued = {
        let mut events = device.events.borrow_mut();
        std::mem::take(&mut *events)
    };
    for event in queued {
        match event {
            BrowserAudioEvent::UnlockCompleted(completion) => {
                let result = device.backend.borrow_mut().finish_unlock(completion);
                match result {
                    Ok(capability) => host.apply_audio_device_feedback(
                        AudioDeviceFeedback::Capability(capability),
                        now_seconds,
                    ),
                    Err(error) => report_browser_error(&mut host, error, now_seconds),
                }
            }
            BrowserAudioEvent::WorkletCompleted(completion) => {
                if let Err(error) = device.backend.borrow_mut().finish_worklet_load(completion) {
                    // The worklet is a music-only capability. Keep the running
                    // AudioContext available to SFX; a later music command will
                    // receive the same typed worklet failure at its own voice.
                    warn!("music worklet initialization diagnostic: {error}");
                }
            }
            BrowserAudioEvent::DeviceFailure(error) => host.apply_audio_device_feedback(
                AudioDeviceFeedback::DeviceFailure(error),
                now_seconds,
            ),
        }
    }

    if !device.initialization_reported {
        let initialization_error = device
            .backend
            .borrow()
            .initialization_error()
            .map(str::to_owned);
        if let Some(error) = initialization_error {
            host.apply_audio_device_feedback(
                AudioDeviceFeedback::DeviceFailure(error),
                now_seconds,
            );
            device.initialization_reported = true;
        }
    }

    let capability = device.backend.borrow().capability();
    host.apply_audio_device_feedback(AudioDeviceFeedback::Capability(capability), now_seconds);

    let feedback_errors = device.backend.borrow_mut().take_feedback_errors();
    for error in feedback_errors {
        report_browser_error(&mut host, error, now_seconds);
    }
    let ended_voices = device.backend.borrow_mut().take_ended_voices();
    for voice in ended_voices {
        host.apply_audio_device_feedback(AudioDeviceFeedback::VoiceEnded(voice), now_seconds);
    }

    let errors = device
        .backend
        .borrow_mut()
        .consume_all(host.take_audio_commands());
    for error in errors {
        report_browser_error(&mut host, error, now_seconds);
    }

    for diagnostic in host.take_audio_diagnostics() {
        warn!("audio output diagnostic: {diagnostic:?}");
    }
}

fn report_browser_error(
    host: &mut PuzzleBevyPlayerHost,
    error: BrowserAudioError,
    now_seconds: f64,
) {
    match error.voice_id() {
        Some(voice) => host.apply_audio_device_feedback(
            AudioDeviceFeedback::VoiceFailure {
                voice,
                error: error.to_string(),
            },
            now_seconds,
        ),
        None => host.apply_audio_device_feedback(
            AudioDeviceFeedback::DeviceFailure(error.to_string()),
            now_seconds,
        ),
    }
}
