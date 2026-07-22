const EDITOR_HTML: &str = include_str!("../static/editor.html");
const EDITOR_COMMANDS_SOURCE: &str = include_str!("../static/editor_commands.js");

fn command_definition(id: &str, next_id: &str) -> &'static str {
    EDITOR_COMMANDS_SOURCE
        .split_once(&format!("id: \"{id}\""))
        .and_then(|(_, tail)| tail.split_once(&format!("id: \"{next_id}\"")))
        .map(|(definition, _)| definition)
        .expect("editor command definition")
}

#[test]
fn visual_source_actions_live_in_the_visual_pane_top_bar() {
    let header = EDITOR_HTML
        .split_once("id=\"visualPaneHeaderActions\"")
        .and_then(|(_, tail)| tail.split_once("<div class=\"tool-pane-scroll\">"))
        .map(|(header, _)| header)
        .expect("visual pane header actions");

    for id in [
        "newVisualButton",
        "visualInsertButton",
        "visualUpdateButton",
        "newVisual3dButton",
        "visual3dInsertButton",
        "visual3dUpdateButton",
    ] {
        assert!(
            header.contains(&format!("id=\"{id}\"")),
            "missing {id} from visual pane header"
        );
        assert_eq!(EDITOR_HTML.matches(&format!("id=\"{id}\"")).count(), 1);
    }
}

#[test]
fn visual_source_action_shortcuts_are_owned_by_the_command_database() {
    let new_visual = command_definition("visual.new", "level.add");
    assert!(new_visual.contains("shortcuts: [{ key: \"n\", modifiers: [\"primary\"] }]"));
    assert!(
        new_visual
            .contains("elements: editorCommandElements(\"#newVisualButton, #newVisual3dButton\")")
    );
    assert!(new_visual.contains("available: visualCommandActive"));

    let add_visual = command_definition("visual.add", "level.source.copy");
    assert!(add_visual.contains("shortcuts: [{ key: \"a\", modifiers: [\"primary\"] }]"));
    assert!(add_visual.contains("#visualInsertButton, #visual3dInsertButton"));
    assert!(add_visual.contains("run: runVisualAddCommand"));

    let save_visual = command_definition("visual.save", "sounds.save");
    assert!(save_visual.contains("shortcuts: [{ key: \"s\", modifiers: [\"primary\"] }]"));
    assert!(save_visual.contains("#visualUpdateButton, #visual3dUpdateButton"));
    assert!(save_visual.contains("editorCommandRouteIs(context, \"visual\""));
    assert!(save_visual.contains("run: runVisualSaveCommand"));
}

#[test]
fn visual_source_buttons_and_keyboard_share_command_invocation() {
    assert!(EDITOR_COMMANDS_SOURCE.contains("function invokeEditorCommand(id, context)"));
    assert!(
        EDITOR_COMMANDS_SOURCE
            .contains("document.addEventListener(\"click\", dispatchEditorCommandClick, true)")
    );
    assert!(EDITOR_COMMANDS_SOURCE.contains("invokeEditorCommand(command.id, context)"));
    assert!(!EDITOR_COMMANDS_SOURCE.contains(".click()"));
}
