use js_sys::Float32Array;
use puzzle_audio::{GeneratedMusicTrack, decode_music_worklet_asset};
use wasm_bindgen::prelude::*;

const WORKLET_QUANTUM_FRAMES: usize = 128;

#[wasm_bindgen]
pub struct WorkletMusicRenderer {
    track: GeneratedMusicTrack,
    cursor: f64,
    canonical_frames_per_output_frame: f64,
    interleaved: Vec<f32>,
    scratch: Vec<f64>,
    left: Vec<f32>,
    right: Vec<f32>,
    rendered_frames: usize,
    playing: bool,
}

#[wasm_bindgen]
impl WorkletMusicRenderer {
    #[wasm_bindgen(constructor)]
    pub fn new(
        payload: &[u8],
        output_sample_rate: f64,
        start_frame: u64,
    ) -> Result<WorkletMusicRenderer, JsValue> {
        let track = decode_music_worklet_asset(payload).map_err(js_error)?;
        if track.channels() != 2 {
            return Err(js_error(format!(
                "AudioWorklet music requires two channels, got {}",
                track.channels()
            )));
        }
        if output_sample_rate <= 0.0 || !output_sample_rate.is_finite() {
            return Err(js_error(format!(
                "AudioWorklet output sample rate must be finite and positive, got {output_sample_rate}"
            )));
        }
        let ratio = f64::from(track.sample_rate()) / output_sample_rate;
        let canonical_frames = (ratio * (WORKLET_QUANTUM_FRAMES - 1) as f64).ceil() + 1.0;
        if !canonical_frames.is_finite() || canonical_frames > (usize::MAX / 2) as f64 {
            return Err(js_error(format!(
                "AudioWorklet sample-rate ratio requires unsupported block size {canonical_frames}"
            )));
        }
        let canonical_samples = canonical_frames as usize * 2;
        let loop_frames = track.loop_frames();
        Ok(Self {
            track,
            cursor: (start_frame % loop_frames) as f64,
            canonical_frames_per_output_frame: ratio,
            interleaved: vec![0.0; canonical_samples],
            scratch: vec![0.0; canonical_samples],
            left: vec![0.0; WORKLET_QUANTUM_FRAMES],
            right: vec![0.0; WORKLET_QUANTUM_FRAMES],
            rendered_frames: 0,
            playing: true,
        })
    }

    pub fn render(&mut self, frame_count: usize) -> Result<(), JsValue> {
        if frame_count > WORKLET_QUANTUM_FRAMES {
            return Err(js_error(format!(
                "AudioWorklet quantum has {frame_count} frames; maximum is {WORKLET_QUANTUM_FRAMES}"
            )));
        }
        self.rendered_frames = frame_count;
        if !self.playing {
            self.left[..frame_count].fill(0.0);
            self.right[..frame_count].fill(0.0);
            return Ok(());
        }
        if frame_count == 0 {
            return Ok(());
        }

        let first_frame = self.cursor.floor() as u64;
        let last_cursor =
            self.cursor + self.canonical_frames_per_output_frame * (frame_count - 1) as f64;
        let canonical_frames = (last_cursor.floor() as u64 - first_frame + 1) as usize;
        let canonical_samples = canonical_frames * 2;
        self.track
            .render_interleaved(
                first_frame,
                &mut self.interleaved[..canonical_samples],
                &mut self.scratch[..canonical_samples],
            )
            .map_err(|error| js_error(error.to_string()))?;

        for offset in 0..frame_count {
            let frame = self.cursor + self.canonical_frames_per_output_frame * offset as f64;
            let source = (frame.floor() as u64 - first_frame) as usize * 2;
            self.left[offset] = self.interleaved[source];
            self.right[offset] = self.interleaved[source + 1];
        }
        self.cursor += self.canonical_frames_per_output_frame * frame_count as f64;
        self.cursor %= self.track.loop_frames() as f64;
        Ok(())
    }

    pub fn copy_left(&self, output: &Float32Array) -> Result<(), JsValue> {
        self.copy_channel(output, &self.left)
    }

    pub fn copy_right(&self, output: &Float32Array) -> Result<(), JsValue> {
        self.copy_channel(output, &self.right)
    }

    pub fn pause(&mut self, at_frame: u64) -> Result<(), JsValue> {
        self.cursor = (at_frame % self.track.loop_frames()) as f64;
        self.playing = false;
        Ok(())
    }

    pub fn resume(&mut self, at_frame: u64) -> Result<(), JsValue> {
        self.cursor = (at_frame % self.track.loop_frames()) as f64;
        self.playing = true;
        Ok(())
    }
}

impl WorkletMusicRenderer {
    fn copy_channel(&self, output: &Float32Array, samples: &[f32]) -> Result<(), JsValue> {
        if output.length() as usize != self.rendered_frames {
            return Err(js_error(format!(
                "AudioWorklet output channel has {} frames; renderer produced {}",
                output.length(),
                self.rendered_frames
            )));
        }
        output.copy_from(&samples[..self.rendered_frames]);
        Ok(())
    }
}

fn js_error(error: impl Into<String>) -> JsValue {
    JsValue::from_str(&error.into())
}
