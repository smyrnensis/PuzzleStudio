fn named_direction_vector(value: &str, line: &str) -> Result<(i16, i16), DiagnosticReport> {
    match value {
        "right" => Ok((1, 0)),
        "left" => Ok((-1, 0)),
        "up" => Ok((0, -1)),
        "down" => Ok((0, 1)),
        _ => Err(parse_error(line, "unknown direction name")),
    }
}

fn parse_scene_definition(
    lines: &[String],
    start: usize,
) -> Result<(SceneDef, usize), DiagnosticReport> {
    let header = split_header_tokens(&lines[start]);
    let name = match header.as_slice() {
        ["scene", "level_menu", ..] => {
            return Err(parse_error(
                &lines[start],
                "scene level_menu template is not supported; use scene <name> with layout { level_menu { ... } }",
            ));
        }
        ["scene", name] => *name,
        _ => {
            return Err(parse_error(
                &lines[start],
                "scene header must be: scene <name>[(param...)]",
            ));
        }
    };
    let (name, _params) = parse_scene_name_and_params(name, &lines[start])?;

    let mut scene = SceneDef {
        name: name.clone(),
        layout: SceneLayoutDef::default(),
        resources: SceneResources::default(),
        state: SceneStateDef::default(),
        components: Vec::new(),
        key_bindings: Vec::new(),
        routines: Vec::new(),
        transitions: Vec::new(),
        puzzle_rule: None,
    };
    let mut handler = Scene2dBlockHandler {
        scene: &mut scene,
    };
    let next = puzzle_scene::parse_scene_block_with_handler(
        lines,
        start + 1,
        &name,
        puzzle_scene::SceneBlockSyntax::Braces,
        &mut handler,
    )?;

    Ok((scene, next))
}

fn parse_scene_name_and_params(
    value: &str,
    line: &str,
) -> Result<(String, Vec<String>), DiagnosticReport> {
    let Some((name, params)) = value.split_once('(') else {
        validate_qualified_identifier(value, line, "scene name")?;
        return Ok((value.to_string(), Vec::new()));
    };
    validate_qualified_identifier(name, line, "scene name")?;
    let params = params
        .strip_suffix(')')
        .ok_or_else(|| parse_error(line, "scene params must end with )"))?;
    let params = if params.trim().is_empty() {
        Vec::new()
    } else {
        params
            .split(',')
            .map(str::trim)
            .map(|param| {
                validate_identifier(param, line, "scene param")?;
                Ok(param.to_string())
            })
            .collect::<Result<Vec<_>, DiagnosticReport>>()?
    };
    Ok((name.to_string(), params))
}

fn resolve_scene_actions(
    scenes: &mut [SceneDef],
    input_labels: &HashMap<InputId, String>,
) -> Result<(), DiagnosticReport> {
    let input_names = input_labels.values().cloned().collect::<HashSet<_>>();
    for scene in scenes {
        validate_scene_puzzle_slots(scene)?;
        resolve_scene_actions_for_scene(scene, &input_names)?;
        validate_scene_routines(scene)?;
        validate_scene_puzzle_rule(scene)?;
    }
    Ok(())
}

fn validate_scene_puzzle_slots(scene: &SceneDef) -> Result<(), DiagnosticReport> {
    let mut names = HashSet::<&str>::new();
    for puzzle in &scene.state.puzzles {
        if !names.insert(puzzle.name.as_str()) {
            return Err(DiagnosticReport::error(format!(
                "scene `{}` declares duplicate puzzle slot `{}`",
                scene.name, puzzle.name
            )));
        }
    }
    Ok(())
}

fn validate_scene_puzzle_rule(scene: &SceneDef) -> Result<(), DiagnosticReport> {
    let Some(rule) = &scene.puzzle_rule else {
        return Ok(());
    };
    let target = rule
        .target
        .split('.')
        .next_back()
        .unwrap_or(rule.target.as_str());
    if scene
        .state
        .puzzles
        .iter()
        .any(|puzzle| puzzle.name == target)
    {
        return Ok(());
    }
    let declared = scene
        .state
        .puzzles
        .iter()
        .map(|puzzle| puzzle.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    Err(DiagnosticReport::error(format!(
        "scene `{}` runs `step {}` but declares no matching puzzle slot (declared: [{}])",
        scene.name, rule.target, declared
    )))
}

fn resolve_scene_actions_for_scene(
    scene: &mut SceneDef,
    input_names: &HashSet<String>,
) -> Result<(), DiagnosticReport> {
    let routine_names = scene
        .routines
        .iter()
        .map(|routine| routine.name.clone())
        .collect::<HashSet<_>>();
    for binding in &mut scene.key_bindings {
        resolve_scene_effect_action(&mut binding.effect, input_names, &routine_names)?;
    }
    for transition in &mut scene.transitions {
        resolve_scene_effect_action(&mut transition.effect, input_names, &routine_names)?;
    }
    for component in &mut scene.components {
        resolve_scene_component_actions(component, input_names, &routine_names)?;
    }
    for routine in &mut scene.routines {
        resolve_scene_effect_action(&mut routine.effect, input_names, &routine_names)?;
    }
    Ok(())
}

fn resolve_scene_component_actions(
    component: &mut SceneComponent,
    input_names: &HashSet<String>,
    routine_names: &HashSet<String>,
) -> Result<(), DiagnosticReport> {
    match component {
        SceneComponent::Button(button) | SceneComponent::Choice(button) => {
            resolve_scene_effect_action(&mut button.effect, input_names, routine_names)
        }
        SceneComponent::Row(container)
        | SceneComponent::Column(container)
        | SceneComponent::Box(container) => {
            for child in &mut container.children {
                resolve_scene_component_actions(child, input_names, routine_names)?;
            }
            Ok(())
        }
        SceneComponent::Conditional(conditional) => {
            for child in &mut conditional.children {
                resolve_scene_component_actions(child, input_names, routine_names)?;
            }
            for child in &mut conditional.else_children {
                resolve_scene_component_actions(child, input_names, routine_names)?;
            }
            Ok(())
        }
        SceneComponent::For(for_view) => {
            for child in &mut for_view.children {
                resolve_scene_component_actions(child, input_names, routine_names)?;
            }
            Ok(())
        }
        SceneComponent::LevelMenu(menu) => {
            if let Some(effect) = &mut menu.action {
                resolve_scene_effect_action(effect, input_names, routine_names)?;
            }
            for button in &mut menu.buttons {
                resolve_scene_effect_action(&mut button.effect, input_names, routine_names)?;
            }
            Ok(())
        }
        SceneComponent::Frame(_)
        | SceneComponent::Title(_)
        | SceneComponent::Subtitle(_)
        | SceneComponent::Text(_) => Ok(()),
    }
}

fn resolve_scene_effect_action(
    effect: &mut SceneEffect,
    input_names: &HashSet<String>,
    routine_names: &HashSet<String>,
) -> Result<(), DiagnosticReport> {
    match effect {
        SceneEffect::RoutineCall(name) => {
            let is_input = input_names.contains(name);
            let is_routine = routine_names.contains(name);
            match (is_input, is_routine) {
                (true, true) => Err(DiagnosticReport::error(format!(
                    "ambiguous scene action `{name}`; write `input {name}` or rename the scene routine"
                ))),
                (true, false) => {
                    *effect = SceneEffect::Input(name.clone());
                    Ok(())
                }
                (false, true) => Ok(()),
                (false, false) => Err(DiagnosticReport::error(format!(
                    "unknown scene action: {name}"
                ))),
            }
        }
        SceneEffect::Conditional { effect, .. } => {
            resolve_scene_effect_action(effect, input_names, routine_names)
        }
        SceneEffect::Sequence(effects) => {
            for effect in effects {
                resolve_scene_effect_action(effect, input_names, routine_names)?;
            }
            Ok(())
        }
        SceneEffect::Input(_)
        | SceneEffect::ComponentEffect(_)
        | SceneEffect::Message { .. }
        | SceneEffect::Wait { .. }
        | SceneEffect::PlaySfx { .. }
        | SceneEffect::PlayMusic { .. }
        | SceneEffect::PauseMusic { .. }
        | SceneEffect::ResumeMusic { .. }
        | SceneEffect::StopMusic { .. }
        | SceneEffect::Goto { .. }
        | SceneEffect::Enter { .. }
        | SceneEffect::Back
        | SceneEffect::Create { .. }
        | SceneEffect::Reset { .. }
        | SceneEffect::Delete { .. }
        | SceneEffect::Show { .. }
        | SceneEffect::Hide { .. }
        | SceneEffect::Toggle { .. }
        | SceneEffect::Focus { .. }
        | SceneEffect::PuzzleNextLevel { .. }
        | SceneEffect::PuzzlePreviousLevel { .. }
        | SceneEffect::GotoLevel { .. }
        | SceneEffect::ResetPuzzle { .. }
        | SceneEffect::LoadPuzzle { .. }
        | SceneEffect::Apply { .. }
        | SceneEffect::Copy { .. }
        | SceneEffect::SetVariable { .. }
        | SceneEffect::ClearUndoHistory
        | SceneEffect::ClearGameProgress
        | SceneEffect::SetCurrentLevel { .. }
        | SceneEffect::ClearCurrentLevel
        | SceneEffect::SetLevelCleared { .. }
        | SceneEffect::ResetPersistentVars => Ok(()),
    }
}

fn add_scene_input_key_controls(
    scenes: &[SceneDef],
    input_labels: &HashMap<InputId, String>,
    controls: &mut Controls,
) {
    let input_ids = input_labels
        .iter()
        .map(|(id, label)| (label.as_str(), *id))
        .collect::<HashMap<_, _>>();
    for scene in scenes {
        for binding in &scene.key_bindings {
            let SceneEffect::Input(input) = &binding.effect else {
                continue;
            };
            let Some(input_id) = input_ids.get(input.as_str()).copied() else {
                continue;
            };
            for key in &binding.keys {
                add_key_trigger_to_controls_unchecked(key, input_id, controls);
            }
        }
    }
}

fn add_key_trigger_to_controls_unchecked(
    key: &KeyTrigger,
    input: InputId,
    controls: &mut Controls,
) {
    match key {
        KeyTrigger::Char(ch) if ch.is_ascii() => {
            controls
                .keys
                .insert((*ch as u8).to_ascii_lowercase(), input);
        }
        KeyTrigger::Char(_) => {}
        KeyTrigger::Named(name) => {
            if let Some(arrow) = named_key_to_arrow(name) {
                controls.arrows.insert(arrow, input);
            } else {
                controls.named.insert(name.clone(), input);
            }
        }
    }
}

fn validate_scene_routines(scene: &SceneDef) -> Result<(), DiagnosticReport> {
    let routine_names = scene
        .routines
        .iter()
        .map(|routine| routine.name.clone())
        .collect::<HashSet<_>>();
    for binding in &scene.key_bindings {
        validate_scene_effect_routine_calls(&binding.effect, &routine_names)?;
    }
    for transition in &scene.transitions {
        validate_scene_effect_routine_calls(&transition.effect, &routine_names)?;
    }
    for component in &scene.components {
        validate_scene_component_routine_calls(component, &routine_names)?;
    }

    let routines = scene
        .routines
        .iter()
        .map(|routine| (routine.name.as_str(), routine))
        .collect::<HashMap<_, _>>();
    let mut checked = HashSet::<String>::new();
    for routine in &scene.routines {
        validate_scene_routine_not_recursive(
            routine.name.as_str(),
            &routines,
            &mut Vec::new(),
            &mut checked,
        )?;
    }
    Ok(())
}

fn validate_scene_component_routine_calls(
    component: &SceneComponent,
    routine_names: &HashSet<String>,
) -> Result<(), DiagnosticReport> {
    match component {
        SceneComponent::Button(button) | SceneComponent::Choice(button) => {
            validate_scene_effect_routine_calls(&button.effect, routine_names)
        }
        SceneComponent::Row(container)
        | SceneComponent::Column(container)
        | SceneComponent::Box(container) => {
            for child in &container.children {
                validate_scene_component_routine_calls(child, routine_names)?;
            }
            Ok(())
        }
        SceneComponent::Conditional(conditional) => {
            for child in &conditional.children {
                validate_scene_component_routine_calls(child, routine_names)?;
            }
            for child in &conditional.else_children {
                validate_scene_component_routine_calls(child, routine_names)?;
            }
            Ok(())
        }
        SceneComponent::For(for_view) => {
            for child in &for_view.children {
                validate_scene_component_routine_calls(child, routine_names)?;
            }
            Ok(())
        }
        SceneComponent::LevelMenu(menu) => {
            if let Some(effect) = &menu.action {
                validate_scene_effect_routine_calls(effect, routine_names)?;
            }
            for button in &menu.buttons {
                validate_scene_effect_routine_calls(&button.effect, routine_names)?;
            }
            Ok(())
        }
        SceneComponent::Frame(_)
        | SceneComponent::Title(_)
        | SceneComponent::Subtitle(_)
        | SceneComponent::Text(_) => Ok(()),
    }
}

fn validate_scene_effect_routine_calls(
    effect: &SceneEffect,
    routine_names: &HashSet<String>,
) -> Result<(), DiagnosticReport> {
    match effect {
        SceneEffect::RoutineCall(name) => {
            if !routine_names.contains(name) {
                return Err(DiagnosticReport::error(format!(
                    "unknown scene routine: {name}"
                )));
            }
            Ok(())
        }
        SceneEffect::Conditional { effect, .. } => {
            validate_scene_effect_routine_calls(effect, routine_names)
        }
        SceneEffect::Sequence(effects) => {
            for effect in effects {
                validate_scene_effect_routine_calls(effect, routine_names)?;
            }
            Ok(())
        }
        SceneEffect::Input(_)
        | SceneEffect::ComponentEffect(_)
        | SceneEffect::Message { .. }
        | SceneEffect::Wait { .. }
        | SceneEffect::PlaySfx { .. }
        | SceneEffect::PlayMusic { .. }
        | SceneEffect::PauseMusic { .. }
        | SceneEffect::ResumeMusic { .. }
        | SceneEffect::StopMusic { .. }
        | SceneEffect::Goto { .. }
        | SceneEffect::Enter { .. }
        | SceneEffect::Back
        | SceneEffect::Create { .. }
        | SceneEffect::Reset { .. }
        | SceneEffect::Delete { .. }
        | SceneEffect::Show { .. }
        | SceneEffect::Hide { .. }
        | SceneEffect::Toggle { .. }
        | SceneEffect::Focus { .. }
        | SceneEffect::PuzzleNextLevel { .. }
        | SceneEffect::PuzzlePreviousLevel { .. }
        | SceneEffect::GotoLevel { .. }
        | SceneEffect::ResetPuzzle { .. }
        | SceneEffect::LoadPuzzle { .. }
        | SceneEffect::Apply { .. }
        | SceneEffect::Copy { .. }
        | SceneEffect::SetVariable { .. }
        | SceneEffect::ClearUndoHistory
        | SceneEffect::ClearGameProgress
        | SceneEffect::SetCurrentLevel { .. }
        | SceneEffect::ClearCurrentLevel
        | SceneEffect::SetLevelCleared { .. }
        | SceneEffect::ResetPersistentVars => Ok(()),
    }
}

fn validate_scene_routine_not_recursive(
    name: &str,
    routines: &HashMap<&str, &SceneRoutineDef>,
    stack: &mut Vec<String>,
    checked: &mut HashSet<String>,
) -> Result<(), DiagnosticReport> {
    if checked.contains(name) {
        return Ok(());
    }
    if stack.iter().any(|active| active == name) {
        stack.push(name.to_string());
        return Err(DiagnosticReport::error(format!(
            "recursive scene routine call: {}",
            stack.join(" -> ")
        )));
    }
    let Some(routine) = routines.get(name) else {
        return Err(DiagnosticReport::error(format!(
            "unknown scene routine: {name}"
        )));
    };
    stack.push(name.to_string());
    for call in scene_effect_routine_calls(&routine.effect) {
        validate_scene_routine_not_recursive(call, routines, stack, checked)?;
    }
    stack.pop();
    checked.insert(name.to_string());
    Ok(())
}

fn scene_effect_routine_calls(effect: &SceneEffect) -> Vec<&str> {
    let mut calls = Vec::new();
    collect_scene_effect_routine_calls(effect, &mut calls);
    calls
}

fn collect_scene_effect_routine_calls<'a>(effect: &'a SceneEffect, calls: &mut Vec<&'a str>) {
    match effect {
        SceneEffect::RoutineCall(name) => calls.push(name.as_str()),
        SceneEffect::Conditional { effect, .. } => {
            collect_scene_effect_routine_calls(effect, calls);
        }
        SceneEffect::Sequence(effects) => {
            for effect in effects {
                collect_scene_effect_routine_calls(effect, calls);
            }
        }
        _ => {}
    }
}

struct Scene2dBlockHandler<'a> {
    scene: &'a mut SceneDef,
}

impl puzzle_scene::SceneBlockHandler for Scene2dBlockHandler<'_> {
    type Error = DiagnosticReport;

    fn parse_state_block(
        &mut self,
        lines: &[String],
        start: usize,
    ) -> Result<usize, DiagnosticReport> {
        Err(parse_error(
            &lines[start],
            "`state { ... }` was removed; declare scene slots and variables in `layout { ... }`",
        ))
    }

    fn parse_layout_block(
        &mut self,
        lines: &[String],
        start: usize,
    ) -> Result<usize, DiagnosticReport> {
        let (layout_block, next_i) = parse_scene_layout_block(lines, start)?;
        self.scene.layout = layout_block.layout;
        self.scene
            .state
            .variables
            .extend(layout_block.state.variables);
        self.scene.state.puzzles.extend(layout_block.state.puzzles);
        self.scene.components.extend(layout_block.components);
        Ok(next_i)
    }

    fn parse_inputs_block(
        &mut self,
        lines: &[String],
        start: usize,
    ) -> Result<usize, DiagnosticReport> {
        Err(parse_error(
            &lines[start],
            "`inputs { ... }` was removed; use `keys { <key...> -> <routine-or-effect> }`",
        ))
    }

    fn parse_keys_block(
        &mut self,
        lines: &[String],
        start: usize,
    ) -> Result<usize, DiagnosticReport> {
        let (bindings, next_i) = parse_scene_keys_block(lines, start)?;
        self.scene.key_bindings.extend(bindings);
        Ok(next_i)
    }

    fn parse_rules_block(
        &mut self,
        lines: &[String],
        start: usize,
    ) -> Result<usize, DiagnosticReport> {
        let (block, next_i) = parse_scene_rules_block(lines, start)?;
        if let Some(puzzle_rule) = block.puzzle_rule {
            self.scene.puzzle_rule = Some(puzzle_rule);
        }
        Ok(next_i)
    }

    fn parse_scene_start_block(
        &mut self,
        lines: &[String],
        start: usize,
    ) -> Result<usize, DiagnosticReport> {
        let (transition, next_i) = parse_scene_lifecycle_block(lines, start)?;
        self.scene.transitions.push(transition);
        Ok(next_i)
    }

    fn parse_inline_directive(
        &mut self,
        lines: &[String],
        start: usize,
    ) -> Result<usize, DiagnosticReport> {
        let tokens = split_header_tokens(&lines[start]);
        match tokens.as_slice() {
            ["resources"] => parse_scene_resources_block(lines, start, &mut self.scene.resources),
            ["var", ..]
            | ["const", ..]
            | ["persistent", "var", ..]
            | ["persistent", "const", ..] => {
                match parse_scene_state_entry(&lines[start], SceneStateLifetime::Instance)? {
                    ParsedSceneStateEntry::Variable(variable) => {
                        self.scene.state.variables.push(variable);
                    }
                    ParsedSceneStateEntry::Puzzle(_) => {
                        return Err(parse_error(
                            &lines[start],
                            "var cannot define a puzzle slot",
                        ));
                    }
                }
                Ok(start + 1)
            }
            ["on_level_start" | "on_level_clear" | "on_last_level_clear"] => Err(parse_error(
                &lines[start],
                "level lifecycle blocks belong inside puzzle; scene lifecycle block must be on_scene_start",
            )),
            ["input", ..] => Err(parse_error(
                &lines[start],
                "scene input handlers are removed; use `keys { <key...> -> <routine-or-effect> }` and `routine <name> { ... }`",
            )),
            ["action", ..] => Err(parse_error(
                &lines[start],
                "`action` scene handlers were removed; use `routine <name> { ... }`",
            )),
            ["routine", ..] => {
                let (routine, next_i) = parse_scene_routine_block(lines, start)?;
                if self
                    .scene
                    .routines
                    .iter()
                    .any(|existing| existing.name == routine.name)
                {
                    return Err(parse_error(&lines[start], "duplicate scene routine"));
                }
                self.scene.routines.push(routine);
                Ok(next_i)
            }
            ["if", ..] => {
                let (transition, next_i) = parse_scene_condition_block(lines, start)?;
                self.scene.transitions.push(transition);
                Ok(next_i)
            }
            [] => Ok(start + 1),
            _ if scene_entry_is_component(&tokens) => {
                let (component, next_i) = parse_scene_component(lines, start)?;
                self.scene.components.push(component);
                Ok(next_i)
            }
            [other, ..] => Err(parse_error(
                &lines[start],
                &format!("unknown scene directive {other}"),
            )),
        }
    }
}

fn parse_scene_resources_block(
    lines: &[String],
    start: usize,
    resources: &mut SceneResources,
) -> Result<usize, DiagnosticReport> {
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let tokens = split_header_tokens(&lines[i]);
        match tokens.as_slice() {
            ["levels", names @ ..] => {
                resources.levels = parse_resource_selection(names, &lines[i])?;
            }
            ["sprites", names @ ..] => {
                resources.sprites = parse_resource_selection(names, &lines[i])?;
            }
            [] => {}
            [other, ..] => {
                return Err(parse_error(
                    &lines[i],
                    &format!("unknown resources directive {other}"),
                ));
            }
        }
        i += 1;
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start],
            "resources missing closing brace",
        ));
    }
    Ok(i + 1)
}

fn parse_resource_selection(
    names: &[&str],
    line: &str,
) -> Result<ResourceSelection, DiagnosticReport> {
    match names {
        [] | ["all"] => Ok(ResourceSelection::All),
        ["none"] => Ok(ResourceSelection::Named(Vec::new())),
        names => {
            let mut selected = Vec::new();
            for name in names {
                if name.chars().any(|ch| matches!(ch, '{' | '}' | ',' | ';')) {
                    return Err(parse_error(
                        line,
                        "resource names must be whitespace-separated",
                    ));
                }
                selected.push((*name).to_string());
            }
            Ok(ResourceSelection::Named(selected))
        }
    }
}

struct ParsedSceneLayoutBlock {
    layout: SceneLayoutDef,
    state: ParsedSceneStateBlock,
    components: Vec<SceneComponent>,
}

fn parse_scene_layout_block(
    lines: &[String],
    start: usize,
) -> Result<(ParsedSceneLayoutBlock, usize), DiagnosticReport> {
    parse_scene_view_like_block(lines, start, "layout")
}

fn parse_scene_view_like_block(
    lines: &[String],
    start: usize,
    block_name: &str,
) -> Result<(ParsedSceneLayoutBlock, usize), DiagnosticReport> {
    let layout = parse_scene_layout_from_header(&lines[start], block_name)?;
    let mut variables = Vec::new();
    let mut puzzles = Vec::<ScenePuzzleDef>::new();
    let mut components = Vec::new();
    let mut hidden = Vec::<String>::new();
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        if let Some((slot, visible)) = parse_layer_visibility(&lines[i])? {
            if visible {
                hidden.retain(|name| name != &slot);
                if !components.iter().any(|component| {
                    scene_puzzle_component_source(component).is_some_and(|name| name == slot)
                }) && let Some(puzzle) = puzzles.iter().find(|puzzle| puzzle.name == slot)
                {
                    components.push(scene_puzzle_slot_component(puzzle));
                }
            } else {
                hidden.push(slot.clone());
                components.retain(|component| {
                    scene_puzzle_component_source(component) != Some(slot.as_str())
                });
            }
            i += 1;
            continue;
        }

        let tokens = split_header_tokens(&lines[i]);
        if matches!(tokens.as_slice(), ["panel", ..]) {
            return Err(parse_error(&lines[i], "unknown layout directive panel"));
        }
        if matches!(tokens.as_slice(), ["if", ..]) {
            let (component, next_i) = parse_view_if_component(lines, i)?;
            components.push(component);
            i = next_i;
            continue;
        }
        if scene_entry_is_component(&tokens) || matches!(tokens.as_slice(), ["puzzle", ..]) {
            let (component, next_i, nested_puzzles) = parse_scene_component_with_puzzles(
                lines,
                i,
                SceneStateLifetime::Instance,
            )?;
            components.push(component);
            puzzles.extend(nested_puzzles);
            i = next_i;
            continue;
        }

        if parse_assignment_row(&lines[i]).is_some() {
            if let Some(declaration) =
                parse_scene_puzzle_layout_declaration(&lines[i], SceneStateLifetime::Instance)?
            {
                if !hidden.iter().any(|name| name == &declaration.puzzle.name) {
                    components.push(scene_puzzle_slot_component_with_layout(
                        &declaration.puzzle,
                        declaration.layout,
                    ));
                }
                puzzles.push(declaration.puzzle);
            } else {
                match parse_scene_state_entry(&lines[i], SceneStateLifetime::Instance)? {
                    ParsedSceneStateEntry::Puzzle(puzzle) => {
                        if !hidden.iter().any(|name| name == &puzzle.name) {
                            components.push(scene_puzzle_slot_component(&puzzle));
                        }
                        puzzles.push(puzzle);
                    }
                    ParsedSceneStateEntry::Variable(variable) => variables.push(variable),
                }
            }
            i += 1;
            continue;
        }

        if let [slot] = tokens.as_slice()
            && is_identifier(slot)
        {
            let puzzle = inferred_scene_puzzle_slot(slot, SceneStateLifetime::Instance);
            if !hidden.iter().any(|name| name == &puzzle.name) {
                components.push(scene_puzzle_slot_component(&puzzle));
            }
            puzzles.push(puzzle);
            i += 1;
            continue;
        }

        let (component, next_i, nested_puzzles) = parse_scene_component_with_puzzles(
            lines,
            i,
            SceneStateLifetime::Instance,
        )?;
        components.push(component);
        puzzles.extend(nested_puzzles);
        i = next_i;
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start],
            &format!("{block_name} missing closing brace"),
        ));
    }

    Ok((
        ParsedSceneLayoutBlock {
            layout,
            state: ParsedSceneStateBlock { variables, puzzles },
            components,
        },
        i + 1,
    ))
}

fn parse_scene_layout_from_header(
    line: &str,
    keyword: &str,
) -> Result<SceneLayoutDef, DiagnosticReport> {
    puzzle_scene::parse_scene_layout_header(line, keyword, puzzle_scene::SceneBlockSyntax::Braces)
        .map_err(DiagnosticReport::from)
}

fn parse_layer_visibility(line: &str) -> Result<Option<(String, bool)>, DiagnosticReport> {
    let Some((name, value)) = parse_assignment_row(line) else {
        return Ok(None);
    };
    let Some(slot) = name.strip_suffix(".visible") else {
        return Ok(None);
    };
    validate_qualified_identifier(slot, line, "layer name")?;
    match value {
        "true" => Ok(Some((slot.to_string(), true))),
        "false" => Ok(Some((slot.to_string(), false))),
        _ => Err(parse_error(line, "layer visibility must be true or false")),
    }
}

fn scene_frame_component(kind: impl Into<String>, source: impl Into<String>) -> SceneComponent {
    scene_frame_component_with_layout(kind, source, SceneLayoutDef::default())
}

fn scene_frame_component_with_layout(
    kind: impl Into<String>,
    source: impl Into<String>,
    layout: SceneLayoutDef,
) -> SceneComponent {
    SceneComponent::Frame(puzzle_scene::FrameComponent {
        kind: kind.into(),
        source: source.into(),
        inputs: Vec::new(),
        layout,
    })
}

fn scene_puzzle_slot_component(puzzle: &ScenePuzzleDef) -> SceneComponent {
    scene_puzzle_slot_component_with_layout(puzzle, SceneLayoutDef::default())
}

fn scene_puzzle_slot_component_with_layout(
    puzzle: &ScenePuzzleDef,
    layout: SceneLayoutDef,
) -> SceneComponent {
    scene_frame_component_with_layout(puzzle.kind.clone(), puzzle.name.clone(), layout)
}

fn scene_puzzle_component_source(component: &SceneComponent) -> Option<&str> {
    match component {
        SceneComponent::Frame(frame) => Some(frame.source.as_str()),
        _ => None,
    }
}

fn parse_scene_components_block(
    lines: &[String],
    start: usize,
    block_name: &str,
) -> Result<(Vec<SceneComponent>, usize), DiagnosticReport> {
    let mut parse_leaf =
        |lines: &[String], index: usize| -> Result<(usize, SceneComponent), DiagnosticReport> {
            let (component, next, _) =
                parse_scene_leaf_component(lines, index, SceneStateLifetime::Instance)?;
            Ok((next, component))
        };
    let (next, components) = puzzle_scene::parse_scene_component_block(
        lines,
        start + 1,
        block_name,
        puzzle_scene::SceneBlockSyntax::Braces,
        &mut parse_leaf,
        &build_scene_container_component,
    )?;
    Ok((components, next))
}

fn parse_scene_component(
    lines: &[String],
    start: usize,
) -> Result<(SceneComponent, usize), DiagnosticReport> {
    let (component, next, _) =
        parse_scene_component_with_puzzles(lines, start, SceneStateLifetime::Instance)?;
    Ok((component, next))
}

fn parse_scene_component_with_puzzles(
    lines: &[String],
    start: usize,
    lifetime: SceneStateLifetime,
) -> Result<(SceneComponent, usize, Vec<ScenePuzzleDef>), DiagnosticReport> {
    let nested_puzzles = std::cell::RefCell::new(Vec::<ScenePuzzleDef>::new());
    let mut parse_leaf =
        |lines: &[String], index: usize| -> Result<(usize, SceneComponent), DiagnosticReport> {
            let (component, next, puzzle) = parse_scene_leaf_component(lines, index, lifetime)?;
            if let Some(puzzle) = puzzle {
                nested_puzzles.borrow_mut().push(puzzle);
            }
            Ok((next, component))
        };
    let (next, component) = puzzle_scene::parse_scene_component_at(
        lines,
        start,
        puzzle_scene::SceneBlockSyntax::Braces,
        &mut parse_leaf,
        &build_scene_container_component,
    )?;
    Ok((component, next, nested_puzzles.into_inner()))
}

fn build_scene_container_component(
    kind: puzzle_scene::SceneComponentKind,
    children: Vec<SceneComponent>,
    layout: SceneLayoutDef,
) -> SceneComponent {
    match kind {
        puzzle_scene::SceneComponentKind::Row => {
            SceneComponent::Row(SceneContainerDef { children, layout })
        }
        puzzle_scene::SceneComponentKind::Column => {
            SceneComponent::Column(SceneContainerDef { children, layout })
        }
        puzzle_scene::SceneComponentKind::Box => {
            SceneComponent::Box(SceneContainerDef { children, layout })
        }
        _ => unreachable!("shared scene parser only builds generic containers"),
    }
}

fn parse_scene_leaf_component(
    lines: &[String],
    start: usize,
    lifetime: SceneStateLifetime,
) -> Result<(SceneComponent, usize, Option<ScenePuzzleDef>), DiagnosticReport> {
    if let Some(declaration) = parse_scene_puzzle_layout_declaration(&lines[start], lifetime)? {
        return Ok((
            scene_puzzle_slot_component_with_layout(&declaration.puzzle, declaration.layout),
            start + 1,
            Some(declaration.puzzle),
        ));
    }
    let tokens = split_header_tokens(&lines[start]);
    match tokens.as_slice() {
        ["puzzle", "current_level"] => Err(parse_error(
            &lines[start],
            "current_level is not scene syntax; declare a puzzle slot with `board = puzzle <name>`",
        )),
        ["puzzle", state_name, attrs @ ..] => {
            if *state_name == "current_level" {
                return Err(parse_error(
                    &lines[start],
                    "current_level is not scene syntax; declare a puzzle slot with `board = puzzle <name>`",
                ));
            }
            if !is_identifier(state_name) {
                return Err(parse_error(
                    &lines[start],
                    "puzzle state name must be an identifier",
                ));
            }
            let layout = parse_scene_layout_attrs_for_line(attrs, &lines[start])?;
            Ok((
                scene_frame_component_with_layout("puzzle", (*state_name).to_string(), layout),
                start + 1,
                None,
            ))
        }
        ["frame", source, attrs @ ..] => {
            if !is_identifier(source) {
                return Err(parse_error(
                    &lines[start],
                    "frame source must be an identifier",
                ));
            }
            let layout = parse_scene_layout_attrs_for_line(attrs, &lines[start])?;
            Ok((
                scene_frame_component_with_layout("frame", (*source).to_string(), layout),
                start + 1,
                None,
            ))
        }
        ["puzzle3", source, attrs @ ..] => {
            if !is_identifier(source) {
                return Err(parse_error(
                    &lines[start],
                    "puzzle3 frame source must be an identifier",
                ));
            }
            let layout = parse_scene_layout_attrs_for_line(attrs, &lines[start])?;
            Ok((
                scene_frame_component_with_layout("puzzle3", (*source).to_string(), layout),
                start + 1,
                None,
            ))
        }
        ["text", ..] => Ok((parse_text_component(&lines[start])?, start + 1, None)),
        ["title", ..] => Ok((parse_title_component(&lines[start], true)?, start + 1, None)),
        ["subtitle", ..] => Ok((parse_title_component(&lines[start], false)?, start + 1, None)),
        ["button", ..] => {
            let (component, next) = parse_button_component(lines, start)?;
            Ok((component, next, None))
        }
        ["choice", ..] => {
            let (component, next) = parse_choice_component(lines, start)?;
            Ok((component, next, None))
        }
        ["if", ..] => {
            let (component, next) = parse_view_if_component(lines, start)?;
            Ok((component, next, None))
        }
        ["for", ..] => {
            let (component, next) = parse_for_component(lines, start)?;
            Ok((component, next, None))
        }
        ["level_menu"] => {
            let (menu, next_i) = parse_level_menu_component(lines, start)?;
            Ok((SceneComponent::LevelMenu(menu), next_i, None))
        }
        ["level_menu", ..] => Err(parse_error(
            &lines[start],
            "level_menu takes no inline source or effect; use scene resources to choose levels",
        )),
        [state_name] if is_identifier(state_name) => Ok((
            scene_frame_component("puzzle", (*state_name).to_string()),
            start + 1,
            None,
        )),
        [other, ..] => Err(parse_error(
            &lines[start],
            &format!("unknown layout directive {other}"),
        )),
        [] => Err(parse_error(&lines[start], "empty layout directive")),
    }
}

fn parse_scene_layout_attrs_for_line(
    attrs: &[&str],
    line: &str,
) -> Result<SceneLayoutDef, DiagnosticReport> {
    puzzle_scene::parse_scene_layout_attrs(attrs).map_err(|error| parse_error(line, &error.message))
}

fn parse_title_component(line: &str, is_title: bool) -> Result<SceneComponent, DiagnosticReport> {
    let keyword = if is_title { "title" } else { "subtitle" };
    let Some(rest) = line.strip_prefix(keyword) else {
        return Err(parse_error(line, "title must be: title <text-or-path>"));
    };
    let rest = rest.trim();
    let content = if rest.is_empty() {
        SceneExpr::Path(vec![keyword.to_string()])
    } else {
        parse_scene_expr(rest, line)?
    };
    let title = SceneTitleDef {
        content,
        layout: SceneLayoutDef::default(),
    };
    Ok(if is_title {
        SceneComponent::Title(title)
    } else {
        SceneComponent::Subtitle(title)
    })
}

fn parse_text_component(line: &str) -> Result<SceneComponent, DiagnosticReport> {
    let Some(rest) = line.strip_prefix("text") else {
        return Err(parse_error(
            line,
            "text must be: text \"<text>\" | text <state>",
        ));
    };
    let rest = rest.trim();
    if let Some(text) = parse_quoted_text(rest) {
        return Ok(SceneComponent::Text(SceneTextDef {
            content: SceneTextContent::Literal(text),
            layout: SceneLayoutDef::default(),
        }));
    }
    if let Some(path) = parse_view_path(rest) {
        return Ok(SceneComponent::Text(SceneTextDef {
            content: SceneTextContent::Path(path),
            layout: SceneLayoutDef::default(),
        }));
    }
    Ok(SceneComponent::Text(SceneTextDef {
        content: SceneTextContent::Expr(parse_scene_expr(rest, line)?),
        layout: SceneLayoutDef::default(),
    }))
}

fn parse_button_like_def(
    lines: &[String],
    start: usize,
    keyword: &str,
) -> Result<(SceneButtonDef, usize), DiagnosticReport> {
    let line = &lines[start];
    let Some(rest) = line.strip_prefix(keyword) else {
        return Err(parse_error(
            line,
            &format!("{keyword} must be: {keyword} \"<label>\" -> <effect>"),
        ));
    };
    let rest = rest.trim();
    if rest.is_empty() {
        return Err(parse_error(
            line,
            &format!("{keyword} must be: {keyword} \"<label>\" -> <effect>"),
        ));
    }

    let (label, effect, next_i) = if parse_assignment_row(rest).is_some() {
        return Err(parse_error(
            line,
            &format!("{keyword} command must use `->`; `=` action assignment was removed"),
        ));
    } else {
        let (label, effect) = require_arrow_row(
            rest,
            &format!("{keyword} must be: {keyword} \"<label>\" -> <effect>"),
        )?;
        let (effect, next_i) = parse_scene_effect_with_optional_block(effect, lines, start)?;
        (parse_button_label(label, line)?, effect, next_i)
    };

    Ok((
        SceneButtonDef {
            label,
            effect,
            layout: SceneLayoutDef::default(),
        },
        next_i,
    ))
}

fn parse_button_def(
    lines: &[String],
    start: usize,
) -> Result<(SceneButtonDef, usize), DiagnosticReport> {
    parse_button_like_def(lines, start, "button")
}

fn parse_button_component(
    lines: &[String],
    start: usize,
) -> Result<(SceneComponent, usize), DiagnosticReport> {
    let (button, next_i) = parse_button_def(lines, start)?;
    Ok((SceneComponent::Button(button), next_i))
}

fn parse_choice_component(
    lines: &[String],
    start: usize,
) -> Result<(SceneComponent, usize), DiagnosticReport> {
    let (choice, next_i) = parse_button_like_def(lines, start, "choice")?;
    Ok((SceneComponent::Choice(choice), next_i))
}

fn parse_view_if_component(
    lines: &[String],
    start: usize,
) -> Result<(SceneComponent, usize), DiagnosticReport> {
    let line = &lines[start];
    let condition = block_header_text(line)
        .strip_prefix("if ")
        .ok_or_else(|| parse_error(line, "layout condition must be: if <condition>"))?
        .trim();
    let condition = parse_scene_expr(condition, line)?;
    let (entry, next_i) = collect_authoring_entry(lines, start)?;
    let body = &entry[1..entry.len().saturating_sub(1)];
    let (else_body, next_i) = collect_view_else_body(lines, next_i, line)?;
    if body.is_empty() {
        return Err(parse_error(
            line,
            "layout condition requires at least one component",
        ));
    }
    let children = parse_scene_component_body(body, "if")?;
    let else_children = if else_body.is_empty() {
        Vec::new()
    } else {
        parse_scene_component_body(&else_body, "else")?
    };
    Ok((
        SceneComponent::Conditional(SceneConditionalDef {
            condition,
            children,
            else_children,
        }),
        next_i,
    ))
}

fn collect_view_else_body(
    lines: &[String],
    start: usize,
    header_line: &str,
) -> Result<(Vec<String>, usize), DiagnosticReport> {
    if !next_line_is_else(lines, start) {
        return Ok((Vec::new(), start));
    }

    let mut body = Vec::new();
    let mut block_stack = vec![AuthoringBlockKind::Other];
    let mut i = start + 1;
    while i < lines.len() {
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        if tokens.first().copied() == Some(BLOCK_CLOSE) {
            let closed = block_stack
                .pop()
                .ok_or_else(|| parse_error(line, "closing brace without layout block"))?;
            i += 1;
            if block_stack.is_empty() {
                return Ok((body, i));
            }
            body.push(line.clone());
            if closed == AuthoringBlockKind::If && next_line_is_else(lines, i) {
                body.push(lines[i].clone());
                i += 1;
                block_stack.push(AuthoringBlockKind::Other);
            }
            continue;
        }
        if let Some(kind) = authoring_nested_block_kind(&tokens, line) {
            block_stack.push(kind);
        }
        body.push(line.clone());
        i += 1;
    }
    Err(parse_error(
        header_line,
        "layout else block missing closing brace",
    ))
}

fn parse_scene_component_body(
    body: &[String],
    block_name: &str,
) -> Result<Vec<SceneComponent>, DiagnosticReport> {
    let mut lines = body.to_vec();
    lines.push(BLOCK_CLOSE.to_string());
    let mut parse_leaf =
        |lines: &[String], index: usize| -> Result<(usize, SceneComponent), DiagnosticReport> {
            let (component, next, _) =
                parse_scene_leaf_component(lines, index, SceneStateLifetime::Instance)?;
            Ok((next, component))
        };
    let (next, components) = puzzle_scene::parse_scene_component_block(
        &lines,
        0,
        block_name,
        puzzle_scene::SceneBlockSyntax::Braces,
        &mut parse_leaf,
        &build_scene_container_component,
    )?;
    debug_assert_eq!(next, lines.len());
    Ok(components)
}

fn parse_for_component(
    lines: &[String],
    start: usize,
) -> Result<(SceneComponent, usize), DiagnosticReport> {
    let tokens = split_header_tokens(&lines[start]);
    let ["for", binding, "in", source] = tokens.as_slice() else {
        return Err(parse_error(
            &lines[start],
            "for layout must be: for <item> in <source>",
        ));
    };
    if !is_identifier(binding) {
        return Err(parse_error(
            &lines[start],
            "for binding must be an identifier",
        ));
    }
    let source = parse_for_source(source, &lines[start])?;
    let (children, next_i) = parse_scene_components_block(lines, start, "for")?;
    Ok((
        SceneComponent::For(SceneForDef {
            binding: (*binding).to_string(),
            source,
            children,
        }),
        next_i,
    ))
}

fn parse_for_source(value: &str, line: &str) -> Result<ForSource, DiagnosticReport> {
    if value == "levels" {
        return Ok(ForSource::Levels);
    }
    if is_identifier(value) {
        return Ok(ForSource::State(value.to_string()));
    }
    Err(parse_error(
        line,
        "for source must be levels or a state identifier",
    ))
}
