use puzzle_authoring::{EditorDraftPosition2d, EditorDraftPosition3d, EditorDraftState};
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
        presentation: EditorAuthoringPresentation,
    },
    EditorPointer {
        command_id: u32,
        surface_id: String,
        committed_frame_revision: u64,
        x_css: f64,
        y_css: f64,
        gesture: EditorPointerGesture,
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
            | Self::EditorPointer { command_id, .. }
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
        match (draft, &presentation.renderer) {
            (EditorDraftState::Grid2d(_), EditorRendererStrategy::Grid2d)
            | (EditorDraftState::Grid3d(_), EditorRendererStrategy::Grid3d { .. }) => Ok(()),
            (EditorDraftState::Grid2d(_), EditorRendererStrategy::Grid3d { .. }) => Err(
                "editor draft renderer mismatch: grid2d draft requires the grid2d renderer strategy",
            ),
            (EditorDraftState::Grid3d(_), EditorRendererStrategy::Grid2d) => Err(
                "editor draft renderer mismatch: grid3d draft requires the grid3d renderer strategy",
            ),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EditorAuthoringPresentation {
    pub surface: EditorAuthoringSurface,
    pub renderer: EditorRendererStrategy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EditorPaintOperation {
    Add,
    Replace,
    Erase,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EditorResizeMode {
    Expand,
    Shrink,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum EditorAuthoringInteraction {
    Paint { operation: EditorPaintOperation },
    Resize { mode: EditorResizeMode },
    Play,
    Observe,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EditorAuthoringSurface {
    pub surface_id: String,
    pub interaction: EditorAuthoringInteraction,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum EditorRendererStrategy {
    Grid2d,
    Grid3d {
        slice_z: Option<u16>,
        hidden_layers: Vec<u16>,
        camera: EditorCamera3d,
        view: EditorView3d,
        settings: EditorGrid3dSettings,
    },
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EditorCamera3d {
    pub projection: RuntimePuzzle3CameraProjection,
    pub yaw_degrees: f64,
    pub pitch_degrees: f64,
    pub roll_degrees: f64,
    pub zoom: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EditorPoint3d {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EditorView3d {
    pub target: EditorPoint3d,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EditorGrid3dSettings {
    pub grid_visible: bool,
    pub occupied_cell_frames: bool,
    pub stage_frame: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EditorPointerGesture {
    Move,
    Press,
    Leave,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum EditorAuthoringHitTarget {
    Cell {
        position: EditorGridPosition,
    },
    Placement {
        position: EditorGridPosition,
    },
    Resize {
        mode: EditorResizeMode,
        axis: EditorGridAxis,
        side: EditorGridSide,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "kind", content = "position", rename_all = "camelCase")]
pub enum EditorGridPosition {
    Grid2d(EditorDraftPosition2d),
    Grid3d(EditorDraftPosition3d),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EditorGridAxis {
    X,
    Y,
    Z,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EditorGridSide {
    Min,
    Max,
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
    #[serde(rename = "PuzzleStudioEditorAuthoringFrame")]
    EditorAuthoringFrame {
        #[serde(rename = "surfaceId")]
        surface_id: String,
        #[serde(rename = "frameRevision")]
        frame_revision: u64,
    },
    #[serde(rename = "PuzzleStudioEditorAuthoringHit")]
    EditorAuthoringHit {
        #[serde(rename = "commandId")]
        command_id: u32,
        #[serde(rename = "surfaceId")]
        surface_id: String,
        #[serde(rename = "frameRevision")]
        frame_revision: u64,
        hit: Option<EditorAuthoringHitTarget>,
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
    use super::{EditorPreviewControlRequest, EditorRendererStrategy};

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
                    "surface":{
                        "surfaceId":"stage",
                        "interaction":{"kind":"paint","operation":"replace"}
                    },
                    "renderer":{
                        "kind":"grid3d",
                        "sliceZ":null,
                        "hiddenLayers":[],
                        "camera":{
                            "projection":"perspective",
                            "yawDegrees":15,
                            "pitchDegrees":30,
                            "rollDegrees":0,
                            "zoom":1
                        },
                        "view":{"target":{"x":0,"y":0,"z":0}},
                        "settings":{
                            "gridVisible":true,
                            "occupiedCellFrames":false,
                            "stageFrame":true
                        }
                    }
                },
                "resources":{}
            }"#,
        )
        .unwrap_err();
        assert!(request.to_string().contains("unknown field `resources`"));
    }

    #[test]
    fn draft_wire_preserves_typed_authoring_surface_identity() {
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
                    "surface":{
                        "surfaceId":"stage",
                        "interaction":{"kind":"paint","operation":"replace"}
                    },
                    "renderer":{
                        "kind":"grid3d",
                        "sliceZ":null,
                        "hiddenLayers":[],
                        "camera":{
                            "projection":"perspective",
                            "yawDegrees":15,
                            "pitchDegrees":30,
                            "rollDegrees":0,
                            "zoom":1
                        },
                        "view":{"target":{"x":0,"y":0,"z":0}},
                        "settings":{
                            "gridVisible":true,
                            "occupiedCellFrames":false,
                            "stageFrame":true
                        }
                    }
                }
            }"#,
        )
        .unwrap();
        let EditorPreviewControlRequest::HydrateDraft { presentation, .. } = request else {
            panic!("expected editor authoring draft request");
        };
        assert_eq!(presentation.surface.surface_id, "stage");
        assert!(matches!(
            presentation.renderer,
            EditorRendererStrategy::Grid3d { .. }
        ));
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
                    "surface":{
                        "surfaceId":"stage",
                        "interaction":{"kind":"paint","operation":"replace"}
                    },
                    "renderer":{
                        "kind":"grid3d",
                        "sliceZ":null,
                        "hiddenLayers":[],
                        "camera":{
                            "projection":"perspective",
                            "yawDegrees":15,
                            "pitchDegrees":30,
                            "rollDegrees":0,
                            "zoom":1
                        },
                        "view":{"target":{"x":0,"y":0,"z":0}},
                        "settings":{
                            "gridVisible":true,
                            "occupiedCellFrames":false,
                            "stageFrame":true
                        }
                    }
                }
            }"#,
        )
        .unwrap();
        assert_eq!(
            request.validate(),
            Err(
                "editor draft renderer mismatch: grid2d draft requires the grid2d renderer strategy"
            )
        );
    }

    #[test]
    fn hydrate_draft_uses_one_lifecycle_for_both_renderer_strategies() {
        let grid2d = serde_json::json!({
            "type": "hydrateDraft",
            "commandId": 11,
            "model": "board2",
            "levelIndex": 0,
            "draft": {
                "kind": "grid2d",
                "level": {
                    "size": {"width": 1, "height": 1},
                    "cells": [{"position": {"x": 0, "y": 0}, "symbol": "P"}]
                }
            },
            "presentation": {
                "surface": {
                    "surfaceId": "stage",
                    "interaction": {"kind": "paint", "operation": "replace"}
                },
                "renderer": {"kind": "grid2d"}
            }
        });
        let grid3d = serde_json::json!({
            "type": "hydrateDraft",
            "commandId": 12,
            "model": "board3",
            "levelIndex": 0,
            "draft": {
                "kind": "grid3d",
                "level": {
                    "size": {"width": 1, "depth": 1, "height": 1},
                    "cells": [{"position": {"x": 0, "y": 0, "z": 0}, "symbol": "P"}]
                }
            },
            "presentation": {
                "surface": {
                    "surfaceId": "stage",
                    "interaction": {"kind": "paint", "operation": "replace"}
                },
                "renderer": {
                    "kind": "grid3d",
                    "sliceZ": null,
                    "hiddenLayers": [],
                    "camera": {
                        "projection": "perspective",
                        "yawDegrees": 15,
                        "pitchDegrees": 30,
                        "rollDegrees": 0,
                        "zoom": 1
                    },
                    "view": {"target": {"x": 0, "y": 0, "z": 0}},
                    "settings": {
                        "gridVisible": true,
                        "occupiedCellFrames": false,
                        "stageFrame": true
                    }
                }
            }
        });
        for value in [grid2d, grid3d] {
            let request = serde_json::from_value::<EditorPreviewControlRequest>(value).unwrap();
            assert!(matches!(
                request,
                EditorPreviewControlRequest::HydrateDraft { .. }
            ));
            assert_eq!(request.validate(), Ok(()));
        }
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
                "presentation":{
                    "surface":{
                        "surfaceId":"main",
                        "interaction":{"kind":"paint","operation":"replace"}
                    },
                    "renderer":{"kind":"grid2d"}
                }
            }"#,
        )
        .unwrap_err();
        assert!(error.to_string().contains("unknown field `legend`"));
    }
}
