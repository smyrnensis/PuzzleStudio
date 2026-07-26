use std::process::Command;

const PUZZLE3_THREE_RENDERER_JS: &str = include_str!("../static/puzzle3_three_renderer.js");

#[test]
fn three_adapter_maps_canonical_right_back_down_axes_at_the_render_boundary() {
    let script = format!(
        r##"
const vm = require("vm");
let source = {};
source = source.replace(
  "window.Puzzle3ThreeRenderer = {{",
  "window.Puzzle3ThreeRenderer = {{ visualPointToRenderPoint, renderPointToVisualPoint,",
);
const context = {{ window: {{}} }};
vm.runInNewContext(source, context);
const api = context.window.Puzzle3ThreeRenderer;
const canonical = [
  {{ x: 1, y: 0, z: 0 }},
  {{ x: 0, y: 1, z: 0 }},
  {{ x: 0, y: 0, z: 1 }},
];
const render = canonical.map(api.visualPointToRenderPoint);
const roundTrip = render.map(api.renderPointToVisualPoint);
process.stdout.write(JSON.stringify({{ render, roundTrip }}));
"##,
        serde_json::to_string(PUZZLE3_THREE_RENDERER_JS).unwrap(),
    );
    let output = Command::new("node")
        .arg("-e")
        .arg(script)
        .output()
        .expect("Node.js is required for the Puzzle3 coordinate adapter test");

    assert!(
        output.status.success(),
        "Puzzle3 coordinate adapter evaluation failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        r#"{"render":[{"x":1,"y":0,"z":0},{"x":0,"y":0,"z":1},{"x":0,"y":-1,"z":0}],"roundTrip":[{"x":1,"y":0,"z":0},{"x":0,"y":1,"z":0},{"x":0,"y":0,"z":1}]}"#
    );
}
