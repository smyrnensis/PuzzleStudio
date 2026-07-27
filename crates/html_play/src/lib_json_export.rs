fn document_metadata_value(document: &puzzle_lang::LoadedDocument, name: &str) -> Option<String> {
    document
        .variables
        .iter()
        .find(|variable| !variable.mutable && variable.name == name)
        .map(|variable| scene_value_to_string(&variable.default))
}

fn document_title(document: &puzzle_lang::LoadedDocument) -> String {
    document_metadata_value(document, "title")
        .expect("compiled document must contain the canonical `title` constant")
}

fn preview_state_document(state: &EditorPreviewState) -> &puzzle_lang::LoadedDocument {
    &state.standalone_export.runtime_loaded_document
}

fn push_editor_preview_data(out: &mut String, state: &EditorPreviewState) {
    let document = preview_state_document(state);
    out.push('{');
    push_json_pair(out, "title", &document_title(document));
    out.push(',');
    out.push_str("\"subtitle\":");
    if let Some(subtitle) = document_metadata_value(document, "subtitle") {
        push_json_string(out, &subtitle);
    } else {
        out.push_str("null");
    }
    out.push(',');
    out.push_str("\"author\":");
    if let Some(author) = document_metadata_value(document, "author") {
        push_json_string(out, &author);
    } else {
        out.push_str("null");
    }
    out.push(',');
    out.push_str("\"homepage\":");
    if let Some(homepage) = document_metadata_value(document, "homepage") {
        push_json_string(out, &homepage);
    } else {
        out.push_str("null");
    }
    out.push(',');
    push_json_pair(out, "source", &state.source);
    out.push(',');
    push_json_pair(out, "puzzlePath", &state.puzzle_path);
    out.push(',');
    push_json_bool(
        out,
        "acceptsModelInput",
        state.runtime.accepts_model_input(),
    );
    out.push(',');
    push_editor_preview_models(out, document);
    out.push(',');
    push_runtime_inputs(out, state);
    out.push(',');
    push_runtime_theme(out, state);
    out.push(',');
    push_export_variables(out, &document.variables);
    out.push(',');
    push_json_number(out, "defaultWaitMs", document.default_wait_ms);
    out.push(',');
    push_export_input_buffer_values(
        out,
        document.input_buffer.queue_during_wait,
        document.input_buffer.fast_forward_wait,
        document.input_buffer.min_wait_ms,
    );
    out.push(',');
    push_export_animation_values(
        out,
        document.animation.tween.enabled,
        document.animation.tween.interval_ms,
    );
    out.push('}');
}

fn editor_preview_build_json(html: &str, state: &EditorPreviewState) -> String {
    let mut metadata_json = String::new();
    push_editor_preview_data(&mut metadata_json, state);
    let mut metadata: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str::<serde_json::Value>(&metadata_json)
            .expect("typed editor preview metadata must serialize")
            .as_object()
            .expect("typed editor preview metadata must be an object")
            .clone();
    let models = metadata
        .remove("models")
        .expect("typed editor preview metadata must contain model artifacts");
    serde_json::to_string(&serde_json::json!({
        "html": html,
        "documentMetadata": metadata,
        "models": models,
    }))
    .expect("typed editor preview build must serialize")
}

fn push_export_boot_data(out: &mut String, editor_preview: bool) {
    out.push('{');
    push_json_pair(out, "engineVersion", env!("CARGO_PKG_VERSION"));
    out.push(',');
    push_json_bool(out, "editorPreview", editor_preview);
    out.push('}');
}

fn push_runtime_inputs(out: &mut String, state: &EditorPreviewState) {
    push_runtime_snapshot_field(out, state, "inputs");
}

fn push_runtime_theme(out: &mut String, state: &EditorPreviewState) {
    push_runtime_snapshot_field(out, state, "theme");
}

fn push_runtime_snapshot_field(out: &mut String, state: &EditorPreviewState, field: &str) {
    let snapshot: serde_json::Value = serde_json::from_str(&state.runtime.snapshot_json())
        .expect("runtime snapshot JSON should parse");
    let value = snapshot
        .get(field)
        .unwrap_or_else(|| panic!("runtime snapshot should contain {field}"));
    push_json_string(out, field);
    out.push(':');
    out.push_str(
        &serde_json::to_string(value)
            .unwrap_or_else(|_| panic!("runtime {field} should serialize")),
    );
}

fn push_editor_preview_models(out: &mut String, document: &puzzle_lang::LoadedDocument) {
    out.push_str("\"models\":{");
    for (index, model) in document.models.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        let (name, kind) = match model {
            LoadedDocumentModel::Puzzle2d { name, .. } => (name, "puzzle2d"),
            LoadedDocumentModel::Puzzle3d { name, .. } => (name, "puzzle3d"),
        };
        push_json_string(out, name);
        out.push_str(":{");
        push_json_pair(out, "kind", kind);
        match model {
            LoadedDocumentModel::Puzzle2d { game, .. } => {
                out.push(',');
                push_export_engine(out, game);
                out.push(',');
                push_puzzle_screen(out, game);
                out.push(',');
                push_export_levels(out, game);
                out.push(',');
                push_export_variables(out, &game.variables);
                out.push(',');
                push_json_number(out, "defaultWaitMs", game.default_wait_ms);
                out.push(',');
                push_export_input_buffer(out, game);
                out.push(',');
                push_export_animation(out, game);
                out.push(',');
                push_export_goal(out, "goal", game.goal.as_ref());
                out.push(',');
                push_export_goal(out, "lose", game.lose.as_ref());
                out.push(',');
                push_export_conditions(out, game);
            }
            LoadedDocumentModel::Puzzle3d {
                game, presentation, ..
            } => {
                out.push_str(",\"fixture\":");
                let fixture = puzzle_lang::export_visual_fixture_json(game, presentation)
                    .expect("validated 3D editor model must export its visual fixture");
                out.push_str(&fixture);
            }
        }
        out.push('}');
    }
    out.push('}');
}

fn standalone_progress_storage(
    document: &puzzle_lang::LoadedDocument,
) -> StandaloneProgressStorage {
    let mut hash = 0xcbf29ce484222325_u64;
    let title = document_title(document);
    progress_hash_str(&mut hash, &title);
    hash = progress_hash_mix(hash, document.models.len() as u64);
    for model in &document.models {
        match model {
            LoadedDocumentModel::Puzzle2d { name, game } => {
                progress_hash_str(&mut hash, name);
                progress_hash_levels(&mut hash, &game.levels);
            }
            LoadedDocumentModel::Puzzle3d { name, game, .. } => {
                progress_hash_str(&mut hash, name);
                progress_hash_levels(&mut hash, &game.levels);
            }
        }
    }
    let save_version = puzzle_play::PROGRESS_SAVE_VERSION;
    StandaloneProgressStorage {
        key: format!(
            "PuzzleStudio.progress.v{save_version}:{}:{hash:016x}",
            title
        ),
        save_version,
    }
}

fn progress_hash_levels<const D: usize, Size: GridSize<D>>(
    hash: &mut u64,
    levels: &[puzzle_lang::LoadedGridLevel<D, Size>],
) {
    *hash = progress_hash_mix(*hash, levels.len() as u64);
    for level in levels {
        progress_hash_str(hash, &level.puzzle);
        progress_hash_str(hash, &level.name);
        for axis in level.initial_state.size.axes() {
            *hash = progress_hash_mix(*hash, u64::from(axis));
        }
        *hash = progress_hash_mix(*hash, level.initial_state.hash());
    }
}

fn progress_hash_str(hash: &mut u64, value: &str) {
    *hash = progress_hash_mix(*hash, value.len() as u64);
    for byte in value.bytes() {
        *hash = progress_hash_mix(*hash, u64::from(byte));
    }
}

fn progress_hash_mix(hash: u64, value: u64) -> u64 {
    (hash ^ value).wrapping_mul(0x100000001b3)
}

fn push_export_animation(out: &mut String, loaded: &LoadedGame) {
    push_export_animation_values(
        out,
        loaded.animation.tween.enabled,
        loaded.animation.tween.interval_ms,
    );
}

fn push_export_animation_values(out: &mut String, enabled: bool, interval_ms: u64) {
    out.push_str("\"animation\":{");
    out.push_str("\"tween\":{");
    push_json_bool(out, "enabled", enabled);
    out.push(',');
    push_json_number(out, "intervalMs", interval_ms);
    out.push('}');
    out.push('}');
}

fn push_export_input_buffer(out: &mut String, loaded: &LoadedGame) {
    push_export_input_buffer_values(
        out,
        loaded.input_buffer.queue_during_wait,
        loaded.input_buffer.fast_forward_wait,
        loaded.input_buffer.min_wait_ms,
    );
}

fn push_export_input_buffer_values(
    out: &mut String,
    queue_during_wait: bool,
    fast_forward_wait: bool,
    min_wait_ms: u64,
) {
    out.push_str("\"inputBuffer\":{");
    push_json_bool(out, "queueDuringWait", queue_during_wait);
    out.push(',');
    push_json_bool(out, "fastForwardWait", fast_forward_wait);
    out.push(',');
    push_json_number(out, "minWaitMs", min_wait_ms);
    out.push('}');
}

fn push_export_variables(out: &mut String, variables: &[puzzle_lang::SceneVarDef]) {
    out.push_str("\"variables\":[");
    for (index, variable) in variables.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_scene_var_def(out, variable);
    }
    out.push(']');
}

fn push_export_engine(out: &mut String, loaded: &LoadedGame) {
    out.push_str("\"engine\":{");
    push_json_number(out, "layerCount", loaded.game.layer_count as u64);
    out.push(',');
    push_export_objects(out, loaded);
    out.push(',');
    push_export_model_variables(out, loaded);
    out.push(',');
    push_export_queries(out, &loaded.game);
    out.push(',');
    out.push_str("\"persistentVars\":[");
    for (index, var) in loaded.persistent_vars.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&var.0.to_string());
    }
    out.push(']');
    out.push(',');
    push_rule_animations(out, loaded);
    out.push(',');
    out.push_str("\"program\":[]");
    out.push(',');
    out.push_str("\"levelStartProgram\":");
    push_empty_rule_program(out);
    out.push(',');
    out.push_str("\"runRulesOnLevelStart\":");
    out.push_str(if loaded.run_rules_on_level_start {
        "true"
    } else {
        "false"
    });
    out.push(',');
    out.push_str("\"levelClearProgram\":");
    push_empty_rule_program(out);
    out.push('}');
}

fn push_rule_animations(out: &mut String, loaded: &LoadedGame) {
    out.push_str("\"ruleAnimations\":{");
    let mut entries = loaded.rule_animations.iter().collect::<Vec<_>>();
    entries.sort_by_key(|(rule, _)| rule.0);
    for (index, (rule, animations)) in entries.into_iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_json_string(out, &rule.0.to_string());
        out.push(':');
        out.push('[');
        for (animation_index, animation) in animations.iter().enumerate() {
            if animation_index > 0 {
                out.push(',');
            }
            push_rule_animation(out, animation);
        }
        out.push(']');
    }
    out.push('}');
}

fn push_rule_animation(out: &mut String, animation: &RuleAnimation) {
    out.push('{');
    push_json_pair(out, "kind", "animate");
    out.push(',');
    push_json_pair(
        out,
        "trigger",
        match animation.trigger {
            RuleAnimationTrigger::Move => "move",
            RuleAnimationTrigger::CantMove => "cantmove",
        },
    );
    out.push(',');
    push_json_pair(out, "name", &animation.name);
    out.push(',');
    out.push_str("\"objects\":[");
    for (index, object) in animation.objects.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&object.0.to_string());
    }
    out.push(']');
    out.push('}');
}

fn runtime_export_json(
    export: &StandaloneRuntimeExport<puzzle_lang::LoadedDocument>,
) -> Result<String, serde_json::Error> {
    puzzle_player_bootstrap::encode_standalone_player_export(export)
}

fn push_export_model_variables(out: &mut String, loaded: &LoadedGame) {
    out.push_str("\"variables\":[");
    let mut entries = loaded.variable_labels.iter().collect::<Vec<_>>();
    entries.sort_by_key(|(variable, _)| variable.0);
    for (index, (variable, name)) in entries.into_iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_number(out, "id", variable.0 as u64);
        out.push(',');
        push_json_pair(out, "name", name);
        out.push('}');
    }
    out.push(']');
}

fn push_empty_rule_program(out: &mut String) {
    out.push_str("[]");
}

fn push_export_objects(out: &mut String, loaded: &LoadedGame) {
    out.push_str("\"objects\":[");
    for id in 1..=loaded.game.object_count() {
        if id > 1 {
            out.push(',');
        }
        let object_id = ObjectId(id as u16);
        let def = loaded
            .game
            .object(object_id)
            .expect("compiled object id should exist");
        let name = loaded.object_name(object_id);
        out.push('{');
        push_json_number(out, "id", def.id.0 as u64);
        out.push(',');
        push_json_number(out, "layer", def.layer_id.0 as u64);
        out.push(',');
        push_json_pair(out, "name", name);
        out.push(',');
        push_json_pair(out, "visual", &visual_name(name));
        out.push('}');
    }
    out.push(']');
}

fn push_export_queries(out: &mut String, game: &CompiledGame) {
    out.push_str("\"queries\":[");
    for (index, condition) in game.condition_defs().iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_number(out, "id", condition.id.0 as u64);
        out.push(',');
        push_condition_value_kind(out, &condition.kind);
        out.push('}');
    }
    out.push(']');
}

fn push_export_levels(out: &mut String, loaded: &LoadedGame) {
    out.push_str("\"levels\":[");
    for (index, level) in loaded.levels.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_number(out, "index", index as u64);
        out.push(',');
        push_json_pair(out, "name", &level.name);
        out.push(',');
        push_json_pair(out, "puzzle", &level.puzzle);
        out.push(',');
        if let Some(pack) = &level.pack {
            push_json_pair(out, "pack", pack);
        } else {
            out.push_str("\"pack\":null");
        }
        out.push(',');
        push_scene_regions(out, Some(level));
        out.push(',');
        out.push_str("\"levelStartProgram\":");
        push_empty_rule_program(out);
        out.push(',');
        out.push_str("\"levelClearProgram\":");
        push_empty_rule_program(out);
        out.push(',');
        out.push_str("\"initialState\":");
        push_state_data(out, &level.initial_state);
        out.push('}');
    }
    out.push(']');
}

fn push_state_data(out: &mut String, state: &State) {
    let snapshot = RuntimeStateSnapshot2d::from_state(state);
    out.push_str(
        &serde_json::to_string(&snapshot)
            .expect("typed runtime state snapshot must serialize to JSON"),
    );
}

fn push_pattern(out: &mut String, pattern: &Pattern) {
    out.push_str("\"pattern\":{\"components\":[");
    for (component_index, component) in pattern.components.iter().enumerate() {
        if component_index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_number(out, "gapCount", component.gap_count as u64);
        out.push(',');
        out.push_str("\"cells\":[");
        for (cell_index, cell) in component.cells.iter().enumerate() {
            if cell_index > 0 {
                out.push(',');
            }
            out.push('{');
            push_offset_named(out, "offset", &cell.offset);
            out.push(',');
            push_json_bool(out, "requireNull", cell.require_null);
            out.push(',');
            push_object_ids(out, "requireObjects", &cell.require_objects);
            out.push(',');
            push_object_ids(out, "forbidObjects", &cell.forbid_objects);
            out.push(',');
            push_mark_patterns(out, "requireMark", &cell.require_mark);
            out.push(',');
            push_mark_patterns(out, "forbidMark", &cell.forbid_mark);
            out.push('}');
        }
        out.push(']');
        out.push('}');
    }
    out.push_str("]}");
}

fn push_mark_patterns(out: &mut String, key: &str, mark: &[MarkPattern]) {
    push_json_string(out, key);
    out.push_str(":[");
    for (index, mark) in mark.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_number(out, "object", mark.object.0 as u64);
        out.push(',');
        push_json_number(out, "mark", mark.mark.0 as u64);
        if let Some(value) = mark.value {
            out.push(',');
            push_json_i64(out, "value", value);
        }
        out.push(',');
        push_json_pair(
            out,
            "match",
            match mark.match_value {
                MarkValueMatch::Any => "any",
                MarkValueMatch::Exact => "exact",
            },
        );
        out.push('}');
    }
    out.push(']');
}

fn push_condition_value_kind(out: &mut String, kind: &ConditionValueKind) {
    out.push_str("\"conditionValueKind\":{");
    match kind {
        ConditionValueKind::CountObjects(objects) => {
            push_json_pair(out, "kind", "count_objects");
            out.push(',');
            push_object_ids(out, "objects", objects);
        }
        ConditionValueKind::ExistsObjects(objects) => {
            push_json_pair(out, "kind", "exists_objects");
            out.push(',');
            push_object_ids(out, "objects", objects);
        }
        ConditionValueKind::NoneObjects(objects) => {
            push_json_pair(out, "kind", "none_objects");
            out.push(',');
            push_object_ids(out, "objects", objects);
        }
        ConditionValueKind::CountMatches(patterns) => {
            push_json_pair(out, "kind", "count_matches");
            out.push(',');
            push_patterns(out, patterns);
        }
        ConditionValueKind::ExistsMatches(patterns) => {
            push_json_pair(out, "kind", "exists_matches");
            out.push(',');
            push_patterns(out, patterns);
        }
        ConditionValueKind::NoneMatches(patterns) => {
            push_json_pair(out, "kind", "none_matches");
            out.push(',');
            push_patterns(out, patterns);
        }
        ConditionValueKind::CountInputMatches(patterns) => {
            push_json_pair(out, "kind", "count_input_matches");
            out.push(',');
            push_input_patterns(out, patterns);
        }
        ConditionValueKind::ExistsInputMatches(patterns) => {
            push_json_pair(out, "kind", "exists_input_matches");
            out.push(',');
            push_input_patterns(out, patterns);
        }
        ConditionValueKind::NoneInputMatches(patterns) => {
            push_json_pair(out, "kind", "none_input_matches");
            out.push(',');
            push_input_patterns(out, patterns);
        }
    }
    out.push('}');
}

fn push_input_patterns(out: &mut String, patterns: &[(InputId, Pattern)]) {
    out.push_str("\"patterns\":[");
    for (index, (input, pattern)) in patterns.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_number(out, "input", input.0 as u64);
        out.push(',');
        push_pattern(out, pattern);
        out.push('}');
    }
    out.push(']');
}

fn push_patterns(out: &mut String, patterns: &[Pattern]) {
    out.push_str("\"patterns\":[");
    for (index, pattern) in patterns.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_pattern(out, pattern);
        out.push('}');
    }
    out.push(']');
}

fn push_export_goal(out: &mut String, key: &str, goal: Option<&GoalCondition>) {
    push_json_string(out, key);
    out.push(':');
    let Some(goal) = goal else {
        out.push_str("null");
        return;
    };
    out.push('{');
    push_json_pair(out, "description", &goal.description);
    out.push(',');
    push_goal_expr_named(out, "expr", &goal.expr);
    out.push('}');
}

fn push_export_conditions(out: &mut String, loaded: &LoadedGame) {
    out.push_str("\"conditions\":{");
    let mut entries = loaded.conditions.iter().collect::<Vec<_>>();
    entries.sort_by(|(left, _), (right, _)| left.cmp(right));
    for (index, (name, condition)) in entries.into_iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_json_string(out, name);
        out.push(':');
        out.push('{');
        push_json_pair(out, "description", &condition.description);
        out.push(',');
        push_goal_expr_named(out, "expr", &condition.expr);
        out.push('}');
    }
    out.push('}');
}

fn push_goal_expr_named(out: &mut String, key: &str, expr: &GoalExpr) {
    push_json_string(out, key);
    out.push(':');
    push_goal_expr(out, expr);
}

fn push_goal_expr(out: &mut String, expr: &GoalExpr) {
    out.push('{');
    match expr {
        GoalExpr::All(exprs) => {
            push_json_pair(out, "kind", "all");
            out.push(',');
            push_goal_exprs(out, exprs);
        }
        GoalExpr::Any(exprs) => {
            push_json_pair(out, "kind", "any");
            out.push(',');
            push_goal_exprs(out, exprs);
        }
        GoalExpr::Clause(clause) => {
            push_json_pair(out, "kind", "clause");
            out.push(',');
            out.push_str("\"value\":");
            push_goal_value(out, &clause.value);
            out.push(',');
            push_comparison_op(out, "op", clause.op);
            out.push(',');
            push_json_i64(out, "expected", clause.expected);
        }
    }
    out.push('}');
}

fn push_goal_exprs(out: &mut String, exprs: &[GoalExpr]) {
    out.push_str("\"exprs\":[");
    for (index, expr) in exprs.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_goal_expr(out, expr);
    }
    out.push(']');
}

fn push_goal_value(out: &mut String, value: &GoalValue) {
    out.push('{');
    match value {
        GoalValue::Variable(variable) => {
            push_json_pair(out, "kind", "variable");
            out.push(',');
            push_json_number(out, "variable", variable.0 as u64);
        }
        GoalValue::Condition(condition) => {
            push_json_pair(out, "kind", "condition");
            out.push(',');
            push_json_number(out, "condition", condition.0 as u64);
        }
        GoalValue::InlineConditionValue(kind) => {
            push_json_pair(out, "kind", "condition_value");
            out.push(',');
            push_condition_value_kind(out, kind);
        }
    }
    out.push('}');
}

fn push_object_ids(out: &mut String, key: &str, objects: &[ObjectId]) {
    push_json_string(out, key);
    out.push_str(":[");
    for (index, object) in objects.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&object.0.to_string());
    }
    out.push(']');
}

fn push_offset_named(out: &mut String, key: &str, offset: &Offset) {
    push_json_string(out, key);
    out.push(':');
    out.push('{');
    match offset {
        Offset::Fixed { delta } => {
            let [dx, dy] = delta.axes();
            push_json_pair(out, "kind", "fixed");
            out.push(',');
            push_json_i64(out, "dx", i64::from(dx));
            out.push(',');
            push_json_i64(out, "dy", i64::from(dy));
        }
        Offset::Variable { base, gap_terms } => {
            let [base_dx, base_dy] = base.axes();
            push_json_pair(out, "kind", "variable");
            out.push(',');
            push_json_i64(out, "baseDx", i64::from(base_dx));
            out.push(',');
            push_json_i64(out, "baseDy", i64::from(base_dy));
            out.push(',');
            out.push_str("\"gapTerms\":[");
            for (index, term) in gap_terms.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                out.push('{');
                push_json_number(out, "gapIndex", term.gap_index as u64);
                out.push(',');
                let [term_dx, term_dy] = term.delta.axes();
                push_json_i64(out, "dx", i64::from(term_dx));
                out.push(',');
                push_json_i64(out, "dy", i64::from(term_dy));
                out.push('}');
            }
            out.push(']');
        }
    }
    out.push('}');
}

fn push_comparison_op(out: &mut String, key: &str, op: ComparisonOp) {
    push_json_pair(
        out,
        key,
        match op {
            ComparisonOp::Eq => "eq",
            ComparisonOp::NotEq => "not_eq",
            ComparisonOp::Greater => "greater",
            ComparisonOp::GreaterEq => "greater_eq",
            ComparisonOp::Less => "less",
            ComparisonOp::LessEq => "less_eq",
        },
    );
}
