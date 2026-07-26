use std::process::Command;

const VISUAL_TWEEN_CORE_JS: &str = include_str!("../static/visual_tween_core.js");
const PUZZLE3_VISUAL_CORE_JS: &str = include_str!("../static/puzzle3_visual_core.js");
const PUZZLE3_THREE_RENDERER_JS: &str = include_str!("../static/puzzle3_three_renderer.js");

#[test]
fn camera_only_render_reuses_three_scene_geometry_and_camera() {
    let script = format!(
        r##"
globalThis.window = globalThis;
globalThis.devicePixelRatio = 1;
globalThis.requestAnimationFrame = () => 0;
globalThis.cancelAnimationFrame = () => {{}};
let disposedGeometryCount = 0;

class MutableVector3 {{
  constructor(x = 0, y = 0, z = 0) {{ this.set(x, y, z); }}
  set(x, y, z) {{ this.x = x; this.y = y; this.z = z; return this; }}
}}
class Scene {{
  constructor() {{ this.children = []; this.background = null; }}
  add(...objects) {{ this.children.push(...objects); }}
  traverse(visitor) {{
    const visit = (object) => {{
      visitor(object);
      for (const child of object.children || []) visit(child);
    }};
    for (const child of this.children) visit(child);
  }}
}}
class PerspectiveCamera {{
  constructor(fov, aspect, near, far) {{
    this.isPerspectiveCamera = true;
    this.fov = fov;
    this.aspect = aspect;
    this.near = near;
    this.far = far;
    this.up = new MutableVector3();
    this.position = new MutableVector3();
  }}
  updateProjectionMatrix() {{}}
  lookAt(target) {{ this.target = target; }}
}}
class DirectionalLight {{
  constructor() {{
    this.position = new MutableVector3();
    this.target = {{ position: new MutableVector3() }};
    this.shadow = {{
      mapSize: {{ set() {{}} }},
      camera: {{ updateProjectionMatrix() {{}} }},
    }};
  }}
}}
class BufferGeometry {{
  constructor() {{ this.attributes = {{}}; }}
  setAttribute(name, value) {{ this.attributes[name] = value; }}
  dispose() {{ disposedGeometryCount += 1; }}
}}
class Mesh {{
  constructor(geometry, material) {{ this.geometry = geometry; this.material = material; }}
}}
class WebGLRenderer {{
  constructor() {{ this.shadowMap = {{}}; this.renderedScenes = []; }}
  setPixelRatio() {{}}
  setSize() {{}}
  setClearColor() {{}}
  render(scene, camera) {{ this.renderedScenes.push({{ scene, camera }}); }}
  dispose() {{}}
}}
globalThis.THREE = {{
  AmbientLight: class {{}},
  BufferGeometry,
  Color: class {{ constructor(value) {{ this.value = value; }} }},
  DirectionalLight,
  Float32BufferAttribute: class {{ constructor(values, size) {{ this.values = values; this.size = size; }} }},
  Mesh,
  MeshLambertMaterial: class {{ dispose() {{}} }},
  PerspectiveCamera,
  Scene,
  Vector3: MutableVector3,
  WebGLRenderer,
}};

{}
{}
{}

function snapshotAt(x, yawDegrees = 0) {{
  return {{
    size: {{ width: 2, depth: 1, height: 1 }},
    render: {{
      camera: {{ projection: "perspective", yawDegrees, pitchDegrees: 35, rollDegrees: 0, zoom: 1 }},
      animation: {{ tween: {{ enabled: true, intervalMs: 100 }} }},
      viewport: null,
    }},
    cells: [{{ position: {{ x, y: 0, z: 0 }}, renderOrder: x, objects: [{{ id: 1, renderOrder: x }}] }}],
    animationEvents: [],
    order: {{ direction_priority: ["down", "right", "front"], priorities: [{{ objects: ["A"] }}] }},
  }};
}}

function resolvedFrameAt(x) {{
  return {{
    continueAnimation: false,
    batches: [{{
      renderOrder: x,
      objectIds: [1],
      cell: [x, 0, 0],
      transform: [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]],
      opacity: 1,
      pixelGeometry: null,
      content: {{
        kind: "voxels",
        width: 1,
        depth: 1,
        height: 1,
        voxels: [{{ position: [0, 0, 0], color: {{ red: 1, green: 1, blue: 1, alpha: 1 }} }}],
      }},
    }}],
  }};
}}

function resolvedSceneAt(x) {{
  return {{
    cells: [{{ position: [x, 0, 0], renderOrder: x, objectIds: [1] }}],
  }};
}}

const canvas = {{
  clientWidth: 640,
  clientHeight: 480,
  width: 640,
  height: 480,
  getBoundingClientRect() {{ return {{ x: 0, y: 0, width: 640, height: 480 }}; }},
}};
const renderer = window.Puzzle3ThreeRenderer.create(canvas);
const firstSnapshot = snapshotAt(0);
const firstResolvedFrame = resolvedFrameAt(0);
renderer.render(firstSnapshot, resolvedSceneAt(0), firstResolvedFrame, {{ background: "transparent" }});
const firstScene = renderer.scene;
const firstCamera = renderer.camera;
const firstGeometry = firstScene.children.find((child) => child.geometry)?.geometry;

firstSnapshot.render.camera.yawDegrees = 45;
renderer.render(firstSnapshot, resolvedSceneAt(0), firstResolvedFrame, {{ background: "transparent" }});
const disposedAfterCamera = disposedGeometryCount;
const secondGeometry = renderer.scene.children.find((child) => child.geometry)?.geometry;

renderer.render(snapshotAt(1, 45), resolvedSceneAt(1), resolvedFrameAt(1), {{ background: "transparent" }});
const value = {{
  sameSceneAfterCamera: renderer.renderer.renderedScenes[0].scene === renderer.renderer.renderedScenes[1].scene,
  sameCameraAfterCamera: firstCamera === renderer.renderer.renderedScenes[1].camera,
  sameGeometryAfterCamera: firstGeometry === secondGeometry,
  disposedAfterCamera,
  changedSceneAfterState: firstScene !== renderer.scene,
  disposedAfterState: disposedGeometryCount,
}};
process.stdout.write(JSON.stringify(value));
"##,
        VISUAL_TWEEN_CORE_JS, PUZZLE3_VISUAL_CORE_JS, PUZZLE3_THREE_RENDERER_JS
    );

    let output = Command::new("node")
        .arg("-e")
        .arg(script)
        .output()
        .expect("Node.js is required for the Puzzle3 Three scene cache test");
    assert!(
        output.status.success(),
        "Puzzle3 Three scene cache evaluation failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value =
        String::from_utf8(output.stdout).expect("Puzzle3 Three scene cache result is UTF-8");
    assert_eq!(
        value,
        r#"{"sameSceneAfterCamera":true,"sameCameraAfterCamera":true,"sameGeometryAfterCamera":true,"disposedAfterCamera":0,"changedSceneAfterState":true,"disposedAfterState":1}"#
    );
}
