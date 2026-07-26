use std::process::Command;

const PUZZLE3_COMPONENT_JS: &str = include_str!("../static/puzzle3_component.js");
const PUZZLE3_THREE_RENDERER_JS: &str = include_str!("../static/puzzle3_three_renderer.js");

#[test]
fn puzzle3_runtime_has_one_three_renderer_with_pixelate_and_unshaded_materials() {
    assert!(!PUZZLE3_COMPONENT_JS.contains("getContext(\"2d\""));
    assert!(!PUZZLE3_COMPONENT_JS.contains("rendererMode"));
    assert!(PUZZLE3_COMPONENT_JS.contains("window.Puzzle3ThreeRenderer.create"));
    assert!(PUZZLE3_COMPONENT_JS.contains("controllerOptions.resolveRenderMoment"));
    assert!(PUZZLE3_COMPONENT_JS.contains("animationElapsedMs"));
    assert!(
        PUZZLE3_THREE_RENDERER_JS
            .contains("input: [\"snapshot\", \"resolvedScene\", \"resolvedFrame\", \"view\"]")
    );
    assert!(PUZZLE3_THREE_RENDERER_JS.contains("requireResolvedRenderScene(resolvedScene)"));
    assert!(!PUZZLE3_THREE_RENDERER_JS.contains("snapshot.cells"));
    assert!(!PUZZLE3_THREE_RENDERER_JS.contains("cell.objects"));
    for forbidden in [
        "currentVisualLayers",
        "hasLoopingVisualAnimation",
        "threeAnimationState",
        "animationSpatialAffine",
        "averageMergedVoxels",
        "snapshot.visuals",
        "PuzzleVisualTweenCore",
        "object.name === viewport.focus",
        "object.visual === viewport.focus",
    ] {
        assert!(
            !PUZZLE3_THREE_RENDERER_JS.contains(forbidden),
            "Three renderer must not own `{forbidden}` semantics"
        );
    }
    assert!(
        PUZZLE3_THREE_RENDERER_JS.contains("this.renderer.setPixelRatio(ratio / rasterScale);")
    );
    assert!(
        PUZZLE3_THREE_RENDERER_JS.contains("imageRendering = pixelate.enabled ? \"pixelated\"")
    );
    assert!(
        PUZZLE3_THREE_RENDERER_JS
            .contains("payload.cellFootprints = threeStageCellFootprints(payload);")
    );

    let script = format!(
        r##"
const vm = require("vm");
let source = {};
source = source.replace(
  "window.Puzzle3ThreeRenderer = {{",
  "window.Puzzle3ThreeRenderer = {{ pixelateSettings, visualShadeEnabled, faceMaterial,",
);
const context = {{ window: {{}} }};
vm.runInNewContext(source, context);
const api = context.window.Puzzle3ThreeRenderer;
const pixelate = api.pixelateSettings({{
  settings: {{ pixelate: {{ enabled: true, scale: 4, smoothing: false }} }},
}});
const material = api.faceMaterial({{
  MeshBasicMaterial: class MeshBasicMaterial {{ constructor(options) {{ this.options = options; }} }},
  MeshLambertMaterial: class MeshLambertMaterial {{}},
}}, "#336699", new Map(), false);
process.stdout.write(JSON.stringify({{
  pixelate,
  material: material.constructor.name,
  shade: api.visualShadeEnabled({{ settings: {{ visual: {{ shade: false }} }} }}),
}}));
"##,
        serde_json::to_string(PUZZLE3_THREE_RENDERER_JS).unwrap(),
    );
    let output = Command::new("node")
        .arg("-e")
        .arg(script)
        .output()
        .expect("Node.js is required for the Puzzle3 Three render contract test");
    assert!(
        output.status.success(),
        "Puzzle3 Three render contract evaluation failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        r#"{"pixelate":{"enabled":true,"scale":4,"smoothing":false},"material":"MeshBasicMaterial","shade":false}"#
    );
}
