use std::{collections::BTreeMap, error::Error, fmt};

use crate::{
    GeneratedMusicTrack, GeneratedSfxClip, MusicRecipe, SfxRecipe, generate_music, generate_sfx,
    render_sfx,
};
pub use puzzle_audio_contract::{MusicAssetId, SfxAssetId};

pub const CANONICAL_AUDIO_SAMPLE_RATE: u32 = 48_000;

#[derive(Clone, Debug, PartialEq)]
pub enum AudioCatalogError {
    TooManySfxAssets { count: usize },
    TooManyMusicAssets { count: usize },
    DuplicateSfxName { name: String },
    DuplicateMusicName { name: String },
    InvalidSfxGain { name: String, gain: f64 },
    InvalidMusicGain { name: String, gain: f64 },
    SfxGeneration { name: String, error: String },
    MusicGeneration { name: String, error: String },
    InvalidSfxSampleRate { name: String, sample_rate: u32 },
    InvalidMusicSampleRate { name: String, sample_rate: u32 },
}

impl fmt::Display for AudioCatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManySfxAssets { count } => {
                write!(
                    formatter,
                    "audio catalog has {count} SFX assets; at most 65536 are supported"
                )
            }
            Self::TooManyMusicAssets { count } => write!(
                formatter,
                "audio catalog has {count} music assets; at most 65536 are supported"
            ),
            Self::DuplicateSfxName { name } => {
                write!(formatter, "duplicate SFX asset name `{name}`")
            }
            Self::DuplicateMusicName { name } => {
                write!(formatter, "duplicate music asset name `{name}`")
            }
            Self::InvalidSfxGain { name, gain } => {
                write!(formatter, "SFX asset `{name}` has invalid gain {gain}")
            }
            Self::InvalidMusicGain { name, gain } => {
                write!(formatter, "music asset `{name}` has invalid gain {gain}")
            }
            Self::SfxGeneration { name, error } => {
                write!(
                    formatter,
                    "SFX asset `{name}` could not be generated: {error}"
                )
            }
            Self::MusicGeneration { name, error } => {
                write!(
                    formatter,
                    "music asset `{name}` could not be generated: {error}"
                )
            }
            Self::InvalidSfxSampleRate { name, sample_rate } => write!(
                formatter,
                "SFX asset `{name}` was generated at {sample_rate} Hz instead of {CANONICAL_AUDIO_SAMPLE_RATE} Hz"
            ),
            Self::InvalidMusicSampleRate { name, sample_rate } => write!(
                formatter,
                "music asset `{name}` was generated at {sample_rate} Hz instead of {CANONICAL_AUDIO_SAMPLE_RATE} Hz"
            ),
        }
    }
}

impl Error for AudioCatalogError {}

#[derive(Clone, Debug)]
pub struct AudioAssetCatalog {
    sfx_names: BTreeMap<String, SfxAssetId>,
    music_names: BTreeMap<String, MusicAssetId>,
    sfx: Vec<GeneratedSfxClip>,
    music: Vec<GeneratedMusicTrack>,
    sfx_gains: Vec<f32>,
    music_gains: Vec<f32>,
}

impl AudioAssetCatalog {
    pub fn compile(
        sfx_recipes: Vec<(String, SfxRecipe)>,
        music_recipes: Vec<(String, MusicRecipe)>,
    ) -> Result<Self, AudioCatalogError> {
        if sfx_recipes.len() > usize::from(u16::MAX) + 1 {
            return Err(AudioCatalogError::TooManySfxAssets {
                count: sfx_recipes.len(),
            });
        }
        if music_recipes.len() > usize::from(u16::MAX) + 1 {
            return Err(AudioCatalogError::TooManyMusicAssets {
                count: music_recipes.len(),
            });
        }

        let mut sfx_names = BTreeMap::new();
        let mut sfx = Vec::with_capacity(sfx_recipes.len());
        let mut sfx_gains = Vec::with_capacity(sfx_recipes.len());
        for (index, (name, recipe)) in sfx_recipes.into_iter().enumerate() {
            let id = SfxAssetId(index as u16);
            if sfx_names.insert(name.clone(), id).is_some() {
                return Err(AudioCatalogError::DuplicateSfxName { name });
            }
            let gain = resolved_gain(&name, recipe.volume, true)?;
            let synthesis_recipe = SfxRecipe {
                volume: 1.0,
                ..recipe
            };
            let generated = generate_sfx(&synthesis_recipe).map_err(|error| {
                AudioCatalogError::SfxGeneration {
                    name: name.clone(),
                    error,
                }
            })?;
            let clip =
                render_sfx(&generated).map_err(|error| AudioCatalogError::SfxGeneration {
                    name: name.clone(),
                    error,
                })?;
            if clip.sample_rate != CANONICAL_AUDIO_SAMPLE_RATE {
                return Err(AudioCatalogError::InvalidSfxSampleRate {
                    name,
                    sample_rate: clip.sample_rate,
                });
            }
            sfx.push(clip);
            sfx_gains.push(gain);
        }

        let mut music_names = BTreeMap::new();
        let mut music = Vec::with_capacity(music_recipes.len());
        let mut music_gains = Vec::with_capacity(music_recipes.len());
        for (index, (name, recipe)) in music_recipes.into_iter().enumerate() {
            let id = MusicAssetId(index as u16);
            if music_names.insert(name.clone(), id).is_some() {
                return Err(AudioCatalogError::DuplicateMusicName { name });
            }
            let gain = resolved_gain(&name, recipe.volume, false)?;
            let synthesis_recipe = MusicRecipe {
                volume: 1.0,
                ..recipe
            };
            let track = generate_music(&synthesis_recipe).map_err(|error| {
                AudioCatalogError::MusicGeneration {
                    name: name.clone(),
                    error,
                }
            })?;
            if track.sample_rate() != CANONICAL_AUDIO_SAMPLE_RATE {
                return Err(AudioCatalogError::InvalidMusicSampleRate {
                    name,
                    sample_rate: track.sample_rate(),
                });
            }
            music.push(track);
            music_gains.push(gain);
        }

        Ok(Self {
            sfx_names,
            music_names,
            sfx,
            music,
            sfx_gains,
            music_gains,
        })
    }

    pub fn resolve_sfx(&self, name: &str) -> Option<SfxAssetId> {
        self.sfx_names.get(name).copied()
    }

    pub fn resolve_music(&self, name: &str) -> Option<MusicAssetId> {
        self.music_names.get(name).copied()
    }

    pub fn sfx(&self, id: SfxAssetId) -> Option<&GeneratedSfxClip> {
        self.sfx.get(usize::from(id.0))
    }

    pub fn music(&self, id: MusicAssetId) -> Option<&GeneratedMusicTrack> {
        self.music.get(usize::from(id.0))
    }

    pub fn sfx_gain(&self, id: SfxAssetId) -> Option<f32> {
        self.sfx_gains.get(usize::from(id.0)).copied()
    }

    pub fn music_gain(&self, id: MusicAssetId) -> Option<f32> {
        self.music_gains.get(usize::from(id.0)).copied()
    }

    pub fn sfx_len(&self) -> usize {
        self.sfx.len()
    }

    pub fn music_len(&self) -> usize {
        self.music.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sfx.is_empty() && self.music.is_empty()
    }

    #[cfg(test)]
    pub(crate) fn for_runtime_tests(sfx_gains: &[f32], music: &[(f32, u64)]) -> Self {
        use std::sync::Arc;

        use crate::{GeneratedMusicTrack, MusicScore};

        Self {
            sfx_names: BTreeMap::new(),
            music_names: BTreeMap::new(),
            sfx: sfx_gains
                .iter()
                .map(|_| GeneratedSfxClip {
                    sample_rate: CANONICAL_AUDIO_SAMPLE_RATE,
                    samples: Arc::from([0.0_f32]),
                })
                .collect(),
            music: music
                .iter()
                .map(|(_, loop_frames)| {
                    GeneratedMusicTrack::from_resolved_score(
                        CANONICAL_AUDIO_SAMPLE_RATE,
                        2,
                        *loop_frames,
                        MusicScore::default(),
                    )
                    .expect("runtime test tracks have addressable loop lengths")
                })
                .collect(),
            sfx_gains: sfx_gains.to_vec(),
            music_gains: music.iter().map(|(gain, _)| *gain).collect(),
        }
    }
}

fn resolved_gain(name: &str, gain: f64, sfx: bool) -> Result<f32, AudioCatalogError> {
    if !gain.is_finite() || gain < 0.0 || gain > f64::from(f32::MAX) {
        return Err(if sfx {
            AudioCatalogError::InvalidSfxGain {
                name: name.to_string(),
                gain,
            }
        } else {
            AudioCatalogError::InvalidMusicGain {
                name: name.to_string(),
                gain,
            }
        });
    }
    Ok(gain as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_catalog_has_no_resolvable_assets() {
        let catalog = AudioAssetCatalog::compile(Vec::new(), Vec::new()).unwrap();

        assert!(catalog.is_empty());
        assert_eq!(catalog.resolve_sfx("missing"), None);
        assert_eq!(catalog.resolve_music("missing"), None);
        assert_eq!(catalog.sfx(SfxAssetId(0)), None);
        assert_eq!(catalog.music(MusicAssetId(0)), None);
    }

    #[test]
    fn gain_validation_is_explicit() {
        let error = AudioAssetCatalog::compile(
            vec![(
                "bad".to_string(),
                SfxRecipe {
                    seed: "seed".to_string(),
                    type_target: "random".to_string(),
                    volume: f64::NAN,
                },
            )],
            Vec::new(),
        )
        .unwrap_err();

        assert!(matches!(
            error,
            AudioCatalogError::InvalidSfxGain { name, gain }
                if name == "bad" && gain.is_nan()
        ));
    }

    #[test]
    fn authored_sfx_volume_is_a_device_gain_not_a_second_synthesis_gain() {
        let recipe = SfxRecipe {
            seed: "volume-boundary".to_string(),
            type_target: "select".to_string(),
            volume: 0.25,
        };
        let catalog =
            AudioAssetCatalog::compile(vec![("click".to_string(), recipe.clone())], Vec::new())
                .unwrap();
        let unit_clip = render_sfx(
            &generate_sfx(&SfxRecipe {
                volume: 1.0,
                ..recipe
            })
            .unwrap(),
        )
        .unwrap();

        assert_eq!(catalog.sfx(SfxAssetId(0)), Some(&unit_clip));
        assert_eq!(catalog.sfx_gain(SfxAssetId(0)), Some(0.25));
    }
}
