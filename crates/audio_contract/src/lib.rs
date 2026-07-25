use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SfxAssetId(pub u16);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MusicAssetId(pub u16);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AudioVoiceId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MusicTarget {
    All,
    Asset(MusicAssetId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AudioCommand {
    PlaySfx { asset: SfxAssetId },
    PlayMusic { asset: MusicAssetId },
    PauseMusic { target: MusicTarget },
    ResumeMusic { target: MusicTarget },
    StopMusic { target: MusicTarget },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioCapabilityState {
    Locked,
    Ready,
    Suspended,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AudioDeviceCommand {
    StartSfx {
        voice: AudioVoiceId,
        asset: SfxAssetId,
        gain: f32,
    },
    StartMusic {
        voice: AudioVoiceId,
        asset: MusicAssetId,
        start_frame: u64,
        gain: f32,
    },
    PauseVoice {
        voice: AudioVoiceId,
        at_frame: u64,
    },
    ResumeVoice {
        voice: AudioVoiceId,
        at_frame: u64,
    },
    StopVoice {
        voice: AudioVoiceId,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioDeviceVoiceKind {
    Sfx(SfxAssetId),
    Music(MusicAssetId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioDeviceStateError {
    DuplicateVoice(AudioVoiceId),
    MissingVoice(AudioVoiceId),
    NonFiniteGain { voice: AudioVoiceId, gain_bits: u32 },
    NegativeGain { voice: AudioVoiceId, gain_bits: u32 },
}

/// Strict device-side projection of runtime-issued voice lifecycles.
///
/// The runtime owns replacement, pause, resume, and stop policy. Device
/// adapters use this registry only to reject command streams that do not match
/// the materialized device state. Callers must validate before touching the
/// device and commit only after that operation has been materialized.
#[derive(Clone, Debug, Default)]
pub struct AudioDeviceVoiceRegistry {
    voices: BTreeMap<AudioVoiceId, AudioDeviceVoiceKind>,
}

impl AudioDeviceVoiceRegistry {
    pub fn validate(&self, command: AudioDeviceCommand) -> Result<(), AudioDeviceStateError> {
        match command {
            AudioDeviceCommand::StartSfx { voice, gain, .. }
            | AudioDeviceCommand::StartMusic { voice, gain, .. } => {
                validate_device_gain(voice, gain)?;
                if self.voices.contains_key(&voice) {
                    return Err(AudioDeviceStateError::DuplicateVoice(voice));
                }
            }
            AudioDeviceCommand::PauseVoice { voice, .. }
            | AudioDeviceCommand::ResumeVoice { voice, .. }
            | AudioDeviceCommand::StopVoice { voice } => {
                if !self.voices.contains_key(&voice) {
                    return Err(AudioDeviceStateError::MissingVoice(voice));
                }
            }
        }
        Ok(())
    }

    pub fn commit(&mut self, command: AudioDeviceCommand) -> Result<(), AudioDeviceStateError> {
        self.validate(command)?;
        match command {
            AudioDeviceCommand::StartSfx { voice, asset, .. } => {
                self.voices.insert(voice, AudioDeviceVoiceKind::Sfx(asset));
            }
            AudioDeviceCommand::StartMusic { voice, asset, .. } => {
                self.voices
                    .insert(voice, AudioDeviceVoiceKind::Music(asset));
            }
            AudioDeviceCommand::StopVoice { voice } => {
                self.voices.remove(&voice);
            }
            AudioDeviceCommand::PauseVoice { .. } | AudioDeviceCommand::ResumeVoice { .. } => {}
        }
        Ok(())
    }

    pub fn voice_ended(&mut self, voice: AudioVoiceId) -> Option<AudioDeviceVoiceKind> {
        self.voices.remove(&voice)
    }

    pub fn voice(&self, voice: AudioVoiceId) -> Option<AudioDeviceVoiceKind> {
        self.voices.get(&voice).copied()
    }

    pub fn is_empty(&self) -> bool {
        self.voices.is_empty()
    }
}

fn validate_device_gain(voice: AudioVoiceId, gain: f32) -> Result<(), AudioDeviceStateError> {
    if !gain.is_finite() {
        return Err(AudioDeviceStateError::NonFiniteGain {
            voice,
            gain_bits: gain.to_bits(),
        });
    }
    if gain < 0.0 {
        return Err(AudioDeviceStateError::NegativeGain {
            voice,
            gain_bits: gain.to_bits(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_requires_an_explicit_runtime_lifecycle() {
        let voice = AudioVoiceId(4);
        let mut registry = AudioDeviceVoiceRegistry::default();
        assert!(registry.is_empty());
        let start = AudioDeviceCommand::StartSfx {
            voice,
            asset: SfxAssetId(2),
            gain: 0.75,
        };

        registry.validate(start).expect("new voice is valid");
        assert_eq!(registry.voice(voice), None, "validation must not commit");
        registry.commit(start).expect("materialized start commits");
        assert!(!registry.is_empty());
        assert_eq!(
            registry.voice(voice),
            Some(AudioDeviceVoiceKind::Sfx(SfxAssetId(2)))
        );
        assert_eq!(
            registry.validate(start),
            Err(AudioDeviceStateError::DuplicateVoice(voice))
        );

        let stop = AudioDeviceCommand::StopVoice { voice };
        registry.commit(stop).expect("known voice can stop");
        assert!(registry.is_empty());
        assert_eq!(
            registry.validate(stop),
            Err(AudioDeviceStateError::MissingVoice(voice))
        );
    }

    #[test]
    fn failed_validation_never_reserves_a_voice() {
        let voice = AudioVoiceId(9);
        let mut registry = AudioDeviceVoiceRegistry::default();
        assert!(matches!(
            registry.commit(AudioDeviceCommand::StartMusic {
                voice,
                asset: MusicAssetId(1),
                start_frame: 0,
                gain: f32::NAN,
            }),
            Err(AudioDeviceStateError::NonFiniteGain { .. })
        ));
        registry
            .commit(AudioDeviceCommand::StartMusic {
                voice,
                asset: MusicAssetId(1),
                start_frame: 0,
                gain: 1.0,
            })
            .expect("invalid gain did not reserve the voice");
    }

    #[test]
    fn natural_end_releases_exactly_one_materialized_voice() {
        let voice = AudioVoiceId(3);
        let mut registry = AudioDeviceVoiceRegistry::default();
        registry
            .commit(AudioDeviceCommand::StartSfx {
                voice,
                asset: SfxAssetId(7),
                gain: 1.0,
            })
            .unwrap();

        assert_eq!(
            registry.voice_ended(voice),
            Some(AudioDeviceVoiceKind::Sfx(SfxAssetId(7)))
        );
        assert_eq!(registry.voice_ended(voice), None);
    }
}
