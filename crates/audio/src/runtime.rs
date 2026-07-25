use std::{collections::BTreeMap, sync::Arc};

use crate::AudioAssetCatalog;
pub use puzzle_audio_contract::{
    AudioCapabilityState, AudioCommand, AudioDeviceCommand, AudioVoiceId, MusicAssetId,
    MusicTarget, SfxAssetId,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AudioDiagnostic {
    MissingSfxAsset { asset: SfxAssetId },
    MissingMusicAsset { asset: MusicAssetId },
    OutputUnavailable,
    DeviceFailure { error: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MusicPlaybackStatus {
    Playing,
    Paused,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MusicPlaybackSnapshot {
    pub asset: MusicAssetId,
    pub status: MusicPlaybackStatus,
    pub cursor_frame: u64,
}

#[derive(Clone, Debug)]
struct MusicPlayback {
    asset: MusicAssetId,
    status: MusicPlaybackStatus,
    cursor_frame: u64,
    clock_epoch: u64,
    voice: Option<AudioVoiceId>,
}

pub struct AudioRuntime {
    catalog: Arc<AudioAssetCatalog>,
    capability: AudioCapabilityState,
    next_voice: u64,
    active_sfx: BTreeMap<SfxAssetId, AudioVoiceId>,
    music: Option<MusicPlayback>,
    diagnostics: Vec<AudioDiagnostic>,
    unavailable_reported: bool,
}

impl AudioRuntime {
    pub fn new(catalog: Arc<AudioAssetCatalog>, capability: AudioCapabilityState) -> Self {
        Self {
            catalog,
            capability,
            next_voice: 1,
            active_sfx: BTreeMap::new(),
            music: None,
            diagnostics: Vec::new(),
            unavailable_reported: false,
        }
    }

    pub fn capability(&self) -> AudioCapabilityState {
        self.capability
    }

    pub fn catalog(&self) -> &Arc<AudioAssetCatalog> {
        &self.catalog
    }

    pub fn apply(&mut self, command: AudioCommand, now_frame: u64) -> Vec<AudioDeviceCommand> {
        match command {
            AudioCommand::PlaySfx { asset } => self.play_sfx(asset),
            AudioCommand::PlayMusic { asset } => self.play_music(asset, now_frame),
            AudioCommand::PauseMusic { target } => self.pause_music(target, now_frame),
            AudioCommand::ResumeMusic { target } => self.resume_music(target, now_frame),
            AudioCommand::StopMusic { target } => self.stop_music(target),
        }
    }

    pub fn set_capability(
        &mut self,
        capability: AudioCapabilityState,
        now_frame: u64,
    ) -> Vec<AudioDeviceCommand> {
        if capability == self.capability {
            return Vec::new();
        }

        self.freeze_music_cursor(now_frame);
        let previous = self.capability;
        self.capability = capability;
        match capability {
            AudioCapabilityState::Ready => {
                self.unavailable_reported = false;
                self.resume_for_ready(now_frame)
            }
            AudioCapabilityState::Locked | AudioCapabilityState::Suspended => {
                self.suspend_output(previous)
            }
            AudioCapabilityState::Unavailable => {
                let mut commands = self
                    .active_sfx
                    .values()
                    .copied()
                    .map(|voice| AudioDeviceCommand::StopVoice { voice })
                    .collect::<Vec<_>>();
                self.active_sfx.clear();
                if let Some(music) = &mut self.music {
                    if let Some(voice) = music.voice.take() {
                        commands.push(AudioDeviceCommand::StopVoice { voice });
                    }
                }
                self.report_unavailable_once();
                commands
            }
        }
    }

    pub fn report_device_failure(
        &mut self,
        error: impl Into<String>,
        now_frame: u64,
    ) -> Vec<AudioDeviceCommand> {
        self.freeze_music_cursor(now_frame);
        self.capability = AudioCapabilityState::Unavailable;
        let mut commands = self
            .active_sfx
            .values()
            .copied()
            .map(|voice| AudioDeviceCommand::StopVoice { voice })
            .collect::<Vec<_>>();
        self.active_sfx.clear();
        if let Some(music) = &mut self.music {
            if let Some(voice) = music.voice.take() {
                commands.push(AudioDeviceCommand::StopVoice { voice });
            }
        }
        self.diagnostics.push(AudioDiagnostic::DeviceFailure {
            error: error.into(),
        });
        self.unavailable_reported = true;
        commands
    }

    pub fn report_voice_failure(
        &mut self,
        voice: AudioVoiceId,
        error: impl Into<String>,
        now_frame: u64,
    ) {
        self.active_sfx.retain(|_, active| *active != voice);
        if self
            .music
            .as_ref()
            .is_some_and(|music| music.voice == Some(voice))
        {
            self.freeze_music_cursor(now_frame);
            self.music = None;
        }
        self.diagnostics.push(AudioDiagnostic::DeviceFailure {
            error: error.into(),
        });
    }

    pub fn voice_ended(&mut self, voice: AudioVoiceId) {
        self.active_sfx.retain(|_, active| *active != voice);
        if self
            .music
            .as_ref()
            .is_some_and(|music| music.voice == Some(voice))
        {
            if let Some(music) = &mut self.music {
                music.voice = None;
            }
        }
    }

    pub fn music_playback(&self, now_frame: u64) -> Option<MusicPlaybackSnapshot> {
        let music = self.music.as_ref()?;
        Some(MusicPlaybackSnapshot {
            asset: music.asset,
            status: music.status,
            cursor_frame: self.current_music_cursor(music, now_frame),
        })
    }

    pub fn take_diagnostics(&mut self) -> Vec<AudioDiagnostic> {
        std::mem::take(&mut self.diagnostics)
    }

    /// Seeks the currently desired music playback without inventing a clock
    /// offset or extending the serialized authoring command contract.
    pub fn seek_music(
        &mut self,
        target: MusicTarget,
        cursor_frame: u64,
        now_frame: u64,
    ) -> Vec<AudioDeviceCommand> {
        let Some(music) = &self.music else {
            return Vec::new();
        };
        if !target_matches(target, music.asset) {
            return Vec::new();
        }
        let loop_frames = self
            .catalog
            .music(music.asset)
            .map(|track| track.loop_frames())
            .unwrap_or(0);
        let cursor_frame = if loop_frames == 0 {
            cursor_frame
        } else {
            cursor_frame % loop_frames
        };
        let music = self.music.as_mut().expect("music was present above");
        music.cursor_frame = cursor_frame;
        music.clock_epoch = now_frame;

        if self.capability != AudioCapabilityState::Ready {
            return Vec::new();
        }
        match (music.status, music.voice) {
            (MusicPlaybackStatus::Playing, Some(voice)) => {
                vec![AudioDeviceCommand::ResumeVoice {
                    voice,
                    at_frame: cursor_frame,
                }]
            }
            (MusicPlaybackStatus::Paused, Some(voice)) => {
                vec![AudioDeviceCommand::PauseVoice {
                    voice,
                    at_frame: cursor_frame,
                }]
            }
            (MusicPlaybackStatus::Playing, None) => self.resume_for_ready(now_frame),
            (MusicPlaybackStatus::Paused, None) => Vec::new(),
        }
    }

    /// Stops every materialized voice and clears desired playback while
    /// preserving the actual device capability.
    ///
    /// This is a Rust host lifecycle reset for catalog reconfiguration, not an
    /// authored or serialized player command.
    pub fn stop_all(&mut self) -> Vec<AudioDeviceCommand> {
        let mut commands = self
            .active_sfx
            .values()
            .copied()
            .map(|voice| AudioDeviceCommand::StopVoice { voice })
            .collect::<Vec<_>>();
        self.active_sfx.clear();
        if let Some(voice) = self.music.take().and_then(|music| music.voice) {
            commands.push(AudioDeviceCommand::StopVoice { voice });
        }
        commands
    }

    fn play_sfx(&mut self, asset: SfxAssetId) -> Vec<AudioDeviceCommand> {
        let Some(gain) = self.catalog.sfx_gain(asset) else {
            self.diagnostics
                .push(AudioDiagnostic::MissingSfxAsset { asset });
            return Vec::new();
        };
        match self.capability {
            AudioCapabilityState::Ready => {}
            AudioCapabilityState::Locked | AudioCapabilityState::Suspended => {
                return Vec::new();
            }
            AudioCapabilityState::Unavailable => {
                self.report_unavailable_once();
                return Vec::new();
            }
        }

        let mut commands = Vec::with_capacity(2);
        if let Some(previous) = self.active_sfx.remove(&asset) {
            commands.push(AudioDeviceCommand::StopVoice { voice: previous });
        }
        let voice = self.allocate_voice();
        self.active_sfx.insert(asset, voice);
        commands.push(AudioDeviceCommand::StartSfx { voice, asset, gain });
        commands
    }

    fn play_music(&mut self, asset: MusicAssetId, now_frame: u64) -> Vec<AudioDeviceCommand> {
        let Some(gain) = self.catalog.music_gain(asset) else {
            self.diagnostics
                .push(AudioDiagnostic::MissingMusicAsset { asset });
            return Vec::new();
        };
        if self.music.as_ref().is_some_and(|music| {
            music.asset == asset && music.status == MusicPlaybackStatus::Playing
        }) {
            return Vec::new();
        }

        let mut commands = Vec::with_capacity(2);
        if let Some(previous) = self.music.take()
            && let Some(voice) = previous.voice
        {
            commands.push(AudioDeviceCommand::StopVoice { voice });
        }

        let voice = (self.capability == AudioCapabilityState::Ready).then(|| self.allocate_voice());
        self.music = Some(MusicPlayback {
            asset,
            status: MusicPlaybackStatus::Playing,
            cursor_frame: 0,
            clock_epoch: now_frame,
            voice,
        });
        if let Some(voice) = voice {
            commands.push(AudioDeviceCommand::StartMusic {
                voice,
                asset,
                start_frame: 0,
                gain,
            });
        } else if self.capability == AudioCapabilityState::Unavailable {
            self.report_unavailable_once();
        }
        commands
    }

    fn pause_music(&mut self, target: MusicTarget, now_frame: u64) -> Vec<AudioDeviceCommand> {
        let Some(music) = &self.music else {
            return Vec::new();
        };
        if music.status == MusicPlaybackStatus::Paused || !target_matches(target, music.asset) {
            return Vec::new();
        }
        self.freeze_music_cursor(now_frame);
        let music = self.music.as_mut().expect("music was present above");
        music.status = MusicPlaybackStatus::Paused;
        music
            .voice
            .map(|voice| AudioDeviceCommand::PauseVoice {
                voice,
                at_frame: music.cursor_frame,
            })
            .into_iter()
            .collect()
    }

    fn resume_music(&mut self, target: MusicTarget, now_frame: u64) -> Vec<AudioDeviceCommand> {
        let Some(music) = &self.music else {
            return Vec::new();
        };
        if music.status == MusicPlaybackStatus::Playing || !target_matches(target, music.asset) {
            return Vec::new();
        }

        let asset = music.asset;
        let cursor = music.cursor_frame;
        let Some(gain) = self.catalog.music_gain(asset) else {
            self.diagnostics
                .push(AudioDiagnostic::MissingMusicAsset { asset });
            return Vec::new();
        };
        let needs_voice = music.voice.is_none() && self.capability == AudioCapabilityState::Ready;
        let new_voice = needs_voice.then(|| self.allocate_voice());
        let music = self.music.as_mut().expect("music was present above");
        music.status = MusicPlaybackStatus::Playing;
        music.clock_epoch = now_frame;
        if let Some(voice) = new_voice {
            music.voice = Some(voice);
            return vec![AudioDeviceCommand::StartMusic {
                voice,
                asset,
                start_frame: cursor,
                gain,
            }];
        }
        if self.capability == AudioCapabilityState::Ready {
            return music
                .voice
                .map(|voice| AudioDeviceCommand::ResumeVoice {
                    voice,
                    at_frame: cursor,
                })
                .into_iter()
                .collect();
        }
        if self.capability == AudioCapabilityState::Unavailable {
            self.report_unavailable_once();
        }
        Vec::new()
    }

    fn stop_music(&mut self, target: MusicTarget) -> Vec<AudioDeviceCommand> {
        let Some(music) = &self.music else {
            return Vec::new();
        };
        if !target_matches(target, music.asset) {
            return Vec::new();
        }
        self.music
            .take()
            .and_then(|music| music.voice)
            .map(|voice| AudioDeviceCommand::StopVoice { voice })
            .into_iter()
            .collect()
    }

    fn suspend_output(&mut self, previous: AudioCapabilityState) -> Vec<AudioDeviceCommand> {
        let mut commands = Vec::new();
        if previous == AudioCapabilityState::Ready {
            commands.extend(
                self.active_sfx
                    .values()
                    .copied()
                    .map(|voice| AudioDeviceCommand::StopVoice { voice }),
            );
            if let Some(music) = &self.music
                && music.status == MusicPlaybackStatus::Playing
                && let Some(voice) = music.voice
            {
                commands.push(AudioDeviceCommand::PauseVoice {
                    voice,
                    at_frame: music.cursor_frame,
                });
            }
        }
        self.active_sfx.clear();
        commands
    }

    fn resume_for_ready(&mut self, now_frame: u64) -> Vec<AudioDeviceCommand> {
        let Some(music) = &self.music else {
            return Vec::new();
        };
        if music.status == MusicPlaybackStatus::Paused {
            return Vec::new();
        }
        let asset = music.asset;
        let cursor = music.cursor_frame;
        let gain = self
            .catalog
            .music_gain(asset)
            .expect("catalog-backed music playback must keep a valid asset");
        let existing_voice = music.voice;
        if let Some(voice) = existing_voice {
            if let Some(music) = &mut self.music {
                music.clock_epoch = now_frame;
            }
            return vec![AudioDeviceCommand::ResumeVoice {
                voice,
                at_frame: cursor,
            }];
        }

        let voice = self.allocate_voice();
        if let Some(music) = &mut self.music {
            music.voice = Some(voice);
            music.clock_epoch = now_frame;
        }
        vec![AudioDeviceCommand::StartMusic {
            voice,
            asset,
            start_frame: cursor,
            gain,
        }]
    }

    fn freeze_music_cursor(&mut self, now_frame: u64) {
        let Some(music) = &self.music else {
            return;
        };
        let cursor = self.current_music_cursor(music, now_frame);
        if let Some(music) = &mut self.music {
            music.cursor_frame = cursor;
            music.clock_epoch = now_frame;
        }
    }

    fn current_music_cursor(&self, music: &MusicPlayback, now_frame: u64) -> u64 {
        let advances = music.status == MusicPlaybackStatus::Playing
            && self.capability == AudioCapabilityState::Ready
            && music.voice.is_some();
        let cursor = music.cursor_frame.saturating_add(
            advances
                .then(|| now_frame.saturating_sub(music.clock_epoch))
                .unwrap_or(0),
        );
        let loop_frames = self
            .catalog
            .music(music.asset)
            .map(|track| track.loop_frames())
            .unwrap_or(0);
        if loop_frames == 0 {
            cursor
        } else {
            cursor % loop_frames
        }
    }

    fn report_unavailable_once(&mut self) {
        if !self.unavailable_reported {
            self.diagnostics.push(AudioDiagnostic::OutputUnavailable);
            self.unavailable_reported = true;
        }
    }

    fn allocate_voice(&mut self) -> AudioVoiceId {
        let voice = AudioVoiceId(self.next_voice);
        self.next_voice = self
            .next_voice
            .checked_add(1)
            .expect("audio voice identifier space exhausted");
        voice
    }
}

fn target_matches(target: MusicTarget, asset: MusicAssetId) -> bool {
    match target {
        MusicTarget::All => true,
        MusicTarget::Asset(target) => target == asset,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runtime(capability: AudioCapabilityState) -> AudioRuntime {
        AudioRuntime::new(
            Arc::new(AudioAssetCatalog::for_runtime_tests(
                &[0.25, 0.75],
                &[(0.5, 1_000), (1.25, 2_000)],
            )),
            capability,
        )
    }

    #[test]
    fn same_sfx_replaces_its_voice_but_different_sfx_does_not() {
        let mut runtime = runtime(AudioCapabilityState::Ready);

        assert_eq!(
            runtime.apply(
                AudioCommand::PlaySfx {
                    asset: SfxAssetId(0),
                },
                0,
            ),
            vec![AudioDeviceCommand::StartSfx {
                voice: AudioVoiceId(1),
                asset: SfxAssetId(0),
                gain: 0.25,
            }]
        );
        assert_eq!(
            runtime.apply(
                AudioCommand::PlaySfx {
                    asset: SfxAssetId(1),
                },
                1,
            ),
            vec![AudioDeviceCommand::StartSfx {
                voice: AudioVoiceId(2),
                asset: SfxAssetId(1),
                gain: 0.75,
            }]
        );
        assert_eq!(
            runtime.apply(
                AudioCommand::PlaySfx {
                    asset: SfxAssetId(0),
                },
                2,
            ),
            vec![
                AudioDeviceCommand::StopVoice {
                    voice: AudioVoiceId(1),
                },
                AudioDeviceCommand::StartSfx {
                    voice: AudioVoiceId(3),
                    asset: SfxAssetId(0),
                    gain: 0.25,
                },
            ]
        );
    }

    #[test]
    fn locked_and_suspended_sfx_are_dropped_instead_of_replayed() {
        let mut runtime = runtime(AudioCapabilityState::Locked);
        assert!(
            runtime
                .apply(
                    AudioCommand::PlaySfx {
                        asset: SfxAssetId(0),
                    },
                    10,
                )
                .is_empty()
        );
        assert!(
            runtime
                .set_capability(AudioCapabilityState::Ready, 20)
                .is_empty()
        );

        assert!(
            runtime
                .set_capability(AudioCapabilityState::Suspended, 30)
                .is_empty()
        );
        assert!(
            runtime
                .apply(
                    AudioCommand::PlaySfx {
                        asset: SfxAssetId(0),
                    },
                    31,
                )
                .is_empty()
        );
        assert!(
            runtime
                .set_capability(AudioCapabilityState::Ready, 40)
                .is_empty()
        );
    }

    #[test]
    fn same_active_music_is_a_no_op_and_different_music_replaces_it() {
        let mut runtime = runtime(AudioCapabilityState::Ready);
        let first = runtime.apply(
            AudioCommand::PlayMusic {
                asset: MusicAssetId(0),
            },
            100,
        );
        assert_eq!(
            first,
            vec![AudioDeviceCommand::StartMusic {
                voice: AudioVoiceId(1),
                asset: MusicAssetId(0),
                start_frame: 0,
                gain: 0.5,
            }]
        );
        assert!(
            runtime
                .apply(
                    AudioCommand::PlayMusic {
                        asset: MusicAssetId(0),
                    },
                    150,
                )
                .is_empty()
        );
        assert_eq!(
            runtime.apply(
                AudioCommand::PlayMusic {
                    asset: MusicAssetId(1),
                },
                160,
            ),
            vec![
                AudioDeviceCommand::StopVoice {
                    voice: AudioVoiceId(1),
                },
                AudioDeviceCommand::StartMusic {
                    voice: AudioVoiceId(2),
                    asset: MusicAssetId(1),
                    start_frame: 0,
                    gain: 1.25,
                },
            ]
        );
    }

    #[test]
    fn named_music_operations_ignore_a_different_asset() {
        let mut runtime = runtime(AudioCapabilityState::Ready);
        runtime.apply(
            AudioCommand::PlayMusic {
                asset: MusicAssetId(0),
            },
            0,
        );

        for command in [
            AudioCommand::PauseMusic {
                target: MusicTarget::Asset(MusicAssetId(1)),
            },
            AudioCommand::ResumeMusic {
                target: MusicTarget::Asset(MusicAssetId(1)),
            },
            AudioCommand::StopMusic {
                target: MusicTarget::Asset(MusicAssetId(1)),
            },
        ] {
            assert!(runtime.apply(command, 100).is_empty());
        }
        assert_eq!(
            runtime.music_playback(100),
            Some(MusicPlaybackSnapshot {
                asset: MusicAssetId(0),
                status: MusicPlaybackStatus::Playing,
                cursor_frame: 100,
            })
        );
    }

    #[test]
    fn pause_resume_and_stop_all_preserve_the_exact_sample_cursor() {
        let mut runtime = runtime(AudioCapabilityState::Ready);
        runtime.apply(
            AudioCommand::PlayMusic {
                asset: MusicAssetId(0),
            },
            100,
        );
        assert_eq!(
            runtime.apply(
                AudioCommand::PauseMusic {
                    target: MusicTarget::All,
                },
                350,
            ),
            vec![AudioDeviceCommand::PauseVoice {
                voice: AudioVoiceId(1),
                at_frame: 250,
            }]
        );
        assert_eq!(
            runtime.music_playback(900).unwrap().cursor_frame,
            250,
            "an authored pause freezes the logical cursor"
        );
        assert_eq!(
            runtime.apply(
                AudioCommand::ResumeMusic {
                    target: MusicTarget::All,
                },
                900,
            ),
            vec![AudioDeviceCommand::ResumeVoice {
                voice: AudioVoiceId(1),
                at_frame: 250,
            }]
        );
        assert_eq!(runtime.music_playback(1_000).unwrap().cursor_frame, 350);
        assert_eq!(
            runtime.apply(
                AudioCommand::StopMusic {
                    target: MusicTarget::All,
                },
                1_000,
            ),
            vec![AudioDeviceCommand::StopVoice {
                voice: AudioVoiceId(1),
            }]
        );
        assert_eq!(runtime.music_playback(1_000), None);
    }

    #[test]
    fn locked_music_is_desired_and_starts_from_its_frozen_cursor_on_unlock() {
        let mut runtime = runtime(AudioCapabilityState::Locked);
        assert!(
            runtime
                .apply(
                    AudioCommand::PlayMusic {
                        asset: MusicAssetId(0),
                    },
                    100,
                )
                .is_empty()
        );
        assert_eq!(runtime.music_playback(600).unwrap().cursor_frame, 0);
        assert_eq!(
            runtime.set_capability(AudioCapabilityState::Ready, 600),
            vec![AudioDeviceCommand::StartMusic {
                voice: AudioVoiceId(1),
                asset: MusicAssetId(0),
                start_frame: 0,
                gain: 0.5,
            }]
        );
        assert_eq!(runtime.music_playback(700).unwrap().cursor_frame, 100);
    }

    #[test]
    fn capability_suspend_freezes_and_reconciles_music_without_overriding_author_pause() {
        let mut runtime = runtime(AudioCapabilityState::Ready);
        runtime.apply(
            AudioCommand::PlayMusic {
                asset: MusicAssetId(0),
            },
            100,
        );
        assert_eq!(
            runtime.set_capability(AudioCapabilityState::Suspended, 400),
            vec![AudioDeviceCommand::PauseVoice {
                voice: AudioVoiceId(1),
                at_frame: 300,
            }]
        );
        assert_eq!(runtime.music_playback(900).unwrap().cursor_frame, 300);
        assert_eq!(
            runtime.set_capability(AudioCapabilityState::Ready, 900),
            vec![AudioDeviceCommand::ResumeVoice {
                voice: AudioVoiceId(1),
                at_frame: 300,
            }]
        );

        runtime.apply(
            AudioCommand::PauseMusic {
                target: MusicTarget::All,
            },
            950,
        );
        runtime.set_capability(AudioCapabilityState::Suspended, 1_000);
        assert!(
            runtime
                .set_capability(AudioCapabilityState::Ready, 2_000)
                .is_empty()
        );
        assert_eq!(
            runtime.music_playback(3_000).unwrap().status,
            MusicPlaybackStatus::Paused
        );
    }

    #[test]
    fn music_cursor_wraps_in_canonical_sample_frames() {
        let mut runtime = runtime(AudioCapabilityState::Ready);
        runtime.apply(
            AudioCommand::PlayMusic {
                asset: MusicAssetId(0),
            },
            10,
        );
        assert_eq!(runtime.music_playback(1_260).unwrap().cursor_frame, 250);
    }

    #[test]
    fn unavailable_output_reports_once_and_retains_desired_music() {
        let mut runtime = runtime(AudioCapabilityState::Unavailable);
        assert!(
            runtime
                .apply(
                    AudioCommand::PlaySfx {
                        asset: SfxAssetId(0),
                    },
                    0,
                )
                .is_empty()
        );
        assert!(
            runtime
                .apply(
                    AudioCommand::PlayMusic {
                        asset: MusicAssetId(0),
                    },
                    0,
                )
                .is_empty()
        );
        assert_eq!(
            runtime.take_diagnostics(),
            vec![AudioDiagnostic::OutputUnavailable]
        );
        assert_eq!(runtime.music_playback(500).unwrap().cursor_frame, 0);
        assert_eq!(
            runtime.set_capability(AudioCapabilityState::Ready, 500),
            vec![AudioDeviceCommand::StartMusic {
                voice: AudioVoiceId(1),
                asset: MusicAssetId(0),
                start_frame: 0,
                gain: 0.5,
            }]
        );
    }

    #[test]
    fn missing_assets_and_device_failures_are_diagnostics_not_panics() {
        let mut runtime = runtime(AudioCapabilityState::Ready);
        assert!(
            runtime
                .apply(
                    AudioCommand::PlaySfx {
                        asset: SfxAssetId(99),
                    },
                    0,
                )
                .is_empty()
        );
        assert!(
            runtime
                .apply(
                    AudioCommand::PlayMusic {
                        asset: MusicAssetId(99),
                    },
                    0,
                )
                .is_empty()
        );
        assert!(
            runtime
                .report_device_failure("device disconnected", 10)
                .is_empty()
        );

        assert_eq!(
            runtime.take_diagnostics(),
            vec![
                AudioDiagnostic::MissingSfxAsset {
                    asset: SfxAssetId(99),
                },
                AudioDiagnostic::MissingMusicAsset {
                    asset: MusicAssetId(99),
                },
                AudioDiagnostic::DeviceFailure {
                    error: "device disconnected".to_string(),
                },
            ]
        );
        assert_eq!(runtime.capability(), AudioCapabilityState::Unavailable);
    }

    #[test]
    fn device_failure_stops_materialized_voices_and_preserves_music_intent() {
        let mut runtime = runtime(AudioCapabilityState::Ready);
        runtime.apply(
            AudioCommand::PlaySfx {
                asset: SfxAssetId(0),
            },
            0,
        );
        runtime.apply(
            AudioCommand::PlayMusic {
                asset: MusicAssetId(0),
            },
            100,
        );

        assert_eq!(
            runtime.report_device_failure("output lost", 350),
            [
                AudioDeviceCommand::StopVoice {
                    voice: AudioVoiceId(1),
                },
                AudioDeviceCommand::StopVoice {
                    voice: AudioVoiceId(2),
                },
            ]
        );
        assert_eq!(
            runtime.music_playback(900),
            Some(MusicPlaybackSnapshot {
                asset: MusicAssetId(0),
                status: MusicPlaybackStatus::Playing,
                cursor_frame: 250,
            })
        );
        assert_eq!(
            runtime.set_capability(AudioCapabilityState::Ready, 900),
            [AudioDeviceCommand::StartMusic {
                voice: AudioVoiceId(3),
                asset: MusicAssetId(0),
                start_frame: 250,
                gain: 0.5,
            }]
        );
    }

    #[test]
    fn one_voice_failure_does_not_make_independent_audio_unavailable() {
        let mut runtime = runtime(AudioCapabilityState::Ready);
        runtime.apply(
            AudioCommand::PlaySfx {
                asset: SfxAssetId(0),
            },
            0,
        );
        runtime.report_voice_failure(AudioVoiceId(1), "voice rejected", 10);

        assert_eq!(runtime.capability(), AudioCapabilityState::Ready);
        assert_eq!(
            runtime.apply(
                AudioCommand::PlaySfx {
                    asset: SfxAssetId(0),
                },
                20,
            ),
            [AudioDeviceCommand::StartSfx {
                voice: AudioVoiceId(2),
                asset: SfxAssetId(0),
                gain: 0.25,
            }]
        );
    }

    #[test]
    fn failed_music_attempt_is_not_rematerialized_by_later_capability_cycles() {
        let mut runtime = runtime(AudioCapabilityState::Ready);
        runtime.apply(
            AudioCommand::PlayMusic {
                asset: MusicAssetId(0),
            },
            100,
        );

        runtime.report_voice_failure(AudioVoiceId(1), "worklet rejected voice", 350);
        assert_eq!(runtime.music_playback(400), None);
        assert!(
            runtime
                .set_capability(AudioCapabilityState::Suspended, 400)
                .is_empty()
        );
        assert!(
            runtime
                .set_capability(AudioCapabilityState::Ready, 500)
                .is_empty()
        );

        assert_eq!(
            runtime.apply(
                AudioCommand::PlayMusic {
                    asset: MusicAssetId(0),
                },
                600,
            ),
            [AudioDeviceCommand::StartMusic {
                voice: AudioVoiceId(2),
                asset: MusicAssetId(0),
                start_frame: 0,
                gain: 0.5,
            }]
        );
    }

    #[test]
    fn seek_music_uses_an_explicit_canonical_cursor_for_playing_and_paused_music() {
        let mut runtime = runtime(AudioCapabilityState::Ready);
        runtime.apply(
            AudioCommand::PlayMusic {
                asset: MusicAssetId(0),
            },
            100,
        );

        assert_eq!(
            runtime.seek_music(MusicTarget::Asset(MusicAssetId(0)), 800, 200),
            [AudioDeviceCommand::ResumeVoice {
                voice: AudioVoiceId(1),
                at_frame: 800,
            }]
        );
        assert_eq!(
            runtime
                .music_playback(250)
                .map(|playback| playback.cursor_frame),
            Some(850)
        );

        runtime.apply(
            AudioCommand::PauseMusic {
                target: MusicTarget::All,
            },
            250,
        );
        assert_eq!(
            runtime.seek_music(MusicTarget::All, 1_200, 300),
            [AudioDeviceCommand::PauseVoice {
                voice: AudioVoiceId(1),
                at_frame: 200,
            }]
        );
        assert_eq!(
            runtime
                .music_playback(900)
                .map(|playback| playback.cursor_frame),
            Some(200)
        );
        assert!(
            runtime
                .seek_music(MusicTarget::Asset(MusicAssetId(9)), 0, 900)
                .is_empty()
        );
    }

    #[test]
    fn stop_all_clears_materialized_and_desired_playback_without_falsifying_capability() {
        let mut runtime = runtime(AudioCapabilityState::Ready);
        runtime.apply(
            AudioCommand::PlaySfx {
                asset: SfxAssetId(0),
            },
            0,
        );
        runtime.apply(
            AudioCommand::PlayMusic {
                asset: MusicAssetId(0),
            },
            10,
        );

        assert_eq!(
            runtime.stop_all(),
            [
                AudioDeviceCommand::StopVoice {
                    voice: AudioVoiceId(1),
                },
                AudioDeviceCommand::StopVoice {
                    voice: AudioVoiceId(2),
                },
            ]
        );
        assert_eq!(runtime.capability(), AudioCapabilityState::Ready);
        assert_eq!(runtime.music_playback(1_000), None);
        assert!(runtime.take_diagnostics().is_empty());
    }

    #[test]
    fn audio_command_wire_rejects_raw_names_and_authoring_recipe_fields() {
        let error = serde_json::from_value::<AudioCommand>(serde_json::json!({
            "kind": "play_sfx",
            "asset": 0,
            "name": "push",
            "seed": "123456",
            "type": "hit",
        }))
        .expect_err("device command contract must not accept authored sound recipes");

        assert!(error.to_string().contains("unknown field"));
    }
}
