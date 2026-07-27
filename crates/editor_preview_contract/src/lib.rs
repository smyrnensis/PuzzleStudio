use puzzle_authoring::EditorDraftState;
use puzzle_runtime_contract::RuntimePuzzle3CameraProjection;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum EditorPreviewControlRequest {
    HydrateState {
        command_id: u32,
        state: Value,
        level_index: u32,
        materialize_level_start: bool,
    },
    HydrateDraft {
        command_id: u32,
        model: String,
        level_index: u32,
        draft: EditorDraftState,
        presentation: EditorDraftPresentation,
    },
    SyntheticKey {
        command_id: u32,
        key: String,
        code: String,
        repeat: bool,
        alt_key: bool,
        ctrl_key: bool,
        meta_key: bool,
        shift_key: bool,
        trace: bool,
    },
    RequestSnapshot {
        command_id: u32,
    },
}

impl EditorPreviewControlRequest {
    pub fn command_id(&self) -> u32 {
        match self {
            Self::HydrateState { command_id, .. }
            | Self::HydrateDraft { command_id, .. }
            | Self::SyntheticKey { command_id, .. }
            | Self::RequestSnapshot { command_id } => *command_id,
        }
    }
}

impl EditorPreviewControlRequest {
    pub fn validate(&self) -> Result<(), &'static str> {
        let Self::HydrateDraft {
            draft,
            presentation,
            ..
        } = self
        else {
            return Ok(());
        };
        match (draft, presentation) {
            (EditorDraftState::Grid2d(_), EditorDraftPresentation::Grid2d { .. })
            | (EditorDraftState::Grid3d(_), EditorDraftPresentation::Spatial3d { .. }) => Ok(()),
            (EditorDraftState::Grid2d(_), EditorDraftPresentation::Spatial3d { .. }) => Err(
                "editor draft state/presentation dimension mismatch: grid2d draft requires grid2d presentation",
            ),
            (EditorDraftState::Grid3d(_), EditorDraftPresentation::Grid2d { .. }) => Err(
                "editor draft state/presentation dimension mismatch: grid3d draft requires spatial3d presentation",
            ),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum EditorDraftPresentation {
    Grid2d {
        surface_id: String,
    },
    Spatial3d {
        surface: SpatialAuthoringSurface,
        camera: SpatialAuthoringCamera,
        view: SpatialAuthoringView,
        settings: SpatialAuthoringSettings,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SpatialAuthoringSurfaceKind {
    Stage,
    Layer,
    Solver,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SpatialAuthoringInteractionMode {
    Paint,
    Expand,
    Shrink,
    Play,
    Observe,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SpatialAuthoringSurface {
    pub surface_id: String,
    pub kind: SpatialAuthoringSurfaceKind,
    pub slice_z: Option<u16>,
    pub hidden_layers: Vec<u16>,
    pub interaction_mode: SpatialAuthoringInteractionMode,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SpatialAuthoringCamera {
    pub projection: RuntimePuzzle3CameraProjection,
    pub yaw_degrees: f64,
    pub pitch_degrees: f64,
    pub roll_degrees: f64,
    pub zoom: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SpatialPoint3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SpatialAuthoringView {
    pub zoom: f64,
    pub target: SpatialPoint3,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SpatialAuthoringSettings {
    pub grid_visible: bool,
    pub occupied_cell_frames: bool,
    pub stage_frame: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type")]
pub enum EditorPreviewObservation {
    #[serde(rename = "PuzzleStudioPreviewRuntimeReady")]
    RuntimeReady,
    #[serde(rename = "PuzzleStudioPreviewState")]
    State {
        #[serde(rename = "commandId", skip_serializing_if = "Option::is_none")]
        command_id: Option<u32>,
        #[serde(flatten)]
        state: Map<String, Value>,
    },
    #[serde(rename = "PuzzleStudioPreviewDebugTrace")]
    DebugTrace {
        #[serde(rename = "commandId")]
        command_id: u32,
        debug: Value,
        snapshot: Value,
    },
    #[serde(rename = "PuzzleStudioPreviewRuntimeError")]
    RuntimeError {
        #[serde(rename = "commandId")]
        command_id: u32,
        label: &'static str,
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use super::{EditorPreviewControlRequest, SpatialAuthoringSurfaceKind};

    #[test]
    fn draft_wire_rejects_adapter_owned_resources() {
        let request = serde_json::from_str::<EditorPreviewControlRequest>(
            r#"{
                "type":"hydrateDraft",
                "commandId":7,
                "model":"space",
                "levelIndex":0,
                "draft":{
                    "kind":"grid3d",
                    "level":{
                        "size":{"width":1,"depth":1,"height":1},
                        "cells":[{"position":{"x":0,"y":0,"z":0},"symbol":"P"}]
                    }
                },
                "presentation":{
                    "kind":"spatial3d",
                    "surface":{
                        "surfaceId":"stage",
                        "kind":"stage",
                        "sliceZ":null,
                        "hiddenLayers":[],
                        "interactionMode":"paint"
                    },
                    "camera":{
                        "projection":"perspective",
                        "yawDegrees":15,
                        "pitchDegrees":30,
                        "rollDegrees":0,
                        "zoom":1
                    },
                    "view":{"zoom":1,"target":{"x":0,"y":0,"z":0}},
                    "settings":{
                        "gridVisible":true,
                        "occupiedCellFrames":false,
                        "stageFrame":true
                    }
                },
                "resources":{}
            }"#,
        )
        .unwrap_err();
        assert!(request.to_string().contains("unknown field `resources`"));
    }

    #[test]
    fn draft_wire_preserves_typed_spatial_surface_identity() {
        let request = serde_json::from_str::<EditorPreviewControlRequest>(
            r#"{
                "type":"hydrateDraft",
                "commandId":7,
                "model":"space",
                "levelIndex":0,
                "draft":{
                    "kind":"grid3d",
                    "level":{
                        "size":{"width":1,"depth":1,"height":1},
                        "cells":[{"position":{"x":0,"y":0,"z":0},"symbol":"P"}]
                    }
                },
                "presentation":{
                    "kind":"spatial3d",
                    "surface":{
                        "surfaceId":"stage",
                        "kind":"stage",
                        "sliceZ":null,
                        "hiddenLayers":[],
                        "interactionMode":"paint"
                    },
                    "camera":{
                        "projection":"perspective",
                        "yawDegrees":15,
                        "pitchDegrees":30,
                        "rollDegrees":0,
                        "zoom":1
                    },
                    "view":{"zoom":1,"target":{"x":0,"y":0,"z":0}},
                    "settings":{
                        "gridVisible":true,
                        "occupiedCellFrames":false,
                        "stageFrame":true
                    }
                }
            }"#,
        )
        .unwrap();
        let EditorPreviewControlRequest::HydrateDraft { presentation, .. } = request else {
            panic!("expected spatial draft request");
        };
        let super::EditorDraftPresentation::Spatial3d { surface, .. } = presentation else {
            panic!("expected spatial presentation");
        };
        assert_eq!(surface.surface_id, "stage");
        assert_eq!(surface.kind, SpatialAuthoringSurfaceKind::Stage);
    }

    #[test]
    fn hydrate_draft_rejects_dimension_mismatch() {
        let request = serde_json::from_str::<EditorPreviewControlRequest>(
            r#"{
                "type":"hydrateDraft",
                "commandId":7,
                "model":"board",
                "levelIndex":0,
                "draft":{
                    "kind":"grid2d",
                    "level":{
                        "size":{"width":1,"height":1},
                        "cells":[{"position":{"x":0,"y":0},"symbol":"P"}]
                    }
                },
                "presentation":{
                    "kind":"spatial3d",
                    "surface":{
                        "surfaceId":"stage",
                        "kind":"stage",
                        "sliceZ":null,
                        "hiddenLayers":[],
                        "interactionMode":"paint"
                    },
                    "camera":{
                        "projection":"perspective",
                        "yawDegrees":15,
                        "pitchDegrees":30,
                        "rollDegrees":0,
                        "zoom":1
                    },
                    "view":{"zoom":1,"target":{"x":0,"y":0,"z":0}},
                    "settings":{
                        "gridVisible":true,
                        "occupiedCellFrames":false,
                        "stageFrame":true
                    }
                }
            }"#,
        )
        .unwrap();
        assert_eq!(
            request.validate(),
            Err(
                "editor draft state/presentation dimension mismatch: grid2d draft requires grid2d presentation"
            )
        );
    }

    #[test]
    fn draft_wire_cannot_override_the_compiled_legend() {
        let error = serde_json::from_str::<EditorPreviewControlRequest>(
            r#"{
                "type":"hydrateDraft",
                "commandId":8,
                "model":"board",
                "levelIndex":0,
                "draft":{
                    "kind":"grid2d",
                    "level":{
                        "size":{"width":1,"height":1},
                        "cells":[{"position":{"x":0,"y":0},"symbol":"P"}],
                        "legend":[{"symbol":"P","objects":["Other"]}]
                    }
                },
                "presentation":{"kind":"grid2d","surfaceId":"main"}
            }"#,
        )
        .unwrap_err();
        assert!(error.to_string().contains("unknown field `legend`"));
    }
}
