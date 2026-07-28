use std::collections::HashSet;

use crate::{
    DiagnosticReport, LoadedDocument, LoadedDocumentModel, LoadedGridGame, RuleEffect,
    RuntimeEffect, SceneComponent, SceneDef, SceneEffect, SoundsDef,
};
use puzzle_core::GridSize;

pub fn validate_loaded_document_sound_references(
    document: &LoadedDocument,
) -> Result<(), DiagnosticReport> {
    let catalog = SoundCatalog::new(&document.sounds);
    for scene in &document.scenes {
        validate_scene(scene, &catalog)?;
    }
    for model in &document.models {
        let game_sounds = match model {
            LoadedDocumentModel::Puzzle2d { game, .. } => &game.sounds,
            LoadedDocumentModel::Puzzle3d { game, .. } => &game.sounds,
        };
        if game_sounds != &document.sounds {
            return Err(DiagnosticReport::error(
                "typed document model sound catalog does not match the document sound catalog",
            ));
        }
        match model {
            LoadedDocumentModel::Puzzle2d { game, .. } => {
                for scene in &game.scenes {
                    validate_scene(scene, &catalog)?;
                }
                validate_model(game, &catalog)?;
            }
            LoadedDocumentModel::Puzzle3d { game, .. } => {
                for scene in &game.scenes {
                    validate_scene(scene, &catalog)?;
                }
                validate_model(game, &catalog)?;
            }
        }
    }
    Ok(())
}

pub(crate) fn validate_loaded_game_sound_references<const D: usize, Size: GridSize<D>>(
    game: &LoadedGridGame<D, Size>,
) -> Result<(), DiagnosticReport> {
    let catalog = SoundCatalog::new(&game.sounds);
    for scene in &game.scenes {
        validate_scene(scene, &catalog)?;
    }
    validate_model(game, &catalog)
}

struct SoundCatalog<'a> {
    sfx: HashSet<&'a str>,
    music: HashSet<&'a str>,
}

impl<'a> SoundCatalog<'a> {
    fn new(sounds: &'a SoundsDef) -> Self {
        Self {
            sfx: sounds.sfx.iter().map(|sound| sound.name.as_str()).collect(),
            music: sounds
                .music
                .iter()
                .map(|sound| sound.name.as_str())
                .collect(),
        }
    }

    fn sfx(&self, name: &str) -> Result<(), DiagnosticReport> {
        if self.sfx.contains(name) {
            Ok(())
        } else {
            Err(DiagnosticReport::error(format!(
                "unknown sfx sound reference `{name}`"
            )))
        }
    }

    fn music(&self, name: &str) -> Result<(), DiagnosticReport> {
        if self.music.contains(name) {
            Ok(())
        } else {
            Err(DiagnosticReport::error(format!(
                "unknown music sound reference `{name}`"
            )))
        }
    }
}

fn validate_model<const D: usize, Size: GridSize<D>>(
    game: &LoadedGridGame<D, Size>,
    catalog: &SoundCatalog<'_>,
) -> Result<(), DiagnosticReport> {
    for effects in game.rule_effects.values() {
        for effect in effects {
            match effect {
                RuleEffect::Runtime(effect) => validate_runtime_effect(effect, catalog)?,
                RuleEffect::Lifecycle(effect) => validate_scene_effect(effect, catalog)?,
            }
        }
    }
    for sound in &game.model_operation_sounds {
        catalog.sfx(&sound.sfx_name)?;
    }
    Ok(())
}

fn validate_scene(scene: &SceneDef, catalog: &SoundCatalog<'_>) -> Result<(), DiagnosticReport> {
    for binding in &scene.key_bindings {
        validate_scene_effect(&binding.effect, catalog)?;
    }
    for routine in &scene.routines {
        validate_scene_effect(&routine.effect, catalog)?;
    }
    for transition in &scene.transitions {
        validate_scene_effect(&transition.effect, catalog)?;
    }
    validate_scene_components(&scene.components, catalog)
}

fn validate_scene_components(
    components: &[SceneComponent],
    catalog: &SoundCatalog<'_>,
) -> Result<(), DiagnosticReport> {
    for component in components {
        match component {
            SceneComponent::Button(button) | SceneComponent::Choice(button) => {
                validate_scene_effect(&button.effect, catalog)?;
            }
            SceneComponent::Row(container)
            | SceneComponent::Column(container)
            | SceneComponent::Box(container) => {
                validate_scene_components(&container.children, catalog)?;
            }
            SceneComponent::Conditional(conditional) => {
                validate_scene_components(&conditional.children, catalog)?;
                validate_scene_components(&conditional.else_children, catalog)?;
            }
            SceneComponent::Viewport(_) | SceneComponent::Frame(_) | SceneComponent::Text(_) => {}
        }
    }
    Ok(())
}

fn validate_runtime_effect(
    effect: &RuntimeEffect,
    catalog: &SoundCatalog<'_>,
) -> Result<(), DiagnosticReport> {
    match effect {
        RuntimeEffect::PlaySfx { name } => catalog.sfx(name),
        RuntimeEffect::PlayMusic { name } => catalog.music(name),
        RuntimeEffect::PauseMusic { name }
        | RuntimeEffect::ResumeMusic { name }
        | RuntimeEffect::StopMusic { name } => match name {
            Some(name) => catalog.music(name),
            None => Ok(()),
        },
        RuntimeEffect::Win
        | RuntimeEffect::Restart
        | RuntimeEffect::NextLevel
        | RuntimeEffect::Again
        | RuntimeEffect::Checkpoint
        | RuntimeEffect::ClearCheckpoint
        | RuntimeEffect::Wait { .. }
        | RuntimeEffect::WaitAnimation
        | RuntimeEffect::EmitAnimation { .. }
        | RuntimeEffect::PresentComponent { .. } => Ok(()),
    }
}

fn validate_scene_effect(
    effect: &SceneEffect,
    catalog: &SoundCatalog<'_>,
) -> Result<(), DiagnosticReport> {
    match effect {
        SceneEffect::PlaySfx { name } => catalog.sfx(name),
        SceneEffect::PlayMusic { name } => catalog.music(name),
        SceneEffect::PauseMusic { name }
        | SceneEffect::ResumeMusic { name }
        | SceneEffect::StopMusic { name } => match name {
            Some(name) => catalog.music(name),
            None => Ok(()),
        },
        SceneEffect::Conditional { effect, .. } => validate_scene_effect(effect, catalog),
        SceneEffect::Sequence { effects } => {
            for effect in effects {
                validate_scene_effect(effect, catalog)?;
            }
            Ok(())
        }
        SceneEffect::Input(_)
        | SceneEffect::ComponentEffect(_)
        | SceneEffect::RoutineCall(_)
        | SceneEffect::PresentComponent { .. }
        | SceneEffect::Wait { .. }
        | SceneEffect::Goto { .. }
        | SceneEffect::Enter { .. }
        | SceneEffect::Back
        | SceneEffect::Create { .. }
        | SceneEffect::Reset { .. }
        | SceneEffect::Delete { .. }
        | SceneEffect::Show { .. }
        | SceneEffect::Hide { .. }
        | SceneEffect::Toggle { .. }
        | SceneEffect::Focus { .. }
        | SceneEffect::Move { .. }
        | SceneEffect::PuzzleNextLevel { .. }
        | SceneEffect::PuzzlePreviousLevel { .. }
        | SceneEffect::GotoLevel { .. }
        | SceneEffect::ResetPuzzle { .. }
        | SceneEffect::LoadPuzzle { .. }
        | SceneEffect::Apply { .. }
        | SceneEffect::Copy { .. }
        | SceneEffect::SetVariable { .. }
        | SceneEffect::ClearUndoHistory
        | SceneEffect::ClearGameProgress
        | SceneEffect::SetCurrentLevel { .. }
        | SceneEffect::ClearCurrentLevel
        | SceneEffect::SetLevelCleared { .. }
        | SceneEffect::ResetPersistentVars => Ok(()),
    }
}
