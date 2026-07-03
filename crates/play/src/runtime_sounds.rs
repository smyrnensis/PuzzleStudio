use puzzle_lang::SoundsDef;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RuntimeSoundsDef {
    pub sfx: Vec<RuntimeSfxSoundDef>,
    pub music: Vec<RuntimeMusicSoundDef>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RuntimeSfxSoundDef {
    pub name: String,
    pub seed: String,
    #[serde(rename = "type")]
    pub type_target: String,
    pub volume: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RuntimeMusicSoundDef {
    pub name: String,
    pub seed: String,
    pub height: f64,
    pub bars: u16,
    pub bpm: u16,
    pub volume: f64,
}

pub fn runtime_sounds_def(sounds: &SoundsDef) -> RuntimeSoundsDef {
    RuntimeSoundsDef {
        sfx: sounds
            .sfx
            .iter()
            .map(|sfx| RuntimeSfxSoundDef {
                name: sfx.name.clone(),
                seed: sfx.seed.clone(),
                type_target: sfx.type_target.clone(),
                volume: sfx.volume,
            })
            .collect(),
        music: sounds
            .music
            .iter()
            .map(|music| RuntimeMusicSoundDef {
                name: music.name.clone(),
                seed: music.seed.clone(),
                height: music.height,
                bars: music.bars,
                bpm: music.bpm,
                volume: music.volume,
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use puzzle_lang::{MusicSoundDef, SfxSoundDef};
    use serde_json::json;

    use super::*;

    #[test]
    fn runtime_sounds_contract_serializes_volume_and_browser_type_key() {
        let sounds = SoundsDef {
            sfx: vec![SfxSoundDef {
                name: "push".to_string(),
                seed: "push".to_string(),
                type_target: "hit".to_string(),
                volume: 1.25,
            }],
            music: vec![MusicSoundDef {
                name: "theme".to_string(),
                seed: "theme".to_string(),
                height: 0.7,
                bars: 2,
                bpm: 120,
                volume: 1.5,
            }],
        };

        let value = serde_json::to_value(runtime_sounds_def(&sounds))
            .expect("runtime sounds contract should serialize");

        assert_eq!(
            value,
            json!({
                "sfx": [{
                    "name": "push",
                    "seed": "push",
                    "type": "hit",
                    "volume": 1.25,
                }],
                "music": [{
                    "name": "theme",
                    "seed": "theme",
                    "height": 0.7,
                    "bars": 2,
                    "bpm": 120,
                    "volume": 1.5,
                }],
            })
        );
        assert!(value["sfx"][0].get("type_target").is_none());
    }
}
