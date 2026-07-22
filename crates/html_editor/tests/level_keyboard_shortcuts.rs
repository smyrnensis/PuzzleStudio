const EDITOR_SOURCE: &str = include_str!("../static/editor.js");
const EDITOR_COMMANDS_SOURCE: &str = include_str!("../static/editor_commands.js");
const EDITOR_LEVEL3D_SOURCE: &str = include_str!("../static/editor_level3d.js");
const EDITOR_WORKBENCH_SOURCE: &str = include_str!("../static/editor_workbench.js");

fn command_definition(id: &str, next_id: &str) -> &'static str {
    EDITOR_COMMANDS_SOURCE
        .split_once(&format!("id: \"{id}\""))
        .and_then(|(_, tail)| tail.split_once(&format!("id: \"{next_id}\"")))
        .map(|(definition, _)| definition)
        .expect("editor command definition")
}

fn assert_shortcut(id: &str, next_id: &str, shortcut: &str) {
    let command = command_definition(id, next_id);
    assert!(
        command.contains(shortcut),
        "{id} is missing shortcut definition {shortcut}"
    );
}

#[test]
fn level_source_actions_share_shortcuts_across_2d_and_3d() {
    let copy = command_definition("level.source.copy", "level.solve");
    assert!(copy.contains("shortcuts: [{ key: \"c\", modifiers: [\"primary\"] }]"));
    assert!(copy.contains("#copyLevelButton, #copyLevel3dButton"));

    let add = command_definition("level.add", "visual.add");
    assert!(add.contains("shortcuts: [{ key: \"a\", modifiers: [\"primary\"] }]"));
    assert!(add.contains("#addLevelButton, #addLevel3dButton"));
    assert!(add.contains("run: runLevelAddCommand"));

    let save = command_definition("level.save", "visual.save");
    assert!(save.contains("shortcuts: [{ key: \"s\", modifiers: [\"primary\"] }]"));
    assert!(save.contains("#updateLevelButton, #updateLevel3dButton"));
    assert!(save.contains("editorCommandRouteIs(context, \"level\""));
    assert!(save.contains("run: runLevelSaveCommand"));

    let solve = command_definition("level.solve", "level.brush");
    assert!(solve.contains("shortcuts: [{ key: \"Enter\", modifiers: [\"primary\"] }]"));
    assert!(solve.contains("#levelSolveShortcutButton, #level3dSolveShortcutButton"));
}

#[test]
fn level_save_shortcut_uses_each_level_editors_source_update_owner() {
    let save_owner = EDITOR_COMMANDS_SOURCE
        .split_once("function runLevelSaveCommand(context) {")
        .and_then(|(_, tail)| tail.split_once("function runVisualSaveCommand(context) {"))
        .map(|(body, _)| body)
        .expect("level save command owner");

    assert!(save_owner.contains("context.route.mode === \"edit\""));
    assert!(save_owner.contains("updateLevelInSource();"));
    assert!(save_owner.contains("context.route.mode === \"level3d\""));
    assert!(save_owner.contains("updateLevel3dInSource();"));
}

#[test]
fn workbench_owns_pane_context_and_commands_own_contextual_routing() {
    assert!(
        EDITOR_WORKBENCH_SOURCE
            .contains("function workbenchCommandContext(source = \"keyboard\", target = null)")
    );
    assert!(EDITOR_WORKBENCH_SOURCE.contains(": focusedWorkPaneId;"));
    assert!(EDITOR_WORKBENCH_SOURCE.contains("mode: commandModeForPaneId(paneId)"));
    assert!(
        EDITOR_COMMANDS_SOURCE.contains("const route = workbenchCommandContext(source, target);")
    );
    assert!(EDITOR_COMMANDS_SOURCE.contains("throw new Error(`Ambiguous editor shortcut:"));
    assert!(!EDITOR_SOURCE.contains("currentToolPaneSaveShortcutMode"));
    assert!(!EDITOR_SOURCE.contains("handleToolPaneSaveShortcut"));
}

#[test]
fn each_pane_save_command_declares_the_same_physical_shortcut() {
    for (id, next_id) in [
        ("workspace.save", "level.save"),
        ("level.save", "visual.save"),
        ("visual.save", "sounds.save"),
        ("sounds.save", "editor.undo"),
    ] {
        assert_shortcut(
            id,
            next_id,
            "shortcuts: [{ key: \"s\", modifiers: [\"primary\"] }]",
        );
    }
    assert!(!EDITOR_COMMANDS_SOURCE.contains("id: \"editor.save\""));
}

#[test]
fn level_and_visual_add_are_distinct_commands_with_the_same_shortcut() {
    assert_shortcut(
        "level.add",
        "visual.add",
        "shortcuts: [{ key: \"a\", modifiers: [\"primary\"] }]",
    );
    assert_shortcut(
        "visual.add",
        "level.source.copy",
        "shortcuts: [{ key: \"a\", modifiers: [\"primary\"] }]",
    );
    assert!(!EDITOR_COMMANDS_SOURCE.contains("id: \"editor.add\""));
    assert!(!EDITOR_COMMANDS_SOURCE.contains("function runEditorAddCommand"));
}

#[test]
fn frequent_level_editing_tools_share_shortcuts_across_2d_and_3d() {
    for (id, next_id, shortcut) in [
        ("level.brush", "level.eraser", "shortcuts: [{ key: \"b\" }]"),
        ("level.eraser", "level.fill", "shortcuts: [{ key: \".\" }]"),
        ("level.fill", "level.grid", "shortcuts: [{ key: \"f\" }]"),
        (
            "level.grid",
            "level.layer.previous",
            "shortcuts: [{ key: \"g\" }]",
        ),
        (
            "level3d.brush",
            "level3d.eraser",
            "shortcuts: [{ key: \"b\" }]",
        ),
        (
            "level3d.eraser",
            "level3d.fill",
            "shortcuts: [{ key: \".\" }]",
        ),
        (
            "level3d.fill",
            "level3d.grid",
            "shortcuts: [{ key: \"f\" }]",
        ),
        (
            "level3d.grid",
            "level3d.play",
            "shortcuts: [{ key: \"g\" }]",
        ),
    ] {
        assert_shortcut(id, next_id, shortcut);
    }
}

#[test]
fn level_layers_move_horizontally_only_in_layer_mode() {
    let previous = command_definition("level.layer.previous", "level.layer.next");
    assert!(previous.contains("shortcuts: [{ key: \"ArrowLeft\" }]"));
    assert!(previous.contains("available: levelLayerCommandActive"));
    assert!(previous.contains("setLevelLayer(level.activeLayer - 1)"));

    let next = command_definition("level.layer.next", "level.play");
    assert!(next.contains("shortcuts: [{ key: \"ArrowRight\" }]"));
    assert!(next.contains("available: levelLayerCommandActive"));
    assert!(next.contains("setLevelLayer(level.activeLayer + 1)"));

    assert!(EDITOR_COMMANDS_SOURCE.contains("&& level.layerMode"));
    assert!(EDITOR_COMMANDS_SOURCE.contains("&& !levelLayerInsertMode"));
    assert!(EDITOR_COMMANDS_SOURCE.contains("&& !levelLayerRemoveMode"));
    assert!(EDITOR_COMMANDS_SOURCE.contains("&& editorCommandTargetInside(context, levelBuilder)"));
}

#[test]
fn level3d_slices_move_vertically_in_the_command_database() {
    assert_shortcut(
        "level3d.slice.previous",
        "level3d.slice.next",
        "shortcuts: [{ key: \"ArrowUp\" }]",
    );
    assert_shortcut(
        "level3d.slice.next",
        "level3d.slice.add-above",
        "shortcuts: [{ key: \"ArrowDown\" }]",
    );
    assert_shortcut(
        "level3d.slice.add-above",
        "level3d.slice.add-below",
        "shortcuts: [{ key: \"[\" }]",
    );
    assert_shortcut(
        "level3d.slice.add-below",
        "level3d.view.toggle",
        "shortcuts: [{ key: \"]\" }]",
    );
    assert!(!EDITOR_LEVEL3D_SOURCE.contains("handleLevel3dSliceHorizontalInput"));
}
