fn parse_sounds_block(
    lines: &[source::LogicalLine],
    start: usize,
    sounds: &mut SoundsDef,
) -> Result<usize, DiagnosticReport> {
    let (node, next_i) = authoring_grammar::parse_placed_authoring_node(
        lines,
        start,
        authoring_grammar::AuthoringKind::Root,
        "sounds missing closing brace",
    )?;
    if node.kind != authoring_grammar::AuthoringKind::SoundsConfig {
        return Err(parse_error(&lines[start], "sounds header must be: sounds"));
    }

    for child in &node.children {
        match child.kind {
            authoring_grammar::AuthoringKind::SfxSoundConfig => {
                apply_sfx_sound_node(child, sounds)?;
            }
            authoring_grammar::AuthoringKind::MusicSoundConfig => {
                apply_music_sound_node(child, sounds)?;
            }
            _ => {
                return Err(parse_error(
                    &child.source_line,
                    &format!("unknown sounds directive {}", child.surface),
                ));
            }
        }
    }

    Ok(next_i)
}

fn apply_sfx_sound_node(
    node: &authoring_grammar::AuthoringNode,
    sounds: &mut SoundsDef,
) -> Result<(), DiagnosticReport> {
    let name = sound_node_name(node, "sfx")?;
    validate_qualified_identifier(name, &node.source_line, "sfx sounds name")?;
    if sounds.sfx.iter().any(|entry| entry.name == name) {
        return Err(parse_error(&node.source_line, "duplicate sfx sounds name"));
    }
    let seed = required_sound_setting(node, "seed")?;
    let type_target = optional_sound_setting(node, "type")?.unwrap_or("random");
    let volume = parse_sound_f64(
        optional_sound_setting(node, "volume")?.unwrap_or("1"),
        &node.source_line,
        "volume",
    )?;
    validate_sound_atom(seed, &node.source_line, "sfx seed")?;
    validate_sound_atom(type_target, &node.source_line, "sfx type")?;
    if volume < 0.0 {
        return Err(parse_error(
            &node.source_line,
            "sfx volume must be zero or greater",
        ));
    }
    sounds.sfx.push(SfxSoundDef {
        name: name.to_string(),
        seed: seed.to_string(),
        type_target: type_target.to_string(),
        volume,
    });
    Ok(())
}

fn apply_music_sound_node(
    node: &authoring_grammar::AuthoringNode,
    sounds: &mut SoundsDef,
) -> Result<(), DiagnosticReport> {
    let name = sound_node_name(node, "music")?;
    validate_qualified_identifier(name, &node.source_line, "music sounds name")?;
    if sounds.music.iter().any(|entry| entry.name == name) {
        return Err(parse_error(
            &node.source_line,
            "duplicate music sounds name",
        ));
    }
    let seed = required_sound_setting(node, "seed")?;
    validate_sound_atom(seed, &node.source_line, "music seed")?;
    let height = parse_sound_f64(
        optional_sound_setting(node, "height")?
            .or(optional_sound_setting(node, "tone")?)
            .unwrap_or("0.5"),
        &node.source_line,
        "height",
    )?;
    let bars = parse_sound_u16(
        optional_sound_setting(node, "bars")?.unwrap_or("8"),
        &node.source_line,
        "bars",
    )?;
    let bpm = parse_sound_u16(
        optional_sound_setting(node, "bpm")?.unwrap_or("110"),
        &node.source_line,
        "bpm",
    )?;
    let volume = parse_sound_f64(
        optional_sound_setting(node, "volume")?.unwrap_or("0.5"),
        &node.source_line,
        "volume",
    )?;
    if !(0.0..=1.0).contains(&height) {
        return Err(parse_error(
            &node.source_line,
            "music height must be between 0 and 1",
        ));
    }
    if !matches!(bars, 8 | 16 | 32 | 64) {
        return Err(parse_error(
            &node.source_line,
            "music bars must be one of 8, 16, 32, or 64",
        ));
    }
    if !(40..=180).contains(&bpm) {
        return Err(parse_error(
            &node.source_line,
            "music bpm must be between 40 and 180",
        ));
    }
    if volume < 0.0 {
        return Err(parse_error(
            &node.source_line,
            "music volume must be zero or greater",
        ));
    }
    sounds.music.push(MusicSoundDef {
        name: name.to_string(),
        seed: seed.to_string(),
        height,
        bars,
        bpm,
        volume,
    });
    Ok(())
}

fn sound_node_name<'a>(
    node: &'a authoring_grammar::AuthoringNode,
    kind: &str,
) -> Result<&'a str, DiagnosticReport> {
    let [name] = node.header_args.as_slice() else {
        return Err(parse_error(
            &node.source_line,
            &format!("{kind} sounds entry must be: {kind} <name>"),
        ));
    };
    Ok(name)
}

fn required_sound_setting<'a>(
    node: &'a authoring_grammar::AuthoringNode,
    key: &str,
) -> Result<&'a str, DiagnosticReport> {
    optional_sound_setting(node, key)?.ok_or_else(|| {
        parse_error(
            &node.source_line,
            &format!("missing required sound setting {key}"),
        )
    })
}

fn optional_sound_setting<'a>(
    node: &'a authoring_grammar::AuthoringNode,
    key: &str,
) -> Result<Option<&'a str>, DiagnosticReport> {
    let Some(row) = node.definition_rows.iter().find(|row| row.key == key) else {
        return Ok(None);
    };
    row.single_value().map(Some).ok_or_else(|| {
        parse_error(
            &row.source_line,
            "sound setting must have exactly one value",
        )
    })
}

#[derive(Clone, Debug)]
struct ModelSoundTrigger {
    kind: ModelSoundTriggerKind,
    objects: Vec<ObjectId>,
    sfx_name: String,
}

type ModelSoundTriggerSpec = model_syntax::ModelSoundTriggerSyntax;
type ModelOperationSoundSpec = model_syntax::ModelOperationSoundSyntax;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ModelSoundTriggerKind {
    Move,
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
                &spec.source,
                &catalog.object_names,
                &catalog.object_schemas,
                &value_sets,
                &catalog.maps,
                &catalog.object_groups,
                &HashMap::new(),
            )
            .map_err(|error| model_sound_selector_error(error, spec))?;
            Ok(ModelSoundTrigger {
                kind: ModelSoundTriggerKind::Move,
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
            &spec.source,
            &format!(
                "unknown model sound trigger object selector `{}`",
                spec.selector
            ),
        )
    } else {
        error
    }
}
