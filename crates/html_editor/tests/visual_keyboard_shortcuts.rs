const EDITOR_VISUAL_SOURCE: &str = include_str!("../static/editor_visual.js");
const EDITOR_COMMANDS_SOURCE: &str = include_str!("../static/editor_commands.js");

#[test]
fn visual_navigation_uses_directional_arrow_keys() {
    assert!(EDITOR_COMMANDS_SOURCE.contains(
        "id: \"visual3d.slice.previous\",\n    group: \"visual\",\n    label: \"Previous slice\",\n    shortcuts: [{ key: \"ArrowUp\" }]"
    ));
    assert!(EDITOR_COMMANDS_SOURCE.contains(
        "id: \"visual3d.slice.next\",\n    group: \"visual\",\n    label: \"Next slice\",\n    shortcuts: [{ key: \"ArrowDown\" }]"
    ));
    assert!(EDITOR_COMMANDS_SOURCE.contains(
        "id: \"visual.frame.previous\",\n    group: \"visual\",\n    label: \"Previous frame\",\n    shortcuts: [{ key: \"ArrowLeft\" }]"
    ));
    assert!(EDITOR_COMMANDS_SOURCE.contains(
        "id: \"visual.frame.next\",\n    group: \"visual\",\n    label: \"Next frame\",\n    shortcuts: [{ key: \"ArrowRight\" }]"
    ));
    assert!(EDITOR_COMMANDS_SOURCE.contains("moveSharedVisualAnimationFrame("));

    assert!(!EDITOR_VISUAL_SOURCE.contains("handleVisualPaneCommandShortcut"));
}

#[test]
fn visual_tools_and_edit_actions_use_the_shared_command_registry() {
    for id in [
        "visual.fill",
        "visual.move",
        "visual.clip",
        "visual.edit.copy",
        "visual.edit.cut",
        "visual.edit.paste",
        "visual.edit.delete",
    ] {
        assert!(EDITOR_COMMANDS_SOURCE.contains(&format!("id: \"{id}\"")));
    }
    assert!(
        EDITOR_COMMANDS_SOURCE.contains("shortcuts: [{ key: \"Delete\" }, { key: \"Backspace\" }]")
    );
    assert!(!EDITOR_VISUAL_SOURCE.contains("visualEditCommandForShortcut"));
}

#[test]
fn commands_accept_multiple_physical_shortcuts_without_a_binding_layer() {
    assert!(
        EDITOR_COMMANDS_SOURCE.contains("shortcuts: [{ key: \"Delete\" }, { key: \"Backspace\" }]")
    );
    assert!(EDITOR_COMMANDS_SOURCE.contains(
        "{ key: \"z\", modifiers: [\"primary\", \"shift\"] },\n      { key: \"y\", modifiers: [\"primary\"] },"
    ));
    assert!(EDITOR_COMMANDS_SOURCE.contains(
        "editorCommandShortcuts(command.id).some((shortcut) => editorShortcutMatches(event, shortcut))"
    ));
    assert!(!EDITOR_COMMANDS_SOURCE.contains("binding:"));
}

#[test]
fn command_number_selects_the_3d_edit_scope() {
    assert!(EDITOR_COMMANDS_SOURCE.contains(
        "id: \"visual3d.scope.slice\",\n    group: \"visual\",\n    label: \"Scope 2D\",\n    shortcuts: [{ key: \"2\", modifiers: [\"primary\"] }]"
    ));
    assert!(EDITOR_COMMANDS_SOURCE.contains(
        "id: \"visual3d.scope.all\",\n    group: \"visual\",\n    label: \"Scope 3D\",\n    shortcuts: [{ key: \"3\", modifiers: [\"primary\"] }]"
    ));
    assert!(EDITOR_COMMANDS_SOURCE.contains("run: () => (setVisual3dEditScope(\"slice\"), true)"));
    assert!(EDITOR_COMMANDS_SOURCE.contains("run: () => (setVisual3dEditScope(\"all\"), true)"));
}

#[test]
fn text_inputs_and_active_clip_tools_keep_their_arrow_keys() {
    assert!(EDITOR_COMMANDS_SOURCE.contains(
        "if (!command.allowTextEntry && editorCommandTextEntryTarget(event.target)) return false;"
    ));
    assert!(EDITOR_COMMANDS_SOURCE.contains(
        "if (dimension === \"3d\") return !visual3dClipActive && !visual3dTranslateActive;"
    ));
    assert!(EDITOR_COMMANDS_SOURCE.contains("return !visualClipActive && !visualTranslateActive;"));
    assert!(
        EDITOR_COMMANDS_SOURCE
            .contains("target?.closest(\"[data-visual3d-camera], [data-visual3d-slice-scrub]\")")
    );
}
