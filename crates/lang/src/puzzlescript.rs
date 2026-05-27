use std::collections::{BTreeMap, BTreeSet};

use crate::{AppError, source::strip_line_comment};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PsSection {
    Prelude,
    Objects,
    Legend,
    Sounds,
    CollisionLayers,
    Rules,
    WinConditions,
    Levels,
}

#[derive(Default)]
struct PsSections {
    prelude: Vec<String>,
    objects: Vec<String>,
    legend: Vec<String>,
    sounds: Vec<String>,
    collision_layers: Vec<String>,
    rules: Vec<String>,
    win_conditions: Vec<String>,
    levels: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PsObjectDef {
    name: String,
    shorthand: Option<char>,
    sprite: Option<PsSpriteDef>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PsAliasDef {
    name: String,
    terms: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PsSpriteDef {
    colors: Vec<String>,
    pattern: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PsSoundDef {
    name: String,
    seed: String,
    trigger: PsSoundTrigger,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PsViewportSize {
    width: usize,
    height: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum PsSoundTrigger {
    Named,
    Event { target: String, event: PsSoundEvent },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PsSoundEvent {
    Create,
    Move,
}

pub fn translate_puzzlescript_to_canonical(source: &str) -> Result<String, AppError> {
    let sections = collect_sections(source);
    let title = parse_title(&sections.prelude);
    let author = parse_author(&sections.prelude);
    let homepage = parse_homepage(&sections.prelude);
    let run_rules_on_level_start = parse_run_rules_on_level_start(&sections.prelude);
    let again_interval = parse_again_interval(&sections.prelude);
    let theme_colors = parse_theme_colors(&sections.prelude);
    let viewport_size = parse_viewport_size(&sections.prelude);
    let sounds = parse_sound_defs(&sections.sounds);
    let startgame_sfx = ps_sound_name(&sounds, "startgame");
    let object_defs = parse_object_defs(&sections.objects);
    let background_object = ps_background_object(&object_defs);
    let aliases = parse_alias_defs(&sections.legend, &object_defs);
    let collision_layers =
        parse_collision_layers(&sections.collision_layers, &object_defs, &aliases);
    let mut out = Vec::new();
    out.push(format!("title {}", canonical_metadata_text(&title)));
    if let Some(author) = &author {
        out.push(format!("author {}", canonical_metadata_text(author)));
    }
    if let Some(homepage) = &homepage {
        out.push(format!("homepage {}", canonical_metadata_text(homepage)));
    }
    if let Some(seconds) = &again_interval {
        out.push(format!("again_interval = {seconds}s"));
    }
    out.push(String::new());
    push_theme_colors(&mut out, &theme_colors);
    push_sounds(&mut out, &sounds);
    out.push("puzzle main {".to_string());
    push_viewport_size(
        &mut out,
        viewport_size,
        ps_viewport_focus(&object_defs, &aliases).as_deref(),
    );
    push_layers(&mut out, &collision_layers);
    push_default_inputs(&mut out);
    push_groups(&mut out, &aliases);
    push_sprites(&mut out, &object_defs);
    push_win_conditions(&mut out, &sections.win_conditions, &object_defs, &aliases);
    push_ps_sound_scratch(&mut out, &sounds);
    push_ps_sound_routines(&mut out, &sounds, &object_defs, &aliases);
    push_rules(
        &mut out,
        &sections.rules,
        &object_defs,
        &aliases,
        run_rules_on_level_start,
        background_object.as_deref(),
        &sounds,
    );
    push_ps_level_clear(&mut out);
    push_levels(
        &mut out,
        &sections.levels,
        &sections.legend,
        &object_defs,
        &aliases,
    );
    out.push("}".to_string());
    out.push(String::new());
    push_playing_scene(
        &mut out,
        &title,
        author.as_deref(),
        startgame_sfx.as_deref(),
        viewport_size,
    );
    Ok(out.join("\n"))
}

fn collect_sections(source: &str) -> PsSections {
    let mut sections = PsSections::default();
    let mut current = PsSection::Prelude;
    let mut in_parenthetical_comment = false;
    for raw_line in source.lines() {
        let line = strip_line_comment(raw_line).trim().to_string();
        if in_parenthetical_comment {
            if line.ends_with(')') {
                in_parenthetical_comment = false;
            }
            continue;
        }
        if line.starts_with('(') {
            if !line.ends_with(')') {
                in_parenthetical_comment = true;
            }
            continue;
        }
        if is_section_separator(&line) {
            continue;
        }

        if let Some(section) = parse_section_header(&line) {
            current = section;
            continue;
        }

        match current {
            PsSection::Prelude => sections.prelude.push(line),
            PsSection::Objects => sections.objects.push(line),
            PsSection::Legend => sections.legend.push(line),
            PsSection::Sounds => sections.sounds.push(line),
            PsSection::CollisionLayers => sections.collision_layers.push(line),
            PsSection::Rules => sections.rules.push(line),
            PsSection::WinConditions => sections.win_conditions.push(line),
            PsSection::Levels => sections.levels.push(line),
        }
    }
    sections
}

fn is_section_separator(line: &str) -> bool {
    line.len() >= 3 && line.chars().all(|ch| ch == '=')
}

fn parse_section_header(line: &str) -> Option<PsSection> {
    match normalize_section_name(line).as_deref()? {
        "objects" => Some(PsSection::Objects),
        "legend" => Some(PsSection::Legend),
        "sounds" => Some(PsSection::Sounds),
        "collisionlayers" => Some(PsSection::CollisionLayers),
        "rules" => Some(PsSection::Rules),
        "winconditions" => Some(PsSection::WinConditions),
        "levels" => Some(PsSection::Levels),
        _ => None,
    }
}

fn normalize_section_name(line: &str) -> Option<String> {
    let normalized = line
        .chars()
        .filter(|ch| !ch.is_whitespace() && *ch != '_')
        .flat_map(char::to_lowercase)
        .collect::<String>();
    (!normalized.is_empty()).then_some(normalized)
}

fn parse_title(prelude: &[String]) -> String {
    prelude
        .iter()
        .find_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }
            if let Some(title) = trimmed.strip_prefix("title").map(str::trim) {
                return (!title.is_empty()).then(|| title.to_string());
            }
            Some(trimmed.to_string())
        })
        .unwrap_or_else(|| "PuzzleScript import".to_string())
}

fn parse_author(prelude: &[String]) -> Option<String> {
    prelude.iter().find_map(|line| {
        let trimmed = line.trim();
        trimmed
            .strip_prefix("author")
            .map(str::trim)
            .filter(|author| !author.is_empty())
            .map(str::to_string)
    })
}

fn parse_homepage(prelude: &[String]) -> Option<String> {
    prelude.iter().find_map(|line| {
        let trimmed = line.trim();
        trimmed
            .strip_prefix("homepage")
            .map(str::trim)
            .filter(|homepage| !homepage.is_empty())
            .map(str::to_string)
    })
}

fn parse_run_rules_on_level_start(prelude: &[String]) -> bool {
    prelude
        .iter()
        .any(|line| line.trim().eq_ignore_ascii_case("run_rules_on_level_start"))
}

fn parse_again_interval(prelude: &[String]) -> Option<String> {
    prelude.iter().find_map(|line| {
        let tokens = line.split_whitespace().collect::<Vec<_>>();
        let [command, seconds] = tokens.as_slice() else {
            return None;
        };
        command
            .eq_ignore_ascii_case("again_interval")
            .then(|| (*seconds).to_string())
    })
}

fn parse_theme_colors(prelude: &[String]) -> Vec<(String, String)> {
    let mut colors = Vec::new();
    for line in prelude {
        let tokens = line.split_whitespace().collect::<Vec<_>>();
        let [name @ ("background_color" | "text_color"), value] = tokens.as_slice() else {
            continue;
        };
        if let Some(color) = ps_color_to_canonical(value) {
            colors.push(((*name).to_string(), color));
        }
    }
    colors
}

fn parse_viewport_size(prelude: &[String]) -> Option<PsViewportSize> {
    prelude.iter().find_map(|line| {
        let tokens = line.split_whitespace().collect::<Vec<_>>();
        match tokens.as_slice() {
            [command, size] if command.eq_ignore_ascii_case("flickscreen") => {
                parse_ps_screen_size(size)
            }
            [command, width, height] if command.eq_ignore_ascii_case("flickscreen") => {
                parse_ps_screen_size_pair(width, height)
            }
            _ => None,
        }
    })
}

fn parse_ps_screen_size(value: &str) -> Option<PsViewportSize> {
    let (width, height) = value.split_once(['x', 'X'])?;
    parse_ps_screen_size_pair(width, height)
}

fn parse_ps_screen_size_pair(width: &str, height: &str) -> Option<PsViewportSize> {
    let width = width.parse::<usize>().ok()?;
    let height = height.parse::<usize>().ok()?;
    (width > 0 && height > 0).then_some(PsViewportSize { width, height })
}

fn push_viewport_size(
    out: &mut Vec<String>,
    viewport_size: Option<PsViewportSize>,
    viewport_focus: Option<&str>,
) {
    if let Some(size) = viewport_size {
        out.push(format!("flickscreen {} {}", size.width, size.height));
        if let Some(focus) = viewport_focus {
            out.push(format!("screen_focus {focus}"));
        }
        out.push(String::new());
    }
}

fn ps_viewport_focus(objects: &[PsObjectDef], aliases: &[PsAliasDef]) -> Option<String> {
    resolve_name("player", objects, aliases)
}

fn push_theme_colors(out: &mut Vec<String>, colors: &[(String, String)]) {
    if colors.is_empty() {
        out.push("theme puzzlescript".to_string());
        out.push(String::new());
        return;
    }
    out.push("theme puzzlescript {".to_string());
    for (name, value) in colors {
        out.push(format!("  {name} {value}"));
    }
    out.push("}".to_string());
    out.push(String::new());
}

fn parse_sound_defs(lines: &[String]) -> Vec<PsSoundDef> {
    let mut sounds = Vec::new();
    for line in lines {
        let body = line
            .split_once('(')
            .map_or(line.as_str(), |(before, _)| before)
            .trim();
        if body.is_empty() {
            continue;
        }
        let tokens = body.split_whitespace().collect::<Vec<_>>();
        let Some(sound) = parse_sound_def_tokens(&tokens) else {
            continue;
        };
        if sounds
            .iter()
            .any(|existing: &PsSoundDef| existing.name.eq_ignore_ascii_case(&sound.name))
        {
            continue;
        }
        sounds.push(sound);
    }
    sounds
}

fn parse_sound_def_tokens(tokens: &[&str]) -> Option<PsSoundDef> {
    match tokens {
        [name, seed] if is_identifier(name) && is_sound_atom(seed) => Some(PsSoundDef {
            name: (*name).to_string(),
            seed: (*seed).to_string(),
            trigger: PsSoundTrigger::Named,
        }),
        [target, event, seed]
            if is_identifier(target)
                && is_sound_atom(seed)
                && parse_sound_event(event).is_some() =>
        {
            let event = parse_sound_event(event)?;
            Some(PsSoundDef {
                name: ps_event_sound_name(target, event),
                seed: (*seed).to_string(),
                trigger: PsSoundTrigger::Event {
                    target: (*target).to_string(),
                    event,
                },
            })
        }
        _ => None,
    }
}

fn parse_sound_event(token: &str) -> Option<PsSoundEvent> {
    match token.to_ascii_lowercase().as_str() {
        "create" => Some(PsSoundEvent::Create),
        "move" => Some(PsSoundEvent::Move),
        _ => None,
    }
}

fn ps_event_sound_name(target: &str, event: PsSoundEvent) -> String {
    let event = match event {
        PsSoundEvent::Create => "create",
        PsSoundEvent::Move => "move",
    };
    format!("{}_{}", target.to_ascii_lowercase(), event)
}

fn push_sounds(out: &mut Vec<String>, sounds: &[PsSoundDef]) {
    if sounds.is_empty() {
        return;
    }
    out.push("sounds {".to_string());
    for sound in sounds {
        out.push(format!(
            "  sfx {} seed={} type=puzzlescript",
            sound.name, sound.seed
        ));
    }
    out.push("}".to_string());
    out.push(String::new());
}

fn push_ps_sound_scratch(out: &mut Vec<String>, sounds: &[PsSoundDef]) {
    if !has_event_sounds(sounds) {
        return;
    }
    out.push("scratch {".to_string());
    let mut emitted = Vec::new();
    for sound in sounds {
        let PsSoundTrigger::Event { target, event } = &sound.trigger else {
            continue;
        };
        let key = ps_sound_scratch_key(target);
        match event {
            PsSoundEvent::Create => {
                push_unique_scratch(out, &mut emitted, format!("__ps_sound_existing_{key}"))
            }
            PsSoundEvent::Move => {}
        }
    }
    out.push("}".to_string());
    out.push(String::new());
}

fn push_ps_sound_routines(
    out: &mut Vec<String>,
    sounds: &[PsSoundDef],
    objects: &[PsObjectDef],
    aliases: &[PsAliasDef],
) {
    if !has_event_sounds(sounds) {
        return;
    }

    out.push("routine __ps_sound_mark_existing once {".to_string());
    for sound in sounds {
        let PsSoundTrigger::Event {
            target,
            event: PsSoundEvent::Create,
        } = &sound.trigger
        else {
            continue;
        };
        let target = ps_sound_target_selector(target, objects, aliases);
        out.push(format!(
            "  once_all [ {target} ] -> [ {target}{{__ps_sound_existing_{}}} ]",
            ps_sound_scratch_key(&target)
        ));
    }
    out.push("}".to_string());
    out.push(String::new());

    out.push("routine __ps_sound_emit_events once {".to_string());
    for sound in sounds {
        let PsSoundTrigger::Event { target, event } = &sound.trigger else {
            continue;
        };
        let target = ps_sound_target_selector(target, objects, aliases);
        let key = ps_sound_scratch_key(&target);
        match event {
            PsSoundEvent::Create => out.push(format!(
                "  once [ {target}{{no __ps_sound_existing_{key}}} ] -> sfx {}",
                sound.name
            )),
            PsSoundEvent::Move => {}
        }
    }
    out.push("}".to_string());
    out.push(String::new());
}

fn push_unique_scratch(out: &mut Vec<String>, emitted: &mut Vec<String>, name: String) {
    if emitted.iter().any(|existing| existing == &name) {
        return;
    }
    out.push(format!("  {name}"));
    emitted.push(name);
}

fn push_ps_sound_call(out: &mut Vec<String>, sounds: &[PsSoundDef], indent: &str, routine: &str) {
    if has_event_sounds(sounds) {
        out.push(format!("{indent}{routine}"));
    }
}

fn has_event_sounds(sounds: &[PsSoundDef]) -> bool {
    sounds.iter().any(|sound| {
        matches!(
            sound.trigger,
            PsSoundTrigger::Event {
                event: PsSoundEvent::Create,
                ..
            }
        )
    })
}

fn ps_sound_scratch_key(target: &str) -> String {
    let mut key = String::new();
    for ch in target.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            key.push(ch.to_ascii_lowercase());
        } else {
            key.push('_');
        }
    }
    if key.is_empty() {
        "target".to_string()
    } else {
        key
    }
}

fn ps_sound_target_selector(
    target: &str,
    objects: &[PsObjectDef],
    aliases: &[PsAliasDef],
) -> String {
    resolve_name(target, objects, aliases).unwrap_or_else(|| target.to_string())
}

fn ps_sound_name(sounds: &[PsSoundDef], name: &str) -> Option<String> {
    sounds
        .iter()
        .find(|sound| sound.name.eq_ignore_ascii_case(name))
        .map(|sound| sound.name.clone())
}

fn parse_object_defs(lines: &[String]) -> Vec<PsObjectDef> {
    let mut objects = Vec::new();
    let mut previous_meaningful = None::<String>;
    let mut i = 0usize;
    while i < lines.len() {
        let line = &lines[i];
        let trimmed = line.trim();
        if trimmed.is_empty() {
            previous_meaningful = None;
            i += 1;
            continue;
        }

        if let Some((name, shorthand)) =
            parse_object_header(trimmed, previous_meaningful.as_deref())
            && !objects
                .iter()
                .any(|existing: &PsObjectDef| existing.name == name)
        {
            let (sprite, next_i) = parse_object_sprite(lines, i + 1);
            objects.push(PsObjectDef {
                name,
                shorthand,
                sprite,
            });
            previous_meaningful = objects
                .last()
                .and_then(|object| object.sprite.as_ref())
                .and_then(|sprite| sprite.pattern.last())
                .cloned()
                .or_else(|| Some(trimmed.to_string()));
            i = next_i;
            continue;
        }
        previous_meaningful = Some(trimmed.to_string());
        i += 1;
    }
    objects
}

fn parse_object_header(line: &str, previous: Option<&str>) -> Option<(String, Option<char>)> {
    let tokens = line.split_whitespace().collect::<Vec<_>>();
    let [name] = tokens.as_slice() else {
        let [name, shorthand] = tokens.as_slice() else {
            return None;
        };
        if !is_identifier(name) || shorthand.chars().count() != 1 {
            return None;
        }
        return previous
            .is_none_or(is_sprite_row)
            .then(|| ((*name).to_string(), shorthand.chars().next()));
    };
    if !is_identifier(name) {
        return None;
    }
    previous
        .is_none_or(is_sprite_row)
        .then(|| ((*name).to_string(), None))
}

fn parse_object_sprite(lines: &[String], start: usize) -> (Option<PsSpriteDef>, usize) {
    let mut i = start;
    while i < lines.len() && lines[i].trim().is_empty() {
        i += 1;
    }
    let Some(colors) = lines.get(i).and_then(|line| parse_ps_color_row(line)) else {
        return (None, start);
    };
    i += 1;

    let mut pattern = Vec::new();
    while i < lines.len()
        && is_sprite_row_for_palette(
            lines[i].trim(),
            colors.len(),
            pattern.first().map(|row: &String| row.chars().count()),
        )
    {
        pattern.push(lines[i].trim().to_string());
        i += 1;
    }

    (Some(PsSpriteDef { colors, pattern }), i)
}

fn parse_ps_color_row(line: &str) -> Option<Vec<String>> {
    let colors = line
        .split_whitespace()
        .map(ps_color_to_canonical)
        .collect::<Option<Vec<_>>>()?;
    (!colors.is_empty()).then_some(colors)
}

fn ps_color_to_canonical(color: &str) -> Option<String> {
    if color.starts_with('#') {
        return Some(color.to_string());
    }
    let mapped = match color.to_ascii_lowercase().as_str() {
        "transparent" => "transparent",
        "black" => "#000000",
        "white" => "#ffffff",
        "gray" | "grey" => "#808080",
        "darkgray" | "darkgrey" => "#404040",
        "lightgray" | "lightgrey" => "#c0c0c0",
        "red" => "#ff0000",
        "darkred" => "#800000",
        "lightred" => "#ff8080",
        "brown" => "#a46322",
        "darkbrown" => "#493c2b",
        "orange" => "#ffa500",
        "yellow" => "#ffff00",
        "green" => "#008000",
        "darkgreen" => "#006400",
        "lightgreen" => "#90ee90",
        "blue" => "#0000ff",
        "darkblue" => "#00008b",
        "lightblue" => "#add8e6",
        "purple" => "#800080",
        "pink" => "#ffc0cb",
        _ => return None,
    };
    Some(mapped.to_string())
}

fn is_sprite_row(line: &str) -> bool {
    !line.is_empty()
        && line
            .chars()
            .all(|ch| ch == '.' || ch.is_ascii_digit() || ch.is_ascii_alphabetic())
        && line.chars().any(|ch| ch == '.' || ch.is_ascii_digit())
}

fn is_sprite_row_for_palette(line: &str, color_count: usize, width: Option<usize>) -> bool {
    if line.is_empty() || width.is_some_and(|width| line.chars().count() != width) {
        return false;
    }
    line.chars().all(|ch| {
        ch == '.'
            || (0..color_count)
                .filter_map(crate::visual_color_token_for_index)
                .any(|token| token == ch)
    })
}

fn parse_alias_defs(lines: &[String], objects: &[PsObjectDef]) -> Vec<PsAliasDef> {
    let mut aliases = Vec::new();
    for line in lines.iter().filter(|line| !line.trim().is_empty()) {
        let Some((left, rhs)) = line.split_once('=') else {
            continue;
        };
        let name = left.trim();
        if name.chars().count() == 1 || !is_identifier(name) {
            continue;
        }
        let terms = split_ps_relation(rhs)
            .into_iter()
            .filter_map(|term| resolve_name(term, objects, &aliases))
            .collect::<Vec<_>>();
        if !terms.is_empty() {
            aliases.push(PsAliasDef {
                name: name.to_string(),
                terms,
            });
        }
    }
    aliases
}

fn parse_collision_layers(
    lines: &[String],
    objects: &[PsObjectDef],
    aliases: &[PsAliasDef],
) -> Vec<(String, Vec<String>)> {
    let mut layers = Vec::new();
    let mut index = 1usize;
    for line in lines.iter().filter(|line| !line.trim().is_empty()) {
        let layer_objects = split_ps_list(line)
            .into_iter()
            .filter_map(|token| expand_layer_term(token, objects, aliases))
            .flatten()
            .collect::<Vec<_>>();
        if layer_objects.is_empty() {
            continue;
        }
        layers.push((format!("layer_{index}"), unique_names(layer_objects)));
        index += 1;
    }
    layers
}

fn expand_layer_term(
    token: &str,
    objects: &[PsObjectDef],
    aliases: &[PsAliasDef],
) -> Option<Vec<String>> {
    if let Some(object) = resolve_object_name(token, objects) {
        return Some(vec![object]);
    }
    let alias = resolve_alias(token, aliases)?;
    Some(expand_alias_terms(alias, objects, aliases, &mut Vec::new()))
}

fn expand_alias_terms(
    alias: &PsAliasDef,
    objects: &[PsObjectDef],
    aliases: &[PsAliasDef],
    seen: &mut Vec<String>,
) -> Vec<String> {
    if seen
        .iter()
        .any(|name| name.eq_ignore_ascii_case(&alias.name))
    {
        return Vec::new();
    }
    seen.push(alias.name.clone());
    let mut expanded = Vec::new();
    for term in &alias.terms {
        if let Some(object) = resolve_object_name(term, objects) {
            expanded.push(object);
        } else if let Some(child) = resolve_alias(term, aliases) {
            expanded.extend(expand_alias_terms(child, objects, aliases, seen));
        }
    }
    seen.pop();
    unique_names(expanded)
}

fn push_groups(out: &mut Vec<String>, aliases: &[PsAliasDef]) {
    if aliases.is_empty() {
        return;
    }
    out.push("group {".to_string());
    for alias in aliases {
        out.push(format!("  {} = {}", alias.name, alias.terms.join(" ")));
    }
    out.push("}".to_string());
    out.push(String::new());
}

fn push_layers(out: &mut Vec<String>, layers: &[(String, Vec<String>)]) {
    out.push("layers {".to_string());
    for (name, objects) in layers {
        out.push(format!("  {name} = {}", objects.join(" ")));
    }
    out.push("}".to_string());
    out.push(String::new());
}

fn push_default_inputs(out: &mut Vec<String>) {
    out.push("inputs {".to_string());
    out.push("  up <- w ArrowUp".to_string());
    out.push("  down <- s ArrowDown".to_string());
    out.push("  left <- a ArrowLeft".to_string());
    out.push("  right <- d ArrowRight".to_string());
    out.push("  restart <- r".to_string());
    out.push("}".to_string());
    out.push(String::new());
}

fn push_sprites(out: &mut Vec<String>, objects: &[PsObjectDef]) {
    let sprites = objects
        .iter()
        .filter_map(|object| object.sprite.as_ref().map(|sprite| (&object.name, sprite)))
        .collect::<Vec<_>>();
    if sprites.is_empty() {
        return;
    }

    out.push("sprites {".to_string());
    for (name, sprite) in sprites {
        out.push(format!("  {name}"));
        out.push(format!("    {}", sprite.colors.join(" ")));
        for row in &sprite.pattern {
            out.push(format!("    {row}"));
        }
        out.push(String::new());
    }
    if matches!(out.last(), Some(line) if line.is_empty()) {
        out.pop();
    }
    out.push("}".to_string());
    out.push(String::new());
}

fn push_legend(
    out: &mut Vec<String>,
    lines: &[String],
    level_lines: &[String],
    objects: &[PsObjectDef],
    aliases: &[PsAliasDef],
) -> BTreeMap<char, char> {
    out.push("legend {".to_string());
    let mut has_empty = false;
    let mut defined_chars = BTreeSet::<char>::new();
    let used_chars = level_chars(level_lines);
    let char_map = ps_level_char_map(lines, &used_chars);

    for object in objects {
        let Some(ch) = object.shorthand else {
            continue;
        };
        out.push(format!("  {ch} = {}", object.name));
        defined_chars.insert(ch);
    }

    for line in lines.iter().filter(|line| !line.trim().is_empty()) {
        let Some((ch, rhs)) = parse_legend_row(line) else {
            continue;
        };
        let Some(ch) = ch.chars().next() else {
            continue;
        };
        let output_ch = char_map.get(&ch).copied().unwrap_or(ch);
        let terms = split_ps_relation(rhs)
            .into_iter()
            .filter_map(|term| resolve_name(term, objects, aliases))
            .collect::<Vec<_>>();
        if terms == ["empty"] {
            out.push(format!("  {output_ch} = empty"));
            has_empty = true;
        } else if !terms.is_empty() {
            out.push(format!("  {output_ch} = {}", terms.join(" ")));
        }
        if !terms.is_empty() {
            defined_chars.insert(output_ch);
            if output_ch.is_ascii_uppercase() {
                let lower = output_ch.to_ascii_lowercase();
                if used_chars.contains(&lower) && !defined_chars.contains(&lower) {
                    if terms == ["empty"] {
                        out.push(format!("  {lower} = empty"));
                    } else {
                        out.push(format!("  {lower} = {}", terms.join(" ")));
                    }
                    defined_chars.insert(lower);
                }
            }
        }
    }
    if !has_empty {
        let empty = choose_empty_legend_char(&defined_chars, &used_chars);
        out.push(format!("  {empty} = empty"));
    }
    out.push("}".to_string());
    out.push(String::new());
    char_map
}

fn ps_background_object(objects: &[PsObjectDef]) -> Option<String> {
    objects
        .iter()
        .find(|object| object.name.eq_ignore_ascii_case("Background"))
        .map(|object| object.name.clone())
}

fn ps_player_selector(objects: &[PsObjectDef], aliases: &[PsAliasDef]) -> String {
    resolve_name("Player", objects, aliases).unwrap_or_else(|| "Player".to_string())
}

fn choose_empty_legend_char(defined_chars: &BTreeSet<char>, used_chars: &BTreeSet<char>) -> char {
    ['.', '_', '~', '`']
        .into_iter()
        .find(|ch| !defined_chars.contains(ch) && !used_chars.contains(ch))
        .unwrap_or('_')
}

fn ps_level_char_map(lines: &[String], used_chars: &BTreeSet<char>) -> BTreeMap<char, char> {
    let mut defined_chars = BTreeSet::<char>::new();
    for line in lines.iter().filter(|line| !line.trim().is_empty()) {
        let Some((ch, _)) = parse_legend_row(line) else {
            continue;
        };
        if let Some(ch) = ch.chars().next() {
            defined_chars.insert(ch);
        }
    }

    let mut remapped = BTreeMap::new();
    let mut reserved = used_chars
        .union(&defined_chars)
        .copied()
        .collect::<BTreeSet<_>>();
    for ch in defined_chars
        .iter()
        .copied()
        .filter(|ch| is_canonical_legend_syntax_char(*ch))
    {
        let replacement = choose_ps_level_char_replacement(&reserved);
        remapped.insert(ch, replacement);
        reserved.insert(replacement);
    }
    remapped
}

fn is_canonical_legend_syntax_char(ch: char) -> bool {
    matches!(ch, '{' | '}' | '"')
}

fn choose_ps_level_char_replacement(reserved: &BTreeSet<char>) -> char {
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789;,:!?'`~@^&=<>"
        .chars()
        .find(|ch| !reserved.contains(ch) && !is_canonical_legend_syntax_char(*ch))
        .unwrap_or('§')
}

fn parse_legend_row(line: &str) -> Option<(&str, &str)> {
    let (left, right) = line.split_once('=')?;
    let ch = left.trim();
    if ch.chars().count() != 1 {
        return None;
    }
    Some((ch, right.trim()))
}

fn remap_ps_level_line(line: &str, char_map: &BTreeMap<char, char>) -> String {
    if char_map.is_empty() || is_level_message(line) {
        return line.to_string();
    }
    line.chars()
        .map(|ch| char_map.get(&ch).copied().unwrap_or(ch))
        .collect()
}

fn level_chars(lines: &[String]) -> BTreeSet<char> {
    let mut chars = BTreeSet::new();
    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() || is_parenthetical_comment(trimmed) {
            continue;
        }
        chars.extend(trimmed.chars());
    }
    chars
}

fn push_win_conditions(
    out: &mut Vec<String>,
    lines: &[String],
    objects: &[PsObjectDef],
    aliases: &[PsAliasDef],
) {
    if !lines.iter().any(|line| !line.trim().is_empty()) {
        return;
    }
    out.push("win_conditions {".to_string());
    for line in lines.iter().filter(|line| !line.trim().is_empty()) {
        out.push(format!(
            "  {}",
            canonical_condition_row(line, objects, aliases)
        ));
    }
    out.push("}".to_string());
    out.push(String::new());
}

fn canonical_condition_row(line: &str, objects: &[PsObjectDef], aliases: &[PsAliasDef]) -> String {
    let tokens = line.split_whitespace().collect::<Vec<_>>();
    if matches!(tokens.first(), Some(first) if first.eq_ignore_ascii_case("all"))
        && tokens.len() == 4
        && tokens[2].eq_ignore_ascii_case("on")
    {
        return format!(
            "all {} on {}",
            resolve_name(tokens[1], objects, aliases).unwrap_or_else(|| tokens[1].to_string()),
            resolve_name(tokens[3], objects, aliases).unwrap_or_else(|| tokens[3].to_string())
        );
    }
    if matches!(tokens.first(), Some(first) if first.eq_ignore_ascii_case("some"))
        && tokens.len() == 2
    {
        return format!(
            "some {}",
            resolve_name(tokens[1], objects, aliases).unwrap_or_else(|| tokens[1].to_string())
        );
    }
    if matches!(tokens.first(), Some(first) if first.eq_ignore_ascii_case("some"))
        && tokens.len() == 4
        && tokens[2].eq_ignore_ascii_case("on")
    {
        return format!(
            "some {} on {}",
            resolve_name(tokens[1], objects, aliases).unwrap_or_else(|| tokens[1].to_string()),
            resolve_name(tokens[3], objects, aliases).unwrap_or_else(|| tokens[3].to_string())
        );
    }
    if matches!(tokens.first(), Some(first) if first.eq_ignore_ascii_case("no"))
        && tokens.len() == 2
    {
        return format!(
            "no {}",
            resolve_name(tokens[1], objects, aliases).unwrap_or_else(|| tokens[1].to_string())
        );
    }
    line.to_string()
}

fn push_rules(
    out: &mut Vec<String>,
    lines: &[String],
    objects: &[PsObjectDef],
    aliases: &[PsAliasDef],
    run_rules_on_level_start: bool,
    background_object: Option<&str>,
    sounds: &[PsSoundDef],
) {
    let player_selector = ps_player_selector(objects, aliases);
    if run_rules_on_level_start {
        out.push("routine __ps_main once {".to_string());
        push_ps_main_rule_body(out, lines, objects, aliases, sounds, "  ");
        out.push("}".to_string());
        out.push(String::new());

        out.push("on_level_start {".to_string());
        push_ps_background_fill(out, background_object, "  ");
        out.push("  __ps_main".to_string());
        out.push("}".to_string());
        out.push(String::new());

        out.push("rules {".to_string());
        out.push(format!(
            "  input directions [ {player_selector} ] -> [ {player_selector}{{>}} ]"
        ));
        out.push("  __ps_main".to_string());
        out.push("}".to_string());
        out.push(String::new());
        return;
    }

    if background_object.is_some() {
        out.push("on_level_start {".to_string());
        push_ps_background_fill(out, background_object, "  ");
        out.push("}".to_string());
        out.push(String::new());
    }

    out.push("rules {".to_string());
    out.push(format!(
        "  input directions [ {player_selector} ] -> [ {player_selector}{{>}} ]"
    ));
    push_ps_main_rule_body(out, lines, objects, aliases, sounds, "  ");
    out.push("}".to_string());
    out.push(String::new());
}

fn push_ps_level_clear(out: &mut Vec<String>) {
    out.push("on_level_clear {".to_string());
    out.push("  wait 0.3s".to_string());
    out.push("  next_level".to_string());
    out.push("}".to_string());
    out.push(String::new());
}

fn push_ps_background_fill(out: &mut Vec<String>, background_object: Option<&str>, indent: &str) {
    let Some(background) = background_object else {
        return;
    };
    out.push(format!(
        "{indent}once_all [ no {background} ] -> [ {background} ]"
    ));
}

fn push_ps_main_rule_body(
    out: &mut Vec<String>,
    lines: &[String],
    objects: &[PsObjectDef],
    aliases: &[PsAliasDef],
    sounds: &[PsSoundDef],
    indent: &str,
) {
    push_ps_main_rule_body_steps(out, lines, objects, aliases, sounds, indent);
}

fn push_ps_main_rule_body_steps(
    out: &mut Vec<String>,
    lines: &[String],
    objects: &[PsObjectDef],
    aliases: &[PsAliasDef],
    sounds: &[PsSoundDef],
    indent: &str,
) {
    push_ps_sound_call(out, sounds, indent, "__ps_sound_mark_existing");
    push_canonical_rule_rows(
        out,
        lines
            .iter()
            .filter(|line| !line.trim().is_empty())
            .filter(|line| !is_late_rule(line))
            .map(String::as_str),
        objects,
        aliases,
        indent,
    );
    out.push(format!("{indent}move"));
    push_canonical_rule_rows(
        out,
        lines
            .iter()
            .filter(|line| !line.trim().is_empty())
            .filter(|line| is_late_rule(line))
            .map(String::as_str),
        objects,
        aliases,
        indent,
    );
    push_ps_sound_call(out, sounds, indent, "__ps_sound_emit_events");
}

fn push_canonical_rule_rows<'a>(
    out: &mut Vec<String>,
    lines: impl Iterator<Item = &'a str>,
    objects: &[PsObjectDef],
    aliases: &[PsAliasDef],
    indent: &str,
) {
    let mut group = Vec::<String>::new();
    for line in lines {
        let is_continuation = line.trim_start().starts_with('+');
        if !is_continuation {
            flush_canonical_rule_group(out, &mut group, indent);
        }
        if let Some(rule) = canonical_rule_row(line, objects, aliases) {
            group.push(rule);
        }
    }
    flush_canonical_rule_group(out, &mut group, indent);
}

fn flush_canonical_rule_group(out: &mut Vec<String>, group: &mut Vec<String>, indent: &str) {
    match group.len() {
        0 => {}
        1 => out.push(format!("{indent}{}", group[0])),
        _ => {
            out.push(format!("{indent}repeat {{"));
            for rule in group.drain(..) {
                out.push(format!("{indent}  {rule}"));
            }
            out.push(format!("{indent}}}"));
            return;
        }
    }
    group.clear();
}

fn is_late_rule(line: &str) -> bool {
    line.trim()
        .trim_start_matches('+')
        .trim_start()
        .split_whitespace()
        .next()
        .is_some_and(|token| token.eq_ignore_ascii_case("late"))
}

fn canonical_rule_row(
    line: &str,
    objects: &[PsObjectDef],
    aliases: &[PsAliasDef],
) -> Option<String> {
    let trimmed = line.trim().trim_start_matches('+').trim();
    if !trimmed.contains("->") {
        return None;
    }
    let has_again = trimmed
        .split_whitespace()
        .any(|token| token.eq_ignore_ascii_case("again"));
    let mut tokens = tokenize_ps_rule(trimmed);
    tokens.retain(|token| !matches!(token.to_ascii_lowercase().as_str(), "again" | "late"));
    if !tokens.iter().any(|token| token == "->") {
        return None;
    }
    tokens = translate_motion_qualifiers(tokens, objects, aliases);
    tokens = attach_direction_prefixes(tokens, objects, aliases);
    let tokens = tokens
        .into_iter()
        .map(|token| resolve_rule_token(&token, objects, aliases))
        .collect::<Vec<_>>();
    let tokens = expand_ps_sfx_effect_tokens(tokens);
    let mut row = tokens.join(" ");
    if has_again {
        row.push_str(" again");
    }
    Some(row)
}

fn expand_ps_sfx_effect_tokens(tokens: Vec<String>) -> Vec<String> {
    let mut expanded = Vec::new();
    for token in tokens {
        if is_ps_sfx_token(&token) {
            expanded.push("sfx".to_string());
            expanded.push(token.to_ascii_lowercase());
        } else {
            expanded.push(token);
        }
    }
    expanded
}

fn is_ps_sfx_token(token: &str) -> bool {
    let Some(seed) = token
        .strip_prefix("sfx")
        .or_else(|| token.strip_prefix("SFX"))
    else {
        return false;
    };
    !seed.is_empty() && seed.chars().all(|ch| ch.is_ascii_digit())
}

fn attach_direction_prefixes(
    tokens: Vec<String>,
    objects: &[PsObjectDef],
    aliases: &[PsAliasDef],
) -> Vec<String> {
    let mut attached = Vec::new();
    let mut i = 0usize;
    while i < tokens.len() {
        if let Some(direction) = canonical_direction_token(&tokens[i])
            && let Some(selector) = tokens
                .get(i + 1)
                .filter(|selector| resolve_name(selector, objects, aliases).is_some())
        {
            attached.push(append_scratch_to_selector(selector, direction));
            i += 2;
            continue;
        }
        attached.push(tokens[i].clone());
        i += 1;
    }
    attached
}

fn translate_motion_qualifiers(
    tokens: Vec<String>,
    objects: &[PsObjectDef],
    aliases: &[PsAliasDef],
) -> Vec<String> {
    let Some(arrow) = tokens.iter().position(|token| token == "->") else {
        return tokens;
    };
    let mut translated =
        translate_motion_qualifiers_on_side(&tokens[..arrow], true, objects, aliases);
    translated.push("->".to_string());
    translated.extend(translate_motion_qualifiers_on_side(
        &tokens[arrow + 1..],
        false,
        objects,
        aliases,
    ));
    translated
}

fn translate_motion_qualifiers_on_side(
    tokens: &[String],
    is_lhs: bool,
    objects: &[PsObjectDef],
    aliases: &[PsAliasDef],
) -> Vec<String> {
    let mut translated = Vec::new();
    let mut i = 0usize;
    while i < tokens.len() {
        let token = &tokens[i];
        let is_moving = token.eq_ignore_ascii_case("moving");
        let is_stationary = token.eq_ignore_ascii_case("stationary");
        if (is_moving || is_stationary)
            && let Some(selector) = tokens
                .get(i + 1)
                .filter(|selector| resolve_name(selector, objects, aliases).is_some())
        {
            if is_lhs {
                let scratch = if is_moving {
                    "directions"
                } else {
                    "no directions"
                };
                translated.push(append_scratch_to_selector(selector, scratch));
            } else {
                translated.push(selector.clone());
            }
            i += 2;
            continue;
        }
        translated.push(token.clone());
        i += 1;
    }
    translated
}

fn append_scratch_to_selector(selector: &str, scratch: &str) -> String {
    if let Some(stripped) = selector.strip_suffix('}') {
        format!("{stripped} {scratch}}}")
    } else {
        format!("{selector}{{{scratch}}}")
    }
}

fn ps_level_chunks(lines: &[String]) -> Vec<Vec<String>> {
    let mut chunks = Vec::new();
    let mut current_map = Vec::new();
    let mut pending_start_messages = Vec::new();
    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() || is_parenthetical_comment(trimmed) {
            if !current_map.is_empty() {
                let mut chunk = std::mem::take(&mut pending_start_messages);
                chunk.append(&mut current_map);
                chunks.push(chunk);
            }
            continue;
        }
        if let Some(message) = parse_level_message(trimmed) {
            if !current_map.is_empty() {
                let mut chunk = std::mem::take(&mut pending_start_messages);
                chunk.append(&mut current_map);
                chunks.push(chunk);
            }
            pending_start_messages.push(message);
            continue;
        }
        current_map.push(trimmed.to_string());
    }
    if !current_map.is_empty() {
        let mut chunk = std::mem::take(&mut pending_start_messages);
        chunk.append(&mut current_map);
        chunks.push(chunk);
    } else if !pending_start_messages.is_empty()
        && let Some(last) = chunks.last_mut()
    {
        last.append(&mut pending_start_messages);
    }
    chunks
}

fn is_level_message(line: &str) -> bool {
    line.split_whitespace()
        .next()
        .is_some_and(|token| token.eq_ignore_ascii_case("message"))
}

fn parse_level_message(line: &str) -> Option<String> {
    if !is_level_message(line) {
        return None;
    }
    let text = line
        .split_once(char::is_whitespace)
        .map(|(_, rest)| rest.trim())
        .unwrap_or("");
    Some(format!("message \"{}\"", escape_canonical_string(text)))
}

fn escape_canonical_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn tokenize_ps_rule(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch.is_whitespace() {
            push_rule_token(&mut tokens, &mut current);
            continue;
        }
        if ch == '-' && chars.peek() == Some(&'>') {
            push_rule_token(&mut tokens, &mut current);
            chars.next();
            tokens.push("->".to_string());
            continue;
        }
        if matches!(ch, '[' | ']' | '|')
            || (current.is_empty() && is_standalone_direction_char(ch, chars.peek().copied()))
        {
            push_rule_token(&mut tokens, &mut current);
            tokens.push(ch.to_string());
            continue;
        }
        current.push(ch);
    }
    push_rule_token(&mut tokens, &mut current);
    tokens
}

fn push_rule_token(tokens: &mut Vec<String>, current: &mut String) {
    if !current.is_empty() {
        tokens.push(std::mem::take(current));
    }
}

fn is_standalone_direction_char(ch: char, next: Option<char>) -> bool {
    matches!(ch, '>' | '<' | '^')
        || (ch == 'v'
            && next.is_none_or(|next| next.is_whitespace() || matches!(next, '[' | ']' | '|')))
}

fn resolve_rule_token(token: &str, objects: &[PsObjectDef], aliases: &[PsAliasDef]) -> String {
    if let Some(direction) = canonical_direction_token(token) {
        return direction.to_string();
    }
    if matches!(token, "[" | "]" | "|" | ">" | "<" | "^" | "v" | "->") {
        return token.to_string();
    }
    if let Some((base, scratch)) = token.split_once('{') {
        if let Some(name) = resolve_name(base, objects, aliases) {
            return format!("{name}{{{scratch}");
        }
    }
    resolve_name(token, objects, aliases).unwrap_or_else(|| token.to_string())
}

fn canonical_direction_token(token: &str) -> Option<&'static str> {
    match token.to_ascii_lowercase().as_str() {
        ">" => Some(">"),
        "<" => Some("<"),
        "^" => Some("^"),
        "v" => Some("v"),
        "up" => Some("up"),
        "down" => Some("down"),
        "left" => Some("left"),
        "right" => Some("right"),
        _ => None,
    }
}

fn push_levels(
    out: &mut Vec<String>,
    lines: &[String],
    legend_lines: &[String],
    objects: &[PsObjectDef],
    aliases: &[PsAliasDef],
) {
    out.push("levels {".to_string());
    let mut legend = Vec::new();
    let char_map = push_legend(&mut legend, legend_lines, lines, objects, aliases);
    for line in legend {
        if line.is_empty() {
            continue;
        }
        out.push(format!("  {line}"));
    }
    let chunks = ps_level_chunks(lines);
    for (index, chunk) in chunks.iter().enumerate() {
        if index > 0 || !legend_lines.is_empty() || !objects.is_empty() {
            out.push(String::new());
        }
        for line in chunk {
            out.push(format!("  {}", remap_ps_level_line(line, &char_map)));
        }
    }
    out.push("}".to_string());
}

fn push_playing_scene(
    out: &mut Vec<String>,
    title: &str,
    author: Option<&str>,
    startgame_sfx: Option<&str>,
    viewport_size: Option<PsViewportSize>,
) {
    out.push("scene title {".to_string());
    out.push("  view {".to_string());
    out.push(format!("    title \"{}\"", escape_scene_text(title)));
    if let Some(author) = author {
        out.push(format!("    subtitle \"by {}\"", escape_scene_text(author)));
    }
    out.push("    if game.has_progress_save {".to_string());
    out.push("      choice \"Continue\" -> input continue_game".to_string());
    out.push("    }".to_string());
    out.push("    choice \"New Game\" -> input new_game".to_string());
    out.push("  }".to_string());
    out.push("  inputs {".to_string());
    out.push("    continue_game <- Enter Space x".to_string());
    out.push("    new_game <- n".to_string());
    out.push("  }".to_string());
    out.push("  rules {".to_string());
    push_title_start_rule(out, "continue_game", &["goto playing"], startgame_sfx);
    push_title_start_rule(
        out,
        "new_game",
        &["clear_game_progress", "goto playing(0)"],
        startgame_sfx,
    );
    out.push("  }".to_string());
    out.push("}".to_string());
    out.push(String::new());

    out.push("scene playing {".to_string());
    out.push("  state {".to_string());
    out.push("    board = puzzle main".to_string());
    out.push("  }".to_string());
    if viewport_size.is_some() {
        out.push("  view {".to_string());
        out.push("    puzzle board".to_string());
        out.push("  }".to_string());
    } else {
        out.push("  view {".to_string());
        out.push("    row {".to_string());
        out.push(format!("      title \"{}\"", escape_scene_text(title)));
        out.push("    }".to_string());
        out.push("    puzzle board".to_string());
        out.push("  }".to_string());
    }
    out.push("  inputs {".to_string());
    out.push("    back <- Escape q".to_string());
    out.push("  }".to_string());
    out.push("  rules {".to_string());
    out.push("    step board".to_string());
    out.push("    if input == back -> goto title".to_string());
    out.push("  }".to_string());
    out.push("}".to_string());
    out.push(String::new());
}

fn push_title_start_rule(
    out: &mut Vec<String>,
    input_name: &str,
    effects: &[&str],
    startgame_sfx: Option<&str>,
) {
    if let Some(name) = startgame_sfx {
        out.push(format!("    if input == {input_name} -> {{"));
        out.push(format!("      sfx {name}"));
        for effect in effects {
            out.push(format!("      {effect}"));
        }
        out.push("    }".to_string());
    } else if let [effect] = effects {
        out.push(format!("    if input == {input_name} -> {effect}"));
    } else {
        out.push(format!("    if input == {input_name} -> {{"));
        for effect in effects {
            out.push(format!("      {effect}"));
        }
        out.push("    }".to_string());
    }
}

fn escape_scene_text(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

fn split_ps_list(line: &str) -> Vec<&str> {
    line.split(',')
        .flat_map(str::split_whitespace)
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .collect()
}

fn split_ps_relation(line: &str) -> Vec<&str> {
    line.split_whitespace()
        .map(str::trim)
        .filter(|token| {
            !token.is_empty()
                && !token.eq_ignore_ascii_case("and")
                && !token.eq_ignore_ascii_case("or")
        })
        .collect()
}

fn resolve_name(token: &str, objects: &[PsObjectDef], aliases: &[PsAliasDef]) -> Option<String> {
    resolve_object_name(token, objects).or_else(|| {
        resolve_alias(token, aliases)
            .map(|alias| alias.name.clone())
            .or_else(|| token_is_empty_alias(token).then_some("empty".to_string()))
    })
}

fn resolve_object_name(token: &str, objects: &[PsObjectDef]) -> Option<String> {
    objects
        .iter()
        .find(|object| object.name.eq_ignore_ascii_case(token))
        .map(|object| object.name.clone())
}

fn resolve_alias<'a>(token: &str, aliases: &'a [PsAliasDef]) -> Option<&'a PsAliasDef> {
    aliases
        .iter()
        .find(|alias| alias.name.eq_ignore_ascii_case(token))
}

fn token_is_empty_alias(token: &str) -> bool {
    matches!(token, "." | "_") || token.eq_ignore_ascii_case("background")
}

fn unique_names(names: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut unique = Vec::new();
    for name in names {
        if seen.insert(name.to_ascii_lowercase()) {
            unique.push(name);
        }
    }
    unique
}

fn is_parenthetical_comment(line: &str) -> bool {
    line.starts_with('(') && line.ends_with(')')
}

fn canonical_metadata_text(value: &str) -> String {
    if is_identifier(value) {
        value.to_string()
    } else {
        format!("\"{}\"", escape_scene_text(value))
    }
}

fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn is_sound_atom(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch == '_' || ch == '-' || ch == '.' || ch.is_ascii_alphanumeric())
}
