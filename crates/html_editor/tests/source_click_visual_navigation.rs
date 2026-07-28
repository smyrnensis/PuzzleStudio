const EDITOR_SOURCE: &str = include_str!("../static/editor.js");
const EDITOR_VISUAL_SOURCE: &str = include_str!("../static/editor_visual.js");
const EDITOR_VISUAL3D_SOURCE: &str = include_str!("../static/editor_visual3d.js");
const EDITOR_VISUAL_DOCUMENT_SOURCE: &str = include_str!("../static/editor_visual_document.js");

fn function_body<'a>(source: &'a str, signature: &str, next_signature: &str) -> &'a str {
    source
        .split_once(signature)
        .and_then(|(_, tail)| tail.split_once(next_signature))
        .map(|(body, _)| body)
        .expect("function body")
}

#[test]
fn visual_target_navigation_precedes_payload_hydration() {
    let visual_loader = function_body(
        EDITOR_VISUAL_SOURCE,
        "function loadVisualSourceTarget(target, options = {}) {",
        "function applyIncompleteVisualSourceTarget(name, target) {",
    );
    let visual3d_loader = function_body(
        EDITOR_VISUAL3D_SOURCE,
        "function loadVisual3dSourceTarget(target, options = {}) {",
        "function visual3dTargetPayload(target) {",
    );

    let visual_navigation = visual_loader
        .find("setPreviewMode(\"visual\")")
        .expect("2D visual navigation");
    let visual_hydration = visual_loader
        .find("parseVisualDefinitionSource(")
        .expect("2D visual hydration");
    let visual3d_navigation = visual3d_loader
        .find("setPreviewMode(\"visual3d\")")
        .expect("3D visual navigation");
    let visual3d_hydration = visual3d_loader
        .find("visual3dTargetPayload(")
        .expect("3D visual hydration");

    assert!(visual_navigation < visual_hydration);
    assert!(visual3d_navigation < visual3d_hydration);
    assert!(visual_loader.contains("target?.sourceVisual?.dimension === \"2d\""));
    assert!(visual3d_loader.contains("target?.sourceVisual?.dimension === \"3d\""));
}

#[test]
fn visual3d_roundtrip_keeps_parser_owned_prelude_rows() {
    let mutation_request = function_body(
        EDITOR_VISUAL3D_SOURCE,
        "function visual3dEditMutationRequest(operation, options = {}) {",
        "async function updateVisual3dInSource() {",
    );
    let target_payload = function_body(
        EDITOR_VISUAL3D_SOURCE,
        "function visual3dTargetPayload(target) {",
        "function setVisual3dEditSource(target, document = activeDocument()) {",
    );

    assert!(mutation_request.contains("preludeRows: visual3d.sourcePreludeRows || []"));
    assert!(target_payload.contains("sourcePreludeRows: documentContract.preludeRows"));
    assert!(
        EDITOR_VISUAL_DOCUMENT_SOURCE.contains(
            "preludeRows: Array.isArray(contract.preludeRows) ? contract.preludeRows : []"
        )
    );
    assert!(
        EDITOR_SOURCE
            .contains("sourcePreludeRows: cloneVisualEditValue(visual3d.sourcePreludeRows || [])")
    );
}

#[test]
fn visual_loaders_surface_parser_owned_contract_diagnostics() {
    let contract_error = function_body(
        EDITOR_VISUAL_SOURCE,
        "function visualSourceContractError(contract) {",
        "function visualPaletteEntrySourceToken(entry) {",
    );
    let visual3d_loader = function_body(
        EDITOR_VISUAL3D_SOURCE,
        "function loadVisual3dSourceTarget(target, options = {}) {",
        "function visual3dTargetPayload(target) {",
    );

    assert!(contract_error.contains("Array.isArray(contract.diagnostics)"));
    assert!(contract_error.contains("return diagnostic;"));
    assert!(visual3d_loader.contains("visualSourceContractError(target.sourceVisual)"));
}
