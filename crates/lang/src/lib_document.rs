pub fn export_loaded_document_visual_fixture_json(
    document: &LoadedDocument,
) -> Result<String, DiagnosticReport> {
    let Some(LoadedDocumentModel::Puzzle3d { puzzle, .. }) = document.single_model() else {
        return Err(DiagnosticReport::error(
            "visual fixture export currently requires a single puzzle3 model".to_string(),
        ));
    };
    let (document_fields, level_bundle_names) =
        puzzle3_document_fixture_fields(document).map_err(|error| {
            DiagnosticReport::error(format!(
                "failed to serialize puzzle3 document fields: {error}"
            ))
        })?;
    export_visual_fixture_json_with_title_scenes_and_animation(
        puzzle,
        Some(&document.title),
        document_fields.as_deref(),
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

pub fn parse_document_assets(source: &str) -> Result<AssetsDef, DiagnosticReport> {
    Ok(parse_document_shell(source)?.assets)
}

fn parse_game2d_document(source: &str) -> Result<LoadedGame, DiagnosticReport> {
    let parts = parse_document_source_parts_from_surface_source(source)?;
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
    let kind = detect_game_document_kind(source)?;
    match kind {
        GameDocumentKind::Puzzle2d | GameDocumentKind::Puzzle3d => {
            parse_single_model_game_document(source, kind)
        }
        GameDocumentKind::Mixed => Err(DiagnosticReport::error(
            "mixed 2D/3D documents are no longer supported; split 2D .puzzle and 3D .puzzle3 sources"
                .to_string(),
        )),
    }
}

fn parse_single_model_game_document(
    source: &str,
    kind: GameDocumentKind,
) -> Result<LoadedDocument, DiagnosticReport> {
    let parts = parse_document_source_parts_from_surface_source(source)?;
    match kind {
        GameDocumentKind::Puzzle2d => parse_puzzle2d_loaded_document(parts),
        GameDocumentKind::Puzzle3d => parse_puzzle3d_loaded_document(parts),
        GameDocumentKind::Mixed => Err(DiagnosticReport::error(
            "single-model document parser received a mixed 2D/3D document".to_string(),
        )),
    }
}

fn parse_puzzle2d_loaded_document(
    parts: DocumentSourceParts,
) -> Result<LoadedDocument, DiagnosticReport> {
    let name =
        first_model_name(&parts.model_source, "puzzle").unwrap_or_else(|| "default".to_string());
    let shell = parts.shell.clone();
    let game = parse_game2d_from_document_parts(parts)?;
    Ok(loaded_document_from_shell(
        shell,
        game.scenes.clone(),
        vec![LoadedDocumentModel::Puzzle2d {
            name,
            game: game.clone(),
        }],
    ))
}

fn parse_puzzle3d_loaded_document(
    parts: DocumentSourceParts,
) -> Result<LoadedDocument, DiagnosticReport> {
    let name =
        first_model_name(&parts.model_source, "puzzle3").unwrap_or_else(|| "default".to_string());
    let mut scenes = parts.scenes;
    resolve_inferred_scene_puzzle_slots(&mut scenes, std::iter::once(("puzzle3", &name)))?;
    let puzzle = crate::puzzle3_parse::parse_puzzle3d_logical_lines(&parts.model_lines)
        .map_err(|error| puzzle3_parse_error_report(error, &parts.model_lines))?;
    let mut scenes = add_implicit_model_scenes(scenes, std::iter::once(("puzzle3", &name)));
    resolve_scene_actions(&mut scenes, &HashMap::new())?;
    Ok(loaded_document_from_shell(
        parts.shell,
        scenes,
        vec![LoadedDocumentModel::Puzzle3d { name, puzzle }],
    ))
}

fn loaded_document_from_shell(
    shell: DocumentShell,
    scenes: Vec<SceneDef>,
    models: Vec<LoadedDocumentModel>,
) -> LoadedDocument {
    LoadedDocument {
        title: shell.title,
        subtitle: shell.subtitle,
        author: shell.author,
        homepage: shell.homepage,
        default_wait_ms: shell.default_wait_ms,
        default_again_ms: shell.default_again_ms,
        input_buffer: shell.input_buffer,
        animation: shell.animation,
        sounds: shell.sounds,
        theme: shell.theme,
        assets: shell.assets,
        scenes,
        models,
    }
}

fn puzzle3_parse_error_report(
    error: ParseError3,
    model_lines: &[source::LogicalLine],
) -> DiagnosticReport {
    let report = match error {
        ParseError3::Message(message) => DiagnosticReport::error(message),
        ParseError3::MessageAtSourceLine {
            message,
            source_line,
        } => DiagnosticReport::error_at_line(message, source_line),
    };
    let lines = model_lines
        .iter()
        .map(|line| line.text.clone())
        .collect::<Vec<_>>();
    let line_numbers = model_lines.iter().map(|line| line.line).collect::<Vec<_>>();
    report_with_source_line_numbers(report, &lines, &line_numbers)
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
        | ["input_buffer", ..]
        | ["animation", ..]
        | ["sounds", ..]
        | ["theme", ..]
        | ["assets", ..]
        | ["sprites", ..] => MixedSectionTarget::Shared,
        ["puzzle", ..] | ["levels", ..] | ["level", ..] => MixedSectionTarget::Puzzle2d,
        ["puzzle3", ..] | ["levels3", ..] => MixedSectionTarget::Puzzle3d,
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
        let resolved_puzzle_kinds = scene
            .state
            .puzzles
            .iter()
            .map(|puzzle| (puzzle.name.clone(), puzzle.kind.clone()))
            .collect::<HashMap<_, _>>();
        resolve_inferred_scene_component_frames(&mut scene.components, &resolved_puzzle_kinds);
    }

    Ok(())
}

fn resolve_inferred_scene_component_frames(
    components: &mut [SceneComponent],
    puzzle_kinds: &HashMap<String, String>,
) {
    for component in components {
        match component {
            SceneComponent::Frame(frame) if frame.kind == INFERRED_SCENE_PUZZLE_KIND => {
                if let Some(kind) = puzzle_kinds.get(&frame.source) {
                    frame.kind = kind.clone();
                }
            }
            _ => {
                if let Some(children) = component.children_mut() {
                    resolve_inferred_scene_component_frames(children, puzzle_kinds);
                }
            }
        }
    }
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

fn puzzle3_document_fixture_fields(
    document: &LoadedDocument,
) -> Result<(Option<String>, Vec<String>), serde_json::Error> {
    let theme = serde_json::to_string(&document.theme)?;
    let (scene_fields, level_bundle_names) = puzzle3_scene_fixture_fields(document);
    let mut out = String::new();
    out.push_str("  \"theme\": ");
    out.push_str(&theme);
    out.push_str(",\n");
    if let Some(scene_fields) = scene_fields {
        out.push_str(&scene_fields);
    } else {
        out.push_str("  \"currentScene\": \"playing\",\n");
        out.push_str("  \"scenes\": [\n");
        out.push_str("    {\n");
        out.push_str("      \"name\": \"playing\",\n");
        out.push_str("      \"puzzles\": [{ \"slot\": \"board\", \"model\": \"default\" }],\n");
        out.push_str("      \"components\": [{ \"kind\": \"puzzle3\", \"source\": \"board\" }]\n");
        out.push_str("    }\n");
        out.push_str("  ],");
    }
    Ok((Some(out), level_bundle_names))
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
    puzzle_scene::write_scene_layout_json(out, &scene.layout);
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
        let mut action = String::new();
        puzzle_scene::write_scene_effect_json(&mut action, &binding.effect);
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
    let default_level_menu_action = SceneEffect::Goto {
        scene: "playing".to_string(),
        params: vec![SceneEffectParam::Level(SceneExpr::Path(vec![
            "level".to_string(),
        ]))],
    };
    let options = puzzle_scene::SceneFixtureJsonOptions {
        frame_kind: Some("puzzle3"),
        default_level_menu_action: Some(&default_level_menu_action),
    };
    for component in &scene.components {
        let mut component_json = String::new();
        let mut note_level_source = |levels: &str| push_unique_string(level_bundle_names, levels);
        if puzzle_scene::write_scene_component_fixture_json(
            &mut component_json,
            component,
            options,
            push_scene_text_content_fixture_fields,
            &mut note_level_source,
        ) {
            if wrote_component {
                out.push_str(", ");
            }
            wrote_component = true;
            out.push_str(&component_json);
        }
    }
    out.push_str("]\n    }");
}

fn push_scene_text_content_fixture_fields(out: &mut String, content: &SceneTextContent) {
    match content {
        SceneTextContent::Literal(value) => {
            out.push_str("\"source\": \"literal\", \"value\": ");
            puzzle_scene::write_json_string(out, value);
        }
        SceneTextContent::Path(path) => {
            out.push_str("\"source\": \"path\", \"path\": ");
            puzzle_scene::write_json_string(out, &path.join("."));
        }
        SceneTextContent::Expr(expr) => {
            out.push_str("\"source\": \"expr\", \"content\": ");
            puzzle_scene::write_scene_expr_json(out, expr);
        }
    }
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
    input_buffer: InputBufferDef,
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
            input_buffer: InputBufferDef::default(),
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
    InputBuffer,
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
        trigger: "input_buffer",
        label: "input_buffer",
        action: ModelTopLevelDirective::InputBuffer,
        expected_group: Some(ModelTopLevelExpectedGroup::Config),
        authoring_surface: true,
    },
    HeaderChoiceAlternative {
        trigger: "animation",
        label: "animation",
        action: ModelTopLevelDirective::Animation,
        expected_group: None,
        authoring_surface: false,
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
        .filter(|alternative| {
            alternative.authoring_surface
                && !crate::authoring_grammar::authoring_head_surface(
                    crate::authoring_grammar::AuthoringKind::Root,
                    alternative.trigger,
                )
        })
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
    let logical_lines = logical_lines_with_locations(source)?;
    parse_document_source_parts_from_logical_lines(logical_lines)
}

fn parse_document_source_parts_from_surface_source(
    source: &str,
) -> Result<DocumentSourceParts, DiagnosticReport> {
    let surface = parse_surface_compile_document(source)?;
    parse_document_source_parts_from_surface_document(&surface)
}

fn parse_document_source_parts_from_surface_document(
    document: &SurfaceDocument,
) -> Result<DocumentSourceParts, DiagnosticReport> {
    if document.logical_lines.is_empty() {
        return Err(DiagnosticReport::error(
            "surface document missing compile logical lines".to_string(),
        ));
    }
    parse_document_source_parts_from_logical_lines(document.logical_lines.clone())
}

fn parse_document_source_parts_from_logical_lines(
    logical_lines: Vec<source::LogicalLine>,
) -> Result<DocumentSourceParts, DiagnosticReport> {
    let shell = parse_document_shell_lines(&logical_lines)?;
    let (model_lines, scenes) = split_document_scene_logical_lines(logical_lines)?;
    let model_lines = strip_document_shell_lines(&model_lines);
    let model_source = model_lines
        .iter()
        .map(|line| line.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    Ok(DocumentSourceParts {
        shell,
        model_source,
        model_lines,
        scenes,
    })
}

fn parse_document_shell(source: &str) -> Result<DocumentShell, DiagnosticReport> {
    let lines = logical_lines_with_locations(source)?;
    parse_document_shell_lines(&lines)
}

fn parse_document_shell_lines(
    lines: &[source::LogicalLine],
) -> Result<DocumentShell, DiagnosticReport> {
    let line_texts = lines
        .iter()
        .map(|line| line.text.clone())
        .collect::<Vec<_>>();
    let mut shell = DocumentShell::default();
    let mut index = 0;
    while index < lines.len() {
        let tokens = split_header_tokens(&line_texts[index]);
        match tokens.as_slice() {
            [] => {
                index += 1;
            }
            ["title", ..] => {
                shell.title = parse_metadata_text(&line_texts[index], "title")?;
                index += 1;
            }
            ["subtitle", ..] => {
                shell.subtitle = Some(parse_metadata_text(&line_texts[index], "subtitle")?);
                index += 1;
            }
            ["author", ..] => {
                shell.author = Some(parse_metadata_text(&line_texts[index], "author")?);
                index += 1;
            }
            ["homepage", ..] => {
                shell.homepage = Some(parse_metadata_text(&line_texts[index], "homepage")?);
                index += 1;
            }
            ["default_wait_time", ..] => {
                shell.default_wait_ms = parse_default_wait_time_directive(&line_texts[index])?;
                index += 1;
            }
            ["again_interval", ..] => {
                shell.default_again_ms = parse_again_interval_directive(&line_texts[index])?;
                index += 1;
            }
            ["input_buffer", ..] => {
                index = parse_input_buffer_block(&line_texts, index, &mut shell.input_buffer)?;
            }
            ["animation", ..] => {
                return Err(parse_error(
                    &line_texts[index],
                    "top-level animation block was removed; put tween_duration under puzzle render",
                ));
            }
            ["sounds"] => {
                if model_sounds_block_starts(&line_texts, index) {
                    index = skip_logical_block(&line_texts, index);
                } else {
                    index = parse_sounds_block(&line_texts, index, &mut shell.sounds)?;
                }
            }
            ["theme"] | ["theme", ..] => {
                index = parse_theme_statement(&line_texts, index, &mut shell.theme)?;
            }
            ["assets"] => {
                index = parse_assets_block(&line_texts, index, &mut shell.assets)?;
            }
            _ => break,
        }
    }
    Ok(shell)
}

fn strip_document_shell_source(source: &str) -> Result<String, DiagnosticReport> {
    let document = parse_surface_structure_document(source);
    let mut out = Vec::new();
    let mut index = 0;
    let mut shell_prefix = true;
    while index < document.lines.len() {
        let line = &document.lines[index];
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
                ["input_buffer", ..] | ["animation", ..] | ["sounds", ..] | ["assets", ..] => {
                    index = skip_surface_shell_block_by_syntax(&document, index);
                    continue;
                }
                ["theme", ..] => {
                    index = if surface_theme_line_is_block(&document, index) {
                        skip_surface_shell_block_by_syntax(&document, index)
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
                    | ["input_buffer", ..]
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
                ["input_buffer", ..] | ["animation", ..] | ["sounds", ..] | ["assets", ..] => {
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
                    | ["input_buffer", ..]
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

fn skip_surface_shell_block_by_syntax(document: &SurfaceDocument, index: usize) -> usize {
    let trimmed = strip_line_comment(&document.lines[index].content).trim();
    let mut next = index + 1;
    let mut brace_depth = raw_brace_delta(trimmed);
    if brace_depth > 0 {
        while next < document.lines.len() && brace_depth > 0 {
            let trimmed = strip_line_comment(&document.lines[next].content).trim();
            brace_depth += raw_brace_delta(trimmed);
            next += 1;
        }
        return next;
    }

    while next < document.lines.len() {
        let trimmed = strip_line_comment(&document.lines[next].content).trim();
        next += 1;
        if trimmed == BLOCK_CLOSE {
            break;
        }
    }
    next
}

fn surface_theme_line_is_block(document: &SurfaceDocument, index: usize) -> bool {
    let trimmed = strip_line_comment(&document.lines[index].content).trim();
    raw_brace_delta(trimmed) > 0
}

fn logical_theme_line_is_block(lines: &[source::LogicalLine], index: usize) -> bool {
    let trimmed = strip_line_comment(&lines[index].text).trim();
    raw_brace_delta(trimmed) > 0
}

fn skip_logical_block(lines: &[String], start: usize) -> usize {
    let mut depth = authoring_line_brace_delta(&lines[start]);
    if depth <= 0 {
        return start + 1;
    }
    let mut index = start + 1;
    while index < lines.len() {
        let next_depth = depth + authoring_line_brace_delta(&lines[index]);
        if next_depth <= 0 {
            return index + 1;
        }
        depth = next_depth;
        index += 1;
    }
    index
}

fn recover_after_directive_error(lines: &[String], index: usize) -> usize {
    if line_opens_recovery_block(&lines[index]) {
        skip_logical_block(lines, index)
    } else {
        index + 1
    }
}

fn line_opens_recovery_block(line: &str) -> bool {
    authoring_line_brace_delta(line) > 0
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

fn split_document_scene_logical_lines(
    logical_lines: Vec<source::LogicalLine>,
) -> Result<(Vec<source::LogicalLine>, Vec<SceneDef>), DiagnosticReport> {
    let lines = logical_lines
        .iter()
        .map(|line| line.text.clone())
        .collect::<Vec<_>>();
    let level_entries = collect_level_expansion_entries(&lines)?;
    let mut model_lines = Vec::new();
    let mut scenes = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let tokens = split_header_tokens(&lines[i]);
        if matches!(tokens.as_slice(), ["scene", ..]) {
            let (scene, next_i) = parse_scene_definition(&lines, i, &level_entries)?;
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

fn collect_level_expansion_entries(
    lines: &[String],
) -> Result<Vec<LevelExpansionEntry>, DiagnosticReport> {
    let mut entries = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let tokens = split_header_tokens(&lines[i]);
        match tokens.as_slice() {
            ["scene", ..] => {
                i = skip_logical_owner_block(lines, i, "scene")?;
            }
            ["puzzle", ..] => {
                i = collect_nested_level_expansion_entries(lines, i + 1, &mut entries)?;
            }
            ["levels", ..] => {
                i = collect_levels2_expansion_entries(lines, i, None, &mut entries)?;
            }
            ["level", ..] => {
                let (level, next_i) = parse_level_block(lines, i, entries.len())?;
                entries.push(LevelExpansionEntry {
                    name: level.name,
                    pack: level.pack,
                });
                i = next_i;
            }
            _ if is_levels3_header_line(&lines[i]) => {
                i = collect_levels3_expansion_entries(lines, i, &mut entries)?;
            }
            _ => i += 1,
        }
    }
    Ok(entries)
}

fn skip_logical_owner_block(
    lines: &[String],
    start: usize,
    block_name: &str,
) -> Result<usize, DiagnosticReport> {
    let mut depth = raw_brace_delta(strip_line_comment(&lines[start]));
    if depth <= 0 {
        depth = 1;
    }
    let mut i = start + 1;
    while i < lines.len() {
        depth += raw_brace_delta(strip_line_comment(&lines[i]));
        i += 1;
        if depth == 0 {
            return Ok(i);
        }
    }
    Err(parse_error(
        &lines[start],
        &format!("{block_name} block missing closing brace"),
    ))
}

fn collect_nested_level_expansion_entries(
    lines: &[String],
    start: usize,
    entries: &mut Vec<LevelExpansionEntry>,
) -> Result<usize, DiagnosticReport> {
    let mut i = start;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let tokens = split_header_tokens(&lines[i]);
        match tokens.as_slice() {
            ["levels", ..] => {
                i = collect_levels2_expansion_entries(lines, i, None, entries)?;
            }
            ["level", ..] => {
                let (level, next_i) = parse_level_block(lines, i, entries.len())?;
                entries.push(LevelExpansionEntry {
                    name: level.name,
                    pack: level.pack,
                });
                i = next_i;
            }
            _ if lines[i].trim_end().ends_with('{') => {
                let (entry, next_i) =
                    collect_authoring_entry(lines, i, AuthoringEntryOwner::GenericBlock)?;
                drop(entry);
                i = next_i;
            }
            _ => i += 1,
        }
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start.saturating_sub(1)],
            "puzzle missing closing brace",
        ));
    }
    Ok(i + 1)
}

fn collect_levels2_expansion_entries(
    lines: &[String],
    start: usize,
    default_puzzle: Option<&str>,
    entries: &mut Vec<LevelExpansionEntry>,
) -> Result<usize, DiagnosticReport> {
    let header = parse_levels_header(&lines[start], default_puzzle)?;
    let mut namespace_count = 0usize;
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let tokens = split_header_tokens(&lines[i]);
        match tokens.as_slice() {
            ["legend"] => {
                let (_, next_i) =
                    collect_authoring_entry(lines, i, AuthoringEntryOwner::GenericBlock)?;
                i = next_i;
            }
            ["level", ..] | ["{"] => {
                namespace_count += 1;
                let auto_name = puzzle_authoring::namespaced_unnamed_level_name(
                    header.pack.as_deref(),
                    entries.len(),
                    namespace_count,
                );
                let header_name = if matches!(tokens.as_slice(), ["{"]) {
                    auto_name
                } else {
                    parse_level_header_name_or_auto(&lines[i], auto_name)?
                };
                let name = if puzzle_authoring::is_braced_level_header(&lines[i])
                    || matches!(tokens.as_slice(), ["{"])
                {
                    braced_level_name_override_or(lines, i, header_name)?
                } else {
                    header_name
                };
                entries.push(LevelExpansionEntry {
                    name,
                    pack: header.pack.clone(),
                });
                let (_, next_i) = if puzzle_authoring::is_braced_level_header(&lines[i])
                    || matches!(tokens.as_slice(), ["{"])
                {
                    collect_authoring_entry(lines, i, AuthoringEntryOwner::GenericBlock)?
                } else {
                    collect_unbraced_level_entry(lines, i + 1)
                };
                i = next_i;
            }
            [] | ["legend", ..] => i += 1,
            _ => {
                namespace_count += 1;
                let name = puzzle_authoring::namespaced_unnamed_level_name(
                    header.pack.as_deref(),
                    entries.len(),
                    namespace_count,
                );
                entries.push(LevelExpansionEntry {
                    name,
                    pack: header.pack.clone(),
                });
                let (_, next_i) = collect_unbraced_level_entry(lines, i);
                i = next_i;
            }
        }
    }
    if i >= lines.len() {
        return Err(parse_error(&lines[start], "levels missing closing brace"));
    }
    Ok(i + 1)
}

fn collect_levels3_expansion_entries(
    lines: &[String],
    start: usize,
    entries: &mut Vec<LevelExpansionEntry>,
) -> Result<usize, DiagnosticReport> {
    let pack = parse_levels3_expansion_pack(&lines[start])?;
    let mut namespace_count = 0usize;
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let line = &lines[i];
        if line == "legend {" {
            let (_, next_i) = collect_authoring_entry(lines, i, AuthoringEntryOwner::GenericBlock)?;
            i = next_i;
            continue;
        }
        if puzzle_authoring::is_braced_level_header(line) || line == "{" {
            namespace_count += 1;
            let auto_name = puzzle_authoring::namespaced_unnamed_level_name(
                pack.as_deref(),
                entries.len(),
                namespace_count,
            );
            let header_name = if line == "{" {
                auto_name
            } else {
                parse_level_header_name_or_auto(line, auto_name)?
            };
            let name = braced_level_name_override_or(lines, i, header_name)?;
            entries.push(LevelExpansionEntry {
                name,
                pack: pack.clone(),
            });
            let (_, next_i) = collect_authoring_entry(lines, i, AuthoringEntryOwner::GenericBlock)?;
            i = next_i;
            continue;
        }
        i += 1;
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start],
            "levels3 block missing closing brace",
        ));
    }
    Ok(i + 1)
}

fn collect_unbraced_level_entry(lines: &[String], start: usize) -> (Vec<String>, usize) {
    let mut body = Vec::new();
    let mut i = start;
    while i < lines.len() {
        let tokens = split_header_tokens(&lines[i]);
        if is_block_close_line(&lines[i]) || matches!(tokens.as_slice(), ["level", ..] | ["{"]) {
            break;
        }
        body.push(lines[i].clone());
        i += 1;
    }
    (body, i)
}

fn is_levels3_header_line(line: &str) -> bool {
    line == "levels3 {"
        || line
            .strip_prefix("levels3 ")
            .is_some_and(|rest| rest.ends_with('{'))
}

fn parse_levels3_expansion_pack(line: &str) -> Result<Option<String>, DiagnosticReport> {
    let header = line
        .strip_prefix("levels3")
        .and_then(|rest| rest.strip_suffix('{'))
        .ok_or_else(|| parse_error(line, "levels3 block must end with {"))?
        .trim();
    if header.is_empty() {
        return Ok(None);
    }
    let parts = header.split_whitespace().collect::<Vec<_>>();
    match parts.as_slice() {
        [name] | [name, "of", _] => Ok(Some((*name).to_string())),
        _ => Err(parse_error(
            line,
            "levels3 header must be: levels3 [name [of model]] {",
        )),
    }
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
    let mut depth = authoring_line_brace_delta(&lines[start]);
    if depth <= 0 {
        return Ok((entry, default_scene, start + 1));
    }
    let mut i = start + 1;
    while i < lines.len() {
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        let next_depth = depth + authoring_line_brace_delta(line);
        if next_depth < 0 {
            return Err(parse_error(line, "closing brace without block"));
        }
        if next_depth == 0 {
            entry.push(logical_lines[i].clone());
            i += 1;
            return Ok((entry, default_scene, i));
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
        entry.push(logical_lines[i].clone());
        depth = next_depth;
        i += 1;
    }
    Ok((vec![logical_lines[start].clone()], None, start + 1))
}

fn skip_scene_layout_block(lines: &[String], start: usize) -> Result<usize, DiagnosticReport> {
    let mut depth = authoring_line_brace_delta(&lines[start]);
    if depth <= 0 {
        return Ok(start + 1);
    }
    let mut i = start + 1;
    while i < lines.len() {
        let line = &lines[i];
        let next_depth = depth + authoring_line_brace_delta(line);
        if next_depth < 0 {
            return Err(parse_error(line, "closing brace without block"));
        }
        i += 1;
        if next_depth == 0 {
            return Ok(i);
        }
        depth = next_depth;
    }
    Err(parse_error(&lines[start], "layout missing closing brace"))
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
    let (layout_block, next_i) = parse_scene_layout_block(&layout_lines, 0, &HashMap::new())?;
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
    let document = parse_surface_structure_document(source);
    for line in &document.lines {
        let tokens = line.tokens.iter().map(String::as_str).collect::<Vec<_>>();
        match (line.scope, tokens.as_slice()) {
            (None, ["puzzle", ..]) => has_2d = true,
            (None, ["puzzle3", ..]) => has_3d = true,
            (_, ["levels3", ..]) => has_3d = true,
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
            ".puzzle files cannot contain 3D puzzle3 or levels3 sections; use .puzzle3"
                .to_string(),
        )),
        (PuzzleSourceProfile::Puzzle3d, GameDocumentKind::Puzzle2d) => Err(DiagnosticReport::error(
            ".puzzle3 files must contain 3D puzzle3 or levels3 sections".to_string(),
        )),
    }
}

fn first_model_name(source: &str, kind: &str) -> Option<String> {
    all_model_names(source, kind).into_iter().next()
}

fn all_model_names(source: &str, kind: &str) -> Vec<String> {
    let document = parse_surface_structure_document(source);
    let mut names = Vec::new();
    for line in &document.lines {
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

#[cfg(test)]
mod document_surface_flow_tests {
    #[test]
    fn document_compile_consumes_surface_document_not_source_scanner() {
        let source = include_str!("lib_document.rs");
        let forbidden_fragments: &[&[&str]] = &[
            &["scan_source", "_context"],
            &["Source", "Context"],
            &["Source", "ContextLine"],
            &["skip_", "context", "_shell_block_by_syntax"],
            &["context", "_theme", "_line_is_block"],
        ];
        for parts in forbidden_fragments {
            let forbidden = parts.concat();
            assert!(
                !source.contains(&forbidden),
                "document compile flow must query parser-owned surface products, not source scanner products via {forbidden}"
            );
        }
    }

    #[test]
    fn document_compile_entrypoints_use_surface_compile_product() {
        let source = include_str!("lib_document.rs");
        assert!(
            source.contains("fn parse_document_source_parts_from_surface_source("),
            "document compile must share one surface-source to document-parts entrypoint"
        );
        assert!(
            source.contains("let surface = parse_surface_compile_document(source)?;"),
            "document parts construction must build the checked surface product before model-specific parsing"
        );
        assert!(
            source.contains("parse_document_source_parts_from_surface_source(source)?"),
            "2D/3D document compile must derive shell, scenes, and model lines from the shared surface product helper"
        );
        let forbidden = "fn parse_game2d_document(source: &str) -> Result<LoadedGame, DiagnosticReport> {\n    let parts = parse_document_source_parts(source)?;";
        assert!(
            !source.contains(forbidden),
            "parse_game2d_document must not bypass the surface product"
        );
    }

    #[test]
    fn standalone_puzzle3_parser_uses_surface_compile_product() {
        let source = include_str!("puzzle3_parse.rs");
        assert!(
            source.contains("crate::parse_surface_compile_document(source)"),
            "public parse_puzzle3d must consume the shared surface compile product"
        );
        for forbidden in [
            "Parser3::new",
            "preprocess_source_lines3",
            "split_structural_line3",
            "update_structural_block_stack3",
        ] {
            assert!(
                !source.contains(forbidden),
                "standalone puzzle3 parsing must not reintroduce raw source preprocessing via {forbidden}"
            );
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn resolve_game_entry(path: impl AsRef<Path>) -> Result<PathBuf, DiagnosticReport> {
    let path = path.as_ref();
    if path.is_dir() {
        if let Some(entry) = game_entry_in_directory(path)? {
            return Ok(entry);
        }
        return Err(DiagnosticReport::error(format!(
            "game folder must contain a .puzzle or .puzzle3 file that declares a puzzle model: {}",
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
        if source_declares_game_entry(&source) {
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
            "puzzle source file declares no puzzle model and no containing game entry was found: {}",
            path.display()
        )));
    }

    Err(DiagnosticReport::error(format!(
        "game entry not found: {}",
        path.display()
    )))
}

pub fn source_declares_game_entry(source: &str) -> bool {
    let mut depth = 0_i32;
    for raw_line in source.lines() {
        let code = raw_line.split("//").next().unwrap_or("");
        let trimmed = code.trim();
        if depth == 0 {
            let first = trimmed.split_whitespace().next().unwrap_or("");
            if matches!(first, "puzzle" | "puzzle3") {
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
    for line in logical_lines_with_locations(source)? {
        let tokens = split_header_tokens(&line.text);
        if matches!(tokens.as_slice(), ["import", _]) {
            paths.push(import_path(tokens[1], &line.text)?);
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
        if source_declares_game_entry(&source) {
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
            if source_declares_game_entry(&source) {
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

/// Expands a 2D puzzle entry against an explicit in-memory document set.
/// Hosts provide files; the language layer alone interprets import syntax.
pub fn expand_game_imports_from_documents(
    entry_path: &str,
    documents: &[(String, String)],
) -> Result<String, DiagnosticReport> {
    let entry = normalize_virtual_import_path(Path::new(entry_path));
    let sources = documents
        .iter()
        .map(|(path, source)| {
            (
                normalize_virtual_import_path(Path::new(path)),
                source.as_str(),
            )
        })
        .collect::<HashMap<_, _>>();
    let source = sources.get(&entry).copied().ok_or_else(|| {
        DiagnosticReport::error(format!(
            "workspace puzzle entry not found: {}",
            entry.display()
        ))
    })?;
    expand_virtual_game_imports(source, &entry, &sources, &mut vec![entry.clone()])
}

fn expand_virtual_game_imports(
    source: &str,
    current_path: &Path,
    sources: &HashMap<PathBuf, &str>,
    import_stack: &mut Vec<PathBuf>,
) -> Result<String, DiagnosticReport> {
    let mut out = String::new();
    for raw_line in source.split_inclusive('\n') {
        let content = raw_line.strip_suffix('\n').unwrap_or(raw_line);
        if let Some(requested) = import_directive_path(content)? {
            if requested.is_absolute() {
                return Err(DiagnosticReport::error(
                    "workspace imports must be relative".to_string(),
                ));
            }
            let base = current_path.parent().unwrap_or_else(|| Path::new(""));
            let resolved = normalize_virtual_import_path(&base.join(requested));
            if import_stack.contains(&resolved) {
                return Err(DiagnosticReport::error(format!(
                    "cyclic import: {}",
                    import_stack
                        .iter()
                        .chain(std::iter::once(&resolved))
                        .map(|path| path.display().to_string())
                        .collect::<Vec<_>>()
                        .join(" -> ")
                )));
            }
            let imported = sources.get(&resolved).copied().ok_or_else(|| {
                DiagnosticReport::error(format!(
                    "import not found: {} from {}",
                    resolved.display(),
                    current_path.display()
                ))
            })?;
            import_stack.push(resolved.clone());
            let expanded = expand_virtual_game_imports(imported, &resolved, sources, import_stack);
            import_stack.pop();
            let expanded = expanded?;
            out.push_str(&expanded);
            if !expanded.ends_with('\n') {
                out.push('\n');
            }
        } else {
            out.push_str(raw_line);
        }
    }
    Ok(out)
}

fn normalize_virtual_import_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(part) => normalized.push(part),
            Component::RootDir | Component::Prefix(_) => {}
        }
    }
    normalized
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
    parse_game2d_expanded_lines_with_shell_inner(&lines, &line_numbers, shell)
        .map_err(|report| report_with_source_line_numbers(report, &lines, &line_numbers))
}

fn lower_win_condition_strategy(
    condition: &ConditionAst,
    strategy: &mut SolverStrategy,
    object_layers: &HashMap<ObjectId, LayerId>,
    mark_names: &HashMap<String, MarkDef>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    input_names: &HashMap<String, InputId>,
    directions: &[Direction],
) -> Result<(), DiagnosticReport> {
    match condition {
        ConditionAst::All(conditions) => {
            for condition in conditions {
                lower_win_condition_strategy(
                    condition,
                    strategy,
                    object_layers,
                    mark_names,
                    value_sets,
                    maps,
                    input_names,
                    directions,
                )?;
            }
        }
        ConditionAst::AllObjectsOn { subjects, covers } => {
            let already_present = strategy.terms.iter().any(|term| {
                matches!(
                    &term.value,
                    QueryExpr::AllOnDistance {
                        subjects: existing_subjects,
                        covers: existing_covers,
                    } if existing_subjects == subjects && existing_covers == covers
                )
            });
            if !already_present {
                strategy.terms.push(SolverStrategyTerm {
                    direction: SolverStrategyDirection::Minimize,
                    value: QueryExpr::AllOnDistance {
                        subjects: subjects.clone(),
                        covers: covers.clone(),
                    },
                    weight: 1,
                });
            }
        }
        ConditionAst::InlineConditionNonZero(ConditionValueAst::NoneMatches(pattern)) => {
            let value = lower_condition_value_kind(
                &ConditionValueAst::CountMatches(pattern.clone()),
                input_names,
                object_layers,
                mark_names,
                value_sets,
                maps,
                directions,
            )?;
            strategy.terms.push(SolverStrategyTerm {
                direction: SolverStrategyDirection::Minimize,
                value: QueryExpr::Value(value),
                weight: 1,
            });
        }
        ConditionAst::InlineConditionNonZero(ConditionValueAst::NoneObjects(objects)) => {
            strategy.terms.push(SolverStrategyTerm {
                direction: SolverStrategyDirection::Minimize,
                value: QueryExpr::Value(ConditionValueKind::CountObjects(objects.clone())),
                weight: 1,
            });
        }
        ConditionAst::Any(_) => {}
        _ => {}
    }
    Ok(())
}

fn parse_game2d_expanded_lines_with_shell_inner(
    lines: &[String],
    line_numbers: &[usize],
    shell: &DocumentShell,
) -> Result<LoadedGame, DiagnosticReport> {
    let mut title = shell.title.clone();
    let mut subtitle = shell.subtitle.clone();
    let mut author = shell.author.clone();
    let mut homepage = shell.homepage.clone();
    let mut layer_count = None;
    let mut empty_char = None;
    let mut named_layers = HashMap::<String, u16>::new();
    let mut catalog = Catalog::default();
    let mut query_definitions = Vec::<QueryDefinitionAst>::new();
    let mut query_names = HashSet::<String>::new();
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
    let mut solver_strategy = None::<SolverStrategyAst>;
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
    let mut input_buffer = shell.input_buffer.clone();

    let mut diagnostics = Vec::new();
    let mut pending_visual_blocks = Vec::<usize>::new();
    let mut pending_level_blocks = Vec::<PendingLevelBlock>::new();
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
                &mut query_definitions,
                &mut query_names,
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
                &mut render_overlays,
                &mut model_sound_triggers,
                &mut model_operation_sounds,
                &mut solver_strategy,
                &mut named_conditions,
                &mut run_rules_on_level_start,
                &mut visuals,
                &mut render,
                &mut animation,
                &mut puzzle_screen,
                &mut pending_level_blocks,
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
                        "top-level `name` metadata was removed; use `title = <text>`",
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
                default_wait_ms = parse_default_wait_time_directive(line)?;
                i += 1;
            }
            ModelTopLevelDirective::AgainInterval => {
                default_again_ms = parse_again_interval_directive(line)?;
                i += 1;
            }
            ModelTopLevelDirective::InputBuffer => {
                i = parse_input_buffer_block(&lines, i, &mut input_buffer)?;
            }
            ModelTopLevelDirective::Animation => {
                diagnostics.extend(
                    parse_error(
                        line,
                        "top-level animation block was removed; put tween_duration under puzzle render",
                    )
                    .into_diagnostics(),
                );
                i = recover_after_directive_error(&lines, i);
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
                let (_, next_i) =
                    collect_authoring_entry(&lines, i, AuthoringEntryOwner::DocumentVisuals)?;
                i = next_i;
            }
            ModelTopLevelDirective::Levels => {
                pending_level_blocks.push(PendingLevelBlock::levels(i, None));
                let (_, next_i) = collect_levels_authoring_entry(&lines, i)?;
                i = next_i;
            }
            ModelTopLevelDirective::Level => {
                pending_level_blocks.push(PendingLevelBlock::level(i, None));
                let (_, next_i) = parse_level_block(&lines, i, 0)?;
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
    for pending_level in &pending_level_blocks {
        if let Err(report) = parse_pending_level_block(
            &lines,
            pending_level,
            &mut level_blocks,
            &mut catalog,
            &mut render_overlays,
            &mut empty_char,
        ) {
            diagnostics.extend(report.into_diagnostics());
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
            let body = parse_level_body(&level, &catalog, empty_char, &named_conditions)?;
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
    let queries = lower_query_definitions(
        &query_definitions,
        &catalog.object_names,
        &catalog.object_schemas,
        &catalog.maps,
        &catalog.object_groups,
        &catalog.variable_names,
        &catalog.object_layers,
        &catalog.mark_names,
        &catalog.visual_objects,
        &value_sets,
        &catalog.input_names,
        &effective_directions,
    )?;
    let mut solver_strategy = lower_solver_strategy(
        solver_strategy,
        &query_definitions,
        &catalog.object_names,
        &catalog.object_schemas,
        &catalog.maps,
        &catalog.object_groups,
        &catalog.variable_names,
        &catalog.object_layers,
        &catalog.mark_names,
        &catalog.visual_objects,
        &value_sets,
        &catalog.input_names,
        &effective_directions,
    )?;
    if let Some((_, win_condition)) = named_conditions
        .get("win_conditions")
        .or_else(|| named_conditions.get("goal"))
    {
        lower_win_condition_strategy(
            win_condition,
            &mut solver_strategy,
            &catalog.object_layers,
            &catalog.mark_names,
            &value_sets,
            &catalog.maps,
            &catalog.input_names,
            &effective_directions,
        )?;
    }
    let condition_defs = lower_condition_defs(
        condition_definitions,
        &catalog.object_layers,
        &catalog.mark_names,
        &value_sets,
        &catalog.maps,
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
                &catalog.variable_names,
                &catalog.condition_names,
                &visual_condition_reads,
                &catalog.mark_names,
                &catalog.visual_objects,
                &value_sets,
                &catalog.maps,
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
        &catalog.constant_variables,
    );
    warnings.extend(collect_visual_overwrite_warnings(&visuals));
    warnings.extend(collect_visual_sprite_grid_warnings(&visuals));
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
        &catalog.variable_names,
        &catalog.constant_variables,
        &catalog.condition_names,
        &visual_condition_reads,
        &catalog.mark_names,
        &model_sound_triggers,
        &animation,
        &value_sets,
        &catalog.maps,
        &effective_directions,
    )?;
    let mut display_rules = programs.visual_rules.clone();
    display_rules.sort();
    display_rules.dedup();
    let game = CompiledGame::new_with_mark_condition_defs_and_program(
        layer_count,
        catalog.object_defs,
        catalog.mark_defs,
        condition_defs,
        programs.main,
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
                &catalog.variable_defaults,
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

    warnings.extend(collect_mark_warnings(&game, &catalog.mark_names));
    let mark_labels = catalog
        .mark_names
        .iter()
        .map(|(name, def)| (def.id, name.clone()))
        .collect::<HashMap<_, _>>();

    Ok(LoadedGame {
        title,
        subtitle,
        author,
        homepage,
        game,
        warnings,
        default_wait_ms,
        default_again_ms,
        input_buffer,
        animation: animation.clone(),
        rule_animations: programs.rule_animations,
        rule_effects: programs.rule_effects,
        rule_debug_info: programs.rule_debug_info,
        level_start_program: programs.level_start,
        display_level_start_program: None,
        level_clear_program: programs.level_clear,
        last_level_clear_program: programs.last_level_clear,
        display_level_clear_program: None,
        display_program: programs.display,
        display_objects: visual_objects,
        display_rules,
        levels,
        run_rules_on_level_start,
        legend,
        controls,
        variables,
        scenes: Vec::new(),
        object_labels: catalog.object_labels,
        object_groups: catalog.object_groups,
        input_labels: catalog.input_labels,
        variable_labels: catalog.variable_labels,
        mark_labels,
        persistent_vars: catalog.persistent_vars,
        condition_labels: catalog.condition_labels,
        queries,
        conditions,
        goal,
        lose,
        solver_strategy,
        sounds,
        model_operation_sounds,
        theme,
        assets,
        visuals,
        render,
        screen: puzzle_screen,
    })
}

fn report_with_source_line_numbers(
    report: DiagnosticReport,
    lines: &[String],
    line_numbers: &[usize],
) -> DiagnosticReport {
    let mut next_search_start_by_source_line = HashMap::<String, usize>::new();
    let diagnostics = report
        .into_diagnostics()
        .into_iter()
        .map(|mut diagnostic| {
            if let Some(span) = &mut diagnostic.primary_span
                && span.line.is_none()
                && let Some(source_line) = span.source_line.as_deref()
                && let Some(line_number) = next_source_line_number(
                    source_line,
                    lines,
                    line_numbers,
                    &mut next_search_start_by_source_line,
                )
            {
                span.line = Some(line_number);
            }
            diagnostic
        })
        .collect();
    DiagnosticReport::from_diagnostics(diagnostics)
}

fn next_source_line_number(
    source_line: &str,
    lines: &[String],
    line_numbers: &[usize],
    next_search_start_by_source_line: &mut HashMap<String, usize>,
) -> Option<usize> {
    let start = next_search_start_by_source_line
        .get(source_line)
        .copied()
        .unwrap_or(0);
    let found = lines
        .iter()
        .enumerate()
        .skip(start)
        .find(|(_, line)| line.as_str() == source_line)
        .map(|(index, _)| index)?;
    next_search_start_by_source_line.insert(source_line.to_string(), found + 1);
    line_numbers.get(found).copied()
}

fn collect_dynamic_selector_warnings(
    definitions: &[RuleDefinitionAst],
    main_statements: Option<&[StatementAst]>,
    level_start_statements: Option<&[StatementAst]>,
    level_clear_statements: Option<&[StatementAst]>,
    last_level_clear_statements: Option<&[StatementAst]>,
    display_statements: Option<&[StatementAst]>,
    level_bodies: &[PreparedLevelBody],
    constant_variables: &[VariableId],
) -> Vec<String> {
    let mut warnings = Vec::new();
    for definition in definitions {
        collect_dynamic_selector_statement_warnings(
            &definition.statements,
            constant_variables,
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
        collect_dynamic_selector_statement_warnings(statements, constant_variables, &mut warnings);
    }
    for body in level_bodies {
        collect_dynamic_selector_statement_warnings(
            &body.level_start_statements,
            constant_variables,
            &mut warnings,
        );
        collect_dynamic_selector_statement_warnings(
            &body.level_clear_statements,
            constant_variables,
            &mut warnings,
        );
    }
    warnings
}

fn collect_dynamic_selector_statement_warnings(
    statements: &[StatementAst],
    constant_variables: &[VariableId],
    warnings: &mut Vec<String>,
) {
    for statement in statements {
        match statement {
            StatementAst::LocalRoutine { definition, .. } => {
                collect_dynamic_selector_statement_warnings(
                    &definition.statements,
                    constant_variables,
                    warnings,
                );
            }
            StatementAst::Rewrite(rewrite) => {
                collect_dynamic_selector_block_warnings(
                    &rewrite.before,
                    constant_variables,
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
                    constant_variables,
                    warnings,
                );
                collect_dynamic_selector_statement_warnings(
                    then_statements,
                    constant_variables,
                    warnings,
                );
                collect_dynamic_selector_statement_warnings(
                    else_statements,
                    constant_variables,
                    warnings,
                );
            }
            StatementAst::Block { statements, .. }
            | StatementAst::Fix { statements, .. }
            | StatementAst::RepeatUntil { statements, .. } => {
                collect_dynamic_selector_statement_warnings(
                    statements,
                    constant_variables,
                    warnings,
                );
            }
            StatementAst::If {
                then_statements,
                else_statements,
                ..
            } => {
                collect_dynamic_selector_statement_warnings(
                    then_statements,
                    constant_variables,
                    warnings,
                );
                collect_dynamic_selector_statement_warnings(
                    else_statements,
                    constant_variables,
                    warnings,
                );
            }
            StatementAst::Call { .. } | StatementAst::Effect { .. } => {}
        }
    }
}

fn collect_dynamic_selector_block_warnings(
    block: &PatternBlock,
    constant_variables: &[VariableId],
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
                        if constant_variables.contains(&guard.variable) {
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

fn collect_mark_warnings(
    game: &CompiledGame,
    mark_names: &HashMap<String, MarkDef>,
) -> Vec<String> {
    let labels = mark_names
        .iter()
        .map(|(name, def)| (def.id, name.as_str()))
        .collect::<HashMap<_, _>>();
    let mut warnings = Vec::new();
    for rule in game.rules() {
        for component in &rule.pattern.components {
            for cell in &component.cells {
                for cell_attr in cell
                    .require_mark
                    .iter()
                    .filter(|attr| attr.object.is_empty())
                {
                    for object_attr in cell
                        .require_mark
                        .iter()
                        .filter(|attr| !attr.object.is_empty())
                    {
                        if cell_attr.mark == object_attr.mark {
                            push_unique_warning(
                                &mut warnings,
                                format!(
                                    "mark `{}` appears on both a cell and an object occurrence in the same cell pattern",
                                    mark_label(cell_attr.mark, &labels)
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
            .flat_map(|cell| cell.require_mark.iter())
        {
            for write in &rule.writes {
                let Some((write_object, write_mark)) = write_mark_target(write) else {
                    continue;
                };
                if pattern_attr.mark != write_mark {
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
                            "mark `{}` changes anchor from {from} to {to} in a rewrite",
                            mark_label(pattern_attr.mark, &labels)
                        ),
                    );
                }
            }
        }
    }
    warnings
}

fn write_mark_target(write: &WriteOp) -> Option<(ObjectId, MarkId)> {
    match write {
        WriteOp::SetMark { object, mark, .. } => Some((*object, *mark)),
        _ => None,
    }
}

fn mark_label<'a>(mark: MarkId, labels: &HashMap<MarkId, &'a str>) -> String {
    labels
        .get(&mark)
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

#[derive(Clone, Copy)]
struct VisualSpriteGrid {
    width: u32,
    height: u32,
}

fn collect_visual_sprite_grid_warnings(visuals: &VisualsDef) -> Vec<String> {
    let grids = visuals
        .sprites
        .iter()
        .filter_map(|sprite| visual_sprite_grid(sprite).map(|grid| (sprite.name.as_str(), grid)))
        .collect::<Vec<_>>();
    let largest = grids
        .iter()
        .flat_map(|(_, grid)| [grid.width, grid.height])
        .max()
        .unwrap_or(1);
    if largest <= 1 {
        return Vec::new();
    }

    let mut warnings = Vec::new();
    for (name, grid) in grids {
        if largest % grid.width == 0 && largest % grid.height == 0 {
            continue;
        }
        push_unique_warning(
            &mut warnings,
            format!(
                "visual sprite `{name}` uses a {}x{} cell grid that does not divide the largest sprite grid {largest}; sprite grids should divide the largest grid because the renderer uses the largest sprite grid as the canvas unit",
                grid.width, grid.height
            ),
        );
    }
    warnings
}

fn visual_sprite_grid(sprite: &VisualSpriteDef) -> Option<VisualSpriteGrid> {
    if let Some(pixels) = sprite.pixels_per_cell {
        return Some(VisualSpriteGrid {
            width: pixels.width,
            height: pixels.height,
        });
    }
    match &sprite.kind {
        VisualSpriteKind::Solid(_) => Some(VisualSpriteGrid {
            width: 1,
            height: 1,
        }),
        VisualSpriteKind::Image { .. } => None,
        VisualSpriteKind::Ascii { pattern, .. } => Some(VisualSpriteGrid {
            width: pattern
                .iter()
                .map(|row| row.chars().count() as u32)
                .max()
                .unwrap_or(1),
            height: pattern.len().max(1) as u32,
        }),
    }
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
    for raw_line in source.split_inclusive('\n') {
        let content = raw_line.strip_suffix('\n').unwrap_or(raw_line);
        if let Some(path) = import_directive_path(content)? {
            let imported = read_import_expanded(base_dir, &path, import_stack, root)?;
            out.push_str(&imported);
            if !imported.ends_with('\n') {
                out.push('\n');
            }
            continue;
        }
        out.push_str(raw_line);
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

fn import_path(token: &str, line: &str) -> Result<PathBuf, DiagnosticReport> {
    let path = token
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .ok_or_else(|| parse_error(line, "import path must be quoted"))?;
    Ok(PathBuf::from(path))
}

fn import_directive_path(line: &str) -> Result<Option<PathBuf>, DiagnosticReport> {
    let line = strip_line_comment(line).trim();
    let tokens = split_header_tokens(line);
    if !matches!(tokens.as_slice(), ["import", _]) {
        return Ok(None);
    }
    import_path(tokens[1], line).map(Some)
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
