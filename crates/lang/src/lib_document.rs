pub fn export_loaded_document_visual_fixture_json(
    document: &LoadedDocument,
) -> Result<String, DiagnosticReport> {
    let Some(LoadedDocumentModel::Puzzle3d {
        game, presentation, ..
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
    export_visual_fixture_json_with_scenes(
        game,
        presentation,
        document_fields.as_deref(),
        &level_bundle_names,
    )
    .map_err(|error| {
        DiagnosticReport::error(format!("failed to export puzzle3 fixture: {error:?}"))
    })
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
    resolve_inferred_scene_puzzle_slots(&mut scenes, [("puzzle", &model_name)], None)?;
    let LoweredModel::Puzzle2d(mut game) = parse_model_from_document_parts(parts)? else {
        unreachable!("2D model dimension was validated before lowering");
    };
    resolve_default_wait_in_scenes(&mut scenes, game.default_wait_ms);
    game.scenes = add_implicit_model_scenes(scenes, [("puzzle", &model_name)]);
    resolve_scene_actions(&mut game.scenes, &game.input_labels, None)?;
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
    validate_puzzle_source(source)?;
    let parts = parse_document_source_parts_from_surface_source(source)?;
    parse_loaded_document_parts(parts, None)
}

pub fn parse_workspace_game(
    entry_path: &str,
    documents: &[WorkspaceSourceDocument],
) -> Result<LoadedDocument, DiagnosticReport> {
    let workspace = crate::WorkspaceAnalysis::new(documents)?;
    workspace.compile_game(entry_path)
}

impl crate::WorkspaceAnalysis {
    pub fn compile_game(&self, entry_path: &str) -> Result<LoadedDocument, DiagnosticReport> {
        let plans = self.module_plan(entry_path)?;
        let namespaces = plans
            .iter()
            .map(|plan| (plan.path.to_string(), plan.namespace.clone()))
            .collect::<HashMap<_, _>>();
        let mut parsed = Vec::with_capacity(plans.len());
        for plan in &plans {
            let parts = plan
                .analysis
                .strict_document_parts()
                .map_err(|report| report.with_file(plan.path))?;
            parsed.push((plan, parts));
        }
        let exports = parsed
            .iter()
            .map(|(plan, parts)| {
                (
                    plan.path.to_string(),
                    WorkspaceModuleExports {
                        models: parts
                            .models
                            .iter()
                            .map(|model| model.name.clone())
                            .collect(),
                        scenes: parts
                            .scenes
                            .iter()
                            .map(|scene| scene.name.clone())
                            .collect(),
                    },
                )
            })
            .collect::<HashMap<_, _>>();

        let mut entry_shell = None;
        let mut models = Vec::new();
        let mut model_catalogs = Vec::new();
        let mut scenes = Vec::new();
        let mut origins = WorkspaceOrigins::default();
        for (plan, mut parts) in parsed {
            if plan.namespace.is_empty() {
                entry_shell = Some(parts.shell.clone());
            } else {
                validate_imported_document_shell(plan.path, &parts.shell)
                    .map_err(|report| report.with_file(plan.path))?;
            }
            qualify_workspace_document_parts(&mut parts, plan, &namespaces, &exports)
                .map_err(|report| report.with_file(plan.path))?;
            for model in &parts.models {
                origins
                    .models
                    .insert(model.name.clone(), plan.path.to_string());
                origins
                    .scenes
                    .insert(model.name.clone(), plan.path.to_string());
            }
            for scene in &parts.scenes {
                origins
                    .scenes
                    .insert(scene.name.clone(), plan.path.to_string());
            }
            models.extend(parts.models);
            model_catalogs.extend(parts.model_catalogs);
            scenes.extend(parts.scenes);
        }
        let shell = entry_shell.ok_or_else(|| {
            DiagnosticReport::error(format!(
                "workspace entry not found in module plan: {entry_path}"
            ))
        })?;
        parse_loaded_document_parts(
            DocumentSourceParts {
                shell,
                models,
                model_catalogs,
                scenes,
                recognition: crate::surface::ParserRecognition::default(),
            },
            Some(&origins),
        )
    }
}

#[derive(Default)]
struct WorkspaceOrigins {
    models: HashMap<String, String>,
    scenes: HashMap<String, String>,
}

struct WorkspaceModuleExports {
    models: HashSet<String>,
    scenes: HashSet<String>,
}

#[derive(Clone, Copy)]
enum WorkspaceExportKind {
    Model,
    Scene,
}

fn validate_imported_document_shell(
    path: &str,
    shell: &DocumentShell,
) -> Result<(), DiagnosticReport> {
    let default = DocumentShell::default();
    let has_settings = shell.default_wait_ms != default.default_wait_ms
        || shell.input_buffer != default.input_buffer
        || shell.animation != default.animation
        || !shell.variables.is_empty()
        || !shell.sounds.sfx.is_empty()
        || !shell.sounds.music.is_empty()
        || shell.theme.name != default.theme.name
        || !shell.theme.variables.is_empty()
        || !shell.assets.entries.is_empty();
    if has_settings {
        return Err(DiagnosticReport::error(format!(
            "imported document `{path}` declares game-wide settings; document settings belong to the workspace entry"
        )));
    }
    Ok(())
}

fn qualify_workspace_document_parts(
    parts: &mut DocumentSourceParts,
    plan: &crate::workspace::WorkspaceModulePlan<'_>,
    namespaces: &HashMap<String, String>,
    exports: &HashMap<String, WorkspaceModuleExports>,
) -> Result<(), DiagnosticReport> {
    for asset in &mut parts.shell.assets.entries {
        asset.path = qualify_workspace_resource_path(plan.path, &asset.path)?;
    }
    for model in &mut parts.models {
        model.name = qualify_local_declaration(&plan.namespace, &model.name);
        for level in &mut model.body.levels.levels {
            if let Some(puzzle) = &mut level.puzzle {
                *puzzle = resolve_workspace_reference(
                    puzzle,
                    WorkspaceExportKind::Model,
                    plan,
                    namespaces,
                    exports,
                )?;
            }
        }
    }
    for scene in &mut parts.scenes {
        scene.name = qualify_local_declaration(&plan.namespace, &scene.name);
        for puzzle in &mut scene.state.puzzles {
            puzzle.model = resolve_workspace_reference(
                &puzzle.model,
                WorkspaceExportKind::Model,
                plan,
                namespaces,
                exports,
            )?;
        }
        for binding in &mut scene.key_bindings {
            qualify_scene_effect(&mut binding.effect, plan, namespaces, exports)?;
        }
        for routine in &mut scene.routines {
            qualify_scene_effect(&mut routine.effect, plan, namespaces, exports)?;
        }
        for transition in &mut scene.transitions {
            qualify_scene_effect(&mut transition.effect, plan, namespaces, exports)?;
        }
        qualify_scene_components(&mut scene.components, plan, namespaces, exports)?;
    }
    Ok(())
}

fn qualify_scene_components(
    components: &mut [SceneComponent],
    plan: &crate::workspace::WorkspaceModulePlan<'_>,
    namespaces: &HashMap<String, String>,
    exports: &HashMap<String, WorkspaceModuleExports>,
) -> Result<(), DiagnosticReport> {
    for component in components {
        match component {
            SceneComponent::Frame(frame) => {
                frame.source = resolve_workspace_reference(
                    &frame.source,
                    WorkspaceExportKind::Scene,
                    plan,
                    namespaces,
                    exports,
                )?;
            }
            SceneComponent::Button(button) | SceneComponent::Choice(button) => {
                qualify_scene_effect(&mut button.effect, plan, namespaces, exports)?;
            }
            SceneComponent::Row(container)
            | SceneComponent::Column(container)
            | SceneComponent::Box(container) => {
                qualify_scene_components(&mut container.children, plan, namespaces, exports)?;
            }
            SceneComponent::Conditional(conditional) => {
                qualify_scene_components(&mut conditional.children, plan, namespaces, exports)?;
                qualify_scene_components(
                    &mut conditional.else_children,
                    plan,
                    namespaces,
                    exports,
                )?;
            }
            SceneComponent::Viewport(_) | SceneComponent::Text(_) => {}
        }
    }
    Ok(())
}

fn qualify_scene_effect(
    effect: &mut SceneEffect,
    plan: &crate::workspace::WorkspaceModulePlan<'_>,
    namespaces: &HashMap<String, String>,
    exports: &HashMap<String, WorkspaceModuleExports>,
) -> Result<(), DiagnosticReport> {
    effect.try_map_scene_references(&mut |scene| {
        resolve_workspace_reference(scene, WorkspaceExportKind::Scene, plan, namespaces, exports)
    })
}

fn resolve_workspace_reference(
    reference: &str,
    kind: WorkspaceExportKind,
    plan: &crate::workspace::WorkspaceModulePlan<'_>,
    namespaces: &HashMap<String, String>,
    exports: &HashMap<String, WorkspaceModuleExports>,
) -> Result<String, DiagnosticReport> {
    let (target_path, name) = if let Some((alias, name)) = reference.split_once(':') {
        if name.contains(':') || !puzzle_authoring::is_identifier(name) {
            return Err(DiagnosticReport::error(format!(
                "workspace reference `{reference}` must name one directly imported declaration"
            )));
        }
        let target = plan.imports.get(alias).ok_or_else(|| {
            DiagnosticReport::error(format!(
                "workspace reference `{reference}` uses unknown import alias `{alias}` in {}",
                plan.path
            ))
        })?;
        (target.as_str(), name)
    } else {
        (plan.path, reference)
    };
    let available = exports.get(target_path).ok_or_else(|| {
        DiagnosticReport::error(format!("workspace module is unavailable: {target_path}"))
    })?;
    let exists = match kind {
        WorkspaceExportKind::Model => available.models.contains(name),
        WorkspaceExportKind::Scene => available.scenes.contains(name),
    };
    if !exists {
        let label = match kind {
            WorkspaceExportKind::Model => "puzzle model",
            WorkspaceExportKind::Scene => "scene",
        };
        return Err(DiagnosticReport::error(format!(
            "unknown {label} reference `{reference}` in {}",
            plan.path
        )));
    }
    let namespace = namespaces.get(target_path).ok_or_else(|| {
        DiagnosticReport::error(format!("workspace module is unreachable: {target_path}"))
    })?;
    Ok(qualify_local_declaration(namespace, name))
}

fn qualify_local_declaration(namespace: &str, name: &str) -> String {
    if namespace.is_empty() {
        name.to_string()
    } else {
        format!("{namespace}:{name}")
    }
}

fn parse_loaded_document_parts(
    parts: DocumentSourceParts,
    origins: Option<&WorkspaceOrigins>,
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
        origins.map(|origins| &origins.scenes),
    )?;
    scenes =
        add_implicit_model_scenes(scenes, model_kinds.iter().map(|(kind, name)| (*kind, name)));
    resolve_default_wait_in_scenes(&mut scenes, shell.default_wait_ms);

    let mut lowered = Vec::with_capacity(models.len());
    let mut input_names = Vec::<String>::new();
    for (model, catalog) in models.iter().zip(&model_catalogs) {
        let mut product = lower_model_with_shell(model, catalog, &shell).map_err(|report| {
            match origins.and_then(|origins| origins.models.get(&model.name)) {
                Some(file) => report.with_file(file),
                None => report,
            }
        })?;
        if let Some(file) = origins.and_then(|origins| origins.models.get(&model.name)) {
            qualify_loaded_model_resource_paths(&mut product, file)
                .map_err(|report| report.with_file(file))?;
        }
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
    resolve_scene_actions(
        &mut scenes,
        &input_labels,
        origins.map(|origins| &origins.scenes),
    )?;

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

fn qualify_loaded_model_resource_paths(
    model: &mut LoweredModel,
    owner_path: &str,
) -> Result<(), DiagnosticReport> {
    let visuals = match model {
        LoweredModel::Puzzle2d(game) => &mut game.visuals,
        LoweredModel::Puzzle3d { game, .. } => &mut game.visuals,
    };
    for visual in &mut visuals.entries {
        if let VisualKind::Image { asset } = &mut visual.kind {
            let path = qualify_workspace_resource_path(owner_path, &asset.path)?;
            *asset = puzzle_assets::VisualImageAssetManifestEntry::from_path(path)
                .map_err(|error| DiagnosticReport::error(error.to_string()))?;
        }
    }
    Ok(())
}

fn qualify_workspace_resource_path(
    owner_path: &str,
    resource_path: &str,
) -> Result<String, DiagnosticReport> {
    if resource_path.starts_with("data:")
        || resource_path.starts_with("http:")
        || resource_path.starts_with("https:")
        || resource_path.starts_with('#')
    {
        return Ok(resource_path.to_string());
    }
    crate::WorkspacePath::parse(owner_path)
        .and_then(|owner| owner.resolve_relative(resource_path))
        .map(|path| path.as_str().to_string())
        .map_err(DiagnosticReport::error)
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
    scene_files: Option<&HashMap<String, String>>,
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
        let file = scene_files
            .and_then(|files| files.get(&scene.name))
            .cloned();
        let result = (|| {
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
            Ok::<(), DiagnosticReport>(())
        })();
        result.map_err(|report| match file {
            Some(file) => report.with_file(file),
            None => report,
        })?;
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
        push_puzzle3_fixture_surface(&mut out, "playing");
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
    push_puzzle3_fixture_surface(&mut out, current_scene);
    out.push_str("  \"scenes\": [\n");
    for (index, scene) in document.scenes.iter().enumerate() {
        if index > 0 {
            out.push_str(",\n");
        }
        push_puzzle3_scene_json(&mut out, document, scene, &mut level_bundle_names);
    }
    out.push_str("\n  ],");
    (Some(out), level_bundle_names)
}

fn push_puzzle3_fixture_surface(out: &mut String, root: &str) {
    let root = json_string(root);
    out.push_str("  \"surface\": { \"root\": ");
    out.push_str(&root);
    out.push_str(", \"focus\": ");
    out.push_str(&root);
    out.push_str(", \"components\": [{ \"id\": ");
    out.push_str(&root);
    out.push_str(", \"definition\": ");
    out.push_str(&root);
    out.push_str(
        ", \"placement\": \"root\", \"visibility\": \"visible\", \"modal\": false }] },\n",
    );
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
    default_wait_ms: u64,
    input_buffer: InputBufferDef,
    animation: AnimationDef,
    variables: Vec<SceneVarDef>,
    sounds: SoundsDef,
    theme: ThemeDef,
    assets: AssetsDef,
}

#[derive(Clone, Debug)]
pub(crate) struct DocumentSourceParts {
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
    for entry in document_entries
        .iter()
        .filter(|entry| entry.directive == puzzle_authoring::PuzzleDirectiveSurface::Import)
    {
        recognition.merge(entry.semantics.fixed.clone());
    }
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
            puzzle_authoring::PuzzleDirectiveSurface::DocumentSetting => match tokens.as_slice() {
                ["default_wait_time", ..] => {
                    shell.default_wait_ms = parse_default_wait_time_directive(&entry.header)?;
                }
                ["theme", ..] => {
                    let lines = document_entry_lines(entry);
                    parse_theme_statement(&lines, 0, &mut shell.theme)?;
                }
                _ => {
                    return Err(parse_error(&entry.header, "unknown document setting"));
                }
            },
            puzzle_authoring::PuzzleDirectiveSurface::Variable => {
                shell
                    .variables
                    .push(parse_top_level_var_directive(&tokens, &entry.header)?);
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

fn document_entry_lines(entry: &model_syntax::PuzzleEntrySyntax) -> Vec<source::LogicalLine> {
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
            let (lines, next_i) =
                collect_authoring_entry(&logical_lines, i, AuthoringEntryOwner::SceneDefinition)?;
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
            let (entry, layout, next_i) = extract_default_model_scene_source(&logical_lines, i)?;
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
            model
                .body
                .levels
                .levels
                .iter()
                .map(|level| LevelProjectionEntry {
                    name: level.name.clone(),
                    pack: level.pack.clone(),
                    puzzle: level.puzzle.clone().unwrap_or_else(|| model.name.clone()),
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

pub fn validate_puzzle_source_for_path(
    source: &str,
    path: impl AsRef<Path>,
) -> Result<(), DiagnosticReport> {
    let path = path.as_ref();
    if !is_puzzle_source_path(path) {
        return Err(DiagnosticReport::error(format!(
            "puzzle source must use .puzzle extension: {}",
            path.display()
        )));
    }
    validate_puzzle_source(source)
}

fn validate_puzzle_source(source: &str) -> Result<(), DiagnosticReport> {
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
    fn puzzle_declaration_dimension_selects_lowering() {
        let document = parse_game_for_path(
            r#"
puzzle space {
dimension = 3
layers {
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
    fn puzzle3_extension_is_rejected() {
        let error = parse_game_for_path(
            r#"
puzzle space {
layers {
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

        assert!(error.contains("must use .puzzle extension"), "{error}");
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn resolve_game_entry(path: impl AsRef<Path>) -> Result<PathBuf, DiagnosticReport> {
    let path = path.as_ref();
    if path.is_dir() {
        return Err(DiagnosticReport::error(format!(
            "game entry must be an explicit .puzzle file, not a directory: {}",
            path.display()
        )));
    }

    if path.is_file() {
        if !is_puzzle_source_path(path) {
            return Err(DiagnosticReport::error(format!(
                "game entry must be a .puzzle file: {}",
                path.display()
            )));
        }
        return Ok(path.to_path_buf());
    }

    Err(DiagnosticReport::error(format!(
        "game entry not found: {}",
        path.display()
    )))
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
    let model_source = source::LogicalLine::new(&model.source_line, model.source_line_number);
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
        layer_count.ok_or_else(|| DiagnosticReport::error("missing layers".to_string()))?;
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
                source: level.source,
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
    for name in visuals
        .order
        .priorities
        .iter()
        .flat_map(|priority| &priority.animations)
    {
        if !visual_names.contains(name) {
            return Err(DiagnosticReport::error(format!(
                "unknown animation visual in layers: !{name}"
            )));
        }
    }
    let animation_visual_names = visuals
        .order
        .priorities
        .iter()
        .flat_map(|priority| priority.animations.iter().cloned())
        .collect::<HashSet<_>>();
    let direction_variant_pairs = catalog
        .object_schemas
        .values()
        .flat_map(|schema| {
            schema.variants.iter().flat_map(move |from| {
                schema.variants.iter().filter_map(move |to| {
                    if from.object == to.object || from.values.len() != to.values.len() {
                        return None;
                    }
                    let changed_axes = from
                        .values
                        .iter()
                        .zip(&to.values)
                        .enumerate()
                        .filter_map(|(index, (left, right))| (left != right).then_some(index))
                        .collect::<Vec<_>>();
                    (changed_axes.len() == 1
                        && schema.axis_types.get(changed_axes[0]).copied().flatten()
                            == Some(ValueType::Direction))
                    .then_some((from.object, to.object))
                })
            })
        })
        .collect::<HashSet<_>>();
    let mut programs = lower_programs(
        &model_source,
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
        &animation_visual_names,
        &animation,
        &direction_variant_pairs,
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
                &prepared.source,
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
