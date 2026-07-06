fn parse_sounds_block(
    lines: &[String],
    start: usize,
    sounds: &mut SoundsDef,
) -> Result<usize, DiagnosticReport> {
    let header = split_header_tokens(&lines[start]);
    if !matches!(header.as_slice(), ["sounds"]) {
        return Err(parse_error(&lines[start], "sounds header must be: sounds"));
    }

    let mut i = start + 1;
    while i < lines.len() {
        let line = &lines[i];
        if is_block_close_line(line) {
            return Ok(i + 1);
        }
        let tokens = split_header_tokens(line);
        match tokens.as_slice() {
            ["sfx", name, settings @ ..] => {
                validate_qualified_identifier(name, line, "sfx sounds name")?;
                if sounds.sfx.iter().any(|entry| entry.name == *name) {
                    return Err(parse_error(line, "duplicate sfx sounds name"));
                }
                let seed = required_sound_setting(settings, "seed", line)?;
                let type_target = optional_sound_setting(settings, "type").unwrap_or("random");
                let volume = parse_sound_f64(
                    optional_sound_setting(settings, "volume").unwrap_or("1"),
                    line,
                    "volume",
                )?;
                validate_sound_atom(seed, line, "sfx seed")?;
                validate_sound_atom(type_target, line, "sfx type")?;
                if volume < 0.0 {
                    return Err(parse_error(line, "sfx volume must be zero or greater"));
                }
                sounds.sfx.push(SfxSoundDef {
                    name: (*name).to_string(),
                    seed: seed.to_string(),
                    type_target: type_target.to_string(),
                    volume,
                });
            }
            ["music", name, settings @ ..] => {
                validate_qualified_identifier(name, line, "music sounds name")?;
                if sounds.music.iter().any(|entry| entry.name == *name) {
                    return Err(parse_error(line, "duplicate music sounds name"));
                }
                let seed = required_sound_setting(settings, "seed", line)?;
                validate_sound_atom(seed, line, "music seed")?;
                let height = parse_sound_f64(
                    optional_sound_setting(settings, "height")
                        .or_else(|| optional_sound_setting(settings, "tone"))
                        .unwrap_or("0.5"),
                    line,
                    "height",
                )?;
                let bars = parse_sound_u16(
                    optional_sound_setting(settings, "bars").unwrap_or("8"),
                    line,
                    "bars",
                )?;
                let bpm = parse_sound_u16(
                    optional_sound_setting(settings, "bpm").unwrap_or("110"),
                    line,
                    "bpm",
                )?;
                let volume = parse_sound_f64(
                    optional_sound_setting(settings, "volume").unwrap_or("0.5"),
                    line,
                    "volume",
                )?;
                if !(0.0..=1.0).contains(&height) {
                    return Err(parse_error(line, "music height must be between 0 and 1"));
                }
                if !matches!(bars, 8 | 16 | 32 | 64) {
                    return Err(parse_error(
                        line,
                        "music bars must be one of 8, 16, 32, or 64",
                    ));
                }
                if !(40..=180).contains(&bpm) {
                    return Err(parse_error(line, "music bpm must be between 40 and 180"));
                }
                if volume < 0.0 {
                    return Err(parse_error(line, "music volume must be zero or greater"));
                }
                sounds.music.push(MusicSoundDef {
                    name: (*name).to_string(),
                    seed: seed.to_string(),
                    height,
                    bars,
                    bpm,
                    volume,
                });
            }
            _ => {
                return Err(parse_error(
                    line,
                    "sounds entry must be: sfx <name> seed=<seed> type=<type> volume=<gain> | music <name> seed=<seed> bars=<8|16|32|64> height=<0..1> bpm=<40..180> volume=<gain>",
                ));
            }
        }
        i += 1;
    }

    Err(parse_error(&lines[start], "sounds missing closing brace"))
}

#[derive(Clone, Debug)]
struct ModelSoundTrigger {
    kind: ModelSoundTriggerKind,
    objects: Vec<ObjectId>,
    sfx_name: String,
}

#[derive(Clone, Debug)]
struct ModelSoundTriggerSpec {
    kind: ModelSoundTriggerKind,
    selector: String,
    sfx_name: String,
    line: String,
}

#[derive(Clone, Debug)]
struct ModelOperationSoundSpec {
    operation: ModelOperationSound,
    sfx_name: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ModelSoundTriggerKind {
    Move,
    CantMove,
}

fn model_sounds_block_starts(lines: &[String], start: usize) -> bool {
    lines.get(start + 1).is_some_and(|first| {
        matches!(
            split_header_tokens(first).as_slice(),
            ["move" | "cantmove", ..]
        )
    })
}

fn parse_model_sounds_block(
    lines: &[String],
    start: usize,
    triggers: &mut Vec<ModelSoundTriggerSpec>,
    operation_sounds: &mut Vec<ModelOperationSoundSpec>,
    allow_operation_sounds: bool,
) -> Result<usize, DiagnosticReport> {
    let header = split_header_tokens(&lines[start]);
    if !matches!(header.as_slice(), ["sounds"]) {
        return Err(parse_error(
            &lines[start],
            "model sounds header must be: sounds",
        ));
    }

    let mut i = start + 1;
    while i < lines.len() {
        let line = &lines[i];
        if is_block_close_line(line) {
            return Ok(i + 1);
        }
        let tokens = split_header_tokens(line);
        let trigger_kind = match tokens.as_slice() {
            ["move", ..] => Some(ModelSoundTriggerKind::Move),
            ["cantmove", ..] => Some(ModelSoundTriggerKind::CantMove),
            _ => None,
        };
        let operation = match tokens.as_slice() {
            ["undo", ..] => Some(ModelOperationSound::Undo),
            ["restart", ..] => Some(ModelOperationSound::Restart),
            _ => None,
        };
        match (trigger_kind, operation, tokens.as_slice()) {
            (Some(kind), _, [_, selector, "->", "sfx", name]) => {
                validate_qualified_identifier(name, line, "sfx name")?;
                triggers.push(ModelSoundTriggerSpec {
                    kind,
                    selector: (*selector).to_string(),
                    sfx_name: (*name).to_string(),
                    line: line.clone(),
                });
            }
            (_, Some(operation), [_, "->", "sfx", name]) if allow_operation_sounds => {
                validate_qualified_identifier(name, line, "sfx name")?;
                operation_sounds.push(ModelOperationSoundSpec {
                    operation,
                    sfx_name: (*name).to_string(),
                });
            }
            (_, Some(_), [_, "->", "sfx", _]) => {
                return Err(parse_error(
                    line,
                    "undo/restart sounds must be inside a puzzle sounds block",
                ));
            }
            _ => {
                return Err(parse_error(
                    line,
                    "model sounds entry must be: move <object-selector> -> sfx <name> | cantmove <object-selector> -> sfx <name> | undo -> sfx <name> | restart -> sfx <name>",
                ));
            }
        }
        i += 1;
    }

    Err(parse_error(
        &lines[start],
        "model sounds missing closing brace",
    ))
}

fn resolve_model_operation_sounds(
    specs: &[ModelOperationSoundSpec],
) -> Vec<ModelOperationSoundDef> {
    specs
        .iter()
        .map(|spec| ModelOperationSoundDef {
            operation: spec.operation,
            sfx_name: spec.sfx_name.clone(),
        })
        .collect()
}

fn resolve_model_sound_triggers(
    specs: &[ModelSoundTriggerSpec],
    catalog: &Catalog,
) -> Result<Vec<ModelSoundTrigger>, DiagnosticReport> {
    let value_sets = catalog_value_sets(catalog);
    specs
        .iter()
        .map(|spec| {
            let selector = resolve_object_selector(
                &spec.selector,
                &spec.line,
                &catalog.object_names,
                &catalog.object_schemas,
                &value_sets,
                &catalog.maps,
                &catalog.object_groups,
                &HashMap::new(),
            )
            .map_err(|error| model_sound_selector_error(error, spec))?;
            if selector
                .alternatives
                .iter()
                .any(|object| catalog.visual_objects.contains(object))
            {
                return Err(parse_error(
                    &spec.line,
                    "model sound triggers cannot target display objects",
                ));
            }
            Ok(ModelSoundTrigger {
                kind: spec.kind,
                objects: selector.alternatives,
                sfx_name: spec.sfx_name.clone(),
            })
        })
        .collect()
}

fn model_sound_selector_error(
    error: DiagnosticReport,
    spec: &ModelSoundTriggerSpec,
) -> DiagnosticReport {
    if error
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.message.starts_with("unknown object selector"))
    {
        parse_error(
            &spec.line,
            &format!(
                "unknown model sound trigger object selector `{}`",
                spec.selector
            ),
        )
    } else {
        error
    }
}
