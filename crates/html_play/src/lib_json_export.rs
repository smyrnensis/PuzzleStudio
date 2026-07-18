fn json_u64_field(source: &str, key: &str) -> Option<u64> {
    let mut value = json_value_after_key(source, key)?.trim_start();
    let end = value
        .find(|ch: char| !ch.is_ascii_digit())
        .unwrap_or(value.len());
    value = &value[..end];
    (!value.is_empty()).then(|| value.parse().ok()).flatten()
}

fn json_u64_array_field(source: &str, key: &str) -> Option<Vec<u64>> {
    json_array_body(source, key).map(|body| {
        body.split(',')
            .filter_map(|value| {
                let trimmed = value.trim();
                (!trimmed.is_empty())
                    .then(|| trimmed.parse().ok())
                    .flatten()
            })
            .collect()
    })
}

fn json_i64_array_field(source: &str, key: &str) -> Option<Vec<i64>> {
    json_array_body(source, key).map(|body| {
        body.split(',')
            .filter_map(|value| {
                let trimmed = value.trim();
                (!trimmed.is_empty())
                    .then(|| trimmed.parse().ok())
                    .flatten()
            })
            .collect()
    })
}

fn json_array_body<'a>(source: &'a str, key: &str) -> Option<&'a str> {
    let value = json_value_after_key(source, key)?.trim_start();
    let rest = value.strip_prefix('[')?;
    let end = rest.find(']')?;
    Some(&rest[..end])
}

fn json_value_after_key<'a>(source: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("\"{key}\"");
    let index = source.find(&needle)?;
    let after_key = &source[index + needle.len()..];
    after_key.trim_start().strip_prefix(':')
}

fn join_visuals_js(base: &str, generated: &str) -> String {
    match (base.trim().is_empty(), generated.is_empty()) {
        (true, true) => String::new(),
        (true, false) => generated.to_string(),
        (false, true) => base.to_string(),
        (false, false) => format!("{base}\n{generated}"),
    }
}

#[cfg(test)]
fn push_export_data(out: &mut String, state: &ServerState) {
    push_export_data_with_source(out, state, true);
}

fn push_export_data_with_source(out: &mut String, state: &ServerState, include_source: bool) {
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
    }
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
    push_compiled_play_bundle(out, &state.loaded);
    out.push(',');
    push_runtime_loaded_document_bundle(out, &state.document);
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

fn push_runtime_export_data(out: &mut String, state: &ServerState) {
    out.push('{');
    push_runtime_loaded_document_bundle(out, &state.document);
    out.push('}');
}

fn push_editor_solver_rules(out: &mut String, loaded: &LoadedGame) {
    out.push('{');
    push_json_number(out, "version", 1);
    out.push(',');
    push_json_pair(out, "modelKind", "2d");
    out.push(',');
    push_compiled_play_bundle(out, loaded);
    out.push_str(",\"loadedGame\":");
    out.push_str(
        &serde_json::to_string(loaded).expect("loaded solver game serialization must succeed"),
    );
    out.push(',');
    push_json_bool(out, "runRulesOnLevelStart", loaded.run_rules_on_level_start);
    out.push(',');
    push_export_goal(out, "goal", loaded.goal.as_ref());
    out.push(',');
    push_export_goal(out, "lose", loaded.lose.as_ref());
    out.push_str(",\"solverStrategy\":");
    out.push_str(
        &serde_json::to_string(&loaded.solver_strategy)
            .expect("solver strategy serialization must succeed"),
    );
    out.push('}');
}

fn push_editor_grid_solver_rules(
    out: &mut String,
    model_kind: &str,
    loaded: &LoadedGridGame<3, Size3>,
) {
    out.push('{');
    push_json_number(out, "version", 1);
    out.push(',');
    push_json_pair(out, "modelKind", model_kind);
    out.push_str(",\"loadedGame\":");
    out.push_str(
        &serde_json::to_string(loaded).expect("loaded grid solver game serialization must succeed"),
    );
    out.push_str(",\"solverStrategy\":");
    out.push_str(
        &serde_json::to_string(&loaded.solver_strategy)
            .expect("grid solver strategy serialization must succeed"),
    );
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

fn push_presentation_events(out: &mut String, loaded: &LoadedGame, events: &[PresentationEvent]) {
    out.push_str("\"presentationEvents\":");
    let events_json = serde_json::to_string(&presentation_events_contract(loaded, events))
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

fn push_compiled_play_bundle(out: &mut String, loaded: &LoadedGame) {
    out.push_str("\"compiledPlay\":{");
    push_json_number(out, "version", 2);
    out.push(',');
    push_json_pair(out, "model", "grid2");
    out.push(',');
    push_compiled_input_labels(out, loaded);
    out.push_str(",\"transition\":[");
    out.push_str(&loaded.game.layer_count.to_string());
    out.push(',');
    push_compact_objects(out, loaded);
    out.push(',');
    push_compact_queries(out, &loaded.game);
    out.push(',');
    push_compact_transition_programs(out, loaded);
    out.push(',');
    push_compact_level_programs(out, loaded);
    out.push_str("]}");
}

fn push_compiled_input_labels(out: &mut String, loaded: &LoadedGame) {
    let mut labels = loaded
        .input_labels
        .iter()
        .map(|(id, label)| (*id, label.as_str()))
        .collect::<Vec<_>>();
    labels.sort_by_key(|(id, _)| *id);

    out.push_str("\"inputLabels\":{");
    for (index, (id, label)) in labels.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_json_string(out, &id.0.to_string());
        out.push(':');
        push_json_string(out, label);
    }
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
    out.push('{');
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
            push_json_effect_fields(out, effect);
        }
    }
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

fn push_runtime_loaded_document_bundle(out: &mut String, document: &puzzle_lang::LoadedDocument) {
    out.push_str("\"runtimeLoadedDocument\":{");
    push_json_number(out, "version", 1);
    out.push_str(",\"document\":");
    let document_json = runtime_loaded_document_json(document)
        .expect("runtime loaded document bundle should serialize");
    out.push_str(&document_json);
    out.push('}');
}

fn runtime_loaded_document_json(
    document: &puzzle_lang::LoadedDocument,
) -> Result<String, serde_json::Error> {
    let mut value = serde_json::to_value(document)?;
    remove_runtime_only_key(&mut value, "solver_strategy");
    serde_json::to_string(&value)
}

fn remove_runtime_only_key(value: &mut serde_json::Value, key: &str) {
    match value {
        serde_json::Value::Object(object) => {
            object.remove(key);
            for value in object.values_mut() {
                remove_runtime_only_key(value, key);
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                remove_runtime_only_key(value, key);
            }
        }
        _ => {}
    }
}

fn push_compact_objects(out: &mut String, loaded: &LoadedGame) {
    out.push('[');
    for id in 1..=loaded.game.object_count() {
        if id > 1 {
            out.push(',');
        }
        let object_id = ObjectId(id as u16);
        let def = loaded
            .game
            .object(object_id)
            .expect("compiled object id should exist");
        out.push('[');
        out.push_str(&def.id.0.to_string());
        out.push(',');
        out.push_str(&def.layer_id.0.to_string());
        out.push(']');
    }
    out.push(']');
}

fn push_compact_queries(out: &mut String, game: &CompiledGame) {
    out.push('[');
    for (index, condition) in game.condition_defs().iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('[');
        out.push_str(&condition.id.0.to_string());
        out.push(',');
        push_compact_condition_value_kind(out, &condition.kind);
        out.push(']');
    }
    out.push(']');
}

fn push_compact_transition_programs(out: &mut String, loaded: &LoadedGame) {
    out.push('[');
    push_compact_rule_program(out, loaded.game.program());
    out.push(',');
    push_compact_optional_rule_program(out, loaded.level_start_program.as_deref());
    out.push(',');
    push_compact_optional_rule_program(out, loaded.level_clear_program.as_deref());
    out.push(']');
}

fn push_compact_level_programs(out: &mut String, loaded: &LoadedGame) {
    out.push('[');
    for (index, level) in loaded.levels.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('[');
        push_compact_optional_rule_program(out, level.level_start_program.as_deref());
        out.push(',');
        push_compact_optional_rule_program(out, level.level_clear_program.as_deref());
        out.push(',');
        push_compact_rule_program(out, &level.program);
        out.push(']');
    }
    out.push(']');
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
        push_json_pair(out, "sprite", &sprite_name(name));
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
    out.push('{');
    push_json_number(out, "width", state.width as u64);
    out.push(',');
    push_json_number(out, "height", state.height as u64);
    out.push(',');
    push_json_number(out, "layerCount", state.layer_count as u64);
    out.push(',');
    out.push_str("\"slots\":[");
    for (index, object) in state.slots().iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&object.0.to_string());
    }
    out.push(']');
    out.push(',');
    out.push_str("\"mark\":[");
    for index in 0..state.slots().len() {
        if index > 0 {
            out.push(',');
        }
        out.push('[');
        for (mark_index, mark) in state.slot_mark_at(index).enumerate() {
            if mark_index > 0 {
                out.push(',');
            }
            out.push('{');
            push_json_number(out, "mark", mark.mark.0 as u64);
            if let Some(value) = mark.value {
                out.push(',');
                push_json_i64(out, "value", value);
            }
            out.push('}');
        }
        out.push(']');
    }
    out.push(']');
    out.push(',');
    out.push_str("\"variables\":[");
    for (index, value) in state.visible_variables().iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&value.to_string());
    }
    out.push_str("],");
    out.push_str("\"levelFiredRules\":[");
    for (index, rule) in state.level_fired_rules().iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&rule.0.to_string());
    }
    out.push(']');
    out.push('}');
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

fn push_state3_data(out: &mut String, state: &GridState<3, Size3>) {
    out.push('{');
    push_json_pair(out, "kind", "puzzle3d");
    out.push(',');
    push_json_number(out, "width", state.size.width as u64);
    out.push(',');
    push_json_number(out, "depth", state.size.depth as u64);
    out.push(',');
    push_json_number(out, "height", state.size.height as u64);
    out.push(',');
    push_json_number(out, "layerCount", state.layer_count as u64);
    out.push(',');
    out.push_str("\"slots\":[");
    for (index, object) in state.slots().iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&object.0.to_string());
    }
    out.push_str("],");
    out.push_str("\"levelFiredRules\":[");
    for (index, rule) in state.level_fired_rules().iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&rule.0.to_string());
    }
    out.push(']');
    out.push('}');
}

fn push_compact_optional_rule_program(out: &mut String, program: Option<&[RuleStep]>) {
    match program {
        Some(program) => push_compact_rule_program(out, program),
        None => out.push_str("[]"),
    }
}

fn push_compact_rule_program(out: &mut String, program: &[RuleStep]) {
    out.push('[');
    for (index, step) in program.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_compact_rule_step(out, step);
    }
    out.push(']');
}

fn push_compact_rule_step(out: &mut String, step: &RuleStep) {
    out.push('[');
    match step {
        RuleStep::Rule(rule) => {
            out.push('0');
            out.push(',');
            push_compact_rule(out, rule);
        }
        RuleStep::ConditionalBlock { condition, steps } => {
            out.push('1');
            out.push(',');
            push_compact_rule_condition(out, condition);
            out.push(',');
            push_compact_rule_program(out, steps);
        }
        RuleStep::ConditionalBranch {
            condition,
            then_steps,
            else_steps,
        } => {
            out.push('5');
            out.push(',');
            push_compact_rule_condition(out, condition);
            out.push(',');
            push_compact_rule_program(out, then_steps);
            out.push(',');
            push_compact_rule_program(out, else_steps);
        }
        RuleStep::Block {
            application,
            stop_condition,
            steps,
        } => {
            out.push('2');
            out.push(',');
            push_compact_rule_application(out, *application);
            out.push(',');
            if let Some(condition) = stop_condition {
                push_compact_rule_condition(out, condition);
            } else {
                out.push_str("null");
            }
            out.push(',');
            push_compact_rule_program(out, steps);
        }
        RuleStep::LocalFrame { frame, steps } => {
            out.push('3');
            out.push(',');
            push_compact_local_frame(out, frame);
            out.push(',');
            push_compact_rule_program(out, steps);
        }
        RuleStep::AfterTriggered { steps, then_steps } => {
            out.push('4');
            out.push(',');
            push_compact_rule_program(out, steps);
            out.push(',');
            push_compact_rule_program(out, then_steps);
        }
    }
    out.push(']');
}

fn push_compact_rule(out: &mut String, rule: &Rule) {
    out.push('[');
    out.push_str(&rule.id.0.to_string());
    out.push(',');
    push_compact_rule_application(out, rule.application);
    out.push(',');
    out.push('[');
    for (index, guard) in rule.guards.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_compact_guard(out, guard);
    }
    out.push(']');
    out.push(',');
    push_compact_pattern(out, &rule.pattern);
    out.push(',');
    out.push('[');
    for (index, write) in rule.writes.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_compact_write(out, write);
    }
    out.push(']');
    out.push(',');
    out.push('[');
    for (index, effect) in rule.effects.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_compact_effect(out, effect);
    }
    out.push_str("]]");
}

fn push_compact_rule_application(out: &mut String, application: RuleApplication) {
    out.push_str(match application {
        RuleApplication::Once => "0",
        RuleApplication::OnceAll => "1",
        RuleApplication::OncePerLevel => "2",
        RuleApplication::UntilStable => "3",
        RuleApplication::Random => "4",
        RuleApplication::RepeatStep => "5",
    });
}

fn push_compact_rule_condition(out: &mut String, condition: &RuleCondition) {
    out.push('[');
    match condition {
        RuleCondition::AnyMatches(patterns) => {
            out.push('0');
            out.push(',');
            push_compact_patterns(out, patterns);
        }
        RuleCondition::NoMatches(patterns) => {
            out.push('1');
            out.push(',');
            push_compact_patterns(out, patterns);
        }
        RuleCondition::AnyInputMatches(patterns) => {
            out.push('2');
            out.push(',');
            push_compact_input_patterns(out, patterns);
        }
        RuleCondition::NoInputMatches(patterns) => {
            out.push('3');
            out.push(',');
            push_compact_input_patterns(out, patterns);
        }
        RuleCondition::GuardBranches(branches) => {
            out.push('4');
            out.push(',');
            out.push('[');
            for (branch_index, branch) in branches.iter().enumerate() {
                if branch_index > 0 {
                    out.push(',');
                }
                out.push('[');
                for (guard_index, guard) in branch.iter().enumerate() {
                    if guard_index > 0 {
                        out.push(',');
                    }
                    push_compact_guard(out, guard);
                }
                out.push(']');
            }
            out.push(']');
        }
    }
    out.push(']');
}

fn push_compact_guard(out: &mut String, guard: &Guard) {
    out.push('[');
    match guard {
        Guard::InputIs(input) => {
            out.push('0');
            out.push(',');
            out.push_str(&input.0.to_string());
        }
        Guard::VariableEquals { variable, value } => {
            out.push('1');
            out.push(',');
            out.push_str(&variable.0.to_string());
            out.push(',');
            push_compact_comparison(out, ComparisonOp::Eq);
            out.push(',');
            out.push_str(&value.to_string());
        }
        Guard::VariableCompare {
            variable,
            op,
            value,
        } => {
            out.push('1');
            out.push(',');
            out.push_str(&variable.0.to_string());
            out.push(',');
            push_compact_comparison(out, *op);
            out.push(',');
            out.push_str(&value.to_string());
        }
        Guard::ConditionEquals { condition, value } => {
            out.push('2');
            out.push(',');
            out.push_str(&condition.0.to_string());
            out.push(',');
            push_compact_comparison(out, ComparisonOp::Eq);
            out.push(',');
            out.push_str(&value.to_string());
        }
        Guard::ConditionNonZero(condition) => {
            out.push('3');
            out.push(',');
            out.push_str(&condition.0.to_string());
        }
        Guard::ConditionCompare {
            condition,
            op,
            value,
        } => {
            out.push('2');
            out.push(',');
            out.push_str(&condition.0.to_string());
            out.push(',');
            push_compact_comparison(out, *op);
            out.push(',');
            out.push_str(&value.to_string());
        }
        Guard::InlineConditionValue { kind, value } => {
            out.push('4');
            out.push(',');
            push_compact_condition_value_kind(out, kind);
            out.push(',');
            push_compact_comparison(out, ComparisonOp::Eq);
            out.push(',');
            out.push_str(&value.to_string());
        }
        Guard::InlineConditionNonZero(kind) => {
            out.push('5');
            out.push(',');
            push_compact_condition_value_kind(out, kind);
        }
        Guard::InlineConditionCompare { kind, op, value } => {
            out.push('4');
            out.push(',');
            push_compact_condition_value_kind(out, kind);
            out.push(',');
            push_compact_comparison(out, *op);
            out.push(',');
            out.push_str(&value.to_string());
        }
    }
    out.push(']');
}

fn push_compact_condition_value_kind(out: &mut String, kind: &ConditionValueKind) {
    out.push('[');
    match kind {
        ConditionValueKind::CountObjects(objects) => {
            out.push('0');
            out.push(',');
            push_compact_object_ids(out, objects);
        }
        ConditionValueKind::ExistsObjects(objects) => {
            out.push('1');
            out.push(',');
            push_compact_object_ids(out, objects);
        }
        ConditionValueKind::NoneObjects(objects) => {
            out.push('2');
            out.push(',');
            push_compact_object_ids(out, objects);
        }
        ConditionValueKind::CountMatches(patterns) => {
            out.push('3');
            out.push(',');
            push_compact_patterns(out, patterns);
        }
        ConditionValueKind::ExistsMatches(patterns) => {
            out.push('4');
            out.push(',');
            push_compact_patterns(out, patterns);
        }
        ConditionValueKind::NoneMatches(patterns) => {
            out.push('5');
            out.push(',');
            push_compact_patterns(out, patterns);
        }
        ConditionValueKind::CountInputMatches(patterns) => {
            out.push('6');
            out.push(',');
            push_compact_input_patterns(out, patterns);
        }
        ConditionValueKind::ExistsInputMatches(patterns) => {
            out.push('7');
            out.push(',');
            push_compact_input_patterns(out, patterns);
        }
        ConditionValueKind::NoneInputMatches(patterns) => {
            out.push('8');
            out.push(',');
            push_compact_input_patterns(out, patterns);
        }
    }
    out.push(']');
}

fn push_compact_patterns(out: &mut String, patterns: &[Pattern]) {
    out.push('[');
    for (index, pattern) in patterns.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_compact_pattern(out, pattern);
    }
    out.push(']');
}

fn push_compact_input_patterns(out: &mut String, patterns: &[(InputId, Pattern)]) {
    out.push('[');
    for (index, (input, pattern)) in patterns.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('[');
        out.push_str(&input.0.to_string());
        out.push(',');
        push_compact_pattern(out, pattern);
        out.push(']');
    }
    out.push(']');
}

fn push_compact_pattern(out: &mut String, pattern: &Pattern) {
    out.push('[');
    for (component_index, component) in pattern.components.iter().enumerate() {
        if component_index > 0 {
            out.push(',');
        }
        out.push('[');
        out.push_str(&component.gap_count.to_string());
        out.push(',');
        out.push('[');
        for (cell_index, cell) in component.cells.iter().enumerate() {
            if cell_index > 0 {
                out.push(',');
            }
            push_compact_match_cell(out, cell);
        }
        out.push_str("]]");
    }
    out.push(']');
}

fn push_compact_match_cell(out: &mut String, cell: &puzzle_core::MatchCell) {
    out.push('[');
    push_compact_offset(out, &cell.offset);
    out.push(',');
    push_compact_object_ids(out, &cell.require_objects);
    out.push(',');
    push_compact_object_sets(out, &cell.require_object_sets);
    out.push(',');
    push_compact_object_ids(out, &cell.forbid_objects);
    out.push(',');
    push_compact_mark_patterns(out, &cell.require_mark);
    out.push(',');
    push_compact_object_set_mark_patterns(out, &cell.require_object_set_mark);
    out.push(',');
    push_compact_mark_patterns(out, &cell.forbid_mark);
    out.push(',');
    push_compact_object_set_mark_patterns(out, &cell.forbid_object_set_mark);
    out.push(',');
    out.push_str(if cell.require_null { "1" } else { "0" });
    out.push(']');
}

fn push_compact_object_sets(out: &mut String, object_sets: &[puzzle_core::ObjectSetMatcher]) {
    out.push('[');
    for (index, object_set) in object_sets.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('[');
        out.push_str(&object_set.binding.to_string());
        out.push(',');
        out.push_str(&object_set.layer.0.to_string());
        out.push(',');
        push_compact_object_ids(out, &object_set.objects);
        out.push(']');
    }
    out.push(']');
}

fn push_compact_object_set_mark_patterns(
    out: &mut String,
    mark: &[puzzle_core::ObjectSetMarkPattern],
) {
    out.push('[');
    for (index, pattern) in mark.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('[');
        out.push_str(&pattern.binding.to_string());
        out.push(',');
        out.push_str(&pattern.mark.0.to_string());
        out.push(',');
        push_compact_optional_i64(out, pattern.value);
        out.push(',');
        push_compact_mark_match(out, pattern.match_value);
        out.push(']');
    }
    out.push(']');
}

fn push_compact_mark_patterns(out: &mut String, mark: &[MarkPattern]) {
    out.push('[');
    for (index, pattern) in mark.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('[');
        out.push_str(&pattern.object.0.to_string());
        out.push(',');
        out.push_str(&pattern.mark.0.to_string());
        out.push(',');
        push_compact_optional_i64(out, pattern.value);
        out.push(',');
        push_compact_mark_match(out, pattern.match_value);
        out.push(']');
    }
    out.push(']');
}

fn push_compact_offset(out: &mut String, offset: &Offset) {
    out.push('[');
    match offset {
        Offset::Fixed { delta } => {
            let [dx, dy] = delta.axes();
            out.push('0');
            out.push(',');
            out.push_str(&dx.to_string());
            out.push(',');
            out.push_str(&dy.to_string());
        }
        Offset::Variable { base, gap_terms } => {
            let [base_dx, base_dy] = base.axes();
            out.push('1');
            out.push(',');
            out.push_str(&base_dx.to_string());
            out.push(',');
            out.push_str(&base_dy.to_string());
            out.push(',');
            out.push('[');
            for (index, term) in gap_terms.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                out.push('[');
                out.push_str(&term.gap_index.to_string());
                out.push(',');
                let [term_dx, term_dy] = term.delta.axes();
                out.push_str(&term_dx.to_string());
                out.push(',');
                out.push_str(&term_dy.to_string());
                out.push(']');
            }
            out.push(']');
        }
    }
    out.push(']');
}

fn push_compact_write(out: &mut String, write: &WriteOp) {
    out.push('[');
    match write {
        WriteOp::Add {
            component,
            offset,
            object,
        } => {
            out.push('0');
            out.push(',');
            out.push_str(&component.to_string());
            out.push(',');
            push_compact_offset(out, offset);
            out.push(',');
            out.push_str(&object.0.to_string());
        }
        WriteOp::AddObjectSet {
            component,
            offset,
            binding,
        } => {
            out.push('6');
            out.push(',');
            out.push_str(&component.to_string());
            out.push(',');
            push_compact_offset(out, offset);
            out.push(',');
            out.push_str(&binding.to_string());
        }
        WriteOp::Remove {
            component,
            offset,
            object,
        } => {
            out.push('1');
            out.push(',');
            out.push_str(&component.to_string());
            out.push(',');
            push_compact_offset(out, offset);
            out.push(',');
            out.push_str(&object.0.to_string());
        }
        WriteOp::RemoveObjectSet {
            component,
            offset,
            binding,
        } => {
            out.push('7');
            out.push(',');
            out.push_str(&component.to_string());
            out.push(',');
            push_compact_offset(out, offset);
            out.push(',');
            out.push_str(&binding.to_string());
        }
        WriteOp::Move {
            component,
            from_offset,
            to_offset,
            object,
        } => {
            out.push('2');
            out.push(',');
            out.push_str(&component.to_string());
            out.push(',');
            push_compact_offset(out, from_offset);
            out.push(',');
            push_compact_offset(out, to_offset);
            out.push(',');
            out.push_str(&object.0.to_string());
        }
        WriteOp::MoveObjectSet {
            component,
            from_offset,
            to_offset,
            binding,
        } => {
            out.push('8');
            out.push(',');
            out.push_str(&component.to_string());
            out.push(',');
            push_compact_offset(out, from_offset);
            out.push(',');
            push_compact_offset(out, to_offset);
            out.push(',');
            out.push_str(&binding.to_string());
        }
        WriteOp::Replace {
            component,
            offset,
            remove,
            add,
        } => {
            out.push('3');
            out.push(',');
            out.push_str(&component.to_string());
            out.push(',');
            push_compact_offset(out, offset);
            out.push(',');
            out.push_str(&remove.0.to_string());
            out.push(',');
            out.push_str(&add.0.to_string());
        }
        WriteOp::SetMark {
            component,
            offset,
            object,
            mark,
            value,
        } => {
            out.push('4');
            out.push(',');
            out.push_str(&component.to_string());
            out.push(',');
            push_compact_offset(out, offset);
            out.push(',');
            out.push_str(&object.0.to_string());
            out.push(',');
            out.push_str(&mark.0.to_string());
            out.push(',');
            push_compact_optional_i64(out, *value);
        }
        WriteOp::SetObjectSetMark {
            component,
            offset,
            binding,
            mark,
            value,
        } => {
            out.push('9');
            out.push(',');
            out.push_str(&component.to_string());
            out.push(',');
            push_compact_offset(out, offset);
            out.push(',');
            out.push_str(&binding.to_string());
            out.push(',');
            out.push_str(&mark.0.to_string());
            out.push(',');
            push_compact_optional_i64(out, *value);
        }
        WriteOp::RemoveMark {
            component,
            offset,
            object,
            mark,
            value,
            match_value,
        } => {
            out.push('5');
            out.push(',');
            out.push_str(&component.to_string());
            out.push(',');
            push_compact_offset(out, offset);
            out.push(',');
            out.push_str(&object.0.to_string());
            out.push(',');
            out.push_str(&mark.0.to_string());
            out.push(',');
            push_compact_optional_i64(out, *value);
            out.push(',');
            push_compact_mark_match(out, *match_value);
        }
        WriteOp::RemoveObjectSetMark {
            component,
            offset,
            binding,
            mark,
            value,
            match_value,
        } => {
            out.push_str("10");
            out.push(',');
            out.push_str(&component.to_string());
            out.push(',');
            push_compact_offset(out, offset);
            out.push(',');
            out.push_str(&binding.to_string());
            out.push(',');
            out.push_str(&mark.0.to_string());
            out.push(',');
            push_compact_optional_i64(out, *value);
            out.push(',');
            push_compact_mark_match(out, *match_value);
        }
    }
    out.push(']');
}

fn push_compact_effect(out: &mut String, effect: &Effect) {
    out.push('[');
    match effect {
        Effect::ObserveMatch => out.push('8'),
        Effect::Cancel => out.push('0'),
        Effect::Win => out.push('1'),
        Effect::Restart => out.push('2'),
        Effect::NextLevel => out.push('3'),
        Effect::Again => out.push('4'),
        Effect::Checkpoint => out.push('5'),
        Effect::ClearCheckpoint => out.push('6'),
        Effect::UpdateVariable {
            variable,
            op,
            value,
        } => {
            out.push('7');
            out.push(',');
            out.push_str(&variable.0.to_string());
            out.push(',');
            push_compact_variable_update(out, *op);
            out.push(',');
            out.push_str(&value.to_string());
        }
    }
    out.push(']');
}

fn push_compact_local_frame(out: &mut String, frame: &puzzle_core::LocalFrame<ObjectId>) {
    out.push('[');
    push_compact_local_frame_extent(out, frame.x);
    out.push(',');
    push_compact_local_frame_extent(out, frame.y);
    out.push(',');
    push_compact_local_frame_extent(out, frame.z);
    out.push(',');
    push_compact_object_ids(out, &frame.focus_objects);
    out.push(']');
}

fn push_compact_local_frame_extent(out: &mut String, extent: puzzle_core::LocalFrameExtent) {
    match extent {
        puzzle_core::LocalFrameExtent::Radius(radius) => out.push_str(&radius.to_string()),
        puzzle_core::LocalFrameExtent::Full => out.push_str("null"),
    }
}

fn push_compact_object_ids(out: &mut String, objects: &[ObjectId]) {
    out.push('[');
    for (index, object) in objects.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&object.0.to_string());
    }
    out.push(']');
}

fn push_compact_mark_match(out: &mut String, value: MarkValueMatch) {
    out.push_str(match value {
        MarkValueMatch::Any => "0",
        MarkValueMatch::Exact => "1",
    });
}

fn push_compact_comparison(out: &mut String, op: ComparisonOp) {
    out.push_str(match op {
        ComparisonOp::Eq => "0",
        ComparisonOp::NotEq => "1",
        ComparisonOp::Greater => "2",
        ComparisonOp::GreaterEq => "3",
        ComparisonOp::Less => "4",
        ComparisonOp::LessEq => "5",
    });
}

fn push_compact_variable_update(out: &mut String, op: VariableUpdateOp) {
    out.push_str(match op {
        VariableUpdateOp::Set => "0",
        VariableUpdateOp::Add => "1",
        VariableUpdateOp::Subtract => "2",
        VariableUpdateOp::Multiply => "3",
        VariableUpdateOp::Divide => "4",
        VariableUpdateOp::Remainder => "5",
    });
}

fn push_compact_optional_i64(out: &mut String, value: Option<i64>) {
    if let Some(value) = value {
        out.push_str(&value.to_string());
    } else {
        out.push_str("null");
    }
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

#[cfg(feature = "solver")]
fn push_solution_response(
    out: &mut String,
    loaded: &LoadedGame,
    response: &PuzzleSolutionResponse,
) {
    out.push('{');
    match response {
        SolutionResponse::Solved {
            depth,
            moves,
            steps,
            observations,
        } => {
            push_json_pair(out, "result", "solved");
            out.push(',');
            push_json_number(out, "depth", *depth as u64);
            out.push(',');
            push_solution_moves(out, loaded, moves);
            out.push(',');
            push_solution_steps(out, loaded, steps);
            out.push(',');
            push_search_observations(out, observations);
        }
        SolutionResponse::Exhausted {
            stats,
            observations,
        } => {
            push_json_pair(out, "result", "exhausted");
            out.push(',');
            push_search_stats(out, stats);
            out.push(',');
            push_search_observations(out, observations);
        }
        SolutionResponse::BudgetExceeded {
            stats,
            observations,
        } => {
            push_json_pair(out, "result", "budget_exceeded");
            out.push(',');
            push_search_stats(out, stats);
            out.push(',');
            push_search_observations(out, observations);
        }
        SolutionResponse::Failed {
            depth,
            error,
            observations,
        } => {
            push_json_pair(out, "result", "failed");
            out.push(',');
            push_json_number(out, "depth", *depth as u64);
            out.push(',');
            push_json_pair(out, "error", error);
            out.push(',');
            push_search_observations(out, observations);
        }
    }
    out.push('}');
}

#[cfg(feature = "solver")]
fn push_reachability_response(
    out: &mut String,
    loaded: &LoadedGame,
    response: &PuzzleSolutionResponse,
) {
    out.push('{');
    push_json_pair(out, "task", "reachability");
    out.push(',');
    match response {
        SolutionResponse::Solved {
            depth,
            moves,
            steps,
            observations,
        } => {
            push_json_pair(out, "result", "reachable");
            out.push(',');
            push_json_bool(out, "reachable", true);
            out.push_str(",\"cost\":{");
            push_json_number(out, "steps", *depth as u64);
            out.push('}');
            out.push(',');
            push_solution_moves(out, loaded, moves);
            out.push(',');
            out.push_str("\"path\":");
            push_solution_path(out, loaded, steps);
            out.push(',');
            push_search_observations(out, observations);
        }
        SolutionResponse::Exhausted {
            stats,
            observations,
        } => {
            push_json_pair(out, "result", "unreachable");
            out.push(',');
            push_json_bool(out, "reachable", false);
            out.push(',');
            push_search_stats(out, stats);
            out.push(',');
            push_search_observations(out, observations);
        }
        SolutionResponse::BudgetExceeded {
            stats,
            observations,
        } => {
            push_json_pair(out, "result", "budget_exceeded");
            out.push(',');
            push_json_bool(out, "reachable", false);
            out.push(',');
            push_search_stats(out, stats);
            out.push(',');
            push_search_observations(out, observations);
        }
        SolutionResponse::Failed {
            depth,
            error,
            observations,
        } => {
            push_json_pair(out, "result", "failed");
            out.push(',');
            push_json_bool(out, "reachable", false);
            out.push(',');
            push_json_number(out, "depth", *depth as u64);
            out.push(',');
            push_json_pair(out, "error", error);
            out.push(',');
            push_search_observations(out, observations);
        }
    }
    out.push('}');
}

#[cfg(feature = "solver")]
fn push_collect_response(out: &mut String, loaded: &LoadedGame, response: &PuzzleCollectResponse) {
    out.push('{');
    push_json_pair(out, "task", "collect");
    out.push(',');
    match response {
        CollectResponse::Completed {
            stats,
            matches,
            observations,
        } => {
            push_json_pair(out, "result", "completed");
            out.push(',');
            push_json_number(out, "count", matches.len() as u64);
            out.push(',');
            push_search_stats(out, stats);
            out.push(',');
            push_collect_matches(out, loaded, matches);
            out.push(',');
            push_search_observations(out, observations);
        }
        CollectResponse::LimitReached {
            stats,
            matches,
            observations,
        } => {
            push_json_pair(out, "result", "limit_reached");
            out.push(',');
            push_json_number(out, "count", matches.len() as u64);
            out.push(',');
            push_search_stats(out, stats);
            out.push(',');
            push_collect_matches(out, loaded, matches);
            out.push(',');
            push_search_observations(out, observations);
        }
        CollectResponse::BudgetExceeded {
            stats,
            matches,
            observations,
        } => {
            push_json_pair(out, "result", "budget_exceeded");
            out.push(',');
            push_json_number(out, "count", matches.len() as u64);
            out.push(',');
            push_search_stats(out, stats);
            out.push(',');
            push_collect_matches(out, loaded, matches);
            out.push(',');
            push_search_observations(out, observations);
        }
        CollectResponse::Failed {
            depth,
            error,
            matches,
            observations,
        } => {
            push_json_pair(out, "result", "failed");
            out.push(',');
            push_json_number(out, "depth", *depth as u64);
            out.push(',');
            push_json_pair(out, "error", error);
            out.push(',');
            push_json_number(out, "count", matches.len() as u64);
            out.push(',');
            push_collect_matches(out, loaded, matches);
            out.push(',');
            push_search_observations(out, observations);
        }
    }
    out.push('}');
}

#[cfg(feature = "solver")]
fn push_collect_matches(
    out: &mut String,
    loaded: &LoadedGame,
    matches: &[CollectMatch<State, InputId>],
) {
    out.push_str("\"matches\":[");
    for (index, collect_match) in matches.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_number(out, "index", index as u64);
        out.push(',');
        push_json_number(out, "depth", collect_match.depth as u64);
        out.push(',');
        match collect_match.score {
            Some(score) => push_json_i64(out, "score", score),
            None => out.push_str("\"score\":null"),
        }
        out.push(',');
        push_solution_moves(out, loaded, &collect_match.moves);
        out.push_str(",\"state\":");
        push_state_data(out, &collect_match.state);
        out.push('}');
    }
    out.push(']');
}

#[cfg(feature = "solver")]
fn push_solution_path(out: &mut String, loaded: &LoadedGame, steps: &[PuzzleSolutionStep]) {
    out.push('[');
    for (index, step) in steps.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_number(out, "index", step.index as u64);
        out.push(',');
        if let Some(input) = step.input {
            out.push_str("\"move\":");
            push_input_move(out, loaded, input);
        } else {
            out.push_str("\"move\":null");
        }
        out.push(',');
        push_scene(out, loaded, &step.state, None, None);
        out.push('}');
    }
    out.push(']');
}

#[cfg(feature = "solver")]
fn push_compiled_solution_response(
    out: &mut String,
    response: &PuzzleSolutionResponse,
    input_labels: &[(InputId, String)],
) -> Result<(), AppError> {
    out.push('{');
    match response {
        SolutionResponse::Solved {
            depth,
            moves,
            steps,
            observations,
        } => {
            push_json_pair(out, "result", "solved");
            out.push(',');
            push_json_number(out, "depth", *depth as u64);
            out.push(',');
            push_compiled_solution_moves(out, moves, input_labels)?;
            out.push(',');
            push_compiled_solution_steps(out, steps, input_labels)?;
            out.push(',');
            push_search_observations(out, observations);
        }
        SolutionResponse::Exhausted {
            stats,
            observations,
        } => {
            push_json_pair(out, "result", "exhausted");
            out.push(',');
            push_search_stats(out, stats);
            out.push(',');
            push_search_observations(out, observations);
        }
        SolutionResponse::BudgetExceeded {
            stats,
            observations,
        } => {
            push_json_pair(out, "result", "budget_exceeded");
            out.push(',');
            push_search_stats(out, stats);
            out.push(',');
            push_search_observations(out, observations);
        }
        SolutionResponse::Failed {
            depth,
            error,
            observations,
        } => {
            push_json_pair(out, "result", "failed");
            out.push(',');
            push_json_number(out, "depth", *depth as u64);
            out.push(',');
            push_json_pair(out, "error", error);
            out.push(',');
            push_search_observations(out, observations);
        }
    }
    out.push('}');
    Ok(())
}

#[cfg(feature = "solver")]
fn push_search_observations(out: &mut String, observations: &[PuzzleSearchObservation]) {
    out.push_str("\"observations\":[");
    for (index, observation) in observations.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_search_observation(out, &observation.state, &observation.progress);
    }
    out.push(']');
}

#[cfg(feature = "solver")]
fn push_search_observation(out: &mut String, state: &State, progress: &SearchProgress) {
    out.push('{');
    push_search_progress(out, progress);
    out.push_str(",\"state\":");
    push_state_data(out, state);
    out.push('}');
}

#[cfg(feature = "solver")]
fn push_search_progress(out: &mut String, progress: &SearchProgress) {
    out.push_str("\"progress\":{");
    push_json_number(out, "visited", progress.visited as u64);
    out.push(',');
    push_json_number(out, "expanded", progress.expanded as u64);
    out.push(',');
    push_json_number(out, "frontier", progress.frontier as u64);
    out.push(',');
    push_json_number(out, "maxDepthReached", progress.max_depth_reached as u64);
    out.push(',');
    push_json_number(out, "depth", progress.depth as u64);
    out.push('}');
}

#[cfg(feature = "solver")]
fn push_compiled_solution_moves(
    out: &mut String,
    inputs: &[InputId],
    input_labels: &[(InputId, String)],
) -> Result<(), AppError> {
    out.push_str("\"moves\":[");
    for (index, input) in inputs.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_compiled_input_move(out, *input, input_labels)?;
    }
    out.push(']');
    Ok(())
}

#[cfg(feature = "solver")]
fn push_compiled_solution_steps(
    out: &mut String,
    steps: &[PuzzleSolutionStep],
    input_labels: &[(InputId, String)],
) -> Result<(), AppError> {
    out.push_str("\"steps\":[");
    for (index, step) in steps.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_number(out, "index", step.index as u64);
        out.push(',');
        if let Some(input) = step.input {
            out.push_str("\"move\":");
            push_compiled_input_move(out, input, input_labels)?;
        } else {
            out.push_str("\"move\":null");
        }
        out.push_str(",\"state\":");
        push_state_data(out, &step.state);
        out.push('}');
    }
    out.push(']');
    Ok(())
}

#[cfg(feature = "solver")]
fn push_compiled_input_move(
    out: &mut String,
    input: InputId,
    input_labels: &[(InputId, String)],
) -> Result<(), AppError> {
    let name = input_labels
        .iter()
        .find_map(|(id, label)| (*id == input).then_some(label.as_str()))
        .ok_or_else(|| {
            AppError::Config(format!(
                "compiled solver input label is missing for input {}",
                input.0
            ))
        })?;
    out.push('{');
    push_json_number(out, "id", input.0 as u64);
    out.push(',');
    push_json_pair(out, "name", name);
    out.push('}');
    Ok(())
}

#[cfg(feature = "solver")]
fn push_spatial_solution_response(
    out: &mut String,
    model: &LoadedGridGame<3, Size3>,
    response: &GridSolutionResponse<3, Size3>,
) {
    out.push('{');
    push_json_pair(out, "model", "puzzle3d");
    out.push(',');
    match response {
        SolutionResponse::Solved {
            depth,
            moves,
            steps,
            observations,
        } => {
            push_json_pair(out, "result", "solved");
            out.push(',');
            push_json_number(out, "depth", *depth as u64);
            out.push(',');
            push_solution_moves3(out, model, moves);
            out.push(',');
            push_spatial_solution_steps(out, model, steps);
            out.push(',');
            push_spatial_search_observations(out, observations);
        }
        SolutionResponse::Exhausted {
            stats,
            observations,
        } => {
            push_json_pair(out, "result", "exhausted");
            out.push(',');
            push_search_stats(out, stats);
            out.push(',');
            push_spatial_search_observations(out, observations);
        }
        SolutionResponse::BudgetExceeded {
            stats,
            observations,
        } => {
            push_json_pair(out, "result", "budget_exceeded");
            out.push(',');
            push_search_stats(out, stats);
            out.push(',');
            push_spatial_search_observations(out, observations);
        }
        SolutionResponse::Failed {
            depth,
            error,
            observations,
        } => {
            push_json_pair(out, "result", "failed");
            out.push(',');
            push_json_number(out, "depth", *depth as u64);
            out.push(',');
            push_json_pair(out, "error", error);
            out.push(',');
            push_spatial_search_observations(out, observations);
        }
    }
    out.push('}');
}

#[cfg(feature = "solver")]
fn push_spatial_search_observations(
    out: &mut String,
    observations: &[GridSearchObservation<3, Size3>],
) {
    out.push_str("\"observations\":[");
    for (index, observation) in observations.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_spatial_search_observation(out, &observation.state, &observation.progress);
    }
    out.push(']');
}

#[cfg(feature = "solver")]
fn push_spatial_search_observation(
    out: &mut String,
    state: &GridState<3, Size3>,
    progress: &SearchProgress,
) {
    out.push('{');
    push_search_progress(out, progress);
    out.push_str(",\"state\":");
    push_state3_data(out, state);
    out.push('}');
}
