fn join_visuals_js(base: &str, generated: &str) -> String {
    match (base.trim().is_empty(), generated.is_empty()) {
        (true, true) => String::new(),
        (true, false) => generated.to_string(),
        (false, true) => base.to_string(),
        (false, false) => format!("{base}\n{generated}"),
    }
}

fn push_editor_preview_data(out: &mut String, state: &ServerState) {
    out.push('{');
    push_json_pair(out, "title", &state.loaded.title);
    out.push(',');
    out.push_str("\"subtitle\":");
    if let Some(subtitle) = &state.loaded.subtitle {
        push_json_string(out, subtitle);
    } else {
        out.push_str("null");
    }
    out.push(',');
    out.push_str("\"author\":");
    if let Some(author) = &state.loaded.author {
        push_json_string(out, author);
    } else {
        out.push_str("null");
    }
    out.push(',');
    out.push_str("\"homepage\":");
    if let Some(homepage) = &state.loaded.homepage {
        push_json_string(out, homepage);
    } else {
        out.push_str("null");
    }
    out.push(',');
    push_json_pair(out, "source", &state.source);
    out.push(',');
    push_json_pair(out, "puzzlePath", &state.puzzle_path);
    out.push(',');
    push_json_pair(
        out,
        "saveKey",
        &progress_save_key(&state.loaded, &state.puzzle_path),
    );
    out.push(',');
    push_json_number(
        out,
        "progressSaveVersion",
        u64::from(puzzle_play::PROGRESS_SAVE_VERSION),
    );
    out.push(',');
    push_json_bool(
        out,
        "acceptsModelInput",
        state.session.accepts_model_input(&state.loaded),
    );
    out.push(',');
    push_export_engine(out, &state.loaded);
    out.push(',');
    push_puzzle_screen(out, &state.loaded);
    out.push(',');
    push_export_levels(out, &state.loaded);
    out.push(',');
    push_inputs(out, &state.loaded);
    out.push(',');
    push_export_variables(out, &state.loaded.variables);
    out.push(',');
    push_scenes(out, "scenes", &state.loaded);
    out.push(',');
    push_scenes(out, "screens", &state.loaded);
    out.push(',');
    push_export_sounds(out, &state.loaded.sounds);
    out.push(',');
    push_export_theme(out, &state.loaded.theme);
    out.push(',');
    push_export_assets(out, &state.loaded);
    out.push(',');
    push_json_number(out, "defaultWaitMs", state.loaded.default_wait_ms);
    out.push(',');
    push_export_input_buffer(out, &state.loaded);
    out.push(',');
    push_export_animation(out, &state.loaded);
    out.push(',');
    push_export_goal(out, "goal", state.loaded.goal.as_ref());
    out.push(',');
    push_export_goal(out, "lose", state.loaded.lose.as_ref());
    out.push(',');
    push_export_conditions(out, &state.loaded);
    out.push('}');
}

fn push_export_boot_data(
    out: &mut String,
    state: &ServerState,
    include_source: bool,
    editor_preview: bool,
) {
    out.push('{');
    push_json_pair(out, "title", &state.loaded.title);
    out.push(',');
    out.push_str("\"subtitle\":");
    if let Some(subtitle) = &state.loaded.subtitle {
        push_json_string(out, subtitle);
    } else {
        out.push_str("null");
    }
    out.push(',');
    out.push_str("\"author\":");
    if let Some(author) = &state.loaded.author {
        push_json_string(out, author);
    } else {
        out.push_str("null");
    }
    out.push(',');
    out.push_str("\"homepage\":");
    if let Some(homepage) = &state.loaded.homepage {
        push_json_string(out, homepage);
    } else {
        out.push_str("null");
    }
    if include_source {
        out.push(',');
        push_json_pair(out, "source", &state.source);
        out.push(',');
        push_json_pair(out, "puzzlePath", &state.puzzle_path);
    }
    out.push(',');
    push_json_bool(out, "editorPreview", editor_preview);
    out.push(',');
    push_json_pair(
        out,
        "saveKey",
        &progress_save_key(&state.loaded, &state.puzzle_path),
    );
    out.push(',');
    push_json_number(
        out,
        "progressSaveVersion",
        u64::from(puzzle_play::PROGRESS_SAVE_VERSION),
    );
    out.push(',');
    push_json_bool(
        out,
        "acceptsModelInput",
        state.session.accepts_model_input(&state.loaded),
    );
    out.push(',');
    push_inputs(out, &state.loaded);
    out.push(',');
    push_export_sounds(out, &state.loaded.sounds);
    out.push(',');
    push_export_theme(out, &state.loaded.theme);
    out.push(',');
    push_json_number(out, "defaultWaitMs", state.loaded.default_wait_ms);
    out.push(',');
    push_export_input_buffer(out, &state.loaded);
    out.push(',');
    push_export_animation(out, &state.loaded);
    out.push('}');
}

fn progress_save_key(loaded: &LoadedGame, puzzle_path: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    progress_hash_str(&mut hash, puzzle_path);
    progress_hash_str(&mut hash, &loaded.title);
    for level in &loaded.levels {
        progress_hash_str(&mut hash, &level.name);
        hash = progress_hash_mix(hash, u64::from(level.initial_state.width));
        hash = progress_hash_mix(hash, u64::from(level.initial_state.height));
        hash = progress_hash_mix(hash, level.initial_state.hash());
    }
    format!("{}:{hash:016x}", loaded.title)
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

fn push_export_assets(out: &mut String, loaded: &LoadedGame) {
    out.push_str("\"assets\":[");
    for (index, asset) in loaded.assets.entries.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_pair(
            out,
            "kind",
            match asset.kind {
                AssetKind::Css => "css",
                AssetKind::Script => "script",
                AssetKind::File => "file",
            },
        );
        out.push(',');
        push_json_pair(out, "path", &asset.path);
        out.push('}');
    }
    out.push(']');
}

fn push_export_theme(out: &mut String, theme: &ThemeDef) {
    out.push_str("\"theme\":{");
    out.push_str("\"name\":");
    if let Some(name) = &theme.name {
        push_json_string(out, name);
    } else {
        out.push_str("null");
    }
    out.push(',');
    out.push_str("\"variables\":{");
    for (index, variable) in theme.variables.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_json_string(out, &variable.name);
        out.push(':');
        push_json_string(out, &variable.value);
    }
    out.push_str("}}");
}

fn push_export_sounds(out: &mut String, sounds: &SoundsDef) {
    out.push_str("\"sounds\":");
    let sounds_json = serde_json::to_string(&runtime_sounds_def(sounds))
        .expect("runtime sounds contract should serialize");
    out.push_str(&sounds_json);
}

fn push_export_animation(out: &mut String, loaded: &LoadedGame) {
    out.push_str("\"animation\":{");
    out.push_str("\"tween\":{");
    push_json_bool(out, "enabled", loaded.animation.tween.enabled);
    out.push(',');
    push_json_number(out, "intervalMs", loaded.animation.tween.interval_ms);
    out.push('}');
    out.push('}');
}

fn push_export_input_buffer(out: &mut String, loaded: &LoadedGame) {
    out.push_str("\"inputBuffer\":{");
    push_json_bool(
        out,
        "queueDuringWait",
        loaded.input_buffer.queue_during_wait,
    );
    out.push(',');
    push_json_bool(
        out,
        "fastForwardWait",
        loaded.input_buffer.fast_forward_wait,
    );
    out.push(',');
    push_json_number(out, "minWaitMs", loaded.input_buffer.min_wait_ms);
    out.push('}');
}

fn push_presentation_events(out: &mut String, events: &[PresentationEvent]) {
    out.push_str("\"presentationEvents\":");
    let events_json = serde_json::to_string(&presentation_events_contract::<2>(events))
        .expect("runtime presentation event contract should serialize");
    out.push_str(&events_json);
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
    push_rule_effects(out, loaded);
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

fn push_rule_effects(out: &mut String, loaded: &LoadedGame) {
    out.push_str("\"ruleEffects\":{");
    let mut entries = loaded.rule_effects.iter().collect::<Vec<_>>();
    entries.sort_by_key(|(rule, _)| rule.0);
    for (index, (rule, effects)) in entries.into_iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_json_string(out, &rule.0.to_string());
        out.push(':');
        out.push('[');
        for (effect_index, effect) in effects.iter().enumerate() {
            if effect_index > 0 {
                out.push(',');
            }
            push_ordered_rule_effect(out, effect);
        }
        out.push(']');
    }
    out.push('}');
}

fn push_ordered_rule_effect(out: &mut String, effect: &RuleEffect) {
    let writes_complete_object = matches!(effect, RuleEffect::Scene { .. });
    if !writes_complete_object {
        out.push('{');
    }
    match effect {
        RuleEffect::Win => push_json_pair(out, "kind", "win"),
        RuleEffect::Restart => push_json_pair(out, "kind", "restart"),
        RuleEffect::NextLevel => push_json_pair(out, "kind", "next_level"),
        RuleEffect::Again => push_json_pair(out, "kind", "again"),
        RuleEffect::Checkpoint => push_json_pair(out, "kind", "checkpoint"),
        RuleEffect::ClearCheckpoint => push_json_pair(out, "kind", "clear_checkpoint"),
        RuleEffect::PlaySfx { name } => {
            push_json_pair(out, "kind", "play_sfx");
            out.push(',');
            push_json_pair(out, "name", name);
        }
        RuleEffect::PlayMusic { name } => {
            push_json_pair(out, "kind", "play_music");
            out.push(',');
            push_json_pair(out, "name", name);
        }
        RuleEffect::PauseMusic { name } => {
            push_json_pair(out, "kind", "pause_music");
            if let Some(name) = name {
                out.push(',');
                push_json_pair(out, "name", name);
            }
        }
        RuleEffect::ResumeMusic { name } => {
            push_json_pair(out, "kind", "resume_music");
            if let Some(name) = name {
                out.push(',');
                push_json_pair(out, "name", name);
            }
        }
        RuleEffect::StopMusic { name } => {
            push_json_pair(out, "kind", "stop_music");
            if let Some(name) = name {
                out.push(',');
                push_json_pair(out, "name", name);
            }
        }
        RuleEffect::Wait { milliseconds } => {
            push_json_pair(out, "kind", "wait");
            out.push(',');
            push_json_number(out, "milliseconds", *milliseconds);
        }
        RuleEffect::WaitAnimation => push_json_pair(out, "kind", "wait_animation"),
        RuleEffect::EmitAnimation {
            name,
            component,
            offset,
        } => {
            push_json_pair(out, "kind", "emit_animation");
            out.push(',');
            push_json_pair(out, "name", name);
            out.push(',');
            push_json_number(out, "component", *component as u64);
            out.push_str(",\"offset\":{");
            push_json_number(out, "x", offset.x as u64);
            out.push(',');
            push_json_number(out, "y", offset.y as u64);
            out.push('}');
        }
        RuleEffect::Message { text, literal } => {
            push_json_pair(out, "kind", "message");
            out.push(',');
            push_json_pair(out, "text", text);
            out.push(',');
            push_json_bool(out, "literal", *literal);
        }
        RuleEffect::Scene { effect } => {
            puzzle_scene::write_scene_effect_json(out, effect);
        }
    }
    if !writes_complete_object {
        out.push('}');
    }
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
    document: &puzzle_lang::LoadedDocument,
) -> Result<String, serde_json::Error> {
    serde_json::to_string(&puzzle_runtime_contract::StandaloneRuntimeExport::new(
        document,
    ))
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

fn push_state2_cells(out: &mut String, state: &State, before: Option<&State>) {
    out.push('[');
    let mut first = true;
    for y in 0..state.height {
        for x in 0..state.width {
            let cell = usize::from(y) * usize::from(state.width) + usize::from(x);
            if before.is_some_and(|before| state2_cell_slots_equal(before, state, cell)) {
                continue;
            }
            let mut objects = Vec::new();
            for layer in 0..state.layer_count {
                let slot = (cell * usize::from(state.layer_count)) + usize::from(layer);
                let object = state.slots()[slot];
                if !object.is_empty() {
                    objects.push(object.0);
                }
            }
            if before.is_none() && objects.is_empty() {
                continue;
            }
            if !first {
                out.push(',');
            }
            first = false;
            out.push('{');
            push_json_number(out, "x", x as u64);
            out.push(',');
            push_json_number(out, "y", y as u64);
            out.push_str(",\"objects\":[");
            for (index, object) in objects.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                out.push_str(&object.to_string());
            }
            out.push_str("]}");
        }
    }
    out.push(']');
}

fn state2_cell_slots_equal(before: &State, after: &State, cell: usize) -> bool {
    if before.width != after.width
        || before.height != after.height
        || before.layer_count != after.layer_count
    {
        return false;
    }
    let layer_count = usize::from(after.layer_count);
    let start = cell * layer_count;
    before.slots()[start..start + layer_count] == after.slots()[start..start + layer_count]
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
