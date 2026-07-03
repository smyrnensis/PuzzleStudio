pub fn export_loaded_document_visual_fixture_json(
    document: &LoadedDocument,
) -> Result<String, DiagnosticReport> {
    let Some(LoadedDocumentModel::Puzzle3d { puzzle, .. }) = document.single_model() else {
        return Err(DiagnosticReport::error(
            "visual fixture export currently requires a single puzzle3 model".to_string(),
        ));
    };
    let (scene_fields, level_bundle_names) = puzzle3_scene_fixture_fields(document);
    export_visual_fixture_json_with_title_scenes_and_animation(
        puzzle,
        Some(&document.title),
        scene_fields.as_deref(),
        &level_bundle_names,
        VisualFixtureAnimation3 {
            tween_enabled: document.animation.tween.enabled,
            tween_interval_ms: document.animation.tween.interval_ms,
        },
    )
    .map_err(|error| {
        DiagnosticReport::error(format!("failed to export puzzle3 fixture: {error:?}"))
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub fn parse_game_file(path: impl AsRef<Path>) -> Result<LoadedDocument, DiagnosticReport> {
    let path = path.as_ref();
    let source = fs::read_to_string(path).map_err(|error| {
        DiagnosticReport::error(format!("failed to read {}: {error}", path.display()))
    })?;
    let profile = puzzle_source_profile_for_path(path).ok_or_else(|| {
        DiagnosticReport::error(format!(
            "game entry must be a .puzzle or .puzzle3 file: {}",
            path.display()
        ))
    })?;
    validate_source_profile(&source, profile)?;
    if profile == PuzzleSourceProfile::Puzzle3d {
        return parse_game_document(&source);
    }
    let expanded = expand_game_imports_for_file(&source, path)?;
    validate_source_profile(&expanded, profile)?;
    parse_game_document(&expanded)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn parse_game2d_file(path: impl AsRef<Path>) -> Result<LoadedGame, DiagnosticReport> {
    let path = path.as_ref();
    if puzzle_source_profile_for_path(path) != Some(PuzzleSourceProfile::Puzzle2d) {
        return Err(DiagnosticReport::error(format!(
            "2D game entry must be a .puzzle file: {}",
            path.display()
        )));
    }
    let source = fs::read_to_string(path).map_err(|error| {
        DiagnosticReport::error(format!("failed to read {}: {error}", path.display()))
    })?;
    let expanded = expand_game_imports_for_file(&source, path)?;
    parse_game2d_document(&expanded)
}

fn parse_game2d_document(source: &str) -> Result<LoadedGame, DiagnosticReport> {
    let parts = parse_document_source_parts(source)?;
    parse_game2d_from_document_parts(parts)
}

fn parse_game2d_from_document_parts(
    parts: DocumentSourceParts,
) -> Result<LoadedGame, DiagnosticReport> {
    let mut scenes = parts.scenes;
    let model_names = all_model_names(&parts.model_source, "puzzle");
    resolve_inferred_scene_puzzle_slots(
        &mut scenes,
        model_names.iter().map(|name| ("puzzle", name)),
    )?;
    let mut game = parse_game2d_expanded_lines_with_shell(parts.model_lines, &parts.shell)?;
    resolve_default_wait_in_scenes(&mut scenes, game.default_wait_ms);
    game.scenes =
        add_implicit_model_scenes(scenes, model_names.iter().map(|name| ("puzzle", name)));
    resolve_scene_actions(&mut game.scenes, &game.input_labels)?;
    add_scene_input_key_controls(&game.scenes, &game.input_labels, &mut game.controls);
    Ok(game)
}

fn parse_game_document(source: &str) -> Result<LoadedDocument, DiagnosticReport> {
    match detect_game_document_kind(source)? {
        GameDocumentKind::Puzzle2d => {
            let parts = parse_document_source_parts(source)?;
            let name = first_model_name(&parts.model_source, "puzzle")
                .unwrap_or_else(|| "default".to_string());
            let shell = parts.shell.clone();
            let game = parse_game2d_from_document_parts(parts)?;
            Ok(LoadedDocument {
                title: shell.title,
                subtitle: shell.subtitle,
                author: shell.author,
                homepage: shell.homepage,
                default_wait_ms: shell.default_wait_ms,
                default_again_ms: shell.default_again_ms,
                animation: shell.animation,
                sounds: shell.sounds,
                theme: shell.theme,
                assets: shell.assets,
                scenes: game.scenes.clone(),
                models: vec![LoadedDocumentModel::Puzzle2d {
                    name,
                    game: game.clone(),
                }],
            })
        }
        GameDocumentKind::Puzzle3d => {
            let parts = parse_document_source_parts(source)?;
            let name = first_model_name(&parts.raw_model_source_without_shell, "puzzle3")
                .unwrap_or_else(|| "default".to_string());
            let mut scenes = parts.scenes;
            resolve_inferred_scene_puzzle_slots(&mut scenes, std::iter::once(("puzzle3", &name)))?;
            let puzzle = parse_puzzle3d(&parts.raw_model_source_without_shell).map_err(
                |error| match error {
                    ParseError3::Message(message) => DiagnosticReport::error(message),
                },
            )?;
            let mut scenes = add_implicit_model_scenes(scenes, std::iter::once(("puzzle3", &name)));
            resolve_scene_actions(&mut scenes, &HashMap::new())?;
            Ok(LoadedDocument {
                title: parts.shell.title,
                subtitle: parts.shell.subtitle,
                author: parts.shell.author,
                homepage: parts.shell.homepage,
                default_wait_ms: parts.shell.default_wait_ms,
                default_again_ms: parts.shell.default_again_ms,
                animation: parts.shell.animation,
                sounds: parts.shell.sounds,
                theme: parts.shell.theme,
                assets: parts.shell.assets,
                scenes,
                models: vec![LoadedDocumentModel::Puzzle3d { name, puzzle }],
            })
        }
        GameDocumentKind::Mixed => parse_mixed_game_document(source),
    }
}

fn parse_mixed_game_document(source: &str) -> Result<LoadedDocument, DiagnosticReport> {
    let parts = parse_document_source_parts(source)?;
    let sources = split_mixed_game_document_source(source)?;
    let model_2d_name =
        first_model_name(&sources.puzzle2d, "puzzle").unwrap_or_else(|| "default".to_string());
    let game_2d_source = strip_document_shell_source(&sources.puzzle2d)?;
    let game_2d = parse_game2d_expanded_with_shell(&game_2d_source, &parts.shell)?;
    let model_3d_name =
        first_model_name(&sources.puzzle3d, "puzzle3").unwrap_or_else(|| "default".to_string());
    let puzzle_3d_source = strip_document_shell_source_raw(&sources.puzzle3d);
    let puzzle_3d = parse_puzzle3d(&puzzle_3d_source).map_err(|error| match error {
        ParseError3::Message(message) => DiagnosticReport::error(message),
    })?;
    let mut scenes = parts.scenes;
    resolve_inferred_scene_puzzle_slots(
        &mut scenes,
        [("puzzle", &model_2d_name), ("puzzle3", &model_3d_name)].into_iter(),
    )?;

    let mut scenes = add_implicit_model_scenes(
        scenes,
        [("puzzle", &model_2d_name), ("puzzle3", &model_3d_name)].into_iter(),
    );
    resolve_scene_actions(&mut scenes, &game_2d.input_labels)?;

    Ok(LoadedDocument {
        title: parts.shell.title,
        subtitle: parts.shell.subtitle,
        author: parts.shell.author,
        homepage: parts.shell.homepage,
        default_wait_ms: parts.shell.default_wait_ms,
        default_again_ms: parts.shell.default_again_ms,
        animation: parts.shell.animation,
        sounds: parts.shell.sounds,
        theme: parts.shell.theme,
        assets: parts.shell.assets,
        scenes,
        models: vec![
            LoadedDocumentModel::Puzzle2d {
                name: model_2d_name,
                game: game_2d,
            },
            LoadedDocumentModel::Puzzle3d {
                name: model_3d_name,
                puzzle: puzzle_3d,
            },
        ],
    })
}

#[derive(Default)]
struct MixedDocumentSources {
    puzzle2d: String,
    puzzle3d: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MixedSectionTarget {
    Puzzle2d,
    Puzzle3d,
    Shared,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MixedSectionRecognition {
    Target(MixedSectionTarget),
    Scene,
}

fn classify_known_mixed_document_section(tokens: &[&str]) -> Option<MixedSectionRecognition> {
    let target = match tokens {
        ["title", ..]
        | ["subtitle", ..]
        | ["author", ..]
        | ["homepage", ..]
        | ["default_wait_time", ..]
        | ["again_interval", ..]
        | ["animation", ..]
        | ["sounds", ..]
        | ["theme", ..]
        | ["assets", ..] => MixedSectionTarget::Shared,
        ["puzzle", ..] | ["levels", ..] | ["sprites", ..] | ["level", ..] => {
            MixedSectionTarget::Puzzle2d
        }
        ["puzzle3", ..] | ["levels3", ..] | ["sprites3", ..] => MixedSectionTarget::Puzzle3d,
        ["var", ..] | ["const", ..] | ["persistent", ..] => MixedSectionTarget::Puzzle2d,
        ["scene", ..] => return Some(MixedSectionRecognition::Scene),
        _ => return None,
    };
    Some(MixedSectionRecognition::Target(target))
}

fn split_mixed_game_document_source(
    source: &str,
) -> Result<MixedDocumentSources, DiagnosticReport> {
    let raw_lines = source.lines().collect::<Vec<_>>();
    let mut sources = MixedDocumentSources::default();
    let mut index = 0usize;
    while index < raw_lines.len() {
        let line = raw_lines[index];
        let trimmed = strip_line_comment(line).trim();
        if trimmed.is_empty() {
            push_raw_line(&mut sources.puzzle2d, line);
            push_raw_line(&mut sources.puzzle3d, line);
            index += 1;
            continue;
        }

        let tokens = split_header_tokens(trimmed);
        let target = match classify_known_mixed_document_section(tokens.as_slice()) {
            Some(MixedSectionRecognition::Target(target)) => target,
            Some(MixedSectionRecognition::Scene) => {
                let next = skip_raw_top_level_block(&raw_lines, index);
                index = next;
                continue;
            }
            None => MixedSectionTarget::Puzzle2d,
        };
        let is_block = mixed_section_is_block(trimmed);
        let next = if is_block {
            skip_raw_top_level_block(&raw_lines, index)
        } else {
            index + 1
        };
        if is_block && matches!(tokens.as_slice(), ["puzzle", ..] | ["puzzle3", ..]) {
            push_raw_model_block_without_default_scene_layouts(
                &raw_lines,
                index,
                target,
                &mut sources,
            );
        } else {
            push_raw_block(&raw_lines, index, next, target, &mut sources);
        }
        index = next;
    }
    Ok(sources)
}

fn push_raw_model_block_without_default_scene_layouts(
    raw_lines: &[&str],
    start: usize,
    target: MixedSectionTarget,
    sources: &mut MixedDocumentSources,
) {
    let mut stripped = Vec::new();
    push_raw_model_without_default_scene_layouts(raw_lines, start, &mut stripped);
    for line in stripped {
        match target {
            MixedSectionTarget::Puzzle2d => push_raw_line(&mut sources.puzzle2d, line),
            MixedSectionTarget::Puzzle3d => push_raw_line(&mut sources.puzzle3d, line),
            MixedSectionTarget::Shared => {
                push_raw_line(&mut sources.puzzle2d, line);
                push_raw_line(&mut sources.puzzle3d, line);
            }
        }
    }
}

fn mixed_section_is_block(trimmed: &str) -> bool {
    trimmed.ends_with('{')
}

fn push_raw_block(
    raw_lines: &[&str],
    start: usize,
    end: usize,
    target: MixedSectionTarget,
    sources: &mut MixedDocumentSources,
) {
    for line in &raw_lines[start..end] {
        match target {
            MixedSectionTarget::Puzzle2d => push_raw_line(&mut sources.puzzle2d, line),
            MixedSectionTarget::Puzzle3d => push_raw_line(&mut sources.puzzle3d, line),
            MixedSectionTarget::Shared => {
                push_raw_line(&mut sources.puzzle2d, line);
                push_raw_line(&mut sources.puzzle3d, line);
            }
        }
    }
}

fn push_raw_line(target: &mut String, line: &str) {
    if !target.is_empty() {
        target.push('\n');
    }
    target.push_str(line);
}

fn add_implicit_model_scenes<'a>(
    mut scenes: Vec<SceneDef>,
    models: impl IntoIterator<Item = (&'a str, &'a String)>,
) -> Vec<SceneDef> {
    let mut existing = scenes
        .iter()
        .map(|scene| scene.name.clone())
        .collect::<HashSet<_>>();
    for (kind, model_name) in models {
        if existing.contains(model_name) {
            continue;
        }
        scenes.push(implicit_model_scene(kind, model_name));
        existing.insert(model_name.clone());
    }
    scenes
}

const INFERRED_SCENE_PUZZLE_KIND: &str = "__model__";

fn resolve_inferred_scene_puzzle_slots<'a>(
    scenes: &mut [SceneDef],
    models: impl IntoIterator<Item = (&'a str, &'a String)>,
) -> Result<(), DiagnosticReport> {
    let mut model_kinds = HashMap::<String, String>::new();
    let mut ambiguous = HashSet::<String>::new();
    for (kind, model_name) in models {
        match model_kinds.get(model_name) {
            Some(existing) if existing != kind => {
                ambiguous.insert(model_name.clone());
            }
            Some(_) => {}
            None => {
                model_kinds.insert(model_name.clone(), kind.to_string());
            }
        }
    }

    for scene in scenes {
        for puzzle in &mut scene.state.puzzles {
            if puzzle.kind != INFERRED_SCENE_PUZZLE_KIND {
                continue;
            }
            if ambiguous.contains(&puzzle.model) {
                return Err(DiagnosticReport::error(format!(
                    "scene puzzle slot `{}` is ambiguous; use `puzzle <name>` or `puzzle3 <name>`",
                    puzzle.model
                )));
            }
            let Some(kind) = model_kinds.get(&puzzle.model) else {
                return Err(DiagnosticReport::error(format!(
                    "scene puzzle slot `{}` does not match a puzzle model",
                    puzzle.model
                )));
            };
            puzzle.kind = kind.clone();
        }
    }

    Ok(())
}

fn implicit_model_scene(kind: &str, model_name: &str) -> SceneDef {
    SceneDef {
        name: model_name.to_string(),
        layout: SceneLayoutDef::default(),
        resources: SceneResources::default(),
        state: SceneStateDef {
            variables: Vec::new(),
            puzzles: vec![ScenePuzzleDef {
                name: model_name.to_string(),
                kind: kind.to_string(),
                model: model_name.to_string(),
                initializer: ScenePuzzleInitializer::CurrentLevel,
                lifetime: SceneStateLifetime::Instance,
            }],
        },
        components: vec![scene_frame_component(kind, model_name)],
        key_bindings: Vec::new(),
        routines: Vec::new(),
        transitions: Vec::new(),
        puzzle_rule: Some(ScenePuzzleRule {
            target: model_name.to_string(),
            rule: "rules".to_string(),
        }),
    }
}

fn puzzle3_scene_fixture_fields(document: &LoadedDocument) -> (Option<String>, Vec<String>) {
    if document.scenes.is_empty() {
        return (None, Vec::new());
    }
    let mut level_bundle_names = Vec::new();
    let current_scene = document
        .scenes
        .iter()
        .find(|scene| scene.name == "title")
        .or_else(|| document.scenes.first())
        .map(|scene| scene.name.as_str())
        .unwrap_or("playing");
    let mut out = String::new();
    out.push_str("  \"currentScene\": ");
    out.push_str(&json_string(current_scene));
    out.push_str(",\n  \"scenes\": [\n");
    for (index, scene) in document.scenes.iter().enumerate() {
        if index > 0 {
            out.push_str(",\n");
        }
        push_puzzle3_scene_json(&mut out, scene, &mut level_bundle_names);
    }
    out.push_str("\n  ],");
    (Some(out), level_bundle_names)
}

fn push_puzzle3_scene_json(
    out: &mut String,
    scene: &SceneDef,
    level_bundle_names: &mut Vec<String>,
) {
    out.push_str("    {\n");
    out.push_str("      \"name\": ");
    out.push_str(&json_string(&scene.name));
    out.push_str(",\n      \"layout\": ");
    push_puzzle3_layout_json(out, &scene.layout);
    out.push_str(",\n      \"puzzles\": [");
    let mut wrote_puzzle = false;
    for puzzle in &scene.state.puzzles {
        if puzzle.kind != "puzzle3" {
            continue;
        }
        if wrote_puzzle {
            out.push_str(", ");
        }
        wrote_puzzle = true;
        out.push_str("{ \"slot\": ");
        out.push_str(&json_string(&puzzle.name));
        out.push_str(", \"model\": ");
        out.push_str(&json_string(&puzzle.model));
        out.push_str(" }");
    }
    out.push_str("],\n      \"keys\": {");
    let mut wrote_key = false;
    for binding in &scene.key_bindings {
        let Some(action) = puzzle3_scene_action_json(&binding.effect, level_bundle_names) else {
            continue;
        };
        for key in &binding.keys {
            if wrote_key {
                out.push_str(", ");
            }
            wrote_key = true;
            out.push_str(&json_string(&key_trigger_name(key)));
            out.push_str(": ");
            out.push_str(&action);
        }
    }
    out.push_str("},\n      \"components\": [");
    let mut wrote_component = false;
    for component in &scene.components {
        if let Some(component_json) = puzzle3_scene_component_json(component, level_bundle_names) {
            if wrote_component {
                out.push_str(", ");
            }
            wrote_component = true;
            out.push_str(&component_json);
        }
    }
    out.push_str("]\n    }");
}

fn puzzle3_scene_component_json(
    component: &SceneComponent,
    level_bundle_names: &mut Vec<String>,
) -> Option<String> {
    match component {
        SceneComponent::Frame(frame) if frame.kind == "puzzle3" => {
            let mut out = format!(
                "{{ \"kind\": \"puzzle3\", \"source\": {}",
                json_string(&frame.source)
            );
            push_puzzle3_inline_layout_json(&mut out, &frame.layout);
            out.push_str(" }");
            Some(out)
        }
        SceneComponent::Title(title) => {
            let mut out = format!(
                "{{ \"kind\": \"title\", \"text\": {}",
                json_string(&scene_expr_fixture_text(&title.content))
            );
            push_puzzle3_inline_layout_json(&mut out, &title.layout);
            out.push_str(" }");
            Some(out)
        }
        SceneComponent::Button(button) | SceneComponent::Choice(button) => {
            let action = puzzle3_scene_action_json(&button.effect, level_bundle_names)?;
            let kind = match component {
                SceneComponent::Choice(_) => "choice",
                _ => "button",
            };
            let mut out = format!(
                "{{ \"kind\": {}, \"label\": {}, \"action\": {}",
                json_string(kind),
                puzzle3_scene_expr_json(&button.label),
                action
            );
            push_puzzle3_inline_layout_json(&mut out, &button.layout);
            out.push_str(" }");
            Some(out)
        }
        SceneComponent::LevelMenu(menu) => {
            let levels = menu.source.as_deref().unwrap_or("levels");
            push_unique_string(level_bundle_names, levels);
            let action = menu
                .action
                .as_ref()
                .and_then(|effect| puzzle3_scene_action_json(effect, level_bundle_names))
                .unwrap_or_else(|| {
                    "{ \"kind\": \"goto\", \"scene\": \"playing\", \"params\": [{ \"kind\": \"level\", \"value\": { \"kind\": \"path\", \"path\": \"level\" } }] }".to_string()
                });
            let mut out = format!(
                "{{ \"kind\": \"level_menu\", \"levels\": {}, \"action\": {}",
                json_string(levels),
                action
            );
            push_puzzle3_inline_layout_json(&mut out, &menu.layout);
            out.push_str(" }");
            Some(out)
        }
        SceneComponent::Row(container) => puzzle3_container_json(
            "row",
            &container.children,
            &container.layout,
            level_bundle_names,
        ),
        SceneComponent::Column(container) => puzzle3_container_json(
            "column",
            &container.children,
            &container.layout,
            level_bundle_names,
        ),
        SceneComponent::Box(container) => puzzle3_container_json(
            "box",
            &container.children,
            &container.layout,
            level_bundle_names,
        ),
        SceneComponent::Conditional(conditional) => {
            let mut out = format!(
                "{{ \"kind\": \"conditional\", \"condition\": {}, \"children\": [",
                json_string(&conditional.condition)
            );
            let mut wrote = false;
            for child in &conditional.children {
                if let Some(child_json) = puzzle3_scene_component_json(child, level_bundle_names) {
                    if wrote {
                        out.push_str(", ");
                    }
                    wrote = true;
                    out.push_str(&child_json);
                }
            }
            out.push_str("], \"elseChildren\": [");
            wrote = false;
            for child in &conditional.else_children {
                if let Some(child_json) = puzzle3_scene_component_json(child, level_bundle_names) {
                    if wrote {
                        out.push_str(", ");
                    }
                    wrote = true;
                    out.push_str(&child_json);
                }
            }
            out.push_str("] }");
            Some(out)
        }
        SceneComponent::For(for_view) => {
            let mut out = format!(
                "{{ \"kind\": \"for\", \"binding\": {}, \"source\": {}, \"children\": [",
                json_string(&for_view.binding),
                json_string(for_view.source.as_str())
            );
            let mut wrote = false;
            for child in &for_view.children {
                if let Some(child_json) = puzzle3_scene_component_json(child, level_bundle_names) {
                    if wrote {
                        out.push_str(", ");
                    }
                    wrote = true;
                    out.push_str(&child_json);
                }
            }
            out.push_str("] }");
            Some(out)
        }
        _ => None,
    }
}

fn puzzle3_container_json(
    kind: &str,
    children: &[SceneComponent],
    layout: &SceneLayoutDef,
    level_bundle_names: &mut Vec<String>,
) -> Option<String> {
    let mut out = format!("{{ \"kind\": {}, \"children\": [", json_string(kind));
    let mut wrote = false;
    for child in children {
        if let Some(child_json) = puzzle3_scene_component_json(child, level_bundle_names) {
            if wrote {
                out.push_str(", ");
            }
            wrote = true;
            out.push_str(&child_json);
        }
    }
    out.push(']');
    push_puzzle3_inline_layout_json(&mut out, layout);
    out.push_str(" }");
    Some(out)
}

fn puzzle3_scene_action_json(
    effect: &SceneEffect,
    _level_bundle_names: &mut Vec<String>,
) -> Option<String> {
    match effect {
        SceneEffect::Goto { scene, params } => {
            let mut out = format!("{{ \"kind\": \"goto\", \"scene\": {}", json_string(scene));
            if !params.is_empty() {
                out.push_str(", \"params\": [");
                for (index, param) in params.iter().enumerate() {
                    if index > 0 {
                        out.push_str(", ");
                    }
                    match param {
                        SceneEffectParam::Level(value) => {
                            out.push_str("{ \"kind\": \"level\", \"value\": ");
                            out.push_str(&puzzle3_scene_expr_json(value));
                            out.push_str(" }");
                        }
                        SceneEffectParam::Named { name, value } => {
                            out.push_str("{ \"kind\": \"named\", \"name\": ");
                            out.push_str(&json_string(name));
                            out.push_str(", \"value\": ");
                            out.push_str(&puzzle3_scene_expr_json(value));
                            out.push_str(" }");
                        }
                    }
                }
                out.push(']');
            }
            out.push_str(" }");
            Some(out)
        }
        _ => None,
    }
}

fn puzzle3_scene_expr_json(expr: &SceneExpr) -> String {
    match expr {
        SceneExpr::Bool(value) => format!("{{ \"kind\": \"bool\", \"value\": {value} }}"),
        SceneExpr::Int(value) => format!("{{ \"kind\": \"int\", \"value\": {value} }}"),
        SceneExpr::Text(value) => format!(
            "{{ \"kind\": \"text\", \"value\": {} }}",
            json_string(value)
        ),
        SceneExpr::Path(path) => {
            format!(
                "{{ \"kind\": \"path\", \"path\": {} }}",
                json_string(&path.join("."))
            )
        }
        SceneExpr::Call { name, args } => {
            let mut out = format!(
                "{{ \"kind\": \"call\", \"name\": {}, \"args\": [",
                json_string(name)
            );
            for (index, arg) in args.iter().enumerate() {
                if index > 0 {
                    out.push_str(", ");
                }
                out.push_str(&puzzle3_scene_expr_json(arg));
            }
            out.push_str("] }");
            out
        }
    }
}

fn scene_expr_fixture_text(expr: &SceneExpr) -> String {
    match expr {
        SceneExpr::Text(value) => value.clone(),
        SceneExpr::Path(path) => path.join("."),
        SceneExpr::Int(value) => value.to_string(),
        SceneExpr::Bool(value) => value.to_string(),
        SceneExpr::Call { name, .. } => name.clone(),
    }
}

fn push_puzzle3_inline_layout_json(out: &mut String, layout: &SceneLayoutDef) {
    if layout.size.is_none()
        && layout.gap.is_none()
        && layout.align == SceneLayoutDef::default().align
        && !layout.scroll
    {
        return;
    }
    out.push_str(", \"layout\": ");
    push_puzzle3_layout_json(out, layout);
}

fn push_puzzle3_layout_json(out: &mut String, layout: &SceneLayoutDef) {
    out.push('{');
    let mut wrote = false;
    if let Some(size) = layout.size {
        out.push_str("\"size\": { \"width\": ");
        out.push_str(&size.width.to_string());
        out.push_str(", \"height\": ");
        out.push_str(&size.height.to_string());
        out.push_str(" }");
        wrote = true;
    }
    if let Some(gap) = layout.gap {
        if wrote {
            out.push_str(", ");
        }
        out.push_str("\"gap\": ");
        out.push_str(&gap.to_string());
        wrote = true;
    }
    if layout.align != SceneLayoutDef::default().align {
        if wrote {
            out.push_str(", ");
        }
        out.push_str("\"align\": { \"x\": ");
        out.push_str(&json_string(match layout.align.x {
            SceneAlignXDef::Left => "left",
            SceneAlignXDef::Center => "center",
            SceneAlignXDef::Right => "right",
        }));
        out.push_str(", \"y\": ");
        out.push_str(&json_string(match layout.align.y {
            SceneAlignYDef::Top => "top",
            SceneAlignYDef::Center => "center",
            SceneAlignYDef::Bottom => "bottom",
        }));
        out.push_str(" }");
        wrote = true;
    }
    if layout.scroll {
        if wrote {
            out.push_str(", ");
        }
        out.push_str("\"scroll\": true");
    }
    out.push('}');
}

fn json_string(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}

fn push_unique_string(values: &mut Vec<String>, value: &str) {
    if !value.is_empty() && !values.iter().any(|existing| existing == value) {
        values.push(value.to_string());
    }
}

fn key_trigger_name(key: &KeyTrigger) -> String {
    match key {
        KeyTrigger::Char(ch) => ch.to_string(),
        KeyTrigger::Named(name) => name.clone(),
    }
}

#[derive(Clone, Debug)]
struct DocumentShell {
    title: String,
    subtitle: Option<String>,
    author: Option<String>,
    homepage: Option<String>,
    default_wait_ms: u64,
    default_again_ms: u64,
    animation: AnimationDef,
    sounds: SoundsDef,
    theme: ThemeDef,
    assets: AssetsDef,
}

#[derive(Clone, Debug)]
struct DocumentSourceParts {
    shell: DocumentShell,
    model_source: String,
    model_lines: Vec<source::LogicalLine>,
    raw_model_source_without_shell: String,
    scenes: Vec<SceneDef>,
}

impl Default for DocumentShell {
    fn default() -> Self {
        Self {
            title: "ASCII play".to_string(),
            subtitle: None,
            author: None,
            homepage: None,
            default_wait_ms: DEFAULT_WAIT_MS,
            default_again_ms: DEFAULT_AGAIN_MS,
            animation: AnimationDef::default(),
            sounds: SoundsDef::default(),
            theme: ThemeDef::default(),
            assets: AssetsDef::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ModelTopLevelDirective {
    Puzzle,
    RemovedModelPrefix,
    RemovedNameMetadata,
    Title,
    Subtitle,
    Author,
    Homepage,
    Variable,
    DefaultWaitTime,
    AgainInterval,
    Animation,
    Scene,
    Sounds,
    Theme,
    Assets,
    Close,
    Sprites,
    Levels,
    Level,
    PuzzleLifecycle,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ModelTopLevelExpectedGroup {
    Metadata,
    Variables,
    Model,
    Content,
    Config,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct HeaderChoiceAlternative<Action, ExpectedGroup> {
    trigger: &'static str,
    label: &'static str,
    action: Action,
    expected_group: Option<ExpectedGroup>,
    authoring_surface: bool,
}

const MODEL_TOP_LEVEL_ALTERNATIVES: &[HeaderChoiceAlternative<
    ModelTopLevelDirective,
    ModelTopLevelExpectedGroup,
>] = &[
    HeaderChoiceAlternative {
        trigger: "puzzle",
        label: "puzzle",
        action: ModelTopLevelDirective::Puzzle,
        expected_group: Some(ModelTopLevelExpectedGroup::Model),
        authoring_surface: true,
    },
    HeaderChoiceAlternative {
        trigger: "model",
        label: "model",
        action: ModelTopLevelDirective::RemovedModelPrefix,
        expected_group: None,
        authoring_surface: false,
    },
    HeaderChoiceAlternative {
        trigger: "name",
        label: "name",
        action: ModelTopLevelDirective::RemovedNameMetadata,
        expected_group: None,
        authoring_surface: false,
    },
    HeaderChoiceAlternative {
        trigger: "title",
        label: "title",
        action: ModelTopLevelDirective::Title,
        expected_group: Some(ModelTopLevelExpectedGroup::Metadata),
        authoring_surface: true,
    },
    HeaderChoiceAlternative {
        trigger: "subtitle",
        label: "subtitle",
        action: ModelTopLevelDirective::Subtitle,
        expected_group: Some(ModelTopLevelExpectedGroup::Metadata),
        authoring_surface: true,
    },
    HeaderChoiceAlternative {
        trigger: "author",
        label: "author",
        action: ModelTopLevelDirective::Author,
        expected_group: Some(ModelTopLevelExpectedGroup::Metadata),
        authoring_surface: true,
    },
    HeaderChoiceAlternative {
        trigger: "homepage",
        label: "homepage",
        action: ModelTopLevelDirective::Homepage,
        expected_group: Some(ModelTopLevelExpectedGroup::Metadata),
        authoring_surface: true,
    },
    HeaderChoiceAlternative {
        trigger: "var",
        label: "var",
        action: ModelTopLevelDirective::Variable,
        expected_group: Some(ModelTopLevelExpectedGroup::Variables),
        authoring_surface: true,
    },
    HeaderChoiceAlternative {
        trigger: "const",
        label: "const",
        action: ModelTopLevelDirective::Variable,
        expected_group: Some(ModelTopLevelExpectedGroup::Variables),
        authoring_surface: true,
    },
    HeaderChoiceAlternative {
        trigger: "persistent",
        label: "persistent var",
        action: ModelTopLevelDirective::Variable,
        expected_group: Some(ModelTopLevelExpectedGroup::Variables),
        authoring_surface: true,
    },
    HeaderChoiceAlternative {
        trigger: "default_wait_time",
        label: "default_wait_time",
        action: ModelTopLevelDirective::DefaultWaitTime,
        expected_group: Some(ModelTopLevelExpectedGroup::Config),
        authoring_surface: true,
    },
    HeaderChoiceAlternative {
        trigger: "again_interval",
        label: "again_interval",
        action: ModelTopLevelDirective::AgainInterval,
        expected_group: Some(ModelTopLevelExpectedGroup::Config),
        authoring_surface: true,
    },
    HeaderChoiceAlternative {
        trigger: "animation",
        label: "animation",
        action: ModelTopLevelDirective::Animation,
        expected_group: Some(ModelTopLevelExpectedGroup::Config),
        authoring_surface: true,
    },
    HeaderChoiceAlternative {
        trigger: "scene",
        label: "scene",
        action: ModelTopLevelDirective::Scene,
        expected_group: None,
        authoring_surface: true,
    },
    HeaderChoiceAlternative {
        trigger: "sounds",
        label: "sounds",
        action: ModelTopLevelDirective::Sounds,
        expected_group: Some(ModelTopLevelExpectedGroup::Content),
        authoring_surface: true,
    },
    HeaderChoiceAlternative {
        trigger: "theme",
        label: "theme",
        action: ModelTopLevelDirective::Theme,
        expected_group: Some(ModelTopLevelExpectedGroup::Content),
        authoring_surface: true,
    },
    HeaderChoiceAlternative {
        trigger: "assets",
        label: "assets",
        action: ModelTopLevelDirective::Assets,
        expected_group: Some(ModelTopLevelExpectedGroup::Content),
        authoring_surface: true,
    },
    HeaderChoiceAlternative {
        trigger: BLOCK_CLOSE,
        label: BLOCK_CLOSE,
        action: ModelTopLevelDirective::Close,
        expected_group: None,
        authoring_surface: false,
    },
    HeaderChoiceAlternative {
        trigger: "sprites",
        label: "sprites",
        action: ModelTopLevelDirective::Sprites,
        expected_group: Some(ModelTopLevelExpectedGroup::Content),
        authoring_surface: true,
    },
    HeaderChoiceAlternative {
        trigger: "levels",
        label: "levels",
        action: ModelTopLevelDirective::Levels,
        expected_group: Some(ModelTopLevelExpectedGroup::Content),
        authoring_surface: true,
    },
    HeaderChoiceAlternative {
        trigger: "level",
        label: "level",
        action: ModelTopLevelDirective::Level,
        expected_group: Some(ModelTopLevelExpectedGroup::Content),
        authoring_surface: true,
    },
];

fn classify_header_choice<Action: Copy, ExpectedGroup>(
    alternatives: &[HeaderChoiceAlternative<Action, ExpectedGroup>],
    trigger: &str,
) -> Option<Action> {
    alternatives
        .iter()
        .find(|alternative| alternative.trigger == trigger)
        .map(|alternative| alternative.action)
}

fn format_header_choice_expected_group<Action, ExpectedGroup: Copy + PartialEq>(
    alternatives: &[HeaderChoiceAlternative<Action, ExpectedGroup>],
    group: ExpectedGroup,
) -> String {
    alternatives
        .iter()
        .filter(|alternative| alternative.expected_group == Some(group))
        .map(|alternative| format!("`{}`", alternative.label))
        .collect::<Vec<_>>()
        .join(", ")
}

fn classify_model_top_level_directive(tokens: &[&str]) -> ModelTopLevelDirective {
    let Some(first) = tokens.first().copied() else {
        return ModelTopLevelDirective::Unknown;
    };
    if puzzle_lifecycle_event(first).is_some() {
        return ModelTopLevelDirective::PuzzleLifecycle;
    }
    classify_header_choice(MODEL_TOP_LEVEL_ALTERNATIVES, first)
        .unwrap_or(ModelTopLevelDirective::Unknown)
}

fn model_top_level_surface_directive(token: &str) -> Option<ModelTopLevelDirective> {
    MODEL_TOP_LEVEL_ALTERNATIVES
        .iter()
        .find(|alternative| alternative.trigger == token && alternative.authoring_surface)
        .map(|alternative| alternative.action)
}

pub(crate) fn model_top_level_completion_keywords() -> Vec<&'static str> {
    MODEL_TOP_LEVEL_ALTERNATIVES
        .iter()
        .filter(|alternative| alternative.authoring_surface)
        .map(|alternative| alternative.trigger)
        .collect()
}

fn format_model_top_level_expected_group(group: ModelTopLevelExpectedGroup) -> String {
    format_header_choice_expected_group(MODEL_TOP_LEVEL_ALTERNATIVES, group)
}

fn model_top_level_expected_directives_message() -> String {
    format!(
        "metadata ({}), variables ({}), a model ({}), content ({}), or config ({})",
        format_model_top_level_expected_group(ModelTopLevelExpectedGroup::Metadata),
        format_model_top_level_expected_group(ModelTopLevelExpectedGroup::Variables),
        format_model_top_level_expected_group(ModelTopLevelExpectedGroup::Model),
        format_model_top_level_expected_group(ModelTopLevelExpectedGroup::Content),
        format_model_top_level_expected_group(ModelTopLevelExpectedGroup::Config),
    )
}

fn unknown_model_top_level_directive_message(other: &str) -> String {
    format!(
        "unknown top-level directive `{other}`; expected {}",
        model_top_level_expected_directives_message()
    )
}

fn misplaced_puzzle_lifecycle_message(lifecycle_block: &str) -> String {
    format!(
        "{lifecycle_block} is a puzzle lifecycle block; put it inside `puzzle <name> {{ ... }}` next to `rules {{ ... }}`"
    )
}

fn parse_document_source_parts(source: &str) -> Result<DocumentSourceParts, DiagnosticReport> {
    let shell = parse_document_shell(source)?;
    let (model_lines, scenes) = split_document_scene_lines(source)?;
    let model_lines = strip_document_shell_lines(&model_lines);
    let model_source = model_lines
        .iter()
        .map(|line| line.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let raw_model_source_without_shell =
        strip_document_scene_source_raw(&strip_document_shell_source_raw(source));
    Ok(DocumentSourceParts {
        shell,
        model_source,
        model_lines,
        raw_model_source_without_shell,
        scenes,
    })
}

fn parse_document_shell(source: &str) -> Result<DocumentShell, DiagnosticReport> {
    let mut shell = DocumentShell::default();
    let lines = logical_lines(source)?;
    let mut index = 0;
    while index < lines.len() {
        let tokens = split_header_tokens(&lines[index]);
        match tokens.as_slice() {
            ["title", ..] => {
                shell.title = parse_metadata_text(&lines[index], "title")?;
                index += 1;
            }
            ["subtitle", ..] => {
                shell.subtitle = Some(parse_metadata_text(&lines[index], "subtitle")?);
                index += 1;
            }
            ["author", ..] => {
                shell.author = Some(parse_metadata_text(&lines[index], "author")?);
                index += 1;
            }
            ["homepage", ..] => {
                shell.homepage = Some(parse_metadata_text(&lines[index], "homepage")?);
                index += 1;
            }
            ["default_wait_time", ..] => {
                shell.default_wait_ms = parse_default_wait_time_directive(&tokens, &lines[index])?;
                index += 1;
            }
            ["again_interval", ..] => {
                shell.default_again_ms = parse_again_interval_directive(&tokens, &lines[index])?;
                index += 1;
            }
            ["animation", ..] => {
                index = parse_animation_block(&lines, index, &mut shell.animation)?;
            }
            ["sounds"] => {
                if model_sounds_block_starts(&lines, index) {
                    index = skip_logical_block(&lines, index);
                } else {
                    index = parse_sounds_block(&lines, index, &mut shell.sounds)?;
                }
            }
            ["theme", name] if next_line_is_not_block_body(&lines, index) => {
                parse_theme_name_directive(&lines[index], name, &mut shell.theme)?;
                index += 1;
            }
            ["theme"] | ["theme", ..] => {
                index = parse_theme_statement(&lines, index, &mut shell.theme)?;
            }
            ["assets"] => {
                index = parse_assets_block(&lines, index, &mut shell.assets)?;
            }
            _ if logical_line_opens_block(tokens.as_slice()) => {
                index = skip_logical_block(&lines, index);
            }
            _ => {
                index += 1;
            }
        }
    }
    Ok(shell)
}

fn strip_document_shell_source(source: &str) -> Result<String, DiagnosticReport> {
    let context = scan_source_context(source);
    let mut out = Vec::new();
    let mut index = 0;
    let mut shell_prefix = true;
    while index < context.lines.len() {
        let line = &context.lines[index];
        let tokens = line.tokens.iter().map(String::as_str).collect::<Vec<_>>();
        if shell_prefix && line.scope.is_none() {
            match tokens.as_slice() {
                ["title", ..]
                | ["subtitle", ..]
                | ["author", ..]
                | ["homepage", ..]
                | ["default_wait_time", ..]
                | ["again_interval", ..] => {
                    index += 1;
                    continue;
                }
                ["animation", ..] | ["sounds", ..] | ["assets", ..] => {
                    index = skip_context_shell_block_by_syntax(&context, index);
                    continue;
                }
                ["theme", ..] => {
                    index = if context_theme_line_is_block(&context, index) {
                        skip_context_shell_block_by_syntax(&context, index)
                    } else {
                        index + 1
                    };
                    continue;
                }
                _ => {}
            }
            if !matches!(
                tokens.as_slice(),
                [] | ["var", ..]
                    | ["const", ..]
                    | ["persistent", "var", ..]
                    | ["title", ..]
                    | ["subtitle", ..]
                    | ["author", ..]
                    | ["homepage", ..]
                    | ["default_wait_time", ..]
                    | ["again_interval", ..]
                    | ["animation", ..]
                    | ["sounds", ..]
                    | ["assets", ..]
                    | ["theme", ..]
            ) {
                shell_prefix = false;
            }
        }

        out.push(line.content.clone());
        index += 1;
    }
    Ok(out.join("\n"))
}

fn strip_document_shell_lines(lines: &[source::LogicalLine]) -> Vec<source::LogicalLine> {
    let mut out = Vec::new();
    let mut index = 0;
    let mut shell_prefix = true;
    while index < lines.len() {
        let line = &lines[index];
        let tokens = split_header_tokens(&line.text);
        if shell_prefix {
            match tokens.as_slice() {
                ["title", ..]
                | ["subtitle", ..]
                | ["author", ..]
                | ["homepage", ..]
                | ["default_wait_time", ..]
                | ["again_interval", ..] => {
                    index += 1;
                    continue;
                }
                ["animation", ..] | ["sounds", ..] | ["assets", ..] => {
                    index = skip_shell_logical_block_by_syntax(lines, index);
                    continue;
                }
                ["theme", ..] => {
                    index = if logical_theme_line_is_block(lines, index) {
                        skip_shell_logical_block_by_syntax(lines, index)
                    } else {
                        index + 1
                    };
                    continue;
                }
                _ => {}
            }
            if !matches!(
                tokens.as_slice(),
                [] | ["var", ..]
                    | ["const", ..]
                    | ["persistent", "var", ..]
                    | ["title", ..]
                    | ["subtitle", ..]
                    | ["author", ..]
                    | ["homepage", ..]
                    | ["default_wait_time", ..]
                    | ["again_interval", ..]
                    | ["animation", ..]
                    | ["sounds", ..]
                    | ["assets", ..]
                    | ["theme", ..]
            ) {
                shell_prefix = false;
            }
        }

        out.push(line.clone());
        index += 1;
    }
    out
}

fn skip_shell_logical_block_by_syntax(lines: &[source::LogicalLine], index: usize) -> usize {
    let trimmed = strip_line_comment(&lines[index].text).trim();
    let mut next = index + 1;
    let mut brace_depth = raw_brace_delta(trimmed);
    if brace_depth > 0 {
        while next < lines.len() && brace_depth > 0 {
            let trimmed = strip_line_comment(&lines[next].text).trim();
            brace_depth += raw_brace_delta(trimmed);
            next += 1;
        }
        return next;
    }

    while next < lines.len() {
        let trimmed = strip_line_comment(&lines[next].text).trim();
        next += 1;
        if trimmed == BLOCK_CLOSE {
            break;
        }
    }
    next
}

fn skip_context_shell_block_by_syntax(context: &source::SourceContext, index: usize) -> usize {
    let trimmed = strip_line_comment(&context.lines[index].content).trim();
    let mut next = index + 1;
    let mut brace_depth = raw_brace_delta(trimmed);
    if brace_depth > 0 {
        while next < context.lines.len() && brace_depth > 0 {
            let trimmed = strip_line_comment(&context.lines[next].content).trim();
            brace_depth += raw_brace_delta(trimmed);
            next += 1;
        }
        return next;
    }

    while next < context.lines.len() {
        let trimmed = strip_line_comment(&context.lines[next].content).trim();
        next += 1;
        if trimmed == BLOCK_CLOSE {
            break;
        }
    }
    next
}

fn context_theme_line_is_block(context: &source::SourceContext, index: usize) -> bool {
    let trimmed = strip_line_comment(&context.lines[index].content).trim();
    if raw_brace_delta(trimmed) > 0 {
        return true;
    }
    context.lines.get(index + 1).is_some_and(|next| {
        let trimmed = strip_line_comment(&next.content).trim();
        trimmed == BLOCK_CLOSE || is_theme_setting_line(trimmed)
    })
}

fn logical_theme_line_is_block(lines: &[source::LogicalLine], index: usize) -> bool {
    let trimmed = strip_line_comment(&lines[index].text).trim();
    if raw_brace_delta(trimmed) > 0 {
        return true;
    }
    lines.get(index + 1).is_some_and(|next| {
        let trimmed = strip_line_comment(&next.text).trim();
        trimmed == BLOCK_CLOSE || is_theme_setting_line(trimmed)
    })
}

fn strip_document_shell_source_raw(source: &str) -> String {
    let raw_lines = source.lines().collect::<Vec<_>>();
    let mut out = Vec::new();
    let mut index = 0;
    let mut brace_depth = 0i32;
    while index < raw_lines.len() {
        let line = raw_lines[index];
        let trimmed = strip_line_comment(line).trim();
        if brace_depth == 0 {
            let tokens = split_header_tokens(trimmed);
            match tokens.as_slice() {
                ["title", ..]
                | ["subtitle", ..]
                | ["author", ..]
                | ["homepage", ..]
                | ["default_wait_time", ..]
                | ["again_interval", ..] => {
                    index += 1;
                    continue;
                }
                ["animation"] | ["animation", ..] | ["sounds"] | ["assets"] => {
                    index = skip_raw_top_level_block(&raw_lines, index);
                    continue;
                }
                ["theme", _] if !trimmed.ends_with('{') => {
                    index += 1;
                    continue;
                }
                ["theme"] | ["theme", ..] => {
                    index = skip_raw_top_level_block(&raw_lines, index);
                    continue;
                }
                _ => {}
            }
        }
        out.push(line);
        brace_depth += raw_brace_delta(trimmed);
        brace_depth = brace_depth.max(0);
        index += 1;
    }
    out.join("\n")
}

fn next_line_is_not_block_body(lines: &[String], index: usize) -> bool {
    let Some(next) = lines.get(index + 1) else {
        return true;
    };
    if is_block_close_line(next) {
        return true;
    }
    let tokens = split_header_tokens(next);
    logical_line_starts_document_boundary(tokens.as_slice())
}

fn logical_line_starts_document_boundary(tokens: &[&str]) -> bool {
    matches!(
        tokens,
        ["title", ..]
            | ["subtitle", ..]
            | ["author", ..]
            | ["homepage", ..]
            | ["default_wait_time", ..]
            | ["again_interval", ..]
            | ["puzzle", ..]
            | ["puzzle3", ..]
            | ["levels", ..]
            | ["levels3", ..]
            | ["sprites", ..]
            | ["sprites3", ..]
            | ["scene", ..]
            | ["sounds"]
            | ["theme", ..]
            | ["assets"]
    )
}

fn logical_line_opens_block(tokens: &[&str]) -> bool {
    matches!(
        tokens,
        ["puzzle", ..]
            | ["puzzle3", ..]
            | ["levels", ..]
            | ["levels3", ..]
            | ["sprites", ..]
            | ["sprites3", ..]
            | ["scene", ..]
            | ["state", ..]
            | ["layout", ..]
            | ["row", ..]
            | ["column", ..]
            | ["box", ..]
            | ["layers", ..]
            | ["tags", ..]
            | ["map", ..]
            | ["scratch", ..]
            | ["groups", ..]
            | ["legend", ..]
            | ["win_conditions", ..]
            | ["lose_conditions", ..]
            | ["routine", ..]
            | ["rules", ..]
            | ["on_display", ..]
            | ["on_level_start", ..]
            | ["on_level_clear", ..]
            | ["on_last_level_clear", ..]
            | ["keys", ..]
            | ["inputs", ..]
            | ["transitions", ..]
            | ["on_scene_start", ..]
            | ["if", ..]
            | ["for", ..]
            | ["fix", ..]
            | ["once"]
            | ["once_all"]
            | ["once_per_level"]
            | ["repeat", ..]
            | ["->"]
            | ["sounds"]
            | ["theme", ..]
            | ["assets"]
    )
}

fn skip_logical_block(lines: &[String], start: usize) -> usize {
    let mut depth = 1usize;
    let mut index = start + 1;
    while index < lines.len() {
        let tokens = split_header_tokens(&lines[index]);
        if is_block_close_line(&lines[index]) {
            depth = depth.saturating_sub(1);
            index += 1;
            if depth == 0 {
                break;
            }
            continue;
        }
        if logical_line_opens_block(tokens.as_slice()) && !logical_line_is_inline_if(&lines[index])
        {
            depth += 1;
        }
        index += 1;
    }
    index
}

fn recover_after_directive_error(lines: &[String], index: usize) -> usize {
    let tokens = split_header_tokens(&lines[index]);
    if logical_line_opens_block(tokens.as_slice()) && !logical_line_is_inline_if(&lines[index]) {
        skip_logical_block(lines, index)
    } else {
        index + 1
    }
}

fn logical_line_is_inline_if(line: &str) -> bool {
    split_header_tokens(line).first().copied() == Some("if") && line.contains("->")
}

fn strip_document_scene_source_raw(source: &str) -> String {
    let raw_lines = source.lines().collect::<Vec<_>>();
    let mut out = Vec::new();
    let mut index = 0;
    let mut brace_depth = 0i32;
    while index < raw_lines.len() {
        let line = raw_lines[index];
        let trimmed = strip_line_comment(line).trim();
        if brace_depth == 0 {
            let tokens = split_header_tokens(trimmed);
            if matches!(tokens.as_slice(), ["scene", ..]) {
                index = skip_raw_top_level_block(&raw_lines, index);
                continue;
            }
            if matches!(tokens.as_slice(), ["puzzle", ..] | ["puzzle3", ..])
                && trimmed.ends_with('{')
            {
                index = push_raw_model_without_default_scene_layouts(&raw_lines, index, &mut out);
                continue;
            }
        }
        out.push(line);
        brace_depth += raw_brace_delta(trimmed);
        brace_depth = brace_depth.max(0);
        index += 1;
    }
    out.join("\n")
}

fn push_raw_model_without_default_scene_layouts<'a>(
    raw_lines: &[&'a str],
    start: usize,
    out: &mut Vec<&'a str>,
) -> usize {
    out.push(raw_lines[start]);
    let mut index = start + 1;
    let mut depth = raw_brace_delta(strip_line_comment(raw_lines[start]).trim());
    while index < raw_lines.len() && depth > 0 {
        let line = raw_lines[index];
        let trimmed = strip_line_comment(line).trim();
        if depth == 1 && matches!(split_header_tokens(trimmed).as_slice(), ["layout", ..]) {
            index = skip_raw_top_level_block(raw_lines, index);
            continue;
        }
        out.push(line);
        depth += raw_brace_delta(trimmed);
        index += 1;
    }
    index
}

fn split_document_scene_lines(
    source: &str,
) -> Result<(Vec<source::LogicalLine>, Vec<SceneDef>), DiagnosticReport> {
    let logical_lines = logical_lines_with_locations(source)?;
    let lines = logical_lines
        .iter()
        .map(|line| line.text.clone())
        .collect::<Vec<_>>();
    let mut model_lines = Vec::new();
    let mut scenes = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let tokens = split_header_tokens(&lines[i]);
        if matches!(tokens.as_slice(), ["scene", ..]) {
            let (scene, next_i) = parse_scene_definition(&lines, i)?;
            scenes.push(scene);
            i = next_i;
        } else if let Some((kind, name)) = model_header_name(tokens.as_slice()) {
            let (entry, default_scene, next_i) =
                extract_default_model_scene(&logical_lines, &lines, i, kind, name)?;
            model_lines.extend(entry);
            if let Some(scene) = default_scene {
                scenes.push(scene);
            }
            i = next_i;
        } else {
            model_lines.push(logical_lines[i].clone());
            i += 1;
        }
    }
    Ok((model_lines, scenes))
}

fn model_header_name<'a>(tokens: &'a [&'a str]) -> Option<(&'a str, &'a str)> {
    match tokens {
        ["puzzle", name, ..] | ["puzzle3", name, ..] => Some((tokens[0], *name)),
        _ => None,
    }
}

fn extract_default_model_scene(
    logical_lines: &[source::LogicalLine],
    lines: &[String],
    start: usize,
    kind: &str,
    name: &str,
) -> Result<(Vec<source::LogicalLine>, Option<SceneDef>, usize), DiagnosticReport> {
    let mut entry = vec![logical_lines[start].clone()];
    let mut default_scene = None;
    let mut depth = 1usize;
    let mut i = start + 1;
    while i < lines.len() {
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        if tokens.first().copied() == Some(BLOCK_CLOSE) || line == "}" {
            depth = depth.saturating_sub(1);
            entry.push(logical_lines[i].clone());
            i += 1;
            if depth == 0 {
                return Ok((entry, default_scene, i));
            }
            continue;
        }
        if depth == 1 && matches!(tokens.as_slice(), ["layout", ..]) {
            if default_scene.is_some() {
                return Err(parse_error(
                    line,
                    "model default scene has duplicate layout block",
                ));
            }
            let next_i = skip_scene_layout_block(lines, i)?;
            default_scene = Some(parse_default_model_scene(lines, i, next_i, kind, name)?);
            i = next_i;
            continue;
        }
        if depth == 1 && matches!(tokens.as_slice(), ["keys"]) {
            let (bindings, next_i) = parse_scene_keys_block(lines, i)?;
            default_scene
                .get_or_insert_with(|| implicit_model_scene(kind, name))
                .key_bindings
                .extend(bindings);
            i = next_i;
            continue;
        }
        if starts_model_nested_block(tokens.as_slice(), line) {
            depth += 1;
        }
        entry.push(logical_lines[i].clone());
        i += 1;
    }
    Ok((vec![logical_lines[start].clone()], None, start + 1))
}

fn skip_scene_layout_block(lines: &[String], start: usize) -> Result<usize, DiagnosticReport> {
    let mut depth = 1usize;
    let mut i = start + 1;
    while i < lines.len() {
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        if tokens.first().copied() == Some(BLOCK_CLOSE) {
            depth = depth.saturating_sub(1);
            i += 1;
            if depth == 0 {
                return Ok(i);
            }
            continue;
        }
        if starts_authoring_block(tokens.as_slice(), line) || line.trim_end().ends_with("->") {
            depth += 1;
        }
        i += 1;
    }
    Err(parse_error(&lines[start], "layout missing closing brace"))
}

fn starts_model_nested_block(tokens: &[&str], line: &str) -> bool {
    logical_line_opens_block(tokens) && !logical_line_is_inline_if(line)
}

fn parse_default_model_scene(
    lines: &[String],
    start: usize,
    end: usize,
    kind: &str,
    name: &str,
) -> Result<SceneDef, DiagnosticReport> {
    let mut layout_lines = lines[start..end].to_vec();
    rewrite_default_model_layout_components(&mut layout_lines, kind, name);
    let (layout_block, next_i) = parse_screen_layout_block(&layout_lines, 0)?;
    debug_assert_eq!(next_i, layout_lines.len());
    let mut scene = implicit_model_scene(kind, name);
    scene.layout = layout_block.layout;
    scene.state.variables.extend(layout_block.state.variables);
    scene.state.puzzles.extend(layout_block.state.puzzles);
    scene.components = layout_block.components;
    Ok(scene)
}

fn rewrite_default_model_layout_components(lines: &mut [String], kind: &str, name: &str) {
    for line in lines {
        if split_header_tokens(line).as_slice() == [kind] {
            *line = format!("{kind} {name}");
        }
    }
}

fn skip_raw_top_level_block(raw_lines: &[&str], start: usize) -> usize {
    let first = strip_line_comment(raw_lines[start]).trim();
    if first.ends_with('{') {
        let mut depth = raw_brace_delta(first);
        let mut index = start + 1;
        while index < raw_lines.len() && depth > 0 {
            let trimmed = strip_line_comment(raw_lines[index]).trim();
            depth += raw_brace_delta(trimmed);
            index += 1;
        }
        index
    } else {
        let mut index = start + 1;
        while index < raw_lines.len() {
            let trimmed = strip_line_comment(raw_lines[index]).trim();
            index += 1;
            if trimmed == BLOCK_CLOSE {
                break;
            }
        }
        index
    }
}

fn raw_brace_delta(line: &str) -> i32 {
    line.chars().filter(|ch| *ch == '{').count() as i32
        - line.chars().filter(|ch| *ch == '}').count() as i32
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GameDocumentKind {
    Puzzle2d,
    Puzzle3d,
    Mixed,
}

fn detect_game_document_kind(source: &str) -> Result<GameDocumentKind, DiagnosticReport> {
    let mut has_2d = false;
    let mut has_3d = false;
    let context = scan_source_context(source);
    for line in &context.lines {
        let tokens = line.tokens.iter().map(String::as_str).collect::<Vec<_>>();
        match (line.scope, tokens.as_slice()) {
            (None, ["puzzle", ..]) => has_2d = true,
            (None, ["puzzle3", ..]) => has_3d = true,
            (_, ["levels3", ..] | ["sprites3", ..]) => has_3d = true,
            _ => {}
        }
    }
    Ok(match (has_2d, has_3d) {
        (true, true) => GameDocumentKind::Mixed,
        (false, true) => GameDocumentKind::Puzzle3d,
        _ => GameDocumentKind::Puzzle2d,
    })
}

pub fn validate_source_profile_for_path(
    source: &str,
    path: impl AsRef<Path>,
) -> Result<(), DiagnosticReport> {
    let path = path.as_ref();
    let profile = puzzle_source_profile_for_path(path).ok_or_else(|| {
        DiagnosticReport::error(format!(
            "puzzle source must use .puzzle or .puzzle3 extension: {}",
            path.display()
        ))
    })?;
    validate_source_profile(source, profile)
}

fn validate_source_profile(
    source: &str,
    profile: PuzzleSourceProfile,
) -> Result<(), DiagnosticReport> {
    let kind = detect_game_document_kind(source)?;
    match (profile, kind) {
        (PuzzleSourceProfile::Puzzle2d, GameDocumentKind::Puzzle2d)
        | (PuzzleSourceProfile::Puzzle3d, GameDocumentKind::Puzzle3d) => Ok(()),
        (_, GameDocumentKind::Mixed) => Err(DiagnosticReport::error(
            "mixed 2D/3D documents are no longer supported; split 2D .puzzle and 3D .puzzle3 sources"
                .to_string(),
        )),
        (PuzzleSourceProfile::Puzzle2d, GameDocumentKind::Puzzle3d) => Err(DiagnosticReport::error(
            ".puzzle files cannot contain 3D puzzle3, levels3, or sprites3 sections; use .puzzle3"
                .to_string(),
        )),
        (PuzzleSourceProfile::Puzzle3d, GameDocumentKind::Puzzle2d) => Err(DiagnosticReport::error(
            ".puzzle3 files must contain 3D puzzle3, levels3, or sprites3 sections".to_string(),
        )),
    }
}

fn first_model_name(source: &str, kind: &str) -> Option<String> {
    all_model_names(source, kind).into_iter().next()
}

fn all_model_names(source: &str, kind: &str) -> Vec<String> {
    let context = scan_source_context(source);
    let mut names = Vec::new();
    for line in &context.lines {
        if line.scope.is_some() {
            continue;
        }
        let tokens = line.tokens.iter().map(String::as_str).collect::<Vec<_>>();
        if let [model_kind, name, ..] = tokens.as_slice()
            && *model_kind == kind
            && !names.iter().any(|existing| existing == name)
        {
            names.push((*name).to_string());
        }
    }
    names
}

#[cfg(not(target_arch = "wasm32"))]
pub fn resolve_game_entry(path: impl AsRef<Path>) -> Result<PathBuf, DiagnosticReport> {
    let path = path.as_ref();
    if path.is_dir() {
        if let Some(entry) = game_entry_in_directory(path)? {
            return Ok(entry);
        }
        return Err(DiagnosticReport::error(format!(
            "game folder must contain a .puzzle or .puzzle3 file with game prelude metadata such as title: {}",
            path.display()
        )));
    }

    if path.is_file() {
        if !is_puzzle_source_path(path) {
            return Err(DiagnosticReport::error(format!(
                "game entry must be a folder, .puzzle file, or .puzzle3 file: {}",
                path.display()
            )));
        }
        let source = fs::read_to_string(path).map_err(|error| {
            DiagnosticReport::error(format!(
                "failed to read game entry {}: {error}",
                path.display()
            ))
        })?;
        if source_has_game_prelude(&source) {
            return Ok(path.to_path_buf());
        }
        let mut dir = path.parent();
        while let Some(current) = dir {
            if let Some(entry) = game_entry_in_directory(current)? {
                return Ok(entry);
            }
            dir = current.parent();
        }
        return Err(DiagnosticReport::error(format!(
            "puzzle source file has no game prelude and no containing game entry was found: {}",
            path.display()
        )));
    }

    Err(DiagnosticReport::error(format!(
        "game entry not found: {}",
        path.display()
    )))
}

pub fn source_has_game_prelude(source: &str) -> bool {
    let mut depth = 0_i32;
    for raw_line in source.lines() {
        let code = raw_line.split("//").next().unwrap_or("");
        let trimmed = code.trim();
        if depth == 0 {
            let first = trimmed.split_whitespace().next().unwrap_or("");
            if matches!(first, "title" | "subtitle" | "author" | "homepage") {
                return true;
            }
        }
        for ch in code.chars() {
            match ch {
                '{' => depth += 1,
                '}' => depth = (depth - 1).max(0),
                _ => {}
            }
        }
    }
    false
}

#[cfg(not(target_arch = "wasm32"))]
pub fn game_import_paths(source: &str) -> Result<Vec<PathBuf>, DiagnosticReport> {
    let mut paths = Vec::new();
    for line in logical_lines(source)? {
        let tokens = split_header_tokens(&line);
        if matches!(tokens.as_slice(), ["import", _]) {
            paths.push(import_path(tokens[1], &line)?);
        }
    }
    Ok(paths)
}

#[cfg(not(target_arch = "wasm32"))]
fn game_entry_in_directory(dir: &Path) -> Result<Option<PathBuf>, DiagnosticReport> {
    let mut candidates = Vec::new();
    for entry in fs::read_dir(dir).map_err(|error| {
        DiagnosticReport::error(format!(
            "failed to read game entry directory {}: {error}",
            dir.display()
        ))
    })? {
        let path = entry
            .map_err(|error| {
                DiagnosticReport::error(format!("failed to read game entry: {error}"))
            })?
            .path();
        if !is_puzzle_source_path(&path) {
            continue;
        }
        let source = fs::read_to_string(&path).map_err(|error| {
            DiagnosticReport::error(format!(
                "failed to read game entry candidate {}: {error}",
                path.display()
            ))
        })?;
        if source_has_game_prelude(&source) {
            candidates.push(path);
        }
    }
    candidates.sort_by(|left, right| {
        let left_rank = game_entry_path_rank(left, dir);
        let right_rank = game_entry_path_rank(right, dir);
        left_rank
            .cmp(&right_rank)
            .then_with(|| left.display().to_string().cmp(&right.display().to_string()))
    });
    Ok(candidates.into_iter().next())
}

#[cfg(not(target_arch = "wasm32"))]
fn game_entry_path_rank(path: &Path, dir: &Path) -> usize {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    let folder_name = dir
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    if name == "game.puzzle" {
        0
    } else if name == "game.puzzle3" {
        1
    } else if !folder_name.is_empty() && name == format!("{folder_name}.puzzle") {
        2
    } else if !folder_name.is_empty() && name == format!("{folder_name}.puzzle3") {
        3
    } else if name == "main.puzzle" {
        4
    } else if name == "main.puzzle3" {
        5
    } else {
        6
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn discover_game_entries(root: impl AsRef<Path>) -> Result<Vec<PathBuf>, DiagnosticReport> {
    let mut candidates = Vec::new();
    for entry in fs::read_dir(root.as_ref()).map_err(|error| {
        DiagnosticReport::error(format!(
            "failed to read game root {}: {error}",
            root.as_ref().display()
        ))
    })? {
        let path = entry
            .map_err(|error| {
                DiagnosticReport::error(format!("failed to read game entry: {error}"))
            })?
            .path();
        if path.is_dir() {
            if let Some(entry) = game_entry_in_directory(&path)? {
                candidates.push(entry);
            }
        } else if is_puzzle_source_path(&path) {
            let source = fs::read_to_string(&path).map_err(|error| {
                DiagnosticReport::error(format!(
                    "failed to read game entry candidate {}: {error}",
                    path.display()
                ))
            })?;
            if source_has_game_prelude(&source) {
                candidates.push(path);
            }
        }
    }
    candidates.sort();
    Ok(candidates)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn expand_game_imports_for_file(
    source: &str,
    path: impl AsRef<Path>,
) -> Result<String, DiagnosticReport> {
    let path = path.as_ref();
    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let mut import_stack = vec![canonical_import_path(path)];
    expand_game_imports(source, base_dir, &mut import_stack, None)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn expand_game_imports_for_file_under_root(
    source: &str,
    path: impl AsRef<Path>,
    root: impl AsRef<Path>,
) -> Result<String, DiagnosticReport> {
    let path = path.as_ref();
    let root = canonical_import_path(root.as_ref());
    let canonical_path = canonical_import_path(path);
    if !canonical_path.starts_with(&root) {
        return Err(DiagnosticReport::error(format!(
            "can only import puzzle files under {}",
            root.display()
        )));
    }
    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let mut import_stack = vec![canonical_path];
    expand_game_imports(source, base_dir, &mut import_stack, Some(&root))
}

fn parse_game2d_expanded_with_shell(
    source: &str,
    shell: &DocumentShell,
) -> Result<LoadedGame, DiagnosticReport> {
    let logical_lines = logical_lines_with_locations(source)?;
    parse_game2d_expanded_lines_with_shell(logical_lines, shell)
}

fn parse_game2d_expanded_lines_with_shell(
    logical_lines: Vec<source::LogicalLine>,
    shell: &DocumentShell,
) -> Result<LoadedGame, DiagnosticReport> {
    let line_numbers = logical_lines
        .iter()
        .map(|line| line.line)
        .collect::<Vec<_>>();
    let lines = logical_lines
        .into_iter()
        .map(|line| line.text)
        .collect::<Vec<_>>();
    let mut title = shell.title.clone();
    let mut subtitle = shell.subtitle.clone();
    let mut author = shell.author.clone();
    let mut homepage = shell.homepage.clone();
    let mut layer_count = None;
    let mut empty_char = None;
    let mut named_layers = HashMap::<String, u16>::new();
    let mut catalog = Catalog::default();
    let mut condition_definitions = Vec::<ConditionDefinitionAst>::new();
    let mut controls = Controls::default();
    let mut directions = Vec::<Direction>::new();
    let mut rule_definitions = Vec::<RuleDefinitionAst>::new();
    let mut main_statements = None;
    let mut main_local_frame = None;
    let mut level_start_statements = None;
    let mut level_start_local_frame = None;
    let mut level_clear_statements = None;
    let mut level_clear_local_frame = None;
    let mut last_level_clear_statements = None;
    let mut last_level_clear_local_frame = None;
    let mut display_statements = None;
    let mut level_blocks = Vec::<LevelBlock>::new();
    let mut puzzle_models = Vec::<String>::new();
    let mut variables = Vec::<SceneVarDef>::new();
    let mut render_overlays = Vec::<(Vec<ObjectId>, char)>::new();
    let mut model_sound_triggers = Vec::<ModelSoundTriggerSpec>::new();
    let mut model_operation_sounds = Vec::<ModelOperationSoundSpec>::new();
    let mut named_conditions = HashMap::<String, (String, ConditionAst)>::new();
    let mut run_rules_on_level_start = false;
    let mut visuals = VisualsDef::default();
    let mut render = PuzzleRenderDef::default();
    let mut animation = shell.animation.clone();
    let mut sounds = shell.sounds.clone();
    let mut theme = shell.theme.clone();
    let mut assets = shell.assets.clone();
    let mut puzzle_screen = PuzzleScreenDef::default();
    let mut default_wait_ms = shell.default_wait_ms;
    let mut default_again_ms = shell.default_again_ms;

    let mut diagnostics = Vec::new();
    let mut pending_visual_blocks = Vec::<usize>::new();
    let mut i = 0;
    while i < lines.len() {
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        if tokens.is_empty() {
            i += 1;
            continue;
        }

        match classify_model_top_level_directive(tokens.as_slice()) {
            ModelTopLevelDirective::Puzzle => match parse_puzzle_definition(
                &lines,
                &line_numbers,
                i,
                &mut layer_count,
                &mut empty_char,
                &mut named_layers,
                &mut catalog,
                &mut condition_definitions,
                &mut controls,
                &mut directions,
                &mut rule_definitions,
                &mut main_statements,
                &mut main_local_frame,
                &mut level_start_statements,
                &mut level_start_local_frame,
                &mut level_clear_statements,
                &mut level_clear_local_frame,
                &mut last_level_clear_statements,
                &mut last_level_clear_local_frame,
                &mut display_statements,
                &mut level_blocks,
                &mut render_overlays,
                &mut model_sound_triggers,
                &mut model_operation_sounds,
                &mut named_conditions,
                &mut run_rules_on_level_start,
                &mut visuals,
                &mut render,
                &mut animation,
                &mut puzzle_screen,
            ) {
                Ok((next_i, puzzle_name)) => {
                    puzzle_models.push(puzzle_name);
                    i = next_i;
                }
                Err(report) => {
                    diagnostics.extend(report.into_diagnostics());
                    i = recover_after_directive_error(&lines, i);
                }
            },
            ModelTopLevelDirective::RemovedModelPrefix => {
                let message = match tokens.as_slice() {
                    ["model", "puzzle3", ..] => {
                        "top-level 3D puzzle definition must be: puzzle3 <name>"
                    }
                    _ => "top-level puzzle definition must be: puzzle <name>",
                };
                diagnostics.extend(parse_error(line, message).into_diagnostics());
                i = recover_after_directive_error(&lines, i);
            }
            ModelTopLevelDirective::RemovedNameMetadata => {
                diagnostics.extend(
                    parse_error(
                        line,
                        "top-level `name` metadata was removed; use `title <text>`",
                    )
                    .into_diagnostics(),
                );
                i += 1;
            }
            ModelTopLevelDirective::Title => {
                title = parse_metadata_text(line, "title")?;
                i += 1;
            }
            ModelTopLevelDirective::Subtitle => {
                subtitle = Some(parse_metadata_text(line, "subtitle")?);
                i += 1;
            }
            ModelTopLevelDirective::Author => {
                author = Some(parse_metadata_text(line, "author")?);
                i += 1;
            }
            ModelTopLevelDirective::Homepage => {
                homepage = Some(parse_metadata_text(line, "homepage")?);
                i += 1;
            }
            ModelTopLevelDirective::Variable => {
                variables.push(parse_top_level_var_directive(&tokens, line)?);
                i += 1;
            }
            ModelTopLevelDirective::DefaultWaitTime => {
                default_wait_ms = parse_default_wait_time_directive(&tokens, line)?;
                i += 1;
            }
            ModelTopLevelDirective::AgainInterval => {
                default_again_ms = parse_again_interval_directive(&tokens, line)?;
                i += 1;
            }
            ModelTopLevelDirective::Animation => {
                i = parse_animation_block(&lines, i, &mut animation)?;
            }
            ModelTopLevelDirective::Scene => {
                diagnostics.extend(parse_error(
                    line,
                    "scene blocks are document-level syntax and must be parsed before the 2D model",
                ).into_diagnostics());
                i = recover_after_directive_error(&lines, i);
            }
            ModelTopLevelDirective::Sounds => {
                if model_sounds_block_starts(&lines, i) {
                    i = parse_model_sounds_block(
                        &lines,
                        i,
                        &mut model_sound_triggers,
                        &mut model_operation_sounds,
                        false,
                    )?;
                } else {
                    i = parse_sounds_block(&lines, i, &mut sounds)?;
                }
            }
            ModelTopLevelDirective::Theme => {
                i = parse_theme_statement(&lines, i, &mut theme)?;
            }
            ModelTopLevelDirective::Assets => {
                i = parse_assets_block(&lines, i, &mut assets)?;
            }
            ModelTopLevelDirective::Close => {
                i += 1;
            }
            ModelTopLevelDirective::Sprites => {
                pending_visual_blocks.push(i);
                let (_, next_i) = collect_authoring_entry(&lines, i)?;
                i = next_i;
            }
            ModelTopLevelDirective::Levels => {
                i = parse_levels_block(
                    &lines,
                    i,
                    &mut level_blocks,
                    &mut catalog,
                    &mut render_overlays,
                    &mut empty_char,
                    None,
                )?;
            }
            ModelTopLevelDirective::Level => {
                let (level, next_i) = parse_level_block(&lines, i, level_blocks.len())?;
                level_blocks.push(level);
                i = next_i;
            }
            ModelTopLevelDirective::PuzzleLifecycle => {
                diagnostics.extend(
                    parse_error(line, &misplaced_puzzle_lifecycle_message(tokens[0]))
                        .into_diagnostics(),
                );
                i = recover_after_directive_error(&lines, i);
            }
            ModelTopLevelDirective::Unknown => {
                diagnostics.extend(
                    parse_error(line, &unknown_model_top_level_directive_message(tokens[0]))
                        .into_diagnostics(),
                );
                i = recover_after_directive_error(&lines, i);
            }
        }
    }
    for visual_start in pending_visual_blocks {
        if let Err(report) = parse_visuals_block(&lines, visual_start, &mut catalog, &mut visuals) {
            diagnostics.extend(report.into_diagnostics());
        }
    }
    if !diagnostics.is_empty() {
        return Err(DiagnosticReport::from_diagnostics(diagnostics));
    }

    let empty_char = empty_char.ok_or_else(|| {
        DiagnosticReport::error(
            "missing empty char; use `levels { legend { . = empty } }`".to_string(),
        )
    })?;
    validate_layer_role_separation(&catalog, &named_layers)?;
    refresh_layer_tags_and_value_sets(&mut named_layers, &mut catalog);
    let layer_count =
        layer_count.ok_or_else(|| DiagnosticReport::error("missing layers".to_string()))?;
    if level_blocks.is_empty() {
        return Err(DiagnosticReport::error("missing level".to_string()));
    }
    resolve_level_block_puzzles(&mut level_blocks, &puzzle_models)?;
    let prepared_level_bodies = level_blocks
        .into_iter()
        .map(|level| {
            let puzzle = level
                .puzzle
                .clone()
                .expect("level puzzle was resolved before preparation");
            let body = parse_level_body(
                &level,
                &catalog,
                empty_char,
                default_wait_ms,
                &named_conditions,
            )?;
            let mut char_objects = catalog.char_objects.clone();
            char_objects.extend(body.local_char_objects.clone());
            Ok(PreparedLevelBody {
                name: level.name,
                pack: level.pack,
                puzzle,
                lines: body.lines,
                char_objects,
                level_start_statements: body.level_start_statements,
                level_clear_statements: body.level_clear_statements,
            })
        })
        .collect::<Result<Vec<_>, DiagnosticReport>>()?;
    add_default_restart_handler(main_statements.as_mut());
    add_implicit_input_guards_to_catalog(
        &rule_definitions,
        main_statements.as_deref(),
        level_start_statements.as_deref(),
        level_clear_statements.as_deref(),
        display_statements.as_deref(),
        &prepared_level_bodies,
        &named_conditions,
        &mut catalog,
    )?;
    if directions.is_empty()
        || (has_cardinal_input_names(&catalog.input_names)
            && !directions_include_all_cardinals(&directions, &catalog.input_names))
    {
        add_cardinal_directions("default inputs", &mut catalog, &mut directions)?;
    }
    add_default_non_direction_inputs("default inputs", &mut catalog)?;
    add_default_key_controls(&catalog.input_names, &mut controls);
    let effective_directions = if directions.is_empty() {
        default_cardinal_directions(&catalog.input_names)
    } else {
        directions.clone()
    };
    let value_sets = catalog_value_sets(&catalog);
    let visual_condition_reads =
        visual_condition_reads(&condition_definitions, &catalog.visual_objects);
    let condition_defs = lower_condition_defs(
        condition_definitions,
        &catalog.object_layers,
        &catalog.scratch_names,
        &value_sets,
        &catalog.input_names,
        &effective_directions,
    )?;
    let mut conditions = named_conditions
        .into_iter()
        .map(|(name, (description, condition))| {
            lower_goal_condition(
                description,
                &condition,
                &catalog.object_layers,
                &catalog.global_names,
                &catalog.condition_names,
                &visual_condition_reads,
                &catalog.scratch_names,
                &catalog.visual_objects,
                &value_sets,
                &catalog.input_names,
                &effective_directions,
            )
            .map(|condition| (name, condition))
        })
        .collect::<Result<HashMap<_, _>, DiagnosticReport>>()?;
    let goal = conditions
        .get("win_conditions")
        .or_else(|| conditions.get("goal"))
        .cloned();
    let lose = conditions
        .get("lose_conditions")
        .or_else(|| conditions.get("lose"))
        .cloned();
    conditions.remove("lose_conditions");
    conditions.remove("lose");
    if run_rules_on_level_start && level_start_statements.is_some() {
        return Err(DiagnosticReport::error(
            "run_rules_on_level_start cannot be combined with on_level_start".to_string(),
        ));
    }
    add_standard_move_rule_if_missing(
        &mut rule_definitions,
        &catalog.object_names,
        &catalog.object_schemas,
        &catalog.object_layers,
        &catalog.visual_objects,
        &value_sets,
        &catalog.maps,
        &catalog.object_groups,
        &catalog.input_names,
        &catalog.global_names,
        &catalog.condition_names,
    )?;
    let visual_objects = catalog.visual_objects.clone();
    let model_sound_triggers = resolve_model_sound_triggers(&model_sound_triggers, &catalog)?;
    let model_operation_sounds = resolve_model_operation_sounds(&model_operation_sounds);
    let mut warnings = collect_dynamic_selector_warnings(
        &rule_definitions,
        main_statements.as_deref(),
        level_start_statements.as_deref(),
        level_clear_statements.as_deref(),
        last_level_clear_statements.as_deref(),
        display_statements.as_deref(),
        &prepared_level_bodies,
        &catalog.constant_globals,
    );
    warnings.extend(collect_visual_overwrite_warnings(&visuals));
    let programs = lower_programs(
        rule_definitions,
        main_statements,
        main_local_frame,
        level_start_statements,
        level_start_local_frame,
        level_clear_statements,
        level_clear_local_frame,
        last_level_clear_statements,
        last_level_clear_local_frame,
        display_statements,
        &prepared_level_bodies,
        &catalog.object_layers,
        &visual_objects,
        &catalog.input_names,
        &catalog.global_names,
        &catalog.constant_globals,
        &catalog.condition_names,
        &visual_condition_reads,
        &catalog.scratch_names,
        &model_sound_triggers,
        &animation,
        &value_sets,
        &effective_directions,
        default_wait_ms,
    )?;
    let game = CompiledGame::new_with_scratch_condition_defs_program_roles(
        layer_count,
        catalog.object_defs,
        catalog.scratch_defs,
        condition_defs,
        programs.main,
        visual_objects.clone(),
        programs.visual_rules.clone(),
    );
    let mut legend = AsciiLegend::new(game.object_count(), empty_char);
    for (object, ch) in &catalog.render_chars {
        legend.set(*object, *ch);
    }
    for object in &visual_objects {
        legend.ignore(*object);
    }
    for (objects, ch) in render_overlays {
        legend.add_overlay(objects, ch);
    }
    let levels = prepared_level_bodies
        .into_iter()
        .enumerate()
        .map(|(index, prepared)| {
            let parsed_level = parse_level(
                &game,
                &prepared.lines,
                empty_char,
                &prepared.char_objects,
                &catalog.global_defaults,
            )?;
            Ok(Level {
                name: prepared.name,
                pack: prepared.pack,
                puzzle: prepared.puzzle,
                initial_state: parsed_level.state,
                regions: parsed_level.regions,
                level_start_program: programs.level_starts[index].clone(),
                level_clear_program: programs.level_clears[index].clone(),
            })
        })
        .collect::<Result<Vec<_>, DiagnosticReport>>()?;

    warnings.extend(collect_scratch_warnings(&game, &catalog.scratch_names));

    Ok(LoadedGame {
        title,
        subtitle,
        author,
        homepage,
        game,
        warnings,
        default_wait_ms,
        default_again_ms,
        animation: animation.clone(),
        rule_animations: programs.rule_animations,
        rule_effects: programs.rule_effects,
        level_start_program: programs.level_start,
        display_level_start_program: None,
        level_clear_program: programs.level_clear,
        last_level_clear_program: programs.last_level_clear,
        display_level_clear_program: None,
        display_program: programs.display,
        levels,
        run_rules_on_level_start,
        legend,
        controls,
        variables,
        scenes: Vec::new(),
        object_labels: catalog.object_labels,
        object_groups: catalog.object_groups,
        input_labels: catalog.input_labels,
        global_labels: catalog.global_labels,
        persistent_vars: catalog.persistent_vars,
        condition_labels: catalog.condition_labels,
        conditions,
        goal,
        lose,
        sounds,
        model_operation_sounds,
        theme,
        assets,
        visuals,
        render,
        screen: puzzle_screen,
    })
}

fn collect_dynamic_selector_warnings(
    definitions: &[RuleDefinitionAst],
    main_statements: Option<&[StatementAst]>,
    level_start_statements: Option<&[StatementAst]>,
    level_clear_statements: Option<&[StatementAst]>,
    last_level_clear_statements: Option<&[StatementAst]>,
    display_statements: Option<&[StatementAst]>,
    level_bodies: &[PreparedLevelBody],
    constant_globals: &[GlobalId],
) -> Vec<String> {
    let mut warnings = Vec::new();
    for definition in definitions {
        collect_dynamic_selector_statement_warnings(
            &definition.statements,
            constant_globals,
            &mut warnings,
        );
    }
    for statements in [
        main_statements,
        level_start_statements,
        level_clear_statements,
        last_level_clear_statements,
        display_statements,
    ]
    .into_iter()
    .flatten()
    {
        collect_dynamic_selector_statement_warnings(statements, constant_globals, &mut warnings);
    }
    for body in level_bodies {
        collect_dynamic_selector_statement_warnings(
            &body.level_start_statements,
            constant_globals,
            &mut warnings,
        );
        collect_dynamic_selector_statement_warnings(
            &body.level_clear_statements,
            constant_globals,
            &mut warnings,
        );
    }
    warnings
}

fn collect_dynamic_selector_statement_warnings(
    statements: &[StatementAst],
    constant_globals: &[GlobalId],
    warnings: &mut Vec<String>,
) {
    for statement in statements {
        match statement {
            StatementAst::Rewrite(rewrite) => {
                collect_dynamic_selector_block_warnings(
                    &rewrite.before,
                    constant_globals,
                    warnings,
                );
            }
            StatementAst::Conditional {
                condition,
                then_statements,
                else_statements,
                ..
            } => {
                collect_dynamic_selector_block_warnings(
                    &condition.pattern,
                    constant_globals,
                    warnings,
                );
                collect_dynamic_selector_statement_warnings(
                    then_statements,
                    constant_globals,
                    warnings,
                );
                collect_dynamic_selector_statement_warnings(
                    else_statements,
                    constant_globals,
                    warnings,
                );
            }
            StatementAst::Block { statements, .. }
            | StatementAst::Fix { statements, .. }
            | StatementAst::RepeatUntil { statements, .. } => {
                collect_dynamic_selector_statement_warnings(statements, constant_globals, warnings);
            }
            StatementAst::If {
                then_statements,
                else_statements,
                ..
            } => {
                collect_dynamic_selector_statement_warnings(
                    then_statements,
                    constant_globals,
                    warnings,
                );
                collect_dynamic_selector_statement_warnings(
                    else_statements,
                    constant_globals,
                    warnings,
                );
            }
            StatementAst::Call { .. }
            | StatementAst::DisplayCall { .. }
            | StatementAst::Effect { .. } => {}
        }
    }
}

fn collect_dynamic_selector_block_warnings(
    block: &PatternBlock,
    constant_globals: &[GlobalId],
    warnings: &mut Vec<String>,
) {
    for component in &block.components {
        for row in &component.rows {
            for part in row {
                let BlockPart::Cell(cell) = part else {
                    continue;
                };
                for selector in cell.require.iter().chain(&cell.forbid) {
                    for guard in selector.dynamic_guards.values().flatten() {
                        if constant_globals.contains(&guard.global) {
                            continue;
                        }
                        push_unique_warning(
                            warnings,
                            format!(
                                "dynamic selector `{}` uses mutable var `{}`; if the var is outside the selector tag slot values, the selector does not match",
                                selector.token, guard.name
                            ),
                        );
                    }
                }
            }
        }
    }
}

fn collect_scratch_warnings(
    game: &CompiledGame,
    scratch_names: &HashMap<String, ScratchDef>,
) -> Vec<String> {
    let labels = scratch_names
        .iter()
        .map(|(name, def)| (def.id, name.as_str()))
        .collect::<HashMap<_, _>>();
    let mut warnings = Vec::new();
    for rule in game.rules() {
        for component in &rule.pattern.components {
            for cell in &component.cells {
                for cell_attr in cell
                    .require_scratch
                    .iter()
                    .filter(|attr| attr.object.is_empty())
                {
                    for object_attr in cell
                        .require_scratch
                        .iter()
                        .filter(|attr| !attr.object.is_empty())
                    {
                        if cell_attr.scratch == object_attr.scratch {
                            push_unique_warning(
                                &mut warnings,
                                format!(
                                    "scratch `{}` appears on both a cell and an object occurrence in the same cell pattern",
                                    scratch_label(cell_attr.scratch, &labels)
                                ),
                            );
                        }
                    }
                }
            }
        }

        for pattern_attr in rule
            .pattern
            .components
            .iter()
            .flat_map(|component| component.cells.iter())
            .flat_map(|cell| cell.require_scratch.iter())
        {
            for write in &rule.writes {
                let Some((write_object, write_scratch)) = write_scratch_target(write) else {
                    continue;
                };
                if pattern_attr.scratch != write_scratch {
                    continue;
                }
                if pattern_attr.object.is_empty() != write_object.is_empty() {
                    let from = if pattern_attr.object.is_empty() {
                        "cell"
                    } else {
                        "object occurrence"
                    };
                    let to = if write_object.is_empty() {
                        "cell"
                    } else {
                        "object occurrence"
                    };
                    push_unique_warning(
                        &mut warnings,
                        format!(
                            "scratch `{}` changes anchor from {from} to {to} in a rewrite",
                            scratch_label(pattern_attr.scratch, &labels)
                        ),
                    );
                }
            }
        }
    }
    warnings
}

fn write_scratch_target(write: &WriteOp) -> Option<(ObjectId, ScratchId)> {
    match write {
        WriteOp::SetScratch {
            object, scratch, ..
        } => Some((*object, *scratch)),
        _ => None,
    }
}

fn scratch_label<'a>(scratch: ScratchId, labels: &HashMap<ScratchId, &'a str>) -> String {
    labels
        .get(&scratch)
        .copied()
        .unwrap_or("__anonymous")
        .to_string()
}

fn push_unique_warning(warnings: &mut Vec<String>, warning: String) {
    if !warnings.contains(&warning) {
        warnings.push(warning);
    }
}

fn collect_visual_overwrite_warnings(visuals: &VisualsDef) -> Vec<String> {
    let mut warnings = Vec::new();
    collect_duplicate_output_key_warnings(
        &mut warnings,
        visuals.sprites.iter().map(|sprite| sprite.name.as_str()),
        "visual sprite",
        "later definition overwrites earlier sprite in generated visuals",
    );
    warnings
}

fn collect_duplicate_output_key_warnings<'a>(
    warnings: &mut Vec<String>,
    keys: impl IntoIterator<Item = &'a str>,
    label: &str,
    consequence: &str,
) {
    let mut seen = HashSet::<&'a str>::new();
    let mut warned = HashSet::<&'a str>::new();
    for key in keys {
        if !seen.insert(key) && warned.insert(key) {
            push_unique_warning(
                warnings,
                format!("{label} `{key}` is defined more than once; {consequence}"),
            );
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn expand_game_imports(
    source: &str,
    base_dir: &Path,
    import_stack: &mut Vec<PathBuf>,
    root: Option<&Path>,
) -> Result<String, DiagnosticReport> {
    let mut out = String::new();
    let mut output_line = 1usize;
    for line in logical_lines_with_locations(source)? {
        while output_line < line.line {
            out.push('\n');
            output_line += 1;
        }
        let tokens = split_header_tokens(&line.text);
        if matches!(tokens.as_slice(), ["import", _]) {
            let path = import_path(tokens[1], &line.text)?;
            let imported = read_import_expanded(base_dir, &path, import_stack, root)?;
            out.push_str(&imported);
            if !imported.ends_with('\n') {
                out.push('\n');
            }
            output_line += imported.lines().count().max(1);
            continue;
        }
        out.push_str(&line.text);
        out.push('\n');
        output_line += 1;
    }
    Ok(out)
}

#[cfg(not(target_arch = "wasm32"))]
fn read_import_expanded(
    base_dir: &Path,
    path: &Path,
    import_stack: &mut Vec<PathBuf>,
    root: Option<&Path>,
) -> Result<String, DiagnosticReport> {
    let resolved = resolve_import_path(base_dir, path);
    let canonical = canonical_import_path(&resolved);
    if let Some(root) = root {
        if !canonical.starts_with(root) {
            return Err(DiagnosticReport::error(format!(
                "can only import puzzle files under {}",
                root.display()
            )));
        }
    }
    if import_stack.contains(&canonical) {
        return Err(DiagnosticReport::error(format!(
            "cyclic import: {}",
            import_stack
                .iter()
                .chain(std::iter::once(&canonical))
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(" -> ")
        )));
    }
    let source = match read_import_path(&resolved) {
        Ok(source) => source,
        Err(error) => return Err(error),
    };
    let nested_base = resolved.parent().unwrap_or(base_dir);
    import_stack.push(canonical);
    let expanded = expand_game_imports(&source, nested_base, import_stack, root);
    import_stack.pop();
    expanded
}

#[cfg(not(target_arch = "wasm32"))]
fn import_path(token: &str, line: &str) -> Result<PathBuf, DiagnosticReport> {
    let path = token
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .ok_or_else(|| parse_error(line, "import path must be quoted"))?;
    Ok(PathBuf::from(path))
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_import_path(base_dir: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn canonical_import_path(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(not(target_arch = "wasm32"))]
fn read_import_path(path: &Path) -> Result<String, DiagnosticReport> {
    fs::read_to_string(path).map_err(|error| {
        DiagnosticReport::error(format!("failed to read {}: {error}", path.display()))
    })
}
