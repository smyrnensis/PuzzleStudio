use std::collections::{BTreeMap, BTreeSet};

use crate::{Diagnostic, DiagnosticReport, source::strip_line_comment};

const PS_MAIN_ROUTINE: &str = "main";
const PS_SOUND_MARK_EXISTING_ROUTINE: &str = "sound_mark_existing";
const PS_SOUND_EMIT_EVENTS_ROUTINE: &str = "sound_emit_events";
const PS_SOUND_EXISTING_MARK_PREFIX: &str = "sound_existing_";
const PS_COPY_SHAPE_PREFIX: &str = "shape_";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PsSection {
    Prelude,
    Tags,
    Objects,
    Legend,
    Mappings,
    Sounds,
    CollisionLayers,
    Rules,
    WinConditions,
    Levels,
}

#[derive(Default)]
struct PsSections {
    prelude: Vec<String>,
    tags: Vec<String>,
    objects: Vec<String>,
    legend: Vec<String>,
    mappings: Vec<String>,
    sounds: Vec<String>,
    collision_layers: Vec<String>,
    rules: Vec<String>,
    win_conditions: Vec<String>,
    levels: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PsObjectDef {
    name: String,
    aliases: Vec<String>,
    shorthand: Option<char>,
    copy_of: Option<String>,
    sprite: Option<PsSpriteDef>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PsObjectHeader {
    name: String,
    aliases: Vec<String>,
    shorthand: Option<char>,
    copy_of: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PsAliasDef {
    name: String,
    terms: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PsTagDef {
    name: String,
    values: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PsMapDef {
    name: String,
    axis: String,
    rows: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PsSpriteDef {
    colors: Vec<String>,
    pattern: Vec<String>,
    rotate_from: Option<String>,
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
    Operation { operation: PsSoundOperation },
    Event { target: String, event: PsSoundEvent },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PsSoundOperation {
    Undo,
    Restart,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PsSoundEvent {
    Create,
    Move,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct PsRuleSections {
    main: Vec<String>,
    routines: Vec<PsSubroutineDef>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PsSubroutineDef {
    name: String,
    lines: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PsLevelChunk {
    name: Option<String>,
    lines: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum PsLayerDef {
    Named {
        name: String,
        selectors: Vec<String>,
    },
    Each {
        selectors: Vec<String>,
    },
}

pub fn translate_puzzlescript_to_canonical(source: &str) -> Result<String, DiagnosticReport> {
    let sections = collect_sections(source);
    reject_unsupported_rule_modifiers(&sections.rules)?;
    let title = parse_title(&sections.prelude)?;
    let author = parse_author(&sections.prelude)?;
    let homepage = parse_homepage(&sections.prelude)?;
    let run_rules_on_level_start = parse_run_rules_on_level_start(&sections.prelude);
    let level_select = parse_level_select(&sections.prelude);
    let case_sensitive = parse_case_sensitive(&sections.prelude);
    let again_interval = parse_again_interval(&sections.prelude);
    let theme_colors = parse_theme_colors(&sections.prelude);
    let viewport_size = parse_viewport_size(&sections.prelude);
    let sounds = parse_sound_defs(&sections.sounds);
    let startgame_sfx = ps_sound_name(&sounds, "startgame");
    let uses_action_input = ps_rules_use_action_input(&sections.rules);
    let rule_sections = parse_ps_rule_sections(&sections.rules);
    reject_generated_routine_conflicts(&rule_sections, &sounds, run_rules_on_level_start)?;
    let tags = parse_tag_defs(&sections.tags);
    let maps = parse_map_defs(&sections.mappings);
    let object_defs = parse_object_defs(&sections.objects);
    let background_object = ps_background_object(&object_defs);
    let aliases = parse_alias_defs(&sections.legend, &object_defs, &tags, &maps, case_sensitive);
    let collision_layers = parse_collision_layers(
        &sections.collision_layers,
        &object_defs,
        &aliases,
        &tags,
        &maps,
        case_sensitive,
    );
    let mut out = Vec::new();
    out.push(format!("title = {}", canonical_metadata_text(&title)));
    if let Some(author) = &author {
        out.push(format!("author = {}", canonical_metadata_text(author)));
    }
    if let Some(homepage) = &homepage {
        out.push(format!("homepage = {}", canonical_metadata_text(homepage)));
    }
    if let Some(seconds) = &again_interval {
        out.push(format!("again_interval = {seconds}s"));
    }
    out.push(String::new());
    push_theme_colors(&mut out, &theme_colors);
    push_sounds(&mut out, &sounds);
    out.push("puzzle main {".to_string());
    push_tags(&mut out, &tags);
    push_maps(&mut out, &maps);
    push_viewport_size(
        &mut out,
        viewport_size,
        ps_viewport_focus(&object_defs, &aliases, &tags, &maps, case_sensitive).as_deref(),
    );
    push_layers(&mut out, &collision_layers);
    push_action_input(&mut out, uses_action_input);
    push_default_inputs(&mut out, uses_action_input);
    push_groups(&mut out, &aliases);
    push_sprites(&mut out, &object_defs);
    push_ps_model_sounds(
        &mut out,
        &sounds,
        &object_defs,
        &aliases,
        &tags,
        &maps,
        case_sensitive,
    );
    push_win_conditions(
        &mut out,
        &sections.win_conditions,
        &object_defs,
        &aliases,
        &tags,
        &maps,
        case_sensitive,
    );
    push_ps_sound_mark(&mut out, &sounds);
    push_ps_sound_routines(
        &mut out,
        &sounds,
        &object_defs,
        &aliases,
        &tags,
        &maps,
        case_sensitive,
    );
    push_rules(
        &mut out,
        &collision_layers,
        &object_defs,
        &aliases,
        &tags,
        &maps,
        case_sensitive,
        run_rules_on_level_start,
        background_object.as_deref(),
        &sounds,
        uses_action_input,
        &rule_sections,
    );
    push_ps_level_clear(&mut out);
    push_levels(
        &mut out,
        &sections.levels,
        &sections.legend,
        &object_defs,
        &aliases,
        &tags,
        &maps,
        case_sensitive,
    );
    out.push("}".to_string());
    out.push(String::new());
    push_playing_scene(
        &mut out,
        &title,
        author.as_deref(),
        startgame_sfx.as_deref(),
        viewport_size,
        level_select,
    );
    Ok(canonical_without_line_indents(&out.join("\n")))
}

fn canonical_without_line_indents(source: &str) -> String {
    source
        .lines()
        .map(str::trim_start)
        .collect::<Vec<_>>()
        .join("\n")
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
            PsSection::Tags => sections.tags.push(line),
            PsSection::Objects => sections.objects.push(line),
            PsSection::Legend => sections.legend.push(line),
            PsSection::Mappings => sections.mappings.push(line),
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
        "tags" => Some(PsSection::Tags),
        "objects" => Some(PsSection::Objects),
        "legend" => Some(PsSection::Legend),
        "mappings" => Some(PsSection::Mappings),
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

fn parse_title(prelude: &[String]) -> Result<String, DiagnosticReport> {
    if let Some(title) = prelude
        .iter()
        .map(|line| parse_ps_prelude_value(line, "title"))
        .find_map(Result::transpose)
        .transpose()?
        .filter(|title| !title.is_empty())
    {
        return Ok(title);
    }

    Ok(prelude
        .iter()
        .find_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || is_ps_prelude_flag_or_directive(trimmed) {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
        .unwrap_or_else(|| "PuzzleScript import".to_string()))
}

fn parse_author(prelude: &[String]) -> Result<Option<String>, DiagnosticReport> {
    prelude
        .iter()
        .map(|line| parse_ps_prelude_value(line, "author"))
        .find_map(Result::transpose)
        .transpose()
}

fn parse_homepage(prelude: &[String]) -> Result<Option<String>, DiagnosticReport> {
    prelude
        .iter()
        .map(|line| parse_ps_prelude_value(line, "homepage"))
        .find_map(Result::transpose)
        .transpose()
}

fn parse_ps_prelude_value(line: &str, key: &str) -> Result<Option<String>, DiagnosticReport> {
    let trimmed = line.trim();
    let Some(rest) = strip_ps_prelude_key(trimmed, key) else {
        return Ok(None);
    };
    let rest = rest.trim_start();
    if rest.is_empty() {
        return Ok(None);
    }
    if rest.starts_with('=') {
        return Err(DiagnosticReport::from_diagnostic(Diagnostic::error(
            format!("PuzzleScript {key} metadata must use `{key} <text>`, not `{key} = <text>`"),
        )));
    }
    let value = rest
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .unwrap_or(rest);
    Ok(Some(value.to_string()))
}

fn strip_ps_prelude_key<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let rest = line.strip_prefix(key)?;
    rest.chars()
        .next()
        .is_some_and(char::is_whitespace)
        .then_some(rest)
}

fn is_ps_prelude_flag_or_directive(line: &str) -> bool {
    let Some(first) = line.split_whitespace().next() else {
        return false;
    };
    matches!(
        first.to_ascii_lowercase().as_str(),
        "case_sensitive"
            | "run_rules_on_level_start"
            | "level_select"
            | "background_color"
            | "background"
            | "text_color"
            | "again_interval"
            | "key_repeat_interval"
            | "sprite_size"
            | "noaction"
            | "flickscreen"
            | "zoomscreen"
            | "color_palette"
    )
}

fn parse_run_rules_on_level_start(prelude: &[String]) -> bool {
    prelude
        .iter()
        .any(|line| line.trim().eq_ignore_ascii_case("run_rules_on_level_start"))
}

fn parse_level_select(prelude: &[String]) -> bool {
    prelude
        .iter()
        .any(|line| line.trim().eq_ignore_ascii_case("level_select"))
}

fn parse_case_sensitive(prelude: &[String]) -> bool {
    prelude
        .iter()
        .any(|line| line.trim().eq_ignore_ascii_case("case_sensitive"))
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
        let [name, value] = tokens.as_slice() else {
            continue;
        };
        let name = match name.to_ascii_lowercase().as_str() {
            "background" | "background_color" => "background_color",
            "text_color" => "text_color",
            _ => continue,
        };
        if let Some(color) = ps_color_to_canonical(value) {
            colors.push((name.to_string(), color));
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

fn ps_viewport_focus(
    objects: &[PsObjectDef],
    aliases: &[PsAliasDef],
    tags: &[PsTagDef],
    maps: &[PsMapDef],
    case_sensitive: bool,
) -> Option<String> {
    resolve_name("player", objects, aliases, tags, maps, case_sensitive)
}

fn push_theme_colors(out: &mut Vec<String>, colors: &[(String, String)]) {
    if colors.is_empty() {
        out.push("theme = \"puzzlescript\"".to_string());
        out.push(String::new());
        return;
    }
    out.push("theme {".to_string());
    out.push("  preset = \"puzzlescript\"".to_string());
    for (name, value) in colors {
        out.push(format!("  {name} = {value}"));
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
        [operation, seed] if is_sound_atom(seed) && parse_sound_operation(operation).is_some() => {
            let operation = parse_sound_operation(operation)?;
            Some(PsSoundDef {
                name: ps_operation_sound_name(operation).to_string(),
                seed: (*seed).to_string(),
                trigger: PsSoundTrigger::Operation { operation },
            })
        }
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

fn parse_sound_operation(token: &str) -> Option<PsSoundOperation> {
    match token.to_ascii_lowercase().as_str() {
        "undo" => Some(PsSoundOperation::Undo),
        "restart" => Some(PsSoundOperation::Restart),
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

fn ps_operation_sound_name(operation: PsSoundOperation) -> &'static str {
    match operation {
        PsSoundOperation::Undo => "undo",
        PsSoundOperation::Restart => "restart",
    }
}

fn push_sounds(out: &mut Vec<String>, sounds: &[PsSoundDef]) {
    if sounds.is_empty() {
        return;
    }
    out.push("sounds {".to_string());
    for sound in sounds {
        out.push(format!(
            "  sfx {} {{ seed = {}; type = puzzlescript }}",
            sound.name, sound.seed
        ));
    }
    out.push("}".to_string());
    out.push(String::new());
}

fn parse_tag_defs(lines: &[String]) -> Vec<PsTagDef> {
    let mut tags = Vec::new();
    for line in lines.iter().filter(|line| !line.trim().is_empty()) {
        let Some((name, rhs)) = line.split_once('=') else {
            continue;
        };
        let name = name.trim();
        if !is_identifier(name) {
            continue;
        }
        let values = rhs
            .split_whitespace()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>();
        if values.is_empty() || tags.iter().any(|existing: &PsTagDef| existing.name == name) {
            continue;
        }
        tags.push(PsTagDef {
            name: name.to_string(),
            values,
        });
    }
    tags
}

fn parse_map_defs(lines: &[String]) -> Vec<PsMapDef> {
    let mut maps = Vec::new();
    let mut i = 0usize;
    while i < lines.len() {
        let line = lines[i].trim();
        if line.is_empty() {
            i += 1;
            continue;
        }
        let Some((axis, name)) = line.split_once("=>") else {
            i += 1;
            continue;
        };
        let axis = axis.trim();
        let name = name.trim();
        if !is_identifier(axis) || !is_identifier(name) {
            i += 1;
            continue;
        }
        let Some(row) = lines.get(i + 1).map(|line| line.trim()) else {
            break;
        };
        let Some((from, to)) = row.split_once("->") else {
            i += 1;
            continue;
        };
        let from = from.split_whitespace().collect::<Vec<_>>();
        let to = to.split_whitespace().collect::<Vec<_>>();
        if from.len() != to.len() || from.is_empty() {
            i += 2;
            continue;
        }
        maps.push(PsMapDef {
            name: name.to_string(),
            axis: axis.to_string(),
            rows: from
                .into_iter()
                .zip(to)
                .map(|(from, to)| (from.to_string(), to.to_string()))
                .collect(),
        });
        i += 2;
    }
    maps
}

fn push_tags(out: &mut Vec<String>, tags: &[PsTagDef]) {
    if tags.is_empty() {
        return;
    }
    out.push("tags {".to_string());
    for tag in tags {
        out.push(format!("  {} = {}", tag.name, tag.values.join(" ")));
    }
    out.push("}".to_string());
    out.push(String::new());
}

fn push_maps(out: &mut Vec<String>, maps: &[PsMapDef]) {
    for map in maps {
        out.push(format!("map {} {} {{", map.name, map.axis));
        for (from, to) in &map.rows {
            out.push(format!("  {from} -> {to}"));
        }
        out.push("}".to_string());
        out.push(String::new());
    }
}

fn push_ps_model_sounds(
    out: &mut Vec<String>,
    sounds: &[PsSoundDef],
    objects: &[PsObjectDef],
    aliases: &[PsAliasDef],
    tags: &[PsTagDef],
    maps: &[PsMapDef],
    case_sensitive: bool,
) {
    if !has_model_sounds(sounds) {
        return;
    }
    out.push("sounds {".to_string());
    for sound in sounds {
        match &sound.trigger {
            PsSoundTrigger::Event {
                target,
                event: PsSoundEvent::Move,
            } => {
                let target =
                    ps_sound_target_selector(target, objects, aliases, tags, maps, case_sensitive);
                out.push(format!("  move {target} -> sfx {}", sound.name));
            }
            PsSoundTrigger::Operation { operation } => {
                let operation = ps_operation_sound_name(*operation);
                out.push(format!("  {operation} -> sfx {}", sound.name));
            }
            PsSoundTrigger::Named | PsSoundTrigger::Event { .. } => {}
        }
    }
    out.push("}".to_string());
    out.push(String::new());
}

fn push_ps_sound_mark(out: &mut Vec<String>, sounds: &[PsSoundDef]) {
    if !has_event_sounds(sounds) {
        return;
    }
    out.push("marks {".to_string());
    let mut emitted = Vec::new();
    for sound in sounds {
        let PsSoundTrigger::Event { target, event } = &sound.trigger else {
            continue;
        };
        let key = ps_sound_mark_key(target);
        match event {
            PsSoundEvent::Create => push_unique_mark(
                out,
                &mut emitted,
                format!("{PS_SOUND_EXISTING_MARK_PREFIX}{key}"),
            ),
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
    tags: &[PsTagDef],
    maps: &[PsMapDef],
    case_sensitive: bool,
) {
    if !has_event_sounds(sounds) {
        return;
    }

    out.push(format!("routine {PS_SOUND_MARK_EXISTING_ROUTINE} once {{"));
    for sound in sounds {
        let PsSoundTrigger::Event {
            target,
            event: PsSoundEvent::Create,
        } = &sound.trigger
        else {
            continue;
        };
        let target = ps_sound_target_selector(target, objects, aliases, tags, maps, case_sensitive);
        out.push(format!(
            "  once_all [ {target} ] -> [ {target}{{{PS_SOUND_EXISTING_MARK_PREFIX}{}}} ]",
            ps_sound_mark_key(&target)
        ));
    }
    out.push("}".to_string());
    out.push(String::new());

    out.push(format!("routine {PS_SOUND_EMIT_EVENTS_ROUTINE} once {{"));
    for sound in sounds {
        let PsSoundTrigger::Event { target, event } = &sound.trigger else {
            continue;
        };
        let target = ps_sound_target_selector(target, objects, aliases, tags, maps, case_sensitive);
        let key = ps_sound_mark_key(&target);
        match event {
            PsSoundEvent::Create => out.push(format!(
                "  once [ {target}{{no {PS_SOUND_EXISTING_MARK_PREFIX}{key}}} ] -> sfx {}",
                sound.name
            )),
            PsSoundEvent::Move => {}
        }
    }
    out.push("}".to_string());
    out.push(String::new());
}

fn push_unique_mark(out: &mut Vec<String>, emitted: &mut Vec<String>, name: String) {
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

fn has_model_sounds(sounds: &[PsSoundDef]) -> bool {
    sounds.iter().any(|sound| {
        matches!(
            sound.trigger,
            PsSoundTrigger::Operation { .. }
                | PsSoundTrigger::Event {
                    event: PsSoundEvent::Move,
                    ..
                }
        )
    })
}

fn ps_sound_mark_key(target: &str) -> String {
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
    tags: &[PsTagDef],
    maps: &[PsMapDef],
    case_sensitive: bool,
) -> String {
    resolve_name(target, objects, aliases, tags, maps, case_sensitive)
        .unwrap_or_else(|| target.to_string())
}

fn ps_sound_name(sounds: &[PsSoundDef], name: &str) -> Option<String> {
    sounds
        .iter()
        .find(|sound| sound.name.eq_ignore_ascii_case(name))
        .map(|sound| sound.name.clone())
}

fn parse_object_defs(lines: &[String]) -> Vec<PsObjectDef> {
    let mut objects = Vec::<PsObjectDef>::new();
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

        if let Some(header) = parse_object_header(trimmed, previous_meaningful.as_deref())
            && !objects
                .iter()
                .any(|existing: &PsObjectDef| existing.name == header.name)
        {
            let (sprite, next_i) = parse_object_sprite(lines, i + 1);
            objects.push(PsObjectDef {
                name: header.name,
                aliases: header.aliases,
                shorthand: header.shorthand,
                copy_of: header.copy_of,
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
    resolve_copy_sprites(&mut objects);
    objects
}

fn parse_object_header(line: &str, previous: Option<&str>) -> Option<PsObjectHeader> {
    if !previous.is_none_or(is_sprite_row) {
        return None;
    }
    let mut tokens = line.split_whitespace().collect::<Vec<_>>();
    let name = tokens.first().copied()?;
    if !is_ps_object_spec(name) {
        return None;
    }
    tokens.remove(0);

    let mut copy_of = None;
    tokens.retain(|token| {
        if let Some(target) = token.strip_prefix("copy:")
            && is_ps_object_spec(target)
        {
            copy_of = Some(target.to_string());
            return false;
        }
        true
    });

    let shorthand = tokens
        .last()
        .filter(|token| is_ps_object_shorthand(token))
        .and_then(|token| token.chars().next());
    if shorthand.is_some() {
        tokens.pop();
    }
    if !tokens.iter().all(|alias| is_identifier(alias)) {
        return None;
    }
    Some(PsObjectHeader {
        name: name.to_string(),
        aliases: tokens.into_iter().map(str::to_string).collect(),
        shorthand,
        copy_of,
    })
}

fn is_ps_object_shorthand(token: &str) -> bool {
    token.chars().count() == 1 && !token.chars().all(char::is_whitespace)
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

    let rotate_from = lines
        .get(i)
        .and_then(|line| parse_ps_rotation_directive(line.trim()));
    if rotate_from.is_some() {
        i += 1;
    }

    (
        Some(PsSpriteDef {
            colors,
            pattern,
            rotate_from,
        }),
        i,
    )
}

fn parse_ps_rotation_directive(line: &str) -> Option<String> {
    let mut parts = line.split(':');
    let command = parts.next()?;
    if !command.eq_ignore_ascii_case("rot") {
        return None;
    }
    let from = parts.next()?.trim();
    let axis = parts.next()?.trim();
    (parts.next().is_none() && !from.is_empty() && !axis.is_empty()).then(|| from.to_string())
}

fn resolve_copy_sprites(objects: &mut [PsObjectDef]) {
    for index in 0..objects.len() {
        let Some(copy_of) = objects[index].copy_of.clone() else {
            continue;
        };
        let Some(source) = objects
            .iter()
            .find(|object| object.name.eq_ignore_ascii_case(&copy_of))
            .and_then(|object| object.sprite.clone())
        else {
            continue;
        };
        match &mut objects[index].sprite {
            Some(sprite) if sprite.pattern.is_empty() => {
                sprite.pattern = source.pattern;
                if sprite.rotate_from.is_none() {
                    sprite.rotate_from = source.rotate_from;
                }
            }
            Some(_) => {}
            None => {
                objects[index].sprite = Some(source);
            }
        }
    }
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

fn parse_alias_defs(
    lines: &[String],
    objects: &[PsObjectDef],
    tags: &[PsTagDef],
    maps: &[PsMapDef],
    case_sensitive: bool,
) -> Vec<PsAliasDef> {
    let mut aliases = Vec::new();
    for object in objects {
        for alias in &object.aliases {
            push_alias_def(
                &mut aliases,
                PsAliasDef {
                    name: alias.clone(),
                    terms: vec![object.name.clone()],
                },
            );
        }
    }
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
            .filter_map(|term| resolve_name(term, objects, &aliases, tags, maps, case_sensitive))
            .collect::<Vec<_>>();
        if !terms.is_empty() {
            push_alias_def(
                &mut aliases,
                PsAliasDef {
                    name: name.to_string(),
                    terms,
                },
            );
        }
    }
    aliases
}

fn push_alias_def(aliases: &mut Vec<PsAliasDef>, alias: PsAliasDef) {
    if aliases
        .iter()
        .any(|existing| existing.name.eq_ignore_ascii_case(&alias.name))
    {
        return;
    }
    aliases.push(alias);
}

fn parse_collision_layers(
    lines: &[String],
    objects: &[PsObjectDef],
    aliases: &[PsAliasDef],
    tags: &[PsTagDef],
    maps: &[PsMapDef],
    case_sensitive: bool,
) -> Vec<PsLayerDef> {
    let mut layers = Vec::new();
    let mut index = 1usize;
    for line in lines.iter().filter(|line| !line.trim().is_empty()) {
        if let Some(each) = parse_ps_next_each_layer(line, tags) {
            layers.push(PsLayerDef::Each {
                selectors: vec![each],
            });
            continue;
        }
        let layer_objects = split_ps_list(line)
            .into_iter()
            .filter_map(|token| {
                expand_layer_term(token, objects, aliases, tags, maps, case_sensitive)
            })
            .flatten()
            .collect::<Vec<_>>();
        if layer_objects.is_empty() {
            continue;
        }
        layers.push(PsLayerDef::Named {
            name: format!("layer{index}"),
            selectors: unique_names(layer_objects),
        });
        index += 1;
    }
    layers
}

fn parse_ps_next_each_layer(line: &str, tags: &[PsTagDef]) -> Option<String> {
    let tokens = line.split_whitespace().collect::<Vec<_>>();
    let [axis, "->", selector] = tokens.as_slice() else {
        return None;
    };
    ps_tag_values(axis, tags)?;
    let (_, selector_axis) = selector.rsplit_once(':')?;
    (selector_axis == *axis).then(|| (*selector).to_string())
}

fn expand_layer_term(
    token: &str,
    objects: &[PsObjectDef],
    aliases: &[PsAliasDef],
    tags: &[PsTagDef],
    maps: &[PsMapDef],
    case_sensitive: bool,
) -> Option<Vec<String>> {
    if let Some(object) = resolve_object_name(token, objects, case_sensitive) {
        return Some(vec![object]);
    }
    if is_ps_tag_selector(token, objects, tags, maps, case_sensitive) {
        return Some(vec![token.to_string()]);
    }
    let alias = resolve_alias(token, aliases, case_sensitive)?;
    Some(expand_alias_terms(
        alias,
        objects,
        aliases,
        tags,
        maps,
        case_sensitive,
        &mut Vec::new(),
    ))
}

fn expand_alias_terms(
    alias: &PsAliasDef,
    objects: &[PsObjectDef],
    aliases: &[PsAliasDef],
    tags: &[PsTagDef],
    maps: &[PsMapDef],
    case_sensitive: bool,
    seen: &mut Vec<String>,
) -> Vec<String> {
    if seen
        .iter()
        .any(|name| ps_name_eq(name, &alias.name, case_sensitive))
    {
        return Vec::new();
    }
    seen.push(alias.name.clone());
    let mut expanded = Vec::new();
    for term in &alias.terms {
        if let Some(object) = resolve_object_name(term, objects, case_sensitive) {
            expanded.push(object);
        } else if is_ps_tag_selector(term, objects, tags, maps, case_sensitive) {
            expanded.push(term.clone());
        } else if let Some(child) = resolve_alias(term, aliases, case_sensitive) {
            expanded.extend(expand_alias_terms(
                child,
                objects,
                aliases,
                tags,
                maps,
                case_sensitive,
                seen,
            ));
        }
    }
    seen.pop();
    unique_names(expanded)
}

fn push_groups(out: &mut Vec<String>, aliases: &[PsAliasDef]) {
    if aliases.is_empty() {
        return;
    }
    out.push("groups {".to_string());
    for alias in aliases {
        out.push(format!("  {} = {}", alias.name, alias.terms.join(" ")));
    }
    out.push("}".to_string());
    out.push(String::new());
}

fn ps_move_layer_names(layers: &[PsLayerDef]) -> Vec<String> {
    layers
        .iter()
        .flat_map(|layer| match layer {
            PsLayerDef::Named { name, selectors } if !selectors.is_empty() => {
                vec![name.clone()]
            }
            PsLayerDef::Each { selectors } => selectors.clone(),
            PsLayerDef::Named { .. } => Vec::new(),
        })
        .collect()
}

fn push_layers(out: &mut Vec<String>, layers: &[PsLayerDef]) {
    out.push("layers {".to_string());
    for layer in layers {
        match layer {
            PsLayerDef::Named { name, selectors } => {
                out.push(format!("  {name} = {}", selectors.join(" ")));
            }
            PsLayerDef::Each { selectors } => {
                out.push(format!("  each {}", selectors.join(" ")));
            }
        }
    }
    out.push("}".to_string());
    out.push(String::new());
}

fn push_action_input(out: &mut Vec<String>, uses_action_input: bool) {
    if !uses_action_input {
        return;
    }
    out.push("input action".to_string());
    out.push(String::new());
}

fn push_default_inputs(out: &mut Vec<String>, uses_action_input: bool) {
    if !uses_action_input {
        return;
    }
    out.push("keys {".to_string());
    out.push("  x Space Enter c -> action".to_string());
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
    let mut shape_sources = ps_copy_shape_sources(objects);
    for (name, sprite) in &sprites {
        if sprite.rotate_from.is_some() {
            shape_sources.insert((*name).clone());
        }
    }

    out.push("sprites {".to_string());
    if !shape_sources.is_empty() {
        out.push("  shapes {".to_string());
        for source in &shape_sources {
            let Some(sprite) = objects
                .iter()
                .find(|object| &object.name == source)
                .and_then(|object| object.sprite.as_ref())
            else {
                continue;
            };
            let shape_name = ps_copy_shape_name(source);
            let header = if let Some(from) = &sprite.rotate_from {
                let table_name = ps_copy_shape_table_name(source);
                format!("    {table_name} rotate from {from} {{")
            } else {
                format!("    {shape_name} {{")
            };
            out.push(header);
            for row in &sprite.pattern {
                out.push(format!("      {row}"));
            }
            out.push("    }".to_string());
        }
        out.push("  }".to_string());
        out.push(String::new());
    }
    for (name, sprite) in sprites {
        let copy_shape = ps_copy_shape_for_object(name, objects, &shape_sources);
        out.push("  sprite {".to_string());
        out.push(format!("    selector = {name}"));
        out.push(format!("    colors = {}", sprite.colors.join(" ")));
        if let Some(shape) = copy_shape {
            out.push(format!(
                "    shape = {}",
                ps_copy_shape_ref_name(&shape, objects)
            ));
            out.push("  }".to_string());
            out.push(String::new());
            continue;
        }
        if !sprite.pattern.is_empty() {
            out.push("    shape =".to_string());
            for row in &sprite.pattern {
                out.push(format!("    {row}"));
            }
        }
        out.push("  }".to_string());
        out.push(String::new());
    }
    if matches!(out.last(), Some(line) if line.is_empty()) {
        out.pop();
    }
    out.push("}".to_string());
    out.push(String::new());
}

fn ps_copy_shape_sources(objects: &[PsObjectDef]) -> BTreeSet<String> {
    let mut sources = BTreeSet::new();
    for object in objects {
        let Some(copy_of) = &object.copy_of else {
            continue;
        };
        if objects
            .iter()
            .any(|source| source.name.eq_ignore_ascii_case(copy_of) && source.sprite.is_some())
        {
            sources.insert(copy_of.clone());
        }
    }
    sources
}

fn ps_copy_shape_for_object(
    name: &str,
    objects: &[PsObjectDef],
    shape_sources: &BTreeSet<String>,
) -> Option<String> {
    if shape_sources.contains(name) {
        return Some(name.to_string());
    }
    objects
        .iter()
        .find(|object| object.name == name)
        .and_then(|object| object.copy_of.as_ref())
        .filter(|copy_of| shape_sources.contains(*copy_of))
        .cloned()
}

fn ps_copy_shape_name(source: &str) -> String {
    let mut name = String::from(PS_COPY_SHAPE_PREFIX);
    for ch in source.chars() {
        if ch == '_' || ch.is_ascii_alphanumeric() {
            name.push(ch);
        } else {
            name.push('_');
        }
    }
    name
}

fn ps_copy_shape_table_name(source: &str) -> String {
    let mut name = String::from(PS_COPY_SHAPE_PREFIX);
    for ch in source.chars() {
        if ch == ':' || ch == '_' || ch.is_ascii_alphanumeric() {
            name.push(ch);
        } else {
            name.push('_');
        }
    }
    name
}

fn ps_copy_shape_ref_name(source: &str, objects: &[PsObjectDef]) -> String {
    let rotate_from = objects
        .iter()
        .find(|object| object.name == source)
        .and_then(|object| object.sprite.as_ref())
        .and_then(|sprite| sprite.rotate_from.as_ref());
    if rotate_from.is_some() {
        ps_copy_shape_table_name(source)
    } else {
        ps_copy_shape_name(source)
    }
}

fn push_legend(
    out: &mut Vec<String>,
    lines: &[String],
    level_lines: &[String],
    objects: &[PsObjectDef],
    aliases: &[PsAliasDef],
    tags: &[PsTagDef],
    maps: &[PsMapDef],
    case_sensitive: bool,
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
            .filter_map(|term| resolve_name(term, objects, aliases, tags, maps, case_sensitive))
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

fn ps_player_selector(
    objects: &[PsObjectDef],
    aliases: &[PsAliasDef],
    tags: &[PsTagDef],
    maps: &[PsMapDef],
    case_sensitive: bool,
) -> String {
    resolve_name("player", objects, aliases, tags, maps, case_sensitive)
        .or_else(|| resolve_name("Player", objects, aliases, tags, maps, case_sensitive))
        .unwrap_or_else(|| "Player".to_string())
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
    tags: &[PsTagDef],
    maps: &[PsMapDef],
    case_sensitive: bool,
) {
    if !lines.iter().any(|line| !line.trim().is_empty()) {
        return;
    }
    out.push("win_conditions {".to_string());
    for line in lines.iter().filter(|line| !line.trim().is_empty()) {
        let line = line
            .split_once('(')
            .map_or(line.as_str(), |(before, _)| before)
            .trim();
        if line.is_empty() {
            continue;
        }
        out.push(format!(
            "  {}",
            canonical_condition_row(line, objects, aliases, tags, maps, case_sensitive)
        ));
    }
    out.push("}".to_string());
    out.push(String::new());
}

fn canonical_condition_row(
    line: &str,
    objects: &[PsObjectDef],
    aliases: &[PsAliasDef],
    tags: &[PsTagDef],
    maps: &[PsMapDef],
    case_sensitive: bool,
) -> String {
    let tokens = line.split_whitespace().collect::<Vec<_>>();
    if matches!(tokens.first(), Some(first) if first.eq_ignore_ascii_case("all"))
        && tokens.len() == 4
        && tokens[2].eq_ignore_ascii_case("on")
    {
        let subject = resolve_name(tokens[1], objects, aliases, tags, maps, case_sensitive)
            .unwrap_or_else(|| tokens[1].to_string());
        let target = resolve_name(tokens[3], objects, aliases, tags, maps, case_sensitive)
            .unwrap_or_else(|| tokens[3].to_string());
        return format!("no [ {subject} no {target} ]");
    }
    if matches!(tokens.first(), Some(first) if first.eq_ignore_ascii_case("some"))
        && tokens.len() == 2
    {
        return format!(
            "some {}",
            resolve_name(tokens[1], objects, aliases, tags, maps, case_sensitive)
                .unwrap_or_else(|| tokens[1].to_string())
        );
    }
    if matches!(tokens.first(), Some(first) if first.eq_ignore_ascii_case("some"))
        && tokens.len() == 4
        && tokens[2].eq_ignore_ascii_case("on")
    {
        return format!(
            "some {} on {}",
            resolve_name(tokens[1], objects, aliases, tags, maps, case_sensitive)
                .unwrap_or_else(|| tokens[1].to_string()),
            resolve_name(tokens[3], objects, aliases, tags, maps, case_sensitive)
                .unwrap_or_else(|| tokens[3].to_string())
        );
    }
    if matches!(tokens.first(), Some(first) if first.eq_ignore_ascii_case("no"))
        && tokens.len() == 2
    {
        return format!(
            "no {}",
            resolve_name(tokens[1], objects, aliases, tags, maps, case_sensitive)
                .unwrap_or_else(|| tokens[1].to_string())
        );
    }
    line.to_string()
}

fn push_rules(
    out: &mut Vec<String>,
    collision_layers: &[PsLayerDef],
    objects: &[PsObjectDef],
    aliases: &[PsAliasDef],
    tags: &[PsTagDef],
    maps: &[PsMapDef],
    case_sensitive: bool,
    run_rules_on_level_start: bool,
    background_object: Option<&str>,
    sounds: &[PsSoundDef],
    uses_action_input: bool,
    rule_sections: &PsRuleSections,
) {
    let player_selector = ps_player_selector(objects, aliases, tags, maps, case_sensitive);
    push_ps_move_routine(out, collision_layers);
    push_ps_subroutines(
        out,
        &rule_sections.routines,
        objects,
        aliases,
        tags,
        maps,
        case_sensitive,
    );
    if run_rules_on_level_start {
        out.push(format!("routine {PS_MAIN_ROUTINE} once {{"));
        push_ps_main_rule_body(
            out,
            &rule_sections.main,
            objects,
            aliases,
            tags,
            maps,
            case_sensitive,
            sounds,
            "  ",
        );
        out.push("}".to_string());
        out.push(String::new());

        out.push("on_level_start {".to_string());
        push_ps_background_fill(out, background_object, "  ");
        out.push(format!("  {PS_MAIN_ROUTINE}"));
        out.push("}".to_string());
        out.push(String::new());

        out.push("rules {".to_string());
        out.push(format!(
            "  input directions [ {player_selector} ] -> [ > {player_selector} ]"
        ));
        push_ps_action_bridge(out, uses_action_input, &player_selector, "  ");
        out.push(format!("  {PS_MAIN_ROUTINE}"));
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
        "  input directions [ {player_selector} ] -> [ > {player_selector} ]"
    ));
    push_ps_action_bridge(out, uses_action_input, &player_selector, "  ");
    push_ps_main_rule_body(
        out,
        &rule_sections.main,
        objects,
        aliases,
        tags,
        maps,
        case_sensitive,
        sounds,
        "  ",
    );
    out.push("}".to_string());
    out.push(String::new());
}

fn push_ps_move_routine(out: &mut Vec<String>, collision_layers: &[PsLayerDef]) {
    let move_layers = ps_move_layer_names(collision_layers);
    if move_layers.is_empty() {
        return;
    }

    out.push("routine move {".to_string());
    out.push("  repeat {".to_string());
    out.push(format!("    for l in {} {{", move_layers.join(" ")));
    out.push("      once_all [ > l | | < l ] -> [ l | {__move_collision} | l ]".to_string());
    out.push("      once_all [ > l | ; | ^ l ] -> [ l | {__move_collision} ; | l ]".to_string());
    out.push("      [ > l | no l no {__move_collision} ] -> [ | l{no directions} ]".to_string());
    out.push("      once_all [ > l ] -> [ l ]".to_string());
    out.push("    }".to_string());
    out.push("    once_all [ {__move_collision} ] -> [ ]".to_string());
    out.push("  }".to_string());
    out.push("}".to_string());
    out.push(String::new());
}

fn parse_ps_rule_sections(lines: &[String]) -> PsRuleSections {
    let mut sections = PsRuleSections::default();
    let mut current_routine = None::<PsSubroutineDef>;
    for line in lines {
        let tokens = line.split_whitespace().collect::<Vec<_>>();
        if tokens
            .first()
            .is_some_and(|token| token.eq_ignore_ascii_case("subroutine"))
            && tokens.len() >= 2
            && is_identifier(tokens[1])
        {
            if let Some(routine) = current_routine.take() {
                sections.routines.push(routine);
            }
            current_routine = Some(PsSubroutineDef {
                name: tokens[1].to_string(),
                lines: Vec::new(),
            });
            continue;
        }
        if let Some(routine) = &mut current_routine {
            routine.lines.push(line.clone());
        } else {
            sections.main.push(line.clone());
        }
    }
    if let Some(routine) = current_routine {
        sections.routines.push(routine);
    }
    sections
}

fn reject_generated_routine_conflicts(
    rule_sections: &PsRuleSections,
    sounds: &[PsSoundDef],
    run_rules_on_level_start: bool,
) -> Result<(), DiagnosticReport> {
    let mut generated = BTreeSet::new();
    if run_rules_on_level_start {
        generated.insert(PS_MAIN_ROUTINE);
    }
    if has_event_sounds(sounds) {
        generated.insert(PS_SOUND_MARK_EXISTING_ROUTINE);
        generated.insert(PS_SOUND_EMIT_EVENTS_ROUTINE);
    }
    if generated.is_empty() {
        return Ok(());
    }
    for routine in &rule_sections.routines {
        if generated.contains(routine.name.as_str()) {
            return Err(DiagnosticReport::from_diagnostic(Diagnostic::error(
                format!(
                    "PuzzleScript subroutine `{}` conflicts with importer-generated routine `{}`",
                    routine.name, routine.name
                ),
            )));
        }
    }
    Ok(())
}

fn push_ps_subroutines(
    out: &mut Vec<String>,
    routines: &[PsSubroutineDef],
    objects: &[PsObjectDef],
    aliases: &[PsAliasDef],
    tags: &[PsTagDef],
    maps: &[PsMapDef],
    case_sensitive: bool,
) {
    for routine in routines {
        out.push(format!("routine {} once {{", routine.name));
        push_canonical_rule_rows(
            out,
            routine
                .lines
                .iter()
                .filter(|line| !line.trim().is_empty())
                .map(String::as_str),
            objects,
            aliases,
            tags,
            maps,
            case_sensitive,
            "  ",
        );
        out.push("}".to_string());
        out.push(String::new());
    }
}

fn push_ps_action_bridge(
    out: &mut Vec<String>,
    uses_action_input: bool,
    player_selector: &str,
    indent: &str,
) {
    if uses_action_input {
        out.push(format!("{indent}if input == action {{"));
        out.push(format!(
            "{indent}  once_all [ {player_selector} ] -> [ {player_selector}{{__action}} ]"
        ));
        out.push(format!("{indent}}}"));
    }
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
    tags: &[PsTagDef],
    maps: &[PsMapDef],
    case_sensitive: bool,
    sounds: &[PsSoundDef],
    indent: &str,
) {
    push_ps_main_rule_body_steps(
        out,
        lines,
        objects,
        aliases,
        tags,
        maps,
        case_sensitive,
        sounds,
        indent,
    );
}

fn push_ps_main_rule_body_steps(
    out: &mut Vec<String>,
    lines: &[String],
    objects: &[PsObjectDef],
    aliases: &[PsAliasDef],
    tags: &[PsTagDef],
    maps: &[PsMapDef],
    case_sensitive: bool,
    sounds: &[PsSoundDef],
    indent: &str,
) {
    push_ps_sound_call(out, sounds, indent, PS_SOUND_MARK_EXISTING_ROUTINE);
    push_canonical_rule_rows(
        out,
        lines
            .iter()
            .filter(|line| !line.trim().is_empty())
            .filter(|line| !is_late_rule(line))
            .map(String::as_str),
        objects,
        aliases,
        tags,
        maps,
        case_sensitive,
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
        tags,
        maps,
        case_sensitive,
        indent,
    );
    push_ps_sound_call(out, sounds, indent, PS_SOUND_EMIT_EVENTS_ROUTINE);
}

fn push_canonical_rule_rows<'a>(
    out: &mut Vec<String>,
    lines: impl Iterator<Item = &'a str>,
    objects: &[PsObjectDef],
    aliases: &[PsAliasDef],
    tags: &[PsTagDef],
    maps: &[PsMapDef],
    case_sensitive: bool,
    indent: &str,
) {
    let mut group = Vec::<String>::new();
    for line in lines {
        let is_continuation = line.trim_start().starts_with('+');
        if !is_continuation {
            flush_canonical_rule_group(out, &mut group, indent);
        }
        if let Some(rules) = canonical_rule_rows(line, objects, aliases, tags, maps, case_sensitive)
        {
            group.extend(rules);
        }
    }
    flush_canonical_rule_group(out, &mut group, indent);
}

fn flush_canonical_rule_group(out: &mut Vec<String>, group: &mut Vec<String>, indent: &str) {
    match group.len() {
        0 => {}
        1 => push_rule_text(out, indent, &group[0]),
        _ => {
            out.push(format!("{indent}repeat {{"));
            for rule in group.drain(..) {
                push_rule_text(out, &format!("{indent}  "), &rule);
            }
            out.push(format!("{indent}}}"));
            return;
        }
    }
    group.clear();
}

fn push_rule_text(out: &mut Vec<String>, indent: &str, text: &str) {
    for line in text.lines() {
        out.push(format!("{indent}{line}"));
    }
}

fn is_late_rule(line: &str) -> bool {
    line.trim()
        .trim_start_matches('+')
        .trim_start()
        .split_whitespace()
        .next()
        .is_some_and(|token| token.eq_ignore_ascii_case("late"))
}

fn canonical_rule_rows(
    line: &str,
    objects: &[PsObjectDef],
    aliases: &[PsAliasDef],
    tags: &[PsTagDef],
    maps: &[PsMapDef],
    case_sensitive: bool,
) -> Option<Vec<String>> {
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
    let variants = ps_tag_prefix_capture_variants(tokens, tags, maps)
        .into_iter()
        .flat_map(direction_prefix_variants)
        .collect::<Vec<_>>();
    let rows = variants
        .into_iter()
        .map(|tokens| {
            canonical_rule_tokens_to_row(
                tokens,
                objects,
                aliases,
                tags,
                maps,
                case_sensitive,
                has_again,
            )
        })
        .collect::<Vec<_>>();
    Some(rows)
}

fn canonical_rule_tokens_to_row(
    mut tokens: Vec<String>,
    objects: &[PsObjectDef],
    aliases: &[PsAliasDef],
    tags: &[PsTagDef],
    maps: &[PsMapDef],
    case_sensitive: bool,
    has_again: bool,
) -> String {
    tokens = translate_motion_qualifiers(tokens, objects, aliases, tags, maps, case_sensitive);
    tokens = attach_direction_prefixes(tokens, objects, aliases, tags, maps, case_sensitive);
    let tokens = tokens
        .into_iter()
        .map(|token| resolve_rule_token(&token, objects, aliases, tags, maps, case_sensitive))
        .collect::<Vec<_>>();
    let tokens = remove_ps_gosub_tokens(tokens);
    let tokens = expand_ps_sfx_effect_tokens(tokens);
    let mut row = tokens.join(" ");
    if has_again {
        row.push_str(" again");
    }
    row
}

fn remove_ps_gosub_tokens(tokens: Vec<String>) -> Vec<String> {
    tokens
        .into_iter()
        .filter(|token| !token.eq_ignore_ascii_case("gosub"))
        .collect()
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
    tags: &[PsTagDef],
    maps: &[PsMapDef],
    case_sensitive: bool,
) -> Vec<String> {
    let mut attached = Vec::new();
    let mut i = 0usize;
    while i < tokens.len() {
        if let Some(direction) = canonical_direction_token(&tokens[i])
            && let Some(selector) = tokens.get(i + 1).filter(|selector| {
                resolve_name(selector, objects, aliases, tags, maps, case_sensitive).is_some()
            })
        {
            attached.push(append_mark_to_selector(
                selector,
                direction,
                objects,
                aliases,
                tags,
                maps,
                case_sensitive,
            ));
            i += 2;
            continue;
        }
        attached.push(tokens[i].clone());
        i += 1;
    }
    attached
}

fn direction_prefix_variants(tokens: Vec<String>) -> Vec<Vec<String>> {
    let Some(pattern_start) = tokens.iter().position(|token| token == "[") else {
        return vec![tokens];
    };
    let prefix_count = tokens[..pattern_start]
        .iter()
        .take_while(|token| ps_rule_direction_prefix(token).is_some())
        .count();
    if prefix_count <= 1 {
        return vec![tokens];
    }

    let prefixes = tokens[..prefix_count]
        .iter()
        .filter_map(|token| ps_rule_direction_prefix(token))
        .map(str::to_string)
        .collect::<Vec<_>>();
    prefixes
        .into_iter()
        .map(|prefix| {
            let mut variant = vec![prefix];
            variant.extend(tokens[prefix_count..].iter().cloned());
            variant
        })
        .collect()
}

fn ps_tag_prefix_capture_variants(
    tokens: Vec<String>,
    tags: &[PsTagDef],
    maps: &[PsMapDef],
) -> Vec<Vec<String>> {
    let Some(pattern_start) = tokens.iter().position(|token| token == "[") else {
        return vec![tokens];
    };
    let tag_prefixes = tokens[..pattern_start]
        .iter()
        .filter(|token| ps_tag_values(token, tags).is_some())
        .cloned()
        .collect::<Vec<_>>();
    if tag_prefixes.is_empty() {
        return vec![tokens];
    }
    let captures = tag_prefixes
        .into_iter()
        .enumerate()
        .map(|(index, axis)| (axis, format!("#{}", index + 1)))
        .collect::<BTreeMap<_, _>>();
    vec![
        tokens
            .iter()
            .enumerate()
            .filter(|(index, token)| {
                *index >= pattern_start
                    || !captures.keys().any(|axis| axis.as_str() == token.as_str())
            })
            .map(|(_, token)| apply_ps_tag_captures(token, &captures, maps))
            .collect::<Vec<_>>(),
    ]
}

fn apply_ps_tag_captures(
    token: &str,
    captures: &BTreeMap<String, String>,
    maps: &[PsMapDef],
) -> String {
    let (selector, mark) = token
        .split_once('{')
        .map_or((token, None), |(selector, mark)| (selector, Some(mark)));
    if !selector.contains(':') {
        return token.to_string();
    }
    let mut parts = selector.split(':').map(str::to_string).collect::<Vec<_>>();
    for part in parts.iter_mut().skip(1) {
        if let Some(label) = captures.get(part.as_str()) {
            part.push_str(label);
            continue;
        }
        if let Some(map) = maps.iter().find(|map| map.name == *part)
            && let Some(label) = captures.get(&map.axis)
        {
            *part = format!("{}({}{})", map.name, map.axis, label);
        }
    }
    let selector = parts.join(":");
    mark.map_or(selector.clone(), |mark| format!("{selector}{{{mark}"))
}

fn translate_motion_qualifiers(
    tokens: Vec<String>,
    objects: &[PsObjectDef],
    aliases: &[PsAliasDef],
    tags: &[PsTagDef],
    maps: &[PsMapDef],
    case_sensitive: bool,
) -> Vec<String> {
    let Some(arrow) = tokens.iter().position(|token| token == "->") else {
        return tokens;
    };
    let mut translated = translate_motion_qualifiers_on_side(
        &tokens[..arrow],
        true,
        objects,
        aliases,
        tags,
        maps,
        case_sensitive,
    );
    translated.push("->".to_string());
    translated.extend(translate_motion_qualifiers_on_side(
        &tokens[arrow + 1..],
        false,
        objects,
        aliases,
        tags,
        maps,
        case_sensitive,
    ));
    translated
}

fn translate_motion_qualifiers_on_side(
    tokens: &[String],
    is_lhs: bool,
    objects: &[PsObjectDef],
    aliases: &[PsAliasDef],
    tags: &[PsTagDef],
    maps: &[PsMapDef],
    case_sensitive: bool,
) -> Vec<String> {
    let mut translated = Vec::new();
    let mut i = 0usize;
    while i < tokens.len() {
        let token = &tokens[i];
        let is_moving = token.eq_ignore_ascii_case("moving");
        let is_stationary = token.eq_ignore_ascii_case("stationary");
        let is_action = token.eq_ignore_ascii_case("action");
        let relative = ps_relative_motion_qualifier(token);
        if (is_moving || is_stationary || is_action || relative.is_some())
            && let Some(selector) = tokens.get(i + 1).filter(|selector| {
                resolve_name(selector, objects, aliases, tags, maps, case_sensitive).is_some()
            })
        {
            if !is_lhs && (is_moving || is_stationary) {
                translated.push(selector.clone());
            } else {
                let mark = if is_moving {
                    "directions"
                } else if is_action {
                    "__action"
                } else if let Some(relative) = relative {
                    relative
                } else {
                    "no directions"
                };
                translated.push(append_mark_to_selector(
                    selector,
                    mark,
                    objects,
                    aliases,
                    tags,
                    maps,
                    case_sensitive,
                ));
            }
            i += 2;
            continue;
        }
        translated.push(token.clone());
        i += 1;
    }
    translated
}

fn append_mark_to_selector(
    selector: &str,
    mark: &str,
    objects: &[PsObjectDef],
    aliases: &[PsAliasDef],
    tags: &[PsTagDef],
    maps: &[PsMapDef],
    case_sensitive: bool,
) -> String {
    let selector = resolve_name(selector, objects, aliases, tags, maps, case_sensitive)
        .unwrap_or_else(|| selector.to_string());
    if puzzle_authoring::mark_sugar_kind(mark) == Some(puzzle_authoring::MarkSugarKind::Movement) {
        return format!("{mark} {selector}");
    }
    if let Some(stripped) = selector.strip_suffix('}') {
        format!("{stripped} {mark}}}")
    } else {
        format!("{selector}{{{mark}}}")
    }
}

fn ps_level_chunks(lines: &[String]) -> Vec<PsLevelChunk> {
    let mut chunks = Vec::new();
    let mut current_map = Vec::new();
    let mut pending_start_messages = Vec::new();
    let mut current_name = None::<String>;
    for line in lines {
        let trimmed = line.trim();
        if let Some(section) = parse_level_section(trimmed) {
            if !current_map.is_empty() {
                push_ps_level_chunk(
                    &mut chunks,
                    &mut current_name,
                    &mut pending_start_messages,
                    &mut current_map,
                );
            }
            current_name = Some(section);
            continue;
        }
        if trimmed.is_empty() || is_parenthetical_comment(trimmed) {
            if !current_map.is_empty() {
                push_ps_level_chunk(
                    &mut chunks,
                    &mut current_name,
                    &mut pending_start_messages,
                    &mut current_map,
                );
            }
            continue;
        }
        if let Some(message) = parse_level_message(trimmed) {
            if !current_map.is_empty() {
                push_ps_level_chunk(
                    &mut chunks,
                    &mut current_name,
                    &mut pending_start_messages,
                    &mut current_map,
                );
            }
            pending_start_messages.push(message);
            continue;
        }
        current_map.push(trimmed.to_string());
    }
    if !current_map.is_empty() {
        push_ps_level_chunk(
            &mut chunks,
            &mut current_name,
            &mut pending_start_messages,
            &mut current_map,
        );
    } else if !pending_start_messages.is_empty()
        && let Some(last) = chunks.last_mut()
    {
        last.lines.append(&mut pending_start_messages);
    }
    chunks
}

fn push_ps_level_chunk(
    chunks: &mut Vec<PsLevelChunk>,
    current_name: &mut Option<String>,
    pending_start_messages: &mut Vec<String>,
    current_map: &mut Vec<String>,
) {
    let mut lines = std::mem::take(pending_start_messages);
    lines.append(current_map);
    chunks.push(PsLevelChunk {
        name: current_name.take(),
        lines,
    });
}

fn is_level_message(line: &str) -> bool {
    line.split_whitespace()
        .next()
        .is_some_and(|token| token.eq_ignore_ascii_case("message"))
}

fn parse_level_section(line: &str) -> Option<String> {
    let (first, rest) = line
        .split_once(char::is_whitespace)
        .map_or((line, ""), |(first, rest)| (first, rest.trim()));
    if !first.eq_ignore_ascii_case("section") {
        return None;
    }
    Some(if rest.is_empty() {
        "section".to_string()
    } else {
        rest.to_string()
    })
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

fn resolve_rule_token(
    token: &str,
    objects: &[PsObjectDef],
    aliases: &[PsAliasDef],
    tags: &[PsTagDef],
    maps: &[PsMapDef],
    case_sensitive: bool,
) -> String {
    if let Some(direction) = canonical_direction_token(token) {
        return direction.to_string();
    }
    if matches!(token, "[" | "]" | "|" | ">" | "<" | "^" | "v" | "->") {
        return token.to_string();
    }
    if token == "..." || token.eq_ignore_ascii_case("no") {
        return token.to_ascii_lowercase();
    }
    if let Some((base, mark)) = token.split_once('{') {
        if let Some(name) = resolve_name(base, objects, aliases, tags, maps, case_sensitive) {
            return format!("{name}{{{mark}");
        }
    }
    resolve_name(token, objects, aliases, tags, maps, case_sensitive)
        .unwrap_or_else(|| token.to_string())
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
        "horizontal" => Some("horizontal"),
        "vertical" => Some("vertical"),
        "orthogonal" => Some("directions"),
        _ => None,
    }
}

fn ps_rule_direction_prefix(token: &str) -> Option<&'static str> {
    match token.to_ascii_lowercase().as_str() {
        "up" => Some("up"),
        "down" => Some("down"),
        "left" => Some("left"),
        "right" => Some("right"),
        "horizontal" => Some("horizontal"),
        "vertical" => Some("vertical"),
        "orthogonal" => Some("directions"),
        _ => None,
    }
}

fn ps_relative_motion_qualifier(token: &str) -> Option<&'static str> {
    match token.to_ascii_lowercase().as_str() {
        "parallel" => Some("parallel"),
        "perpendicular" => Some("perpendicular"),
        _ => None,
    }
}

fn reject_unsupported_rule_modifiers(lines: &[String]) -> Result<(), DiagnosticReport> {
    for line in lines.iter().filter(|line| !line.trim().is_empty()) {
        let trimmed = line.trim().trim_start_matches('+').trim();
        let first = trimmed.split_whitespace().next();
        if first.is_some_and(|token| token.eq_ignore_ascii_case("random")) {
            return Err(DiagnosticReport::from_diagnostic(
                Diagnostic::error("PuzzleScript random rules are not supported by this importer")
                    .with_source_line(line.clone()),
            ));
        }
    }
    Ok(())
}

fn ps_rules_use_action_input(lines: &[String]) -> bool {
    lines
        .iter()
        .flat_map(|line| tokenize_ps_rule(line))
        .any(|token| token.eq_ignore_ascii_case("action"))
}

fn push_levels(
    out: &mut Vec<String>,
    lines: &[String],
    legend_lines: &[String],
    objects: &[PsObjectDef],
    aliases: &[PsAliasDef],
    tags: &[PsTagDef],
    maps: &[PsMapDef],
    case_sensitive: bool,
) {
    out.push("levels {".to_string());
    let mut legend = Vec::new();
    let char_map = push_legend(
        &mut legend,
        legend_lines,
        lines,
        objects,
        aliases,
        tags,
        maps,
        case_sensitive,
    );
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
        if let Some(name) = &chunk.name {
            out.push(format!("  level \"{}\"", escape_canonical_string(name)));
        }
        for line in &chunk.lines {
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
    level_select: bool,
) {
    out.push("scene title {".to_string());
    out.push("  layout {".to_string());
    out.push(format!("    title = \"{}\"", escape_scene_text(title)));
    if let Some(author) = author {
        out.push(format!(
            "    subtitle = \"by {}\"",
            escape_scene_text(author)
        ));
    }
    out.push("    if has_progress_save {".to_string());
    out.push("      choice \"Continue\" -> continue_game".to_string());
    out.push("    }".to_string());
    out.push("    choice \"New Game\" -> new_game".to_string());
    if level_select {
        out.push("    choice \"Level Select\" -> goto level_select".to_string());
    }
    out.push("  }".to_string());
    push_scene_routine(out, "continue_game", &["goto playing"], startgame_sfx);
    push_scene_routine(
        out,
        "new_game",
        &["clear_game_progress", "goto playing(0)"],
        startgame_sfx,
    );
    out.push("}".to_string());
    out.push(String::new());

    out.push("scene playing {".to_string());
    if viewport_size.is_some() {
        out.push("  layout {".to_string());
        out.push("    puzzle board = main".to_string());
        out.push("  }".to_string());
    } else {
        out.push("  layout {".to_string());
        out.push("    row {".to_string());
        out.push(format!("      title = \"{}\"", escape_scene_text(title)));
        out.push("    }".to_string());
        out.push("    puzzle board = main".to_string());
        out.push("  }".to_string());
    }
    out.push("  keys {".to_string());
    out.push("    Escape q -> back".to_string());
    out.push("  }".to_string());
    out.push("  rules {".to_string());
    out.push("    step board".to_string());
    out.push("  }".to_string());
    push_scene_routine(out, "back", &["goto title"], None);
    out.push("}".to_string());
    out.push(String::new());

    if level_select {
        out.push("scene level_select {".to_string());
        out.push("  layout {".to_string());
        out.push("    level_menu {".to_string());
        out.push("      show_index = true".to_string());
        out.push("      show_solved = true".to_string());
        out.push("      button \"Back\" -> goto title".to_string());
        out.push("    }".to_string());
        out.push("  }".to_string());
        out.push("}".to_string());
        out.push(String::new());
    }
}

fn push_scene_routine(
    out: &mut Vec<String>,
    name: &str,
    effects: &[&str],
    startgame_sfx: Option<&str>,
) {
    out.push(format!("  routine {name} {{"));
    if let Some(name) = startgame_sfx {
        out.push(format!("    sfx {name}"));
        for effect in effects {
            out.push(format!("    {effect}"));
        }
    } else {
        for effect in effects {
            out.push(format!("    {effect}"));
        }
    }
    out.push("  }".to_string());
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

fn resolve_name(
    token: &str,
    objects: &[PsObjectDef],
    aliases: &[PsAliasDef],
    tags: &[PsTagDef],
    maps: &[PsMapDef],
    case_sensitive: bool,
) -> Option<String> {
    if case_sensitive {
        return resolve_alias(token, aliases, case_sensitive)
            .map(|alias| alias.name.clone())
            .or_else(|| token_is_empty_alias(token, case_sensitive).then_some("empty".to_string()))
            .or_else(|| resolve_object_name(token, objects, case_sensitive))
            .or_else(|| {
                is_ps_tag_selector(token, objects, tags, maps, case_sensitive)
                    .then(|| token.to_string())
            });
    }
    resolve_object_name(token, objects, case_sensitive)
        .or_else(|| {
            is_ps_tag_selector(token, objects, tags, maps, case_sensitive)
                .then(|| token.to_string())
        })
        .or_else(|| {
            resolve_alias(token, aliases, case_sensitive)
                .map(|alias| alias.name.clone())
                .or_else(|| {
                    token_is_empty_alias(token, case_sensitive).then_some("empty".to_string())
                })
        })
}

fn resolve_object_name(
    token: &str,
    objects: &[PsObjectDef],
    case_sensitive: bool,
) -> Option<String> {
    objects
        .iter()
        .find(|object| ps_name_eq(&object.name, token, case_sensitive))
        .map(|object| object.name.clone())
}

fn resolve_alias<'a>(
    token: &str,
    aliases: &'a [PsAliasDef],
    case_sensitive: bool,
) -> Option<&'a PsAliasDef> {
    aliases
        .iter()
        .find(|alias| ps_name_eq(&alias.name, token, case_sensitive))
}

fn ps_name_eq(left: &str, right: &str, case_sensitive: bool) -> bool {
    if case_sensitive {
        left == right
    } else {
        left.eq_ignore_ascii_case(right)
    }
}

fn is_ps_object_spec(value: &str) -> bool {
    let mut parts = value.split(':');
    let Some(first) = parts.next() else {
        return false;
    };
    is_identifier(first) && parts.all(is_ps_object_spec_part)
}

fn is_ps_object_spec_part(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn is_ps_tag_selector(
    token: &str,
    objects: &[PsObjectDef],
    tags: &[PsTagDef],
    maps: &[PsMapDef],
    case_sensitive: bool,
) -> bool {
    let parts = token.split(':').collect::<Vec<_>>();
    if parts.len() <= 1 || !ps_object_family_exists(parts[0], objects, case_sensitive) {
        return false;
    }
    parts[1..]
        .iter()
        .all(|part| ps_tag_values(part, tags).is_some() || maps.iter().any(|map| map.name == *part))
}

fn ps_object_family_exists(base: &str, objects: &[PsObjectDef], case_sensitive: bool) -> bool {
    objects.iter().any(|object| {
        ps_name_eq(&object.name, base, case_sensitive)
            || ps_family_name_matches(&object.name, base, case_sensitive)
    })
}

fn ps_family_name_matches(object_name: &str, base: &str, case_sensitive: bool) -> bool {
    let Some((object_base, _)) = object_name.split_once(':') else {
        return false;
    };
    ps_name_eq(object_base, base, case_sensitive)
}

fn ps_tag_values(name: &str, tags: &[PsTagDef]) -> Option<Vec<String>> {
    if let Some(tag) = tags.iter().find(|tag| tag.name == name) {
        return Some(tag.values.clone());
    }
    match name {
        "directions" => Some(
            ["up", "down", "left", "right"]
                .into_iter()
                .map(str::to_string)
                .collect(),
        ),
        "horizontal" => Some(["left", "right"].into_iter().map(str::to_string).collect()),
        "vertical" => Some(["up", "down"].into_iter().map(str::to_string).collect()),
        _ => None,
    }
}

fn token_is_empty_alias(token: &str, case_sensitive: bool) -> bool {
    matches!(token, "." | "_")
        || if case_sensitive {
            token == "background"
        } else {
            token.eq_ignore_ascii_case("background")
        }
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
    format!("\"{}\"", escape_scene_text(value))
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
