pub fn export_loaded_document_visual_fixture_json(
    document: &LoadedDocument,
) -> Result<String, DiagnosticReport> {
    let Some(LoadedDocumentModel::Puzzle3d {
        game,
        presentation,
        ..
    }) = document.single_model()
    else {
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
    export_visual_fixture_json_with_title_and_scenes(
        game,
        presentation,
        Some(&document.title),
        document_fields.as_deref(),
        &level_bundle_names,
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
    let expanded = expand_game_imports_for_file(&source, path)?;
    validate_source_profile(&expanded, profile)?;
    parse_game_document_with_profile(&expanded, profile)
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
    let logical_lines = logical_lines_with_locations(source)?;
    let entries = model_syntax::parse_document_entries(&logical_lines)?;
    let mut assets = AssetsDef::default();
    for entry in entries {
        if entry.directive != puzzle_authoring::PuzzleDirectiveSurface::DocumentShell
            || split_header_tokens(&entry.header.text).first().copied() != Some("assets")
        {
            continue;
        }
        model_syntax::validate_closed_entries(std::slice::from_ref(&entry), "assets")?;
        parse_assets_block(&document_entry_lines(&entry), 0, &mut assets)?;
    }
    Ok(assets)
}

fn parse_game2d_document(source: &str) -> Result<LoadedGame, DiagnosticReport> {
    let parts = parse_document_source_parts_from_surface_source(source)?;
    parse_game2d_from_document_parts(parts)
}

fn parse_game2d_from_document_parts(
    parts: DocumentSourceParts,
) -> Result<LoadedGame, DiagnosticReport> {
    let [model] = parts.models.as_slice() else {
        return Err(DiagnosticReport::error(
            "2D game entrypoint requires exactly one puzzle model".to_string(),
        ));
    };
    if model.dimension != ModelDimension::Two {
        return Err(DiagnosticReport::error(
            "2D entrypoint received a 3D model".to_string(),
        ));
    }
    let mut scenes = parts.scenes.clone();
    let model_name = model.name.clone();
    resolve_inferred_scene_puzzle_slots(&mut scenes, [("puzzle", &model_name)])?;
    let LoweredModel::Puzzle2d(mut game) = parse_model_from_document_parts(parts)? else {
        unreachable!("2D model dimension was validated before lowering");
    };
    resolve_default_wait_in_scenes(&mut scenes, game.default_wait_ms);
    game.scenes = add_implicit_model_scenes(scenes, [("puzzle", &model_name)]);
    resolve_scene_actions(&mut game.scenes, &game.input_labels)?;
    add_scene_input_key_controls(&game.scenes, &game.input_labels, &mut game.controls);
    Ok(game)
}

fn parse_model_from_document_parts(
    parts: DocumentSourceParts,
) -> Result<LoweredModel, DiagnosticReport> {
    let [model] = parts.models.as_slice() else {
        return Err(DiagnosticReport::error(
            "model entrypoint requires exactly one puzzle model".to_string(),
        ));
    };
    let [catalog] = parts.model_catalogs.as_slice() else {
        return Err(DiagnosticReport::error(
            "model entrypoint requires exactly one canonical puzzle Catalog".to_string(),
        ));
    };
    lower_model_with_shell(model, catalog, &parts.shell)
}

fn parse_game_document(source: &str) -> Result<LoadedDocument, DiagnosticReport> {
    validate_source_profile(source, PuzzleSourceProfile::Puzzle2d)?;
    parse_game_document_with_profile(source, PuzzleSourceProfile::Puzzle2d)
}

fn parse_game_document_with_profile(
    source: &str,
    profile: PuzzleSourceProfile,
) -> Result<LoadedDocument, DiagnosticReport> {
    let parts = parse_document_source_parts_from_surface_source(source)?;
    validate_document_source_profile(&parts, profile)?;
    parse_loaded_document_parts(parts)
}

fn validate_document_source_profile(
    parts: &DocumentSourceParts,
    source_profile: PuzzleSourceProfile,
) -> Result<(), DiagnosticReport> {
    if source_profile == PuzzleSourceProfile::Puzzle3d {
        let invalid = parts
            .models
            .iter()
            .find(|model| !model.dimension_is_explicit || model.dimension != ModelDimension::Three);
        if let Some(model) = invalid {
            return Err(DiagnosticReport::error_at_source_line_number(
                format!(
                    "puzzle `{}` in a .puzzle3 file must explicitly declare `dimension = 3`",
                    model.name
                ),
                model.source_line.clone(),
                model.source_line_number,
            ));
        }
    }

    Ok(())
}

fn parse_loaded_document_parts(
    parts: DocumentSourceParts,
) -> Result<LoadedDocument, DiagnosticReport> {
    let DocumentSourceParts {
        shell,
        models,
        model_catalogs,
        mut scenes,
        ..
    } = parts;
    if models.len() != model_catalogs.len() {
        return Err(DiagnosticReport::error(
            "canonical puzzle model/Catalog count mismatch".to_string(),
        ));
    }

    let model_kinds = models
        .iter()
        .map(|model| (model_dimension_kind(model.dimension), model.name.clone()))
        .collect::<Vec<_>>();
    resolve_inferred_scene_puzzle_slots(
        &mut scenes,
        model_kinds.iter().map(|(kind, name)| (*kind, name)),
    )?;
    scenes = add_implicit_model_scenes(
        scenes,
        model_kinds.iter().map(|(kind, name)| (*kind, name)),
    );
    resolve_default_wait_in_scenes(&mut scenes, shell.default_wait_ms);

    let mut lowered = Vec::with_capacity(models.len());
    let mut input_names = Vec::<String>::new();
    for (model, catalog) in models.iter().zip(&model_catalogs) {
        let product = lower_model_with_shell(model, catalog, &shell)?;
        match &product {
            LoweredModel::Puzzle2d(game) => {
                for name in game.input_labels.values() {
                    push_unique_string(&mut input_names, name);
                }
            }
            LoweredModel::Puzzle3d { game, .. } => {
                for name in game.input_labels.values() {
                    push_unique_string(&mut input_names, name);
                }
            }
        }
        lowered.push(product);
    }
    let mut input_labels = HashMap::with_capacity(input_names.len());
    for (index, name) in input_names.into_iter().enumerate() {
        let id = u16::try_from(index).map_err(|_| {
            DiagnosticReport::error("document declares more than 65536 distinct inputs".to_string())
        })?;
        input_labels.insert(InputId(id), name);
    }
    resolve_scene_actions(&mut scenes, &input_labels)?;

    let mut loaded_models = Vec::with_capacity(models.len());
    for (model, product) in models.into_iter().zip(lowered) {
        match product {
            LoweredModel::Puzzle2d(mut game) => {
                add_scene_input_key_controls(&scenes, &game.input_labels, &mut game.controls);
                game.scenes = scenes.clone();
                loaded_models.push(LoadedDocumentModel::Puzzle2d {
                    name: model.name,
                    game,
                });
            }
            LoweredModel::Puzzle3d {
                mut game,
                presentation,
            } => {
                add_scene_input_key_controls(&scenes, &game.input_labels, &mut game.controls);
                game.scenes = scenes.clone();
                loaded_models.push(LoadedDocumentModel::Puzzle3d {
                    name: model.name,
                    game,
                    presentation,
                });
            }
        }
    }
    Ok(loaded_document_from_shell(shell, scenes, loaded_models))
}

fn model_dimension_kind(dimension: ModelDimension) -> &'static str {
    match dimension {
        ModelDimension::Two => "puzzle",
        ModelDimension::Three => "puzzle3",
    }
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
        input_buffer: shell.input_buffer,
        animation: shell.animation,
        variables: shell.variables,
        sounds: shell.sounds,
        theme: shell.theme,
        assets: shell.assets,
        scenes,
        models,
    }
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
        let mut resolved_puzzle_kinds = HashMap::new();
        for puzzle in &scene.state.puzzles {
            if ambiguous.contains(&puzzle.model) {
                return Err(DiagnosticReport::error(format!(
                    "scene puzzle slot `{}` references an ambiguous puzzle model",
                    puzzle.name
                )));
            }
            let Some(kind) = model_kinds.get(&puzzle.model) else {
                return Err(DiagnosticReport::error(format!(
                    "scene puzzle slot `{}` references unknown puzzle model `{}`",
                    puzzle.name, puzzle.model
                )));
            };
            resolved_puzzle_kinds.insert(puzzle.name.clone(), kind.clone());
        }
        resolve_scene_viewport_projections(&mut scene.components, &resolved_puzzle_kinds);
    }

    Ok(())
}

fn resolve_scene_viewport_projections(
    components: &mut [SceneComponent],
    puzzle_kinds: &HashMap<String, String>,
) {
    for component in components {
        match component {
            SceneComponent::Viewport(viewport) => {
                if let Some(kind) = puzzle_kinds.get(&viewport.source) {
                    viewport.projection = if kind == "puzzle3" {
                        puzzle_scene::ViewportProjection::ThreeD
                    } else {
                        puzzle_scene::ViewportProjection::TwoD
                    };
                }
            }
            _ => {
                if let Some(children) = component.children_mut() {
                    resolve_scene_viewport_projections(children, puzzle_kinds);
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
                model: model_name.to_string(),
                initializer: ScenePuzzleInitializer::CurrentLevel,
                lifetime: SceneStateLifetime::Instance,
            }],
        },
        components: vec![scene_viewport_component(
            model_name,
            if kind == "puzzle3" {
                puzzle_scene::ViewportProjection::ThreeD
            } else {
                puzzle_scene::ViewportProjection::TwoD
            },
        )],
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
        .first()
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
        push_puzzle3_scene_json(&mut out, document, scene, &mut level_bundle_names);
    }
    out.push_str("\n  ],");
    (Some(out), level_bundle_names)
}

fn push_puzzle3_scene_json(
    out: &mut String,
    document: &LoadedDocument,
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
        if !document.models.iter().any(|model| {
            matches!(model, LoadedDocumentModel::Puzzle3d { name, .. } if name == &puzzle.model)
        }) {
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
    let options = puzzle_scene::SceneFixtureJsonOptions {
        viewport_projection: Some(puzzle_scene::ViewportProjection::ThreeD),
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
    input_buffer: InputBufferDef,
    animation: AnimationDef,
    variables: Vec<SceneVarDef>,
    sounds: SoundsDef,
    theme: ThemeDef,
    assets: AssetsDef,
}

#[derive(Clone, Debug)]
struct DocumentSourceParts {
    shell: DocumentShell,
    models: Vec<model_syntax::PuzzleModelSyntax>,
    model_catalogs: Vec<Catalog>,
    scenes: Vec<SceneDef>,
    recognition: crate::surface::ParserRecognition,
}

#[derive(Clone, Debug)]
enum PendingSceneSource {
    Explicit {
        name: String,
        lines: Vec<source::LogicalLine>,
    },
    Model {
        kind: String,
        name: String,
        layout: Option<Vec<source::LogicalLine>>,
    },
}

impl PendingSceneSource {
    fn name(&self) -> &str {
        match self {
            Self::Explicit { name, .. } | Self::Model { name, .. } => name,
        }
    }
}

impl Default for DocumentShell {
    fn default() -> Self {
        Self {
            title: "Untitled puzzle".to_string(),
            subtitle: None,
            author: None,
            homepage: None,
            default_wait_ms: DEFAULT_WAIT_MS,
            input_buffer: InputBufferDef::default(),
            animation: AnimationDef::default(),
            variables: Vec::new(),
            sounds: SoundsDef::default(),
            theme: ThemeDef::default(),
            assets: AssetsDef::default(),
        }
    }
}

const MODEL_TOP_LEVEL_STRUCTURAL_KEYWORDS: &[&str] =
    &["puzzle", "scene", "visuals", "levels", "level"];

pub(crate) fn model_top_level_completion_keywords() -> Vec<&'static str> {
    MODEL_TOP_LEVEL_STRUCTURAL_KEYWORDS
        .iter()
        .copied()
        .filter(|keyword| {
            !crate::authoring_grammar::authoring_head_surface(
                crate::authoring_grammar::AuthoringKind::Root,
                keyword,
            )
        })
        .collect()
}

#[cfg(test)]
fn parse_document_source_parts(source: &str) -> Result<DocumentSourceParts, DiagnosticReport> {
    let logical_lines = logical_lines_with_locations(source)?;
    parse_document_source_parts_from_logical_lines(logical_lines)
}

fn parse_document_source_parts_from_surface_source(
    source: &str,
) -> Result<DocumentSourceParts, DiagnosticReport> {
    ParseSnapshot::parse(source, None).into_strict_document_parts()
}

#[cfg(test)]
fn parse_document_source_parts_from_logical_lines(
    logical_lines: Vec<source::LogicalLine>,
) -> Result<DocumentSourceParts, DiagnosticReport> {
    let mut recognition = crate::surface::ParserRecognition::default();
    let (model_lines, pending_scenes) =
        split_document_scene_sources(logical_lines, &mut recognition)?;
    let document_entries = model_syntax::parse_document_entries(&model_lines)?;
    model_syntax::validate_closed_entries(&document_entries, "document")?;
    let shell = parse_document_shell_entries(&document_entries)?;
    let models = model_syntax::parse_puzzle_models_from_document_entries(&document_entries)?;
    model_syntax::validate_puzzle_model_diagnostics(&models)?;
    let scenes = parse_pending_scene_sources(&pending_scenes, &models, &mut recognition)?;
    let mut model_catalogs = Vec::with_capacity(models.len());
    for model in &models {
        let parsed_catalog = build_puzzle_catalog(model);
        recognition.merge(parsed_catalog.recognition);
        model_catalogs.push(parsed_catalog.value?);
    }
    Ok(DocumentSourceParts {
        shell,
        models,
        model_catalogs,
        scenes,
        recognition,
    })
}

fn parse_document_shell_entries(
    entries: &[model_syntax::PuzzleEntrySyntax],
) -> Result<DocumentShell, DiagnosticReport> {
    let mut shell = DocumentShell::default();
    for entry in entries {
        let tokens = split_header_tokens(&entry.header.text);
        match entry.directive {
            puzzle_authoring::PuzzleDirectiveSurface::Metadata => match tokens.as_slice() {
            ["title", ..] => {
                shell.title = parse_metadata_text(&entry.header, "title")?;
            }
            ["subtitle", ..] => {
                shell.subtitle = Some(parse_metadata_text(&entry.header, "subtitle")?);
            }
            ["author", ..] => {
                shell.author = Some(parse_metadata_text(&entry.header, "author")?);
            }
            ["homepage", ..] => {
                shell.homepage = Some(parse_metadata_text(&entry.header, "homepage")?);
            }
            ["default_wait_time", ..] => {
                shell.default_wait_ms = parse_default_wait_time_directive(&entry.header)?;
            }
            ["theme", ..] => {
                let lines = document_entry_lines(entry);
                parse_theme_statement(&lines, 0, &mut shell.theme)?;
            }
            _ => {
                return Err(parse_error(
                    &entry.header,
                    "unknown document metadata directive",
                ));
            }
        },
            puzzle_authoring::PuzzleDirectiveSurface::Variable => {
                shell.variables.push(parse_top_level_var_directive(
                    &tokens,
                    &entry.header,
                )?);
            }
            puzzle_authoring::PuzzleDirectiveSurface::InputBuffer => {
                let lines = document_entry_lines(entry);
                parse_input_buffer_block(&lines, 0, &mut shell.input_buffer)?;
            }
            puzzle_authoring::PuzzleDirectiveSurface::RemovedAnimation => {
                return Err(parse_error(
                    &entry.header,
                    "top-level animation block was removed; put tween_duration under puzzle render",
                ));
            }
            puzzle_authoring::PuzzleDirectiveSurface::DocumentShell => {
                let lines = document_entry_lines(entry);
                match tokens.first().copied() {
                    Some("sounds") => {
                        parse_sounds_block(&lines, 0, &mut shell.sounds)?;
                    }
                    Some("theme") => {
                        parse_theme_statement(&lines, 0, &mut shell.theme)?;
                    }
                    Some("assets") => {
                        parse_assets_block(&lines, 0, &mut shell.assets)?;
                    }
                    _ => {
                        return Err(parse_error(
                            &entry.header,
                            "unknown document shell directive",
                        ));
                    }
                }
            }
            _ => {}
        }
    }
    Ok(shell)
}

fn document_entry_lines(
    entry: &model_syntax::PuzzleEntrySyntax,
) -> Vec<source::LogicalLine> {
    let mut lines = Vec::with_capacity(entry.body.len() + 2);
    lines.push(entry.header.clone());
    lines.extend(entry.body.iter().cloned());
    if let Some(closing) = &entry.closing {
        lines.push(closing.clone());
    }
    lines
}

fn split_document_scene_sources(
    logical_lines: Vec<source::LogicalLine>,
    recognition: &mut crate::surface::ParserRecognition,
) -> Result<(Vec<source::LogicalLine>, Vec<PendingSceneSource>), DiagnosticReport> {
    predeclare_document_owner_completion_symbols(&logical_lines, recognition);
    let mut model_lines = Vec::new();
    let mut scenes = Vec::<PendingSceneSource>::new();
    let mut model_scene_indices = HashMap::<String, usize>::new();
    let mut i = 0;
    while i < logical_lines.len() {
        let tokens = split_header_tokens(&logical_lines[i]);
        if matches!(tokens.as_slice(), ["scene", ..]) {
            let declaration = crate::syntax::named_block_declaration_syntax(&tokens, "scene")
                .ok_or_else(|| {
                    parse_error(
                        &logical_lines[i],
                        "scene header must be: scene <name>[(param...)]",
                    )
                })?;
            let (name, _) = parse_scene_name_and_params(declaration.name, &logical_lines[i])?;
            let (lines, next_i) = collect_authoring_entry(
                &logical_lines,
                i,
                AuthoringEntryOwner::SceneDefinition,
            )?;
            let scene = PendingSceneSource::Explicit {
                name: name.clone(),
                lines,
            };
            if let Some(index) = model_scene_indices.remove(scene.name()) {
                scenes[index] = scene;
            } else {
                scenes.push(scene);
            }
            i = next_i;
        } else if let Some((kind, name)) = model_header_name(tokens.as_slice()) {
            let (entry, layout, next_i) =
                extract_default_model_scene_source(&logical_lines, i)?;
            model_lines.extend(entry);
            if !scenes.iter().any(|scene| scene.name() == name) {
                let scene = PendingSceneSource::Model {
                    kind: kind.to_string(),
                    name: name.to_string(),
                    layout,
                };
                model_scene_indices.insert(name.to_string(), scenes.len());
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

fn parse_pending_scene_sources(
    pending: &[PendingSceneSource],
    models: &[model_syntax::PuzzleModelSyntax],
    recognition: &mut crate::surface::ParserRecognition,
) -> Result<Vec<SceneDef>, DiagnosticReport> {
    let levels = models
        .iter()
        .flat_map(|model| {
            model.body.levels.levels.iter().map(|level| LevelProjectionEntry {
                name: level.name.clone(),
                pack: level.pack.clone(),
                puzzle: level
                    .puzzle
                    .clone()
                    .unwrap_or_else(|| model.name.clone()),
            })
        })
        .collect::<Vec<_>>();
    pending
        .iter()
        .map(|source| match source {
            PendingSceneSource::Explicit { lines, .. } => {
                let (scene, next) = parse_scene_definition(lines, 0, &levels, recognition)?;
                debug_assert!(
                    next == lines.len()
                        || (next + 1 == lines.len() && is_block_close_line(&lines[next]))
                );
                Ok(scene)
            }
            PendingSceneSource::Model {
                kind,
                name,
                layout: Some(lines),
            } => parse_default_model_scene(lines, kind, name, &levels),
            PendingSceneSource::Model {
                kind,
                name,
                layout: None,
            } => Ok(implicit_model_scene(kind, name)),
        })
        .collect()
}

fn predeclare_document_owner_completion_symbols(
    lines: &[source::LogicalLine],
    recognition: &mut crate::surface::ParserRecognition,
) {
    let mut depth = 0i32;
    for line in lines {
        if depth == 0 {
            let tokens = split_header_tokens(line);
            if matches!(tokens.first(), Some(&"scene")) {
                if let Some(declaration) =
                    crate::syntax::named_block_declaration_syntax(&tokens, "scene")
                    && let Ok((name, _)) = parse_scene_name_and_params(declaration.name, line)
                {
                    recognition.completion_symbols.scenes.insert(name);
                }
            } else if let Some((_, name)) = model_header_name(tokens.as_slice()) {
                recognition
                    .completion_symbols
                    .puzzles
                    .insert(name.to_string());
            }
        }
        depth += line.structural_brace_delta();
    }
}

fn model_header_name<'a>(tokens: &'a [&'a str]) -> Option<(&'a str, &'a str)> {
    match tokens {
        ["puzzle", name, ..] => Some((tokens[0], *name)),
        _ => None,
    }
}

fn extract_default_model_scene_source(
    logical_lines: &[source::LogicalLine],
    start: usize,
) -> Result<
    (
        Vec<source::LogicalLine>,
        Option<Vec<source::LogicalLine>>,
        usize,
    ),
    DiagnosticReport,
> {
    let mut entry = vec![logical_lines[start].clone()];
    let mut default_scene = None;
    let mut depth = logical_lines[start].structural_brace_delta();
    if depth <= 0 {
        return Ok((entry, default_scene, start + 1));
    }
    let mut i = start + 1;
    while i < logical_lines.len() {
        let line = &logical_lines[i];
        let tokens = split_header_tokens(line);
        let next_depth = depth + line.structural_brace_delta();
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
            let next_i = skip_scene_layout_block(logical_lines, i)?;
            default_scene = Some(logical_lines[i..next_i].to_vec());
            i = next_i;
            continue;
        }
        entry.push(logical_lines[i].clone());
        depth = next_depth;
        i += 1;
    }
    Ok((entry, default_scene, i))
}

fn skip_scene_layout_block(
    lines: &[source::LogicalLine],
    start: usize,
) -> Result<usize, DiagnosticReport> {
    let mut depth = lines[start].structural_brace_delta();
    if depth <= 0 {
        return Ok(start + 1);
    }
    let mut i = start + 1;
    while i < lines.len() {
        let line = &lines[i];
        let next_depth = depth + line.structural_brace_delta();
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
    layout_lines: &[source::LogicalLine],
    kind: &str,
    name: &str,
    levels: &[LevelProjectionEntry],
) -> Result<SceneDef, DiagnosticReport> {
    let mut recognition = crate::surface::ParserRecognition::default();
    let iterables = scene_iterable_catalog(levels, &SceneResources::default());
    let (layout_block, next_i) = parse_scene_layout_block(
        layout_lines,
        0,
        SceneLayoutOwner::ModelDefault {
            puzzle_slot: name,
            iterables: &iterables,
        },
        &mut recognition,
    )?;
    debug_assert_eq!(next_i, layout_lines.len());
    let mut scene = implicit_model_scene(kind, name);
    scene.layout = layout_block.layout;
    scene.state.variables.extend(layout_block.state.variables);
    scene.state.puzzles.extend(layout_block.state.puzzles);
    scene.components = layout_block.components;
    Ok(scene)
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
    _profile: PuzzleSourceProfile,
) -> Result<(), DiagnosticReport> {
    let document = parse_surface_structure_document(source);
    if document.lines.iter().any(|line| {
        line.scope.is_none() && line.tokens.first().is_some_and(|token| token == "puzzle3")
    }) {
        return Err(DiagnosticReport::error(
            "`puzzle3` was removed; use `puzzle <name> { dimension = 3 ... }`".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod document_surface_flow_tests {
    use super::*;

    #[test]
    fn strict_surface_compile_uses_one_canonical_scan() {
        let source = "puzzle board {\nrules {\n}\n}\n";
        let (parsed, canonical_scans) =
            source::count_canonical_scans(|| parse_surface_compile_document(source));

        parsed.unwrap();
        assert_eq!(canonical_scans, 1);
    }

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
            source.contains("ParseSnapshot::parse(source, None).into_strict_document_parts()"),
            "document parts construction must consume the checked parser snapshot directly"
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
    fn standalone_puzzle3_parser_does_not_exist() {
        let parser_path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/puzzle3_parse.rs");
        assert!(
            !parser_path.exists(),
            "3D parsing must remain part of the canonical parser"
        );
        assert!(!include_str!("lib.rs").contains("mod puzzle3_parse"));
    }

    #[test]
    fn source_dimension_selects_lowering_independently_of_puzzle_extension() {
        let document = parse_game_for_path(
            r#"
puzzle space {
dimension = 3
slots {
actor = Player
}
rules {
}
}
"#,
            "space.puzzle",
        )
        .unwrap();

        assert!(matches!(
            document.single_model(),
            Some(LoadedDocumentModel::Puzzle3d { name, .. }) if name == "space"
        ));
    }

    #[test]
    fn puzzle3_extension_does_not_implicitly_select_dimension_three() {
        let error = parse_game_for_path(
            r#"
puzzle space {
slots {
actor = Player
}
rules {
}
}
"#,
            "space.puzzle3",
        )
        .unwrap_err()
        .to_string();

        assert!(
            error.contains("must explicitly declare `dimension = 3`"),
            "{error}"
        );
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
            if first == "puzzle" {
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
    documents: &[WorkspaceSourceDocument],
) -> Result<String, DiagnosticReport> {
    Ok(expand_game_imports_from_documents_with_origins(entry_path, documents)?.source)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpandedWorkspaceSource {
    pub source: String,
    line_origins: Vec<WorkspaceSourceLineOrigin>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct WorkspaceSourceLineOrigin {
    path: String,
    line: usize,
}

impl ExpandedWorkspaceSource {
    pub fn remap_diagnostic_report(&self, report: DiagnosticReport) -> DiagnosticReport {
        let diagnostics = report
            .into_diagnostics()
            .into_iter()
            .map(|mut diagnostic| {
                let Some(span) = diagnostic.primary_span.as_mut() else {
                    return diagnostic;
                };
                let Some(line) = span.line else {
                    return diagnostic;
                };
                let Some(origin) = self.line_origins.get(line.saturating_sub(1)) else {
                    return diagnostic;
                };
                span.file = Some(origin.path.clone());
                span.line = Some(origin.line);
                diagnostic
            })
            .collect();
        DiagnosticReport::from_diagnostics(diagnostics)
    }
}

/// Expands workspace imports while retaining the original file and line for
/// every emitted source line. Consumers that compile the expanded source must
/// remap their diagnostics through the returned value before exposing them.
pub fn expand_game_imports_from_documents_with_origins(
    entry_path: &str,
    documents: &[WorkspaceSourceDocument],
) -> Result<ExpandedWorkspaceSource, DiagnosticReport> {
    let entry = normalize_virtual_import_path(Path::new(entry_path));
    let mut sources = HashMap::new();
    for document in documents {
        let path = normalize_virtual_import_path(Path::new(&document.path));
        if path.as_os_str().is_empty() {
            return Err(DiagnosticReport::error(
                "workspace document path must not be empty".to_string(),
            ));
        }
        if sources
            .insert(path.clone(), document.source.as_str())
            .is_some()
        {
            return Err(DiagnosticReport::error(format!(
                "duplicate workspace document path: {}",
                path.display()
            )));
        }
    }
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
) -> Result<ExpandedWorkspaceSource, DiagnosticReport> {
    let mut out = String::new();
    let mut line_origins = Vec::new();
    for (line_index, raw_line) in source.split_inclusive('\n').enumerate() {
        let source_line_number = line_index + 1;
        let content = raw_line.strip_suffix('\n').unwrap_or(raw_line);
        let requested = import_directive_path(content).map_err(|report| {
            workspace_import_diagnostic(report, current_path, source_line_number, content)
        })?;
        if let Some(requested) = requested {
            if requested.is_absolute() {
                return Err(workspace_import_diagnostic(
                    DiagnosticReport::error("workspace imports must be relative"),
                    current_path,
                    source_line_number,
                    content,
                ));
            }
            let base = current_path.parent().unwrap_or_else(|| Path::new(""));
            let resolved = normalize_virtual_import_path(&base.join(requested));
            if import_stack.contains(&resolved) {
                return Err(workspace_import_diagnostic(
                    DiagnosticReport::error(format!(
                        "cyclic import: {}",
                        import_stack
                            .iter()
                            .chain(std::iter::once(&resolved))
                            .map(|path| path.display().to_string())
                            .collect::<Vec<_>>()
                            .join(" -> ")
                    )),
                    current_path,
                    source_line_number,
                    content,
                ));
            }
            let imported = sources.get(&resolved).copied().ok_or_else(|| {
                workspace_import_diagnostic(
                    DiagnosticReport::error(format!(
                        "import not found: {} from {}",
                        resolved.display(),
                        current_path.display()
                    )),
                    current_path,
                    source_line_number,
                    content,
                )
            })?;
            import_stack.push(resolved.clone());
            let expanded = expand_virtual_game_imports(imported, &resolved, sources, import_stack);
            import_stack.pop();
            let expanded = expanded?;
            line_origins.extend(expanded.line_origins);
            if expanded.source.is_empty() {
                line_origins.push(WorkspaceSourceLineOrigin {
                    path: resolved.display().to_string(),
                    line: 1,
                });
            }
            out.push_str(&expanded.source);
            if !expanded.source.ends_with('\n') {
                out.push('\n');
            }
        } else {
            out.push_str(raw_line);
            line_origins.push(WorkspaceSourceLineOrigin {
                path: current_path.display().to_string(),
                line: source_line_number,
            });
        }
    }
    Ok(ExpandedWorkspaceSource {
        source: out,
        line_origins,
    })
}

fn workspace_import_diagnostic(
    report: DiagnosticReport,
    path: &Path,
    line: usize,
    source_line: &str,
) -> DiagnosticReport {
    let diagnostics = report
        .into_diagnostics()
        .into_iter()
        .map(|mut diagnostic| {
            let span = diagnostic
                .primary_span
                .get_or_insert_with(|| DiagnosticSpan::source_line(source_line));
            span.file = Some(path.display().to_string());
            span.line = Some(line);
            if span.source_line.is_none() {
                span.source_line = Some(source_line.to_string());
            }
            diagnostic
        })
        .collect();
    DiagnosticReport::from_diagnostics(diagnostics)
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

enum LoweredModel {
    Puzzle2d(LoadedGame),
    Puzzle3d {
        game: LoadedGridGame<3, puzzle_core::Size3>,
        presentation: SpatialPresentation,
    },
}

fn lower_model_with_shell(
    model: &model_syntax::PuzzleModelSyntax,
    model_catalog: &Catalog,
    shell: &DocumentShell,
) -> Result<LoweredModel, DiagnosticReport> {
    lower_model_with_shell_inner(model, model_catalog, shell)
}

fn lower_win_condition_strategy(
    condition: &ConditionAst,
    strategy: &mut CanonicalSolverStrategy,
    object_layers: &HashMap<ObjectId, LayerId>,
    mark_names: &HashMap<String, MarkDef>,
    value_sets: &HashMap<String, Vec<String>>,
    maps: &HashMap<String, ValueMap>,
    input_names: &HashMap<String, InputId>,
    directions: &[OrientationEnvironment],
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
                    CanonicalQueryExpr::AllOnDistance {
                        subjects: existing_subjects,
                        covers: existing_covers,
                    } if existing_subjects == subjects && existing_covers == covers
                )
            });
            if !already_present {
                strategy.terms.push(CanonicalSolverStrategyTerm {
                    direction: SolverStrategyDirection::Minimize,
                    value: CanonicalQueryExpr::AllOnDistance {
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
            strategy.terms.push(CanonicalSolverStrategyTerm {
                direction: SolverStrategyDirection::Minimize,
                value: CanonicalQueryExpr::Value(value),
                weight: 1,
            });
        }
        ConditionAst::InlineConditionNonZero(ConditionValueAst::NoneObjects(objects)) => {
            strategy.terms.push(CanonicalSolverStrategyTerm {
                direction: SolverStrategyDirection::Minimize,
                value: CanonicalQueryExpr::Value(CanonicalConditionValueKind::CountObjects(
                    objects.clone(),
                )),
                weight: 1,
            });
        }
        ConditionAst::Any(_) => {}
        _ => {}
    }
    Ok(())
}

fn lower_model_with_shell_inner(
    model: &model_syntax::PuzzleModelSyntax,
    model_catalog: &Catalog,
    shell: &DocumentShell,
) -> Result<LoweredModel, DiagnosticReport> {
    let title = shell.title.clone();
    let subtitle = shell.subtitle.clone();
    let author = shell.author.clone();
    let homepage = shell.homepage.clone();
    let mut layer_count = None;
    let mut empty_char = Some('.');
    let mut named_layers = HashMap::<String, u16>::new();
    let mut catalog = Catalog::default();
    let mut query_definitions = Vec::<QueryDefinitionAst>::new();
    let mut query_names = HashSet::<String>::new();
    let mut condition_definitions = Vec::<ConditionDefinitionAst>::new();
    let mut controls = Controls::default();
    let mut directions = Vec::<DirectionalInput>::new();
    let mut rule_definitions = Vec::<RuleDefinitionAst>::new();
    let mut main_statements = None;
    let mut main_local_frame = None;
    let mut level_start_statements = None;
    let mut level_start_local_frame = None;
    let mut level_clear_statements = None;
    let mut level_clear_local_frame = None;
    let mut last_level_clear_statements = None;
    let mut last_level_clear_local_frame = None;
    let mut level_blocks = Vec::<LevelBlock>::new();
    let mut puzzle_models = Vec::<String>::new();
    let variables = shell.variables.clone();
    let mut render_overlays = Vec::<(Vec<ObjectId>, char)>::new();
    let mut model_sound_triggers = Vec::<ModelSoundTriggerSpec>::new();
    let mut model_operation_sounds = Vec::<ModelOperationSoundSpec>::new();
    let mut solver_strategy = None::<SolverStrategyAst>;
    let mut named_conditions = HashMap::<String, (String, ConditionAst)>::new();
    let mut run_rules_on_level_start = false;
    let mut visuals = VisualsDef::default();
    let mut render = PuzzleRenderDef::default();
    let mut animation = shell.animation.clone();
    let sounds = shell.sounds.clone();
    let theme = shell.theme.clone();
    let assets = shell.assets.clone();
    let mut puzzle_screen = PuzzleScreenDef::default();
    let default_wait_ms = shell.default_wait_ms;
    let input_buffer = shell.input_buffer.clone();

    let mut parser_recognition = crate::surface::ParserRecognition::default();
    puzzle_models.push(lower_puzzle_model(
        model,
        model_catalog,
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
        &mut level_blocks,
        &mut parser_recognition,
    )?);

    refresh_layer_tags_and_value_sets(&mut named_layers, &mut catalog);
    let layer_count =
        layer_count.ok_or_else(|| DiagnosticReport::error("missing slots".to_string()))?;
    resolve_level_block_puzzles(&mut level_blocks, &puzzle_models)?;
    let prepared_level_bodies = level_blocks
        .into_iter()
        .map(|level| {
            let puzzle = level
                .puzzle
                .clone()
                .expect("level puzzle was resolved before preparation");
            let body = parse_level_body(&level, &catalog, &named_conditions)?;
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
                rules_before_statements: body.rules_before_statements,
                rules_after_statements: body.rules_after_statements,
            })
        })
        .collect::<Result<Vec<_>, DiagnosticReport>>()?;
    add_default_restart_handler(main_statements.as_mut());
    add_implicit_input_guards_to_catalog(
        &rule_definitions,
        main_statements.as_deref(),
        level_start_statements.as_deref(),
        level_clear_statements.as_deref(),
        &prepared_level_bodies,
        &named_conditions,
        &mut catalog,
    )?;
    let spatial_direction_names = catalog
        .value_sets
        .get("directions")
        .cloned()
        .unwrap_or_default();
    if !directions_include_all_spatial(&directions, &spatial_direction_names, &catalog.input_names)
    {
        add_spatial_directions("default inputs", &mut catalog, &mut directions)?;
    }
    add_default_non_direction_inputs("default inputs", &mut catalog)?;
    add_default_key_controls(model.dimension, &catalog.input_names, &mut controls);
    let effective_directional_inputs = directions.clone();
    let dimension = model.dimension;
    let spatial_domain = SpatialDomain::new(dimension);
    let effective_directions = effective_directional_inputs
        .iter()
        .map(|binding| spatial_domain.expand_directional_input(binding))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    let value_sets = catalog_value_sets(&catalog);
    let queries = lower_query_definitions(
        &query_definitions,
        &catalog.object_names,
        &catalog.object_schemas,
        &catalog.maps,
        &catalog.object_groups,
        &catalog.variable_names,
        &catalog.object_layers,
        &catalog.mark_names,
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
                &catalog.mark_names,
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
    let model_sound_triggers = resolve_model_sound_triggers(&model_sound_triggers, &catalog)?;
    let model_operation_sounds = resolve_model_operation_sounds(&model_operation_sounds);
    let mut warnings = collect_dynamic_selector_warnings(
        &rule_definitions,
        main_statements.as_deref(),
        level_start_statements.as_deref(),
        level_clear_statements.as_deref(),
        last_level_clear_statements.as_deref(),
        &prepared_level_bodies,
        &catalog.constant_variables,
    );
    warnings.extend(collect_visual_overwrite_warnings(&visuals));
    warnings.extend(collect_visual_grid_warnings(&visuals));
    let visual_names = visuals
        .entries
        .iter()
        .map(|visual| visual.name.clone())
        .collect::<HashSet<_>>();
    let mut programs = lower_programs(
        rule_definitions,
        main_statements,
        main_local_frame,
        level_start_statements,
        level_start_local_frame,
        level_clear_statements,
        level_clear_local_frame,
        last_level_clear_statements,
        last_level_clear_local_frame,
        &prepared_level_bodies,
        &catalog.object_layers,
        &catalog.input_names,
        &catalog.variable_names,
        &catalog.constant_variables,
        &catalog.condition_names,
        &catalog.mark_names,
        &model_sound_triggers,
        &visual_names,
        &animation,
        &value_sets,
        &catalog.maps,
        &effective_directions,
    )?;
    let canonical_game =
        puzzle_core::GridCompiledGame::<3>::new_with_mark_condition_defs_and_program(
            layer_count,
            catalog.object_defs.clone(),
            catalog.mark_defs.clone(),
            condition_defs.clone(),
            programs.main.clone(),
        );
    if dimension == ModelDimension::Three {
        let materialized = crate::spatial_materialize3::materialize_spatial_model(
            model,
            &catalog,
            &canonical_game,
            &mut programs,
            &controls,
            &catalog.input_labels,
            &render,
            &visuals,
        )?;
        let mut legend = AsciiLegend::new(canonical_game.object_count(), empty_char);
        for (object, ch) in &catalog.render_chars {
            legend.set(*object, *ch);
        }
        for (objects, ch) in render_overlays {
            legend.add_overlay(objects, ch);
        }
        let mark_labels = catalog
            .mark_names
            .iter()
            .map(|(name, def)| (def.id, name.clone()))
            .collect::<HashMap<_, _>>();
        let game = LoadedGridGame {
            title,
            subtitle,
            author,
            homepage,
            game: canonical_game,
            inputs: materialized.inputs,
            warnings,
            default_wait_ms,
            input_buffer,
            animation,
            rule_animations: programs.rule_animations,
            rule_effects: programs.rule_effects,
            rule_debug_info: programs.rule_debug_info,
            level_start_program: programs
                .level_start
                .map(puzzle_core::GridExecutableProgram::new),
            level_clear_program: programs
                .level_clear
                .map(puzzle_core::GridExecutableProgram::new),
            last_level_clear_program: programs
                .last_level_clear
                .map(puzzle_core::GridExecutableProgram::new),
            program_catalog: materialized.program_catalog,
            levels: materialized.levels,
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
        };
        return Ok(LoweredModel::Puzzle3d {
            game,
            presentation: materialized.presentation,
        });
    }
    crate::spatial_materialize2::validate_visuals(&visuals)?;
    let game = crate::spatial_materialize2::game(&canonical_game)?;
    let mut legend = AsciiLegend::new(game.object_count(), empty_char);
    for (object, ch) in &catalog.render_chars {
        legend.set(*object, *ch);
    }
    for (objects, ch) in render_overlays {
        legend.add_overlay(objects, ch);
    }
    let mut program_catalog = puzzle_core::GridProgramCatalog::default();
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
            )
            .value?;
            let program = match &programs.level_programs[index] {
                LoweredLevelProgram::Main => puzzle_core::GridProgramSequence::main(),
                LoweredLevelProgram::WithSurrounding { before, after } => {
                    let before = (!before.is_empty())
                        .then(|| crate::spatial_materialize2::executable(before))
                        .transpose()?
                        .map(|program| program_catalog.intern(program));
                    let after = (!after.is_empty())
                        .then(|| crate::spatial_materialize2::executable(after))
                        .transpose()?
                        .map(|program| program_catalog.intern(program));
                    puzzle_core::GridProgramSequence::with_surrounding(before, after)
                }
            };
            let level_start_program = programs.level_starts[index]
                .as_deref()
                .map(crate::spatial_materialize2::executable)
                .transpose()?
                .map(|program| program_catalog.intern(program));
            let level_clear_program = programs.level_clears[index]
                .as_deref()
                .map(crate::spatial_materialize2::executable)
                .transpose()?
                .map(|program| program_catalog.intern(program));
            Ok(Level {
                name: prepared.name,
                pack: prepared.pack,
                puzzle: prepared.puzzle,
                initial_state: parsed_level.state,
                regions: parsed_level.regions,
                program,
                level_start_program,
                level_clear_program,
            })
        })
        .collect::<Result<Vec<_>, DiagnosticReport>>()?;

    warnings.extend(collect_mark_warnings(&game, &catalog.mark_names));
    let mark_labels = catalog
        .mark_names
        .iter()
        .map(|(name, def)| (def.id, name.clone()))
        .collect::<HashMap<_, _>>();

    let queries = crate::spatial_materialize2::queries(queries)?;
    let conditions = crate::spatial_materialize2::goals(conditions)?;
    let goal = goal
        .as_ref()
        .map(crate::spatial_materialize2::goal)
        .transpose()?;
    let lose = lose
        .as_ref()
        .map(crate::spatial_materialize2::goal)
        .transpose()?;
    let solver_strategy = crate::spatial_materialize2::solver(&solver_strategy)?;

    Ok(LoweredModel::Puzzle2d(LoadedGame {
        title,
        subtitle,
        author,
        homepage,
        game,
        inputs: crate::spatial_orientation::materialize_inputs(
            ModelDimension::Two,
            &controls,
            &catalog.input_labels,
        )?,
        warnings,
        default_wait_ms,
        input_buffer,
        animation: animation.clone(),
        rule_animations: programs.rule_animations,
        rule_effects: programs.rule_effects,
        rule_debug_info: programs.rule_debug_info,
        level_start_program: programs
            .level_start
            .as_deref()
            .map(crate::spatial_materialize2::executable)
            .transpose()?,
        level_clear_program: programs
            .level_clear
            .as_deref()
            .map(crate::spatial_materialize2::executable)
            .transpose()?,
        last_level_clear_program: programs
            .last_level_clear
            .as_deref()
            .map(crate::spatial_materialize2::executable)
            .transpose()?,
        program_catalog,
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
    }))
}

fn collect_dynamic_selector_warnings(
    definitions: &[RuleDefinitionAst],
    main_statements: Option<&[StatementAst]>,
    level_start_statements: Option<&[StatementAst]>,
    level_clear_statements: Option<&[StatementAst]>,
    last_level_clear_statements: Option<&[StatementAst]>,
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
    ]
    .into_iter()
    .flatten()
    {
        collect_dynamic_selector_statement_warnings(statements, constant_variables, &mut warnings);
    }
    for body in level_bodies {
        collect_dynamic_selector_statement_warnings(
            &body.rules_before_statements,
            constant_variables,
            &mut warnings,
        );
        collect_dynamic_selector_statement_warnings(
            &body.rules_after_statements,
            constant_variables,
            &mut warnings,
        );
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
        visuals.entries.iter().map(|visual| visual.name.as_str()),
        "visual",
        "later definition overwrites earlier visual in generated visuals",
    );
    warnings
}

#[derive(Clone, Copy)]
struct VisualGrid {
    width: u32,
    height: u32,
}

fn collect_visual_grid_warnings(visuals: &VisualsDef) -> Vec<String> {
    let grids = visuals
        .entries
        .iter()
        .filter_map(|visual| visual_grid(visual).map(|grid| (visual.name.as_str(), grid)))
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
                "visual `{name}` uses a {}x{} cell grid that does not divide the largest visual grid {largest}; visual grids should divide the largest grid because the renderer uses the largest visual grid as the canvas unit",
                grid.width, grid.height
            ),
        );
    }
    warnings
}

fn visual_grid(visual: &VisualDef) -> Option<VisualGrid> {
    if let Some(pixels) = visual.pixels_per_cell {
        return Some(VisualGrid {
            width: pixels.width,
            height: pixels.height,
        });
    }
    match &visual.kind {
        VisualKind::Solid(_) => Some(VisualGrid {
            width: 1,
            height: 1,
        }),
        VisualKind::Image { .. } => None,
        VisualKind::Ascii { .. } => {
            let pattern = visual.frames.first()?.planes.first()?;
            Some(VisualGrid {
                width: pattern
                    .iter()
                    .map(|row| row.chars().count() as u32)
                    .max()
                    .unwrap_or(1),
                height: pattern.len().max(1) as u32,
            })
        }
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
