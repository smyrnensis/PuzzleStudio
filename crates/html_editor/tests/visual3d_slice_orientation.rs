const EDITOR_VISUAL3D_SOURCE: &str = include_str!("../static/editor_visual3d.js");

fn function_body<'a>(source: &'a str, signature: &str, next_signature: &str) -> &'a str {
    source
        .split_once(signature)
        .and_then(|(_, tail)| tail.split_once(next_signature))
        .map(|(body, _)| body)
        .expect("function body")
}

#[test]
fn visual3d_document_rows_and_slices_keep_ascii_order() {
    let payload = function_body(
        EDITOR_VISUAL3D_SOURCE,
        "function visual3dTargetPayload(target) {",
        "function setVisual3dEditSource(target, document = activeDocument()) {",
    );
    let world_to_source = function_body(
        EDITOR_VISUAL3D_SOURCE,
        "function visual3dEditFrames() {",
        "function visual3dEditMutationRequest(operation, options = {}) {",
    );
    let plane_coordinates = function_body(
        EDITOR_VISUAL3D_SOURCE,
        "function visual3dCoordsFromPlane(axis, stack, u, v) {",
        "function visual3dPlaneWorldSlice(axis, stack) {",
    );
    let plane_slice = function_body(
        EDITOR_VISUAL3D_SOURCE,
        "function visual3dPlaneWorldSlice(axis, stack) {",
        "function visual3dCurrentSliceDescriptor() {",
    );

    assert!(payload.contains("documentContract.cellsByFrame.map((layers) => layers.flat())"));
    assert!(world_to_source.contains("visual3dCellIndex(x, y, z)"));
    assert!(!world_to_source.contains("visual3d.height - 1"));
    assert!(!world_to_source.contains("visual3d.depth - 1"));
    assert!(plane_coordinates.contains("return { x: u, y: v, z: fixed };"));
    assert!(plane_slice.contains("return normalized;"));
}
