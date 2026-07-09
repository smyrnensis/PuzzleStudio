fn define_object_spec(
    spec: &str,
    layer: u16,
    render_spec: Option<&str>,
    line: &str,
    value_sets: &HashMap<String, Vec<String>>,
    axis_kinds: &HashMap<String, AxisKind>,
    object_schemas: &mut HashMap<String, ObjectSchema>,
    object_names: &mut HashMap<String, ObjectId>,
    object_labels: &mut HashMap<ObjectId, String>,
    object_layers: &mut HashMap<ObjectId, LayerId>,
    object_defs: &mut Vec<ObjectDef>,
    render_chars: &mut HashMap<ObjectId, char>,
    char_objects: &mut HashMap<char, Vec<ObjectId>>,
) -> Result<Vec<ObjectId>, DiagnosticReport> {
    let parts = spec.split(':').collect::<Vec<_>>();
    let base = parts[0];
    if parts.len() == 1 {
        if object_names.contains_key(spec) {
            return Err(parse_error(line, "duplicate object"));
        }
        let id = add_object_variant(
            spec,
            layer,
            object_names,
            object_labels,
            object_layers,
            object_defs,
        );
        if let Some(render) = render_spec {
            let ch = parse_render_chars(render, line)?
                .into_iter()
                .next()
                .ok_or_else(|| parse_error(line, "missing object render char"))?;
            render_chars.insert(id, ch);
            char_objects.insert(ch, vec![id]);
        }
        return Ok(vec![id]);
    }

    if object_schemas.contains_key(base) {
        return Err(parse_error(line, "duplicate object family"));
    }

    let axes = parts[1..]
        .iter()
        .map(|axis| {
            if !value_sets.contains_key(*axis) {
                return Err(parse_error(
                    line,
                    "object schema tag slot must name a tag set",
                ));
            }
            Ok((*axis).to_string())
        })
        .collect::<Result<Vec<_>, DiagnosticReport>>()?;
    let schema_axis_kinds = validate_object_schema_axes(&axes, axis_kinds, line)?;
    let value_combinations = expand_axis_values(&axes, value_sets, line)?;
    let render_chars_for_variants = render_spec
        .map(|render| render_chars_for_variants(render, value_combinations.len(), line))
        .transpose()?;
    let mut variants = Vec::with_capacity(value_combinations.len());
    let mut created = Vec::with_capacity(value_combinations.len());

    for (index, values) in value_combinations.into_iter().enumerate() {
        let name = format!("{base}:{}", values.join(":"));
        let id = add_object_variant(
            &name,
            layer,
            object_names,
            object_labels,
            object_layers,
            object_defs,
        );
        if let Some(chars) = &render_chars_for_variants {
            let ch = chars[index];
            render_chars.insert(id, ch);
            if index == 0 {
                char_objects.insert(ch, vec![id]);
            } else if chars.iter().filter(|candidate| **candidate == ch).count() == 1 {
                char_objects.insert(ch, vec![id]);
            }
        }
        created.push(id);
        variants.push(ObjectVariant { values, object: id });
    }

    object_schemas.insert(
        base.to_string(),
        ObjectSchema {
            axes,
            axis_kinds: schema_axis_kinds,
            variants,
        },
    );
    Ok(created)
}

fn validate_object_schema_axes(
    axes: &[String],
    axis_kinds: &HashMap<String, AxisKind>,
    line: &str,
) -> Result<Vec<Option<AxisKind>>, DiagnosticReport> {
    let mut seen = Vec::<String>::new();
    let mut has_rotation = false;
    let mut has_translation = false;
    let mut kinds = Vec::with_capacity(axes.len());
    for axis in axes {
        if seen.contains(axis) {
            return Err(parse_error(
                line,
                "object schema cannot repeat the same tag slot",
            ));
        }
        seen.push(axis.clone());
        let kind = axis_kinds.get(axis).copied();
        match kind {
            Some(AxisKind::Rotation) if has_rotation => {
                return Err(parse_error(
                    line,
                    "object schema can have at most one rotation axis",
                ));
            }
            Some(AxisKind::Translation) if has_translation => {
                return Err(parse_error(
                    line,
                    "object schema can have at most one translation axis",
                ));
            }
            Some(AxisKind::Rotation) => has_rotation = true,
            Some(AxisKind::Translation) => has_translation = true,
            None => {}
        }
        kinds.push(kind);
    }
    Ok(kinds)
}

fn add_object_variant(
    name: &str,
    layer: u16,
    object_names: &mut HashMap<String, ObjectId>,
    object_labels: &mut HashMap<ObjectId, String>,
    object_layers: &mut HashMap<ObjectId, LayerId>,
    object_defs: &mut Vec<ObjectDef>,
) -> ObjectId {
    let id = ObjectId((object_defs.len() + 1) as u16);
    object_names.insert(name.to_string(), id);
    object_labels.insert(id, name.to_string());
    object_layers.insert(id, LayerId(layer));
    object_defs.push(ObjectDef {
        id,
        layer_id: LayerId(layer),
    });
    id
}

fn expand_axis_values(
    axes: &[String],
    value_sets: &HashMap<String, Vec<String>>,
    line: &str,
) -> Result<Vec<Vec<String>>, DiagnosticReport> {
    let mut combinations = vec![Vec::<String>::new()];
    for axis in axes {
        let values = value_sets
            .get(axis)
            .ok_or_else(|| parse_error(line, "unknown object schema tag set"))?;
        let mut next = Vec::new();
        for prefix in &combinations {
            for value in values {
                let mut variant = prefix.clone();
                variant.push(value.clone());
                next.push(variant);
            }
        }
        combinations = next;
    }
    Ok(combinations)
}

fn render_chars_for_variants(
    render: &str,
    variant_count: usize,
    line: &str,
) -> Result<Vec<char>, DiagnosticReport> {
    let chars = parse_render_chars(render, line)?;
    if chars.len() == 1 {
        return Ok(vec![chars[0]; variant_count]);
    }
    if chars.len() == variant_count {
        return Ok(chars);
    }
    Err(parse_error(
        line,
        "object schema render chars must be one char or one char per variant",
    ))
}

fn parse_render_chars(render: &str, line: &str) -> Result<Vec<char>, DiagnosticReport> {
    let chars = render.chars().collect::<Vec<_>>();
    if chars.is_empty() {
        return Err(parse_error(line, "missing object render char"));
    }
    Ok(chars)
}

type OverlayDefs = Vec<(Vec<ObjectId>, char)>;

#[derive(Clone, Debug)]
struct VisualShapeTable {
    axis: String,
    entries: HashMap<String, Vec<String>>,
}

#[derive(Clone, Debug)]
struct VisualShapeRotation {
    map: Option<String>,
    from: String,
}

impl VisualShapeRotation {
    fn intrinsic(from: &str) -> Self {
        Self {
            map: None,
            from: from.to_string(),
        }
    }

    fn using(map: &str, from: &str) -> Self {
        Self {
            map: Some(map.to_string()),
            from: from.to_string(),
        }
    }
}

#[derive(Clone, Debug)]
struct SpriteEntrySpec {
    source_line: String,
    selector: Option<String>,
    color_exprs: Option<Vec<(char, String)>>,
    offset: VisualSpriteOffset,
    sampling: Option<VisualSpriteSampling>,
    loop_duration_ms: Option<u64>,
    loop_frame_duration_ms: Option<u64>,
    image_source: Option<String>,
    shape_ref: Option<(String, ValueExpr)>,
    inline_pattern: Option<Vec<String>>,
    loop_animation: Option<VisualSpriteLoopDef>,
    rotation: Option<VisualShapeRotation>,
}

impl SpriteEntrySpec {
    fn new(source_line: &str, rotation: Option<VisualShapeRotation>) -> Self {
        Self {
            source_line: source_line.to_string(),
            selector: None,
            color_exprs: None,
            offset: VisualSpriteOffset::default(),
            sampling: None,
            loop_duration_ms: None,
            loop_frame_duration_ms: None,
            image_source: None,
            shape_ref: None,
            inline_pattern: None,
            loop_animation: None,
            rotation,
        }
    }

    fn selector(&self) -> Result<&str, DiagnosticReport> {
        self.selector
            .as_deref()
            .ok_or_else(|| parse_error(&self.source_line, "sprite entry missing selector"))
    }

    fn set_image(&mut self, source: &str, line: &str) -> Result<(), DiagnosticReport> {
        if self.image_source.is_some() {
            return Err(parse_error(line, "duplicate sprite image"));
        }
        self.image_source = Some(parse_sprite_image_path(source, line)?);
        Ok(())
    }

    fn set_offset(&mut self, x: &str, y: &str, line: &str) -> Result<(), DiagnosticReport> {
        self.offset = VisualSpriteOffset {
            x: parse_cell_offset_value(x, line, "sprite offset x")?,
            y: parse_cell_offset_value(y, line, "sprite offset y")?,
        };
        Ok(())
    }

    fn set_sampling(&mut self, value: &str, line: &str) -> Result<(), DiagnosticReport> {
        if self.sampling.is_some() {
            return Err(parse_error(line, "duplicate sprite sampling"));
        }
        self.sampling = Some(parse_sprite_sampling(value, line)?);
        Ok(())
    }

    fn set_duration(&mut self, value: &str, line: &str) -> Result<(), DiagnosticReport> {
        if self.loop_duration_ms.is_some() {
            return Err(parse_error(line, "duplicate sprite duration"));
        }
        self.loop_duration_ms = Some(parse_wait_duration_ms(value, line)?);
        Ok(())
    }

    fn set_frame_duration(&mut self, value: &str, line: &str) -> Result<(), DiagnosticReport> {
        if self.loop_frame_duration_ms.is_some() {
            return Err(parse_error(line, "duplicate sprite frame_duration"));
        }
        self.loop_frame_duration_ms = Some(parse_wait_duration_ms(value, line)?);
        Ok(())
    }

    fn set_shape_ref(&mut self, value: &str, line: &str) -> Result<(), DiagnosticReport> {
        self.shape_ref = Some(parse_sprite_shape_ref(value, line)?);
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct VisualColorTable {
    axis: String,
    entries: HashMap<String, String>,
}

fn parse_visuals_block(
    lines: &[String],
    start: usize,
    catalog: &Catalog,
    visuals: &mut VisualsDef,
) -> Result<usize, DiagnosticReport> {
    let mut shapes = HashMap::<String, VisualShapeTable>::new();
    let mut plain_shapes = HashMap::<String, Vec<String>>::new();
    let mut color_aliases = HashMap::<String, String>::new();
    let mut colors = HashMap::<String, VisualColorTable>::new();
    let mut i = start + 1;

    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        match tokens.as_slice() {
            [] => i += 1,
            ["palette"] => {
                i = parse_visual_palette_block(lines, i, catalog, &mut color_aliases, &mut colors)?;
            }
            ["colors"] => {
                return Err(parse_error(
                    line,
                    "colors block was renamed to palette; sprite color rows still use colors",
                ));
            }
            ["palettes"] => {
                return Err(parse_error(
                    line,
                    "palettes block was renamed to palette",
                ));
            }
            ["shapes"] => {
                i = parse_visual_shapes_block(lines, i, catalog, &mut plain_shapes, &mut shapes)?;
            }
            ["shape", table_ref] => {
                if !table_ref.contains(':') {
                    if plain_shapes.contains_key(*table_ref) {
                        return Err(parse_error(line, "duplicate visual shape"));
                    }
                    let (pattern, next_i) = parse_visual_plain_shape(lines, i)?;
                    plain_shapes.insert((*table_ref).to_string(), pattern);
                    i = next_i;
                    continue;
                }
                let (name, axis) = parse_visual_table_ref(table_ref, line)?;
                if shapes.contains_key(&name) {
                    return Err(parse_error(line, "duplicate visual shape"));
                }
                let (table, next_i) = parse_visual_shape_table(lines, i, &axis, None, catalog)?;
                shapes.insert(name, table);
                i = next_i;
            }
            ["shape", table_ref, "rotate", "from", from] => {
                let (name, axis) = parse_visual_table_ref(table_ref, line)?;
                if shapes.contains_key(&name) {
                    return Err(parse_error(line, "duplicate visual shape"));
                }
                let rotation = VisualShapeRotation::intrinsic(from);
                let (table, next_i) =
                    parse_visual_shape_table(lines, i, &axis, Some(rotation), catalog)?;
                shapes.insert(name, table);
                i = next_i;
            }
            ["shape", table_ref, "rotate", "using", map, "from", from] => {
                let (name, axis) = parse_visual_table_ref(table_ref, line)?;
                if shapes.contains_key(&name) {
                    return Err(parse_error(line, "duplicate visual shape"));
                }
                let rotation = VisualShapeRotation::using(map, from);
                let (table, next_i) =
                    parse_visual_shape_table(lines, i, &axis, Some(rotation), catalog)?;
                shapes.insert(name, table);
                i = next_i;
            }
            ["shape", table_ref, "rotate", map, "from", from] => {
                let (name, axis) = parse_visual_table_ref(table_ref, line)?;
                if shapes.contains_key(&name) {
                    return Err(parse_error(line, "duplicate visual shape"));
                }
                let rotation = VisualShapeRotation::using(map, from);
                let (table, next_i) =
                    parse_visual_shape_table(lines, i, &axis, Some(rotation), catalog)?;
                shapes.insert(name, table);
                i = next_i;
            }
            ["palette", table_ref] => {
                let (name, axis) = parse_visual_table_ref(table_ref, line)?;
                if colors.contains_key(&name) {
                    return Err(parse_error(line, "duplicate visual colors"));
                }
                let (table, next_i) = parse_visual_color_table(lines, i, &axis, catalog)?;
                colors.insert(name, table);
                i = next_i;
            }
            ["colors", ..] => {
                return Err(parse_error(
                    line,
                    "colors table was renamed to palette; sprite color rows still use colors",
                ));
            }
            ["sprite"] if is_block_header_line(line) => {
                i = parse_sprite_entry(
                    lines,
                    i,
                    &plain_shapes,
                    &shapes,
                    &color_aliases,
                    &colors,
                    catalog,
                    visuals,
                )?;
            }
            [other, ..] => {
                if crate::authoring_grammar::authoring_kind_content_attachment(
                    crate::authoring_grammar::AuthoringKind::SpritesConfig,
                ) == Some(crate::authoring_grammar::ContentAttachment::SpriteEntries)
                {
                    i = parse_sprite_attachment_entry(
                        lines,
                        i,
                        &plain_shapes,
                        &shapes,
                        &color_aliases,
                        &colors,
                        catalog,
                        visuals,
                    )?;
                    continue;
                }
                return Err(parse_error(
                    line,
                    &format!("unknown sprites directive {other}"),
                ));
            }
        }
    }
    if i >= lines.len() {
        return Err(parse_error(&lines[start], "sprites missing closing brace"));
    }
    Ok(i + 1)
}

fn parse_visual_palette_block(
    lines: &[String],
    start: usize,
    catalog: &Catalog,
    color_aliases: &mut HashMap<String, String>,
    colors: &mut HashMap<String, VisualColorTable>,
) -> Result<usize, DiagnosticReport> {
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        match tokens.as_slice() {
            [] => i += 1,
            [name, "=", color] => {
                color_aliases.insert((*name).to_string(), (*color).to_string());
                i += 1;
            }
            [table_ref] => {
                let (name, axis) = parse_visual_table_ref(table_ref, line)?;
                if colors.contains_key(&name) {
                    return Err(parse_error(line, "duplicate visual colors"));
                }
                let (table, next_i) = parse_visual_color_table(lines, i, &axis, catalog)?;
                colors.insert(name, table);
                i = next_i;
            }
            _ => {
                return Err(parse_error(
                    line,
                    "palette row must be: <name> = <color> | <name>:<tag_set>",
                ));
            }
        }
    }
    if i >= lines.len() {
        return Err(parse_error(&lines[start], "palette missing closing brace"));
    }
    Ok(i + 1)
}

fn parse_visual_shapes_block(
    lines: &[String],
    start: usize,
    catalog: &Catalog,
    plain_shapes: &mut HashMap<String, Vec<String>>,
    shapes: &mut HashMap<String, VisualShapeTable>,
) -> Result<usize, DiagnosticReport> {
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        match tokens.as_slice() {
            [] => i += 1,
            [name] if !name.contains(':') => {
                let (pattern, next_i) = parse_visual_plain_shape(lines, i)?;
                plain_shapes.insert((*name).to_string(), pattern);
                i = next_i;
            }
            [table_ref] => {
                if let Some((name, axis, value)) =
                    parse_visual_shape_value_ref(table_ref, line, catalog)?
                {
                    let (pattern, next_i) = parse_visual_shape_value_pattern(lines, i, &[], false)?;
                    insert_visual_shape_value(shapes, name, axis, value, pattern, line)?;
                    i = next_i;
                } else {
                    let (name, axis) = parse_visual_table_ref(table_ref, line)?;
                    let (table, next_i) = parse_visual_shape_table(lines, i, &axis, None, catalog)?;
                    shapes.insert(name, table);
                    i = next_i;
                }
            }
            [table_ref, "rotate", "from", from] => {
                let (name, axis) = parse_visual_table_ref(table_ref, line)?;
                let rotation = VisualShapeRotation::intrinsic(from);
                let (table, next_i) =
                    parse_visual_shape_table(lines, i, &axis, Some(rotation), catalog)?;
                shapes.insert(name, table);
                i = next_i;
            }
            [table_ref, "rotate", "using", map, "from", from] => {
                let (name, axis) = parse_visual_table_ref(table_ref, line)?;
                let rotation = VisualShapeRotation::using(map, from);
                let (table, next_i) =
                    parse_visual_shape_table(lines, i, &axis, Some(rotation), catalog)?;
                shapes.insert(name, table);
                i = next_i;
            }
            [table_ref, "rotate", map, "from", from] => {
                let (name, axis) = parse_visual_table_ref(table_ref, line)?;
                let rotation = VisualShapeRotation::using(map, from);
                let (table, next_i) =
                    parse_visual_shape_table(lines, i, &axis, Some(rotation), catalog)?;
                shapes.insert(name, table);
                i = next_i;
            }
            _ => {
                return Err(parse_error(
                    line,
                    "shape row must be: <name> | <name>:<tag_set>",
                ));
            }
        }
    }
    if i >= lines.len() {
        return Err(parse_error(&lines[start], "shapes missing closing brace"));
    }
    Ok(i + 1)
}

#[allow(clippy::too_many_arguments)]
fn parse_sprite_entry(
    lines: &[String],
    start: usize,
    plain_shapes: &HashMap<String, Vec<String>>,
    shapes: &HashMap<String, VisualShapeTable>,
    color_aliases: &HashMap<String, String>,
    color_tables: &HashMap<String, VisualColorTable>,
    catalog: &Catalog,
    visuals: &mut VisualsDef,
) -> Result<usize, DiagnosticReport> {
    let (body_lines, next_i) =
        collect_braced_body_lines(lines, start, "sprite entry missing closing brace")?;

    let mut entry = SpriteEntrySpec::new(&lines[start], None);
    apply_braced_sprite_attachment_body(&mut entry, &body_lines)?;
    lower_sprite_entry(
        entry,
        plain_shapes,
        shapes,
        color_aliases,
        color_tables,
        catalog,
        visuals,
    )?;
    Ok(next_i)
}

struct SpriteAttachmentEntry {
    source_line: String,
    selector: String,
    body_lines: Vec<String>,
    braced: bool,
    next_i: usize,
}

#[allow(clippy::too_many_arguments)]
fn parse_sprite_attachment_entry(
    lines: &[String],
    start: usize,
    plain_shapes: &HashMap<String, Vec<String>>,
    shapes: &HashMap<String, VisualShapeTable>,
    color_aliases: &HashMap<String, String>,
    color_tables: &HashMap<String, VisualColorTable>,
    catalog: &Catalog,
    visuals: &mut VisualsDef,
) -> Result<usize, DiagnosticReport> {
    let attachment = collect_sprite_attachment_entry(lines, start)?;
    let mut entry = SpriteEntrySpec::new(&attachment.source_line, None);
    entry.selector = Some(attachment.selector);
    if attachment.braced {
        apply_braced_sprite_attachment_body(&mut entry, &attachment.body_lines)?;
    } else {
        apply_unbraced_sprite_attachment_body(&mut entry, &attachment.body_lines)?;
    }
    lower_sprite_entry(
        entry,
        plain_shapes,
        shapes,
        color_aliases,
        color_tables,
        catalog,
        visuals,
    )?;
    Ok(attachment.next_i)
}

fn collect_sprite_attachment_entry(
    lines: &[String],
    start: usize,
) -> Result<SpriteAttachmentEntry, DiagnosticReport> {
    let source_line = lines[start].clone();
    let header = block_header_text(&source_line);
    let tokens = split_header_tokens(header);
    let [selector] = tokens.as_slice() else {
        return Err(parse_error(
            &source_line,
            "sprite attachment header must be one selector",
        ));
    };
    let selector = (*selector).to_string();
    if is_block_header_line(&source_line) {
        let (body_lines, next_i) =
            collect_braced_body_lines(lines, start, "sprite attachment missing closing brace")?;
        return Ok(SpriteAttachmentEntry {
            source_line,
            selector,
            body_lines,
            braced: true,
            next_i,
        });
    }

    let mut body_lines = Vec::new();
    let mut i = start + 1;
    let mut nested_depth = 0i32;
    while i < lines.len() {
        if is_block_close_line(&lines[i]) && nested_depth == 0 {
            break;
        }
        if split_header_tokens(&lines[i]).is_empty() {
            if nested_depth > 0 {
                body_lines.push(lines[i].clone());
                i += 1;
                continue;
            }
            break;
        }
        if is_block_close_line(&lines[i]) {
            nested_depth -= 1;
        }
        body_lines.push(lines[i].clone());
        if is_block_header_line(&lines[i]) {
            nested_depth += 1;
        }
        i += 1;
    }
    Ok(SpriteAttachmentEntry {
        source_line,
        selector,
        body_lines,
        braced: false,
        next_i: i,
    })
}

fn collect_braced_body_lines(
    lines: &[String],
    start: usize,
    missing_close_message: &str,
) -> Result<(Vec<String>, usize), DiagnosticReport> {
    let mut body_lines = Vec::new();
    let mut depth = 0usize;
    let mut i = start + 1;
    while i < lines.len() {
        if is_block_close_line(&lines[i]) {
            if depth == 0 {
                return Ok((body_lines, i + 1));
            }
            depth -= 1;
            body_lines.push(lines[i].clone());
            i += 1;
            continue;
        }
        body_lines.push(lines[i].clone());
        if is_block_header_line(&lines[i]) {
            depth += 1;
        }
        i += 1;
    }
    Err(parse_error(&lines[start], missing_close_message))
}

fn apply_braced_sprite_attachment_body(
    entry: &mut SpriteEntrySpec,
    body_lines: &[String],
) -> Result<(), DiagnosticReport> {
    apply_unbraced_sprite_attachment_body(entry, body_lines)
}

fn push_sprite_animation_frame(
    frames: &mut Vec<Vec<String>>,
    frame: &mut Vec<String>,
    line: &str,
) -> Result<(), DiagnosticReport> {
    if frame.is_empty() {
        return Err(parse_error(
            line,
            "sprite animation frame requires at least one row",
        ));
    }
    validate_visual_pattern(frame, line)?;
    frames.push(std::mem::take(frame));
    Ok(())
}

fn validate_sprite_animation_frames(
    frames: &[Vec<String>],
    line: &str,
) -> Result<(), DiagnosticReport> {
    if frames.len() < 2 {
        return Err(parse_error(
            line,
            "sprite animation requires at least two frames",
        ));
    }
    let width = frames[0][0].chars().count();
    let height = frames[0].len();
    if frames
        .iter()
        .any(|frame| frame.len() != height || frame[0].chars().count() != width)
    {
        return Err(parse_error(
            line,
            "sprite animation frames must have the same size",
        ));
    }
    Ok(())
}

fn apply_sprite_ascii_frames(
    entry: &mut SpriteEntrySpec,
    mut frames: Vec<Vec<String>>,
    mut frame: Vec<String>,
    has_frame_separator: bool,
    source_line: &str,
) -> Result<(), DiagnosticReport> {
    if !has_frame_separator {
        if !frame.is_empty() {
            validate_visual_pattern(&frame, source_line)?;
            entry.inline_pattern = Some(frame);
        }
        return Ok(());
    }
    push_sprite_animation_frame(&mut frames, &mut frame, source_line)?;
    validate_sprite_animation_frames(&frames, source_line)?;
    let frame_count = u64::try_from(frames.len())
        .map_err(|_| parse_error(source_line, "sprite animation has too many frames"))?;
    let frame_duration_ms = entry.loop_frame_duration_ms;
    let duration_ms = match (entry.loop_duration_ms, frame_duration_ms) {
        (Some(duration_ms), Some(frame_duration_ms)) => {
            let expected_duration_ms = frame_duration_ms.checked_mul(frame_count).ok_or_else(|| {
                parse_error(source_line, "sprite frame_duration is too large")
            })?;
            if duration_ms != expected_duration_ms {
                return Err(parse_error(
                    source_line,
                    "sprite duration must equal frame_duration multiplied by frame count",
                ));
            }
            duration_ms
        }
        (Some(duration_ms), None) => duration_ms,
        (None, Some(frame_duration_ms)) => frame_duration_ms
            .checked_mul(frame_count)
            .ok_or_else(|| parse_error(source_line, "sprite frame_duration is too large"))?,
        (None, None) => {
            return Err(parse_error(
                source_line,
                "sprite animation missing duration or frame_duration",
            ));
        }
    };
    entry.inline_pattern = Some(frames[0].clone());
    entry.loop_animation = Some(VisualSpriteLoopDef {
        duration_ms,
        frames,
    });
    Ok(())
}

fn apply_unbraced_sprite_attachment_body(
    entry: &mut SpriteEntrySpec,
    body_lines: &[String],
) -> Result<(), DiagnosticReport> {
    let mut frames = Vec::<Vec<String>>::new();
    let mut frame = Vec::<String>::new();
    let mut has_frame_separator = false;
    for line in body_lines {
        if let Some(rotation) = parse_visual_shape_rotation_directive(line)? {
            if entry.rotation.is_some() {
                return Err(parse_error(line, "duplicate sprite rotation"));
            }
            entry.rotation = Some(rotation);
            continue;
        }
        let tokens = split_header_tokens(line);
        if tokens.is_empty() {
            continue;
        }
        match tokens.as_slice() {
            ["selector", "=", selector] | ["selector", selector] => {
                if entry.selector.is_some() {
                    return Err(parse_error(line, "duplicate sprite selector"));
                }
                entry.selector = Some((*selector).to_string());
            }
            ["image", "=", source] | ["image", source] => {
                entry.set_image(source, line)?;
            }
            ["offset", "=", x, y] | ["offset", x, y] => {
                entry.set_offset(x, y, line)?;
            }
            ["sampling", "=", value] | ["sampling", value] => {
                entry.set_sampling(value, line)?;
            }
            ["duration", "=", value] | ["duration", value] => {
                entry.set_duration(value, line)?;
            }
            ["frame_duration", "=", value] | ["frame_duration", value] => {
                entry.set_frame_duration(value, line)?;
            }
            ["shape", "="] => {
                if !frame.is_empty() || has_frame_separator {
                    return Err(parse_error(line, "sprite shape must precede ASCII rows"));
                }
            }
            ["shape", value] => {
                if !frame.is_empty() || has_frame_separator {
                    return Err(parse_error(line, "sprite shape must precede ASCII rows"));
                }
                entry.set_shape_ref(value, line)?;
            }
            ["shape", "=", value] => {
                if !frame.is_empty() || has_frame_separator {
                    return Err(parse_error(line, "sprite shape must precede ASCII rows"));
                }
                entry.set_shape_ref(value, line)?;
            }
            ["colors", "=", values @ ..] => {
                if !frame.is_empty() || has_frame_separator {
                    return Err(parse_error(line, "sprite colors must precede ASCII rows"));
                }
                if values.is_empty() {
                    return Err(parse_error(line, "sprite colors row must name colors"));
                }
                entry.color_exprs = Some(visual_colors_from_tokens(values, line)?);
            }
            ["colors", values @ ..] => {
                if !frame.is_empty() || has_frame_separator {
                    return Err(parse_error(line, "sprite colors must precede ASCII rows"));
                }
                if values.is_empty() {
                    return Err(parse_error(line, "sprite colors row must name colors"));
                }
                entry.color_exprs = Some(visual_colors_from_tokens(values, line)?);
            }
            [value]
                if entry.color_exprs.is_some()
                    && frame.is_empty()
                    && !has_frame_separator
                    && is_sprite_shorthand_duration_token(value) =>
            {
                entry.set_duration(value, line)?;
            }
            [">"] => {
                has_frame_separator = true;
                push_sprite_animation_frame(&mut frames, &mut frame, line)?;
            }
            [property, "=", ..] => {
                return Err(parse_error(
                    line,
                    &format!("unknown sprite property {property}"),
                ));
            }
            _ if entry.color_exprs.is_none() => {
                entry.color_exprs = Some(visual_colors_from_tokens(&tokens, line)?);
            }
            [row] => {
                frame.push((*row).to_string());
            }
            _ => {
                return Err(parse_error(
                    line,
                    "sprite ASCII row must be a single token row",
                ));
            }
        }
    }
    let source_line = entry.source_line.clone();
    apply_sprite_ascii_frames(entry, frames, frame, has_frame_separator, &source_line)
}

fn is_sprite_shorthand_duration_token(value: &str) -> bool {
    let Some(number) = value
        .strip_suffix("ms")
        .or_else(|| value.strip_suffix('s'))
    else {
        return false;
    };
    !number.is_empty() && number.chars().any(|ch| ch.is_ascii_digit())
}

#[allow(clippy::too_many_arguments)]
fn lower_sprite_entry(
    mut entry: SpriteEntrySpec,
    plain_shapes: &HashMap<String, Vec<String>>,
    shapes: &HashMap<String, VisualShapeTable>,
    color_aliases: &HashMap<String, String>,
    color_tables: &HashMap<String, VisualColorTable>,
    catalog: &Catalog,
    visuals: &mut VisualsDef,
) -> Result<(), DiagnosticReport> {
    let selector = entry.selector()?.to_string();
    let line = entry.source_line.as_str();
    if entry.shape_ref.is_none()
        && entry.rotation.is_none()
        && entry
            .inline_pattern
            .as_ref()
            .is_some_and(|pattern| pattern.len() == 1)
    {
        let candidate = entry.inline_pattern.as_ref().unwrap()[0].as_str();
        if visual_shape_ref_exists(candidate, plain_shapes, shapes) {
            entry.shape_ref = Some(parse_sprite_shape_ref(candidate, line)?);
            entry.inline_pattern = None;
        }
    }
    if let Some(source) = entry.image_source {
        if entry.color_exprs.is_some()
            || entry.inline_pattern.is_some()
            || entry.shape_ref.is_some()
            || entry.loop_animation.is_some()
            || entry.rotation.is_some()
        {
            return Err(parse_error(
                line,
                "image sprite cannot also define ASCII colors, shape, loop, or rotate",
            ));
        }
        add_image_visuals(
            &selector,
            line,
            &source,
            entry.offset,
            entry.sampling,
            catalog,
            visuals,
        )?;
    } else if let Some(rotation) = entry.rotation {
        let color_exprs = entry
            .color_exprs
            .ok_or_else(|| parse_error(line, "sprite entry missing colors"))?;
        validate_loop_animation_palette(entry.loop_animation.as_ref(), &color_exprs, line)?;
        let Some(pattern) = entry.inline_pattern else {
            return Err(parse_error(line, "sprite rotation requires inline ASCII rows"));
        };
        validate_visual_pattern_palette(&pattern, &color_exprs, line)?;
        let targets = expand_visual_selector(&selector, line, catalog)?;
        let axis = visual_rotation_axis_for_targets(&targets, catalog, &rotation, line)?;
        let mut entries = HashMap::new();
        entries.insert(rotation.from.clone(), pattern);
        let values = catalog_value_set(catalog, &axis)
            .ok_or_else(|| parse_error(line, "visual rotation tag set must exist"))?;
        expand_visual_shape_rotations(&mut entries, values, catalog, &axis, &rotation, line)?;
        let shape = VisualShapeTable { axis, entries };
        add_ascii_visuals(
            &selector,
            line,
            &shape,
            &ValueExpr::Binding(shape.axis.clone()),
            &color_exprs,
            entry.offset,
            entry.sampling,
            entry.loop_animation,
            color_aliases,
            color_tables,
            catalog,
            visuals,
        )?;
    } else if let Some((shape_name, shape_value)) = entry.shape_ref {
        let color_exprs = entry
            .color_exprs
            .ok_or_else(|| parse_error(line, "sprite entry missing colors"))?;
        validate_loop_animation_palette(entry.loop_animation.as_ref(), &color_exprs, line)?;
        if let Some(shape) = shapes.get(&shape_name) {
            add_ascii_visuals(
                &selector,
                line,
                shape,
                &shape_value,
                &color_exprs,
                entry.offset,
                entry.sampling,
                entry.loop_animation,
                color_aliases,
                color_tables,
                catalog,
                visuals,
            )?;
        } else {
            let pattern = plain_shapes
                .get(&shape_name)
                .ok_or_else(|| parse_error(line, "unknown sprite shape"))?;
            add_inline_ascii_visuals(
                &selector,
                line,
                pattern,
                &color_exprs,
                entry.offset,
                entry.sampling,
                entry.loop_animation,
                color_aliases,
                color_tables,
                catalog,
                visuals,
            )?;
        }
    } else if let Some(pattern) = entry.inline_pattern {
        let color_exprs = entry
            .color_exprs
            .ok_or_else(|| parse_error(line, "sprite entry missing colors"))?;
        validate_loop_animation_palette(entry.loop_animation.as_ref(), &color_exprs, line)?;
        add_inline_ascii_visuals(
            &selector,
            line,
            &pattern,
            &color_exprs,
            entry.offset,
            entry.sampling,
            entry.loop_animation,
            color_aliases,
            color_tables,
            catalog,
            visuals,
        )?;
    } else {
        let color_exprs = entry
            .color_exprs
            .ok_or_else(|| parse_error(line, "sprite entry missing colors"))?;
        let [(_, color)] = color_exprs.as_slice() else {
            return Err(parse_error(line, "solid sprite requires exactly one color"));
        };
        validate_loop_animation_palette(entry.loop_animation.as_ref(), &color_exprs, line)?;
        add_solid_visuals(
            &selector,
            line,
            color,
            entry.offset,
            entry.sampling,
            entry.loop_animation,
            color_aliases,
            color_tables,
            catalog,
            visuals,
        )?;
    }
    Ok(())
}

fn visual_colors_from_tokens(
    tokens: &[&str],
    line: &str,
) -> Result<Vec<(char, String)>, DiagnosticReport> {
    tokens
        .iter()
        .enumerate()
        .map(|(index, color)| {
            let token = visual_color_token_for_index(index)
                .ok_or_else(|| parse_error(line, "sprite supports at most 62 colors"))?;
            Ok((token, (*color).to_string()))
        })
        .collect()
}

fn parse_cell_offset_value(value: &str, line: &str, label: &str) -> Result<f64, DiagnosticReport> {
    if let Some((numerator, denominator)) = value.split_once('/') {
        let numerator = numerator
            .parse::<f64>()
            .map_err(|_| parse_error(line, &format!("{label} must be a number or fraction")))?;
        let denominator = denominator
            .parse::<f64>()
            .map_err(|_| parse_error(line, &format!("{label} must be a number or fraction")))?;
        if denominator == 0.0 {
            return Err(parse_error(line, &format!("{label} denominator must not be zero")));
        }
        return Ok(numerator / denominator);
    }
    value
        .parse::<f64>()
        .map_err(|_| parse_error(line, &format!("{label} must be a number or fraction")))
}

fn parse_sprite_sampling(
    value: &str,
    line: &str,
) -> Result<VisualSpriteSampling, DiagnosticReport> {
    match value {
        "pixelated" => Ok(VisualSpriteSampling::Pixelated),
        "smooth" => Ok(VisualSpriteSampling::Smooth),
        _ => Err(parse_error(
            line,
            "sprite sampling must be pixelated or smooth",
        )),
    }
}

fn parse_sprite_image_path(value: &str, line: &str) -> Result<String, DiagnosticReport> {
    let path = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .ok_or_else(|| parse_error(line, "sprite image path must be quoted"))?;
    if path.is_empty() {
        return Err(parse_error(line, "sprite image path must not be empty"));
    }
    if path.starts_with('/')
        || path.contains('\\')
        || path.split('/').any(|part| part == "..")
        || path.contains("://")
    {
        return Err(parse_error(
            line,
            "sprite image path must be a game-folder relative path",
        ));
    }
    if !is_visual_image_source(path) {
        return Err(parse_error(
            line,
            "sprite image must use .png, .jpg, .jpeg, or .svg",
        ));
    }
    Ok(path.to_string())
}

fn visual_rotation_axis_for_targets(
    targets: &[VisualSelectorTarget],
    catalog: &Catalog,
    rotation: &VisualShapeRotation,
    line: &str,
) -> Result<String, DiagnosticReport> {
    let first = targets
        .first()
        .ok_or_else(|| parse_error(line, "visual selector matched no objects"))?;
    let mut candidates = first
        .bindings
        .keys()
        .filter(|axis| {
            catalog_value_set(catalog, axis)
                .is_some_and(|values| values.iter().any(|value| value == &rotation.from))
        })
        .cloned()
        .collect::<Vec<_>>();
    candidates.retain(|axis| {
        targets
            .iter()
            .all(|target| target.bindings.contains_key(axis))
    });
    let [axis] = candidates.as_slice() else {
        return Err(parse_error(
            line,
            "sprite rotation requires exactly one matching selector tag set",
        ));
    };
    Ok(axis.clone())
}

fn visual_table_key<T>(
    expr: &ValueExpr,
    axis: &str,
    entries: &HashMap<String, T>,
    bindings: &HashMap<String, String>,
    maps: &HashMap<String, ValueMap>,
    line: &str,
) -> Result<String, DiagnosticReport> {
    if let ValueExpr::Binding(name) = expr {
        if let Some(value) = bindings.get(name) {
            return Ok(value.clone());
        }
        if name == axis
            && let Some(value) = bindings.get(axis)
        {
            return Ok(value.clone());
        }
        if entries.contains_key(name) {
            return Ok(name.clone());
        }
    }
    let env = visual_value_env(bindings);
    if value_expr_result_axis(expr, &env, maps, line)? != axis {
        return Err(parse_error(line, "visual table tag set mismatch"));
    }
    eval_bound_value_expr(expr, &env, maps, line)
}

fn parse_visual_plain_shape(
    lines: &[String],
    start: usize,
) -> Result<(Vec<String>, usize), DiagnosticReport> {
    let is_braced = is_block_header_line(&lines[start]);
    let mut pattern = Vec::new();
    let mut i = start + 1;
    let mut width = None::<usize>;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        if lines[i].is_empty() {
            if is_braced {
                i += 1;
                continue;
            }
            break;
        }
        let row_tokens = split_header_tokens(&lines[i]);
        let [row] = row_tokens.as_slice() else {
            return Err(parse_error(
                &lines[i],
                "visual shape row must be a single token row",
            ));
        };
        let row_width = row.chars().count();
        if !is_braced
            && let Some(expected_width) = width
            && row_width != expected_width
        {
            return Err(parse_error(
                &lines[i],
                "visual shape rows must be equal-width ascii",
            ));
        }
        width = Some(row_width);
        pattern.push((*row).to_string());
        i += 1;
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start],
            "visual shape missing closing brace",
        ));
    }
    validate_visual_pattern(&pattern, &lines[start])?;
    let next_i = if is_braced { i + 1 } else { i };
    Ok((pattern, next_i))
}

fn parse_sprite_shape_ref(
    shape_ref: &str,
    line: &str,
) -> Result<(String, ValueExpr), DiagnosticReport> {
    let (shape_name, shape_value) = if shape_ref.contains(':') {
        parse_visual_table_expr(shape_ref, line)?
    } else {
        (shape_ref.to_string(), ValueExpr::Binding(String::new()))
    };
    Ok((shape_name, shape_value))
}

fn visual_shape_ref_exists(
    shape_ref: &str,
    plain_shapes: &HashMap<String, Vec<String>>,
    shapes: &HashMap<String, VisualShapeTable>,
) -> bool {
    if let Some((shape_name, _)) = shape_ref.split_once(':') {
        shapes.contains_key(shape_name)
    } else {
        plain_shapes.contains_key(shape_ref) || shapes.contains_key(shape_ref)
    }
}

fn validate_visual_pattern_palette(
    pattern: &[String],
    color_exprs: &[(char, String)],
    line: &str,
) -> Result<(), DiagnosticReport> {
    let colors = color_exprs
        .iter()
        .map(|(token, _)| *token)
        .collect::<HashSet<_>>();
    for row in pattern {
        for token in row.chars() {
            if token == '.' || colors.contains(&token) {
                continue;
            }
            return Err(parse_error(
                line,
                "sprite pattern references a color outside the color row",
            ));
        }
    }
    Ok(())
}

fn validate_loop_animation_palette(
    loop_animation: Option<&VisualSpriteLoopDef>,
    color_exprs: &[(char, String)],
    line: &str,
) -> Result<(), DiagnosticReport> {
    let Some(loop_animation) = loop_animation else {
        return Ok(());
    };
    for frame in &loop_animation.frames {
        validate_visual_pattern_palette(frame, color_exprs, line)?;
    }
    Ok(())
}

pub(crate) fn is_visual_color_token(value: &str) -> bool {
    value.starts_with('#') || crate::syntax::is_visual_named_color(value)
}

fn is_visual_image_source(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.ends_with(".png")
        || lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".svg")
}

pub(crate) fn visual_color_token_for_index(index: usize) -> Option<char> {
    const TOKENS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    TOKENS.get(index).map(|token| *token as char)
}

fn parse_visual_table_ref(value: &str, line: &str) -> Result<(String, String), DiagnosticReport> {
    let Some((name, axis)) = value.split_once(':') else {
        return Err(parse_error(line, "visual table must be: <name>:<tag_set>"));
    };
    if !is_identifier(name) {
        return Err(parse_error(line, "visual table name must be an identifier"));
    }
    if !is_identifier(axis) {
        return Err(parse_error(
            line,
            "visual table tag set must be an identifier",
        ));
    }
    Ok((name.to_string(), axis.to_string()))
}

fn parse_visual_table_expr(
    value: &str,
    line: &str,
) -> Result<(String, ValueExpr), DiagnosticReport> {
    let Some((name, value)) = value.split_once(':') else {
        return Err(parse_error(
            line,
            "visual table must be: <name>:<value-expr>",
        ));
    };
    if !is_identifier(name) {
        return Err(parse_error(line, "visual table name must be an identifier"));
    }
    Ok((name.to_string(), parse_value_expr(value, line)?))
}

fn parse_visual_shape_value_ref(
    value: &str,
    line: &str,
    catalog: &Catalog,
) -> Result<Option<(String, String, String)>, DiagnosticReport> {
    let Some((name, value)) = value.split_once(':') else {
        return Ok(None);
    };
    if !is_identifier(name) {
        return Err(parse_error(line, "visual table name must be an identifier"));
    }
    if catalog_value_set(catalog, value).is_some() {
        return Ok(None);
    }
    if !is_identifier(value) {
        return Ok(None);
    }
    let axis = infer_visual_shape_value_axis(value, line, catalog)?;
    Ok(Some((name.to_string(), axis, value.to_string())))
}

fn infer_visual_shape_value_axis(
    value: &str,
    line: &str,
    catalog: &Catalog,
) -> Result<String, DiagnosticReport> {
    let axes = catalog_value_sets(catalog)
        .into_iter()
        .filter_map(|(axis, values)| {
            values
                .iter()
                .any(|candidate| candidate == value)
                .then_some(axis)
        })
        .collect::<Vec<_>>();
    let [axis] = axes.as_slice() else {
        return Err(parse_error(
            line,
            "visual shape value must belong to exactly one tag set",
        ));
    };
    Ok(axis.clone())
}

fn insert_visual_shape_value(
    shapes: &mut HashMap<String, VisualShapeTable>,
    name: String,
    axis: String,
    value: String,
    pattern: Vec<String>,
    line: &str,
) -> Result<(), DiagnosticReport> {
    let table = shapes.entry(name).or_insert_with(|| VisualShapeTable {
        axis: axis.clone(),
        entries: HashMap::new(),
    });
    if table.axis != axis {
        return Err(parse_error(line, "visual shape tag set mismatch"));
    }
    if table.entries.insert(value, pattern).is_some() {
        return Err(parse_error(line, "duplicate visual shape value"));
    }
    Ok(())
}

fn parse_visual_shape_value_pattern(
    lines: &[String],
    start: usize,
    table_values: &[String],
    stop_on_table_value: bool,
) -> Result<(Vec<String>, usize), DiagnosticReport> {
    let is_braced = is_block_header_line(&lines[start]);
    let mut pattern = Vec::new();
    let mut i = start + 1;
    while i < lines.len() {
        if is_block_close_line(&lines[i]) {
            if is_braced {
                validate_visual_pattern(&pattern, &lines[start])?;
                return Ok((pattern, i + 1));
            }
            break;
        }
        if !is_braced {
            if lines[i].is_empty() {
                if pattern.is_empty() {
                    i += 1;
                    continue;
                }
                break;
            }
            if !pattern.is_empty()
                && stop_on_table_value
                && is_visual_shape_table_value_header(&lines[i], table_values)
            {
                break;
            }
            if !pattern.is_empty() && is_visual_shape_individual_value_header(&lines[i]) {
                break;
            }
        }
        let row_tokens = split_header_tokens(&lines[i]);
        let [row] = row_tokens.as_slice() else {
            return Err(parse_error(
                &lines[i],
                "visual shape row must be a single token row",
            ));
        };
        pattern.push((*row).to_string());
        i += 1;
    }
    if is_braced {
        return Err(parse_error(
            &lines[start],
            "visual shape value missing closing brace",
        ));
    }
    validate_visual_pattern(&pattern, &lines[start])?;
    Ok((pattern, i))
}

fn is_visual_shape_table_value_header(line: &str, values: &[String]) -> bool {
    let tokens = split_header_tokens(line);
    let [value] = tokens.as_slice() else {
        return false;
    };
    values.iter().any(|candidate| candidate == value)
}

fn is_visual_shape_individual_value_header(line: &str) -> bool {
    let tokens = split_header_tokens(line);
    let [value] = tokens.as_slice() else {
        return false;
    };
    value
        .split_once(':')
        .is_some_and(|(name, value)| is_identifier(name) && is_identifier(value))
}

fn parse_visual_shape_table(
    lines: &[String],
    start: usize,
    axis: &str,
    rotation: Option<VisualShapeRotation>,
    catalog: &Catalog,
) -> Result<(VisualShapeTable, usize), DiagnosticReport> {
    let values = catalog_value_set(catalog, axis).ok_or_else(|| {
        parse_error(
            &lines[start],
            "visual shape tag set must name an existing tag set",
        )
    })?;
    let mut entries = HashMap::new();
    let mut i = start + 1;
    if let Some(rotation) = rotation {
        let mut pattern = Vec::new();
        while i < lines.len() && !is_block_close_line(&lines[i]) {
            let row_tokens = split_header_tokens(&lines[i]);
            let [row] = row_tokens.as_slice() else {
                return Err(parse_error(
                    &lines[i],
                    "visual shape row must be a single token row",
                ));
            };
            pattern.push((*row).to_string());
            i += 1;
        }
        if i >= lines.len() {
            return Err(parse_error(
                &lines[start],
                "visual shape missing closing brace",
            ));
        }
        validate_visual_pattern(&pattern, &lines[i])?;
        entries.insert(rotation.from.clone(), pattern);
        expand_visual_shape_rotations(
            &mut entries,
            values,
            catalog,
            axis,
            &rotation,
            &lines[start],
        )?;
        return Ok((
            VisualShapeTable {
                axis: axis.to_string(),
                entries,
            },
            i + 1,
        ));
    }
    let mut rotation = None::<VisualShapeRotation>;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        if lines[i].is_empty() {
            i += 1;
            continue;
        }
        if let Some(parsed_rotation) = parse_visual_shape_rotation_directive(&lines[i])? {
            if rotation.is_some() {
                return Err(parse_error(&lines[i], "duplicate visual shape rotation"));
            }
            if lines[i].trim_end().ends_with('{') {
                let mut pattern = Vec::new();
                i += 1;
                while i < lines.len() && !is_block_close_line(&lines[i]) {
                    let row_tokens = split_header_tokens(&lines[i]);
                    let [row] = row_tokens.as_slice() else {
                        return Err(parse_error(
                            &lines[i],
                            "visual shape row must be a single token row",
                        ));
                    };
                    pattern.push((*row).to_string());
                    i += 1;
                }
                if i >= lines.len() {
                    return Err(parse_error(
                        &lines[start],
                        "visual shape rotation missing closing brace",
                    ));
                }
                validate_visual_pattern(&pattern, &lines[i])?;
                if entries
                    .insert(parsed_rotation.from.clone(), pattern)
                    .is_some()
                {
                    return Err(parse_error(
                        &lines[i],
                        "visual shape rotation source duplicates explicit shape value",
                    ));
                }
                rotation = Some(parsed_rotation);
                i += 1;
                continue;
            }
            if !entries.contains_key(&parsed_rotation.from) {
                let mut pattern = Vec::new();
                i += 1;
                while i < lines.len() && !is_block_close_line(&lines[i]) {
                    let row_tokens = split_header_tokens(&lines[i]);
                    let [row] = row_tokens.as_slice() else {
                        return Err(parse_error(
                            &lines[i],
                            "visual shape row must be a single token row",
                        ));
                    };
                    pattern.push((*row).to_string());
                    i += 1;
                }
                if i >= lines.len() {
                    return Err(parse_error(
                        &lines[start],
                        "visual shape rotation missing closing brace",
                    ));
                }
                validate_visual_pattern(&pattern, &lines[i])?;
                entries.insert(parsed_rotation.from.clone(), pattern);
                rotation = Some(parsed_rotation);
                continue;
            }
            rotation = Some(parsed_rotation);
            i += 1;
            continue;
        }
        let value = block_header_text(&lines[i]);
        if !values.iter().any(|candidate| candidate == value) {
            return Err(parse_error(
                &lines[i],
                "visual shape value is not in tag set",
            ));
        }
        let (pattern, next_i) = parse_visual_shape_value_pattern(lines, i, values, true)?;
        if entries.insert(value.to_string(), pattern).is_some() {
            return Err(parse_error(&lines[i], "duplicate visual shape value"));
        }
        i = next_i;
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start],
            "visual shape missing closing brace",
        ));
    }
    if let Some(rotation) = rotation {
        expand_visual_shape_rotations(
            &mut entries,
            values,
            catalog,
            axis,
            &rotation,
            &lines[start],
        )?;
    }
    Ok((
        VisualShapeTable {
            axis: axis.to_string(),
            entries,
        },
        i + 1,
    ))
}

fn parse_visual_shape_rotation_directive(
    line: &str,
) -> Result<Option<VisualShapeRotation>, DiagnosticReport> {
    let tokens = split_header_tokens(block_header_text(line));
    match tokens.as_slice() {
        ["rotate", "from", from] => Ok(Some(VisualShapeRotation::intrinsic(from))),
        ["rotate", "using", map, "from", from] => Ok(Some(VisualShapeRotation::using(map, from))),
        ["rotate", map, "from", from] => Ok(Some(VisualShapeRotation::using(map, from))),
        ["rotate", ..] => Err(parse_error(
            line,
            "visual shape rotation must be: rotate from <value> | rotate using <map> from <value>",
        )),
        _ => Ok(None),
    }
}

fn validate_visual_pattern(pattern: &[String], line: &str) -> Result<(), DiagnosticReport> {
    if pattern.is_empty() {
        return Err(parse_error(
            line,
            "visual shape value requires at least one row",
        ));
    }
    let width = pattern[0].chars().count();
    if width == 0
        || pattern
            .iter()
            .any(|row| row.chars().count() != width || !row.is_ascii())
    {
        return Err(parse_error(
            line,
            "visual shape rows must be equal-width ascii",
        ));
    }
    if pattern.iter().any(|row| row.contains(['{', '}'])) {
        return Err(parse_error(line, "ASCII rows cannot contain braces"));
    }
    Ok(())
}

fn expand_visual_shape_rotations(
    entries: &mut HashMap<String, Vec<String>>,
    values: &[String],
    catalog: &Catalog,
    axis: &str,
    rotation: &VisualShapeRotation,
    line: &str,
) -> Result<(), DiagnosticReport> {
    if !values.iter().any(|value| value == &rotation.from) {
        return Err(parse_error(
            line,
            "visual rotation source is not in tag set",
        ));
    }
    let rotation_values = visual_rotation_values(values, catalog, axis, rotation, line)?;
    let mut value = rotation.from.clone();
    let mut pattern = entries
        .get(&value)
        .cloned()
        .ok_or_else(|| parse_error(line, "visual rotation source shape missing"))?;
    let mut visited = Vec::new();

    loop {
        if visited.iter().any(|seen| seen == &value) {
            break;
        }
        visited.push(value.clone());
        let next = rotation_values
            .get(&value)
            .ok_or_else(|| parse_error(line, "visual rotation map value missing"))?
            .clone();
        let next_pattern = rotate_visual_pattern_clockwise(&pattern);
        if next == rotation.from {
            break;
        }
        if let Some(existing) = entries.get(&next) {
            if existing != &next_pattern {
                return Err(parse_error(
                    line,
                    "visual rotation conflicts with explicit shape value",
                ));
            }
        } else {
            entries.insert(next.clone(), next_pattern.clone());
        }
        value = next;
        pattern = next_pattern;
    }

    if visited.len() != values.len() || values.iter().any(|value| !entries.contains_key(value)) {
        return Err(parse_error(
            line,
            "visual rotation map must cycle through every shape tag value",
        ));
    }
    Ok(())
}

fn visual_rotation_values(
    values: &[String],
    catalog: &Catalog,
    axis: &str,
    rotation: &VisualShapeRotation,
    line: &str,
) -> Result<HashMap<String, String>, DiagnosticReport> {
    if let Some(map_name) = &rotation.map {
        let map = catalog
            .maps
            .get(map_name)
            .ok_or_else(|| parse_error(line, "unknown visual rotation map"))?;
        if map.axis != axis {
            return Err(parse_error(line, "visual rotation map tag set mismatch"));
        }
        return Ok(map.values.clone());
    }

    intrinsic_cardinal_visual_rotation_values(values, line)
}

fn intrinsic_cardinal_visual_rotation_values(
    values: &[String],
    line: &str,
) -> Result<HashMap<String, String>, DiagnosticReport> {
    const CARDINAL_ROTATION: [(&str, &str); 4] = [
        ("up", "right"),
        ("right", "down"),
        ("down", "left"),
        ("left", "up"),
    ];
    if values.len() != CARDINAL_ROTATION.len()
        || !CARDINAL_ROTATION
            .iter()
            .all(|(value, _)| values.iter().any(|candidate| candidate == value))
    {
        return Err(parse_error(
            line,
            "visual rotation without a map requires tag values up, right, down, left",
        ));
    }
    Ok(CARDINAL_ROTATION
        .into_iter()
        .map(|(from, to)| (from.to_string(), to.to_string()))
        .collect())
}

fn rotate_visual_pattern_clockwise(pattern: &[String]) -> Vec<String> {
    let rows = pattern
        .iter()
        .map(|row| row.chars().collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let height = rows.len();
    let width = rows.first().map_or(0, Vec::len);
    let mut rotated = Vec::with_capacity(width);
    for x in 0..width {
        let mut row = String::with_capacity(height);
        for y in (0..height).rev() {
            row.push(rows[y][x]);
        }
        rotated.push(row);
    }
    rotated
}

fn parse_visual_color_table(
    lines: &[String],
    start: usize,
    axis: &str,
    catalog: &Catalog,
) -> Result<(VisualColorTable, usize), DiagnosticReport> {
    let values = catalog_value_set(catalog, axis).ok_or_else(|| {
        parse_error(
            &lines[start],
            "visual colors tag set must name an existing tag set",
        )
    })?;
    let mut entries = HashMap::new();
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let tokens = split_header_tokens(&lines[i]);
        let [value, "=", color] = tokens.as_slice() else {
            return Err(parse_error(
                &lines[i],
                "visual color row must be: <value> = <color>",
            ));
        };
        if !values.iter().any(|candidate| candidate == value) {
            return Err(parse_error(
                &lines[i],
                "visual color value is not in tag set",
            ));
        }
        if entries
            .insert((*value).to_string(), (*color).to_string())
            .is_some()
        {
            return Err(parse_error(&lines[i], "duplicate visual color value"));
        }
        i += 1;
    }
    if i >= lines.len() {
        return Err(parse_error(
            &lines[start],
            "visual colors missing closing brace",
        ));
    }
    Ok((
        VisualColorTable {
            axis: axis.to_string(),
            entries,
        },
        i + 1,
    ))
}

fn add_ascii_visuals(
    selector: &str,
    line: &str,
    shape: &VisualShapeTable,
    shape_value_expr: &ValueExpr,
    color_exprs: &[(char, String)],
    offset: VisualSpriteOffset,
    sampling: Option<VisualSpriteSampling>,
    loop_animation: Option<VisualSpriteLoopDef>,
    color_aliases: &HashMap<String, String>,
    color_tables: &HashMap<String, VisualColorTable>,
    catalog: &Catalog,
    visuals: &mut VisualsDef,
) -> Result<(), DiagnosticReport> {
    for target in expand_visual_selector(selector, line, catalog)? {
        let env = visual_value_env(&target.bindings);
        if value_expr_result_axis(shape_value_expr, &env, &catalog.maps, line)? != shape.axis {
            return Err(parse_error(line, "visual shape tag set mismatch"));
        }
        let shape_value = eval_bound_value_expr(shape_value_expr, &env, &catalog.maps, line)?;
        if !catalog_value_set(catalog, &shape.axis)
            .is_some_and(|values| values.iter().any(|value| value == &shape_value))
        {
            return Err(parse_error(line, "visual shape value is not in tag set"));
        }
        let pattern = shape
            .entries
            .get(&shape_value)
            .ok_or_else(|| parse_error(line, "visual shape value missing"))?
            .clone();
        validate_visual_pattern_palette(&pattern, color_exprs, line)?;
        let colors = color_exprs
            .iter()
            .map(|(token, expr)| {
                Ok(VisualColorDef {
                    token: *token,
                    color: resolve_visual_color_expr(
                        expr,
                        &target.bindings,
                        color_aliases,
                        color_tables,
                        &catalog.maps,
                        line,
                    )?,
                })
            })
            .collect::<Result<Vec<_>, DiagnosticReport>>()?;
        let sprite = sprite_name_for_object(&target.object_name);
        visuals.aliases.push(VisualAliasDef {
            object: target.object_name,
            sprite: sprite.clone(),
        });
        visuals.sprites.push(VisualSpriteDef {
            name: sprite,
            offset,
            fit: VisualSpriteFit::default(),
            sampling,
            loop_animation: loop_animation.clone(),
            pixels_per_cell: None,
            kind: VisualSpriteKind::Ascii { pattern, colors },
        });
    }
    Ok(())
}

fn add_inline_ascii_visuals(
    selector: &str,
    line: &str,
    pattern: &[String],
    color_exprs: &[(char, String)],
    offset: VisualSpriteOffset,
    sampling: Option<VisualSpriteSampling>,
    loop_animation: Option<VisualSpriteLoopDef>,
    color_aliases: &HashMap<String, String>,
    color_tables: &HashMap<String, VisualColorTable>,
    catalog: &Catalog,
    visuals: &mut VisualsDef,
) -> Result<(), DiagnosticReport> {
    validate_visual_pattern_palette(pattern, color_exprs, line)?;
    for target in expand_visual_selector(selector, line, catalog)? {
        let colors = color_exprs
            .iter()
            .map(|(token, expr)| {
                Ok(VisualColorDef {
                    token: *token,
                    color: resolve_visual_color_expr(
                        expr,
                        &target.bindings,
                        color_aliases,
                        color_tables,
                        &catalog.maps,
                        line,
                    )?,
                })
            })
            .collect::<Result<Vec<_>, DiagnosticReport>>()?;
        let sprite = sprite_name_for_object(&target.object_name);
        visuals.aliases.push(VisualAliasDef {
            object: target.object_name,
            sprite: sprite.clone(),
        });
        visuals.sprites.push(VisualSpriteDef {
            name: sprite,
            offset,
            fit: VisualSpriteFit::default(),
            sampling,
            loop_animation: loop_animation.clone(),
            pixels_per_cell: None,
            kind: VisualSpriteKind::Ascii {
                pattern: pattern.to_vec(),
                colors,
            },
        });
    }
    Ok(())
}

fn add_solid_visuals(
    selector: &str,
    line: &str,
    color_expr: &str,
    offset: VisualSpriteOffset,
    sampling: Option<VisualSpriteSampling>,
    loop_animation: Option<VisualSpriteLoopDef>,
    color_aliases: &HashMap<String, String>,
    color_tables: &HashMap<String, VisualColorTable>,
    catalog: &Catalog,
    visuals: &mut VisualsDef,
) -> Result<(), DiagnosticReport> {
    for target in expand_visual_selector(selector, line, catalog)? {
        let sprite = sprite_name_for_object(&target.object_name);
        let color = resolve_visual_color_expr(
            color_expr,
            &target.bindings,
            color_aliases,
            color_tables,
            &catalog.maps,
            line,
        )?;
        visuals.aliases.push(VisualAliasDef {
            object: target.object_name,
            sprite: sprite.clone(),
        });
        visuals.sprites.push(VisualSpriteDef {
            name: sprite,
            offset,
            fit: VisualSpriteFit::default(),
            sampling,
            loop_animation: loop_animation.clone(),
            pixels_per_cell: None,
            kind: VisualSpriteKind::Solid(color),
        });
    }
    Ok(())
}

fn add_image_visuals(
    selector: &str,
    line: &str,
    source: &str,
    offset: VisualSpriteOffset,
    sampling: Option<VisualSpriteSampling>,
    catalog: &Catalog,
    visuals: &mut VisualsDef,
) -> Result<(), DiagnosticReport> {
    for target in expand_visual_selector(selector, line, catalog)? {
        let sprite = sprite_name_for_object(&target.object_name);
        visuals.aliases.push(VisualAliasDef {
            object: target.object_name,
            sprite: sprite.clone(),
        });
        visuals.sprites.push(VisualSpriteDef {
            name: sprite,
            offset,
            fit: VisualSpriteFit::default(),
            sampling,
            loop_animation: None,
            pixels_per_cell: None,
            kind: VisualSpriteKind::Image {
                source: source.to_string(),
            },
        });
    }
    Ok(())
}

fn resolve_visual_color_expr(
    expr: &str,
    bindings: &HashMap<String, String>,
    color_aliases: &HashMap<String, String>,
    color_tables: &HashMap<String, VisualColorTable>,
    maps: &HashMap<String, ValueMap>,
    line: &str,
) -> Result<String, DiagnosticReport> {
    resolve_visual_color_expr_with_aliases(expr, bindings, color_aliases, color_tables, maps, line)
}

fn resolve_visual_color_expr_with_aliases(
    expr: &str,
    bindings: &HashMap<String, String>,
    color_aliases: &HashMap<String, String>,
    color_tables: &HashMap<String, VisualColorTable>,
    maps: &HashMap<String, ValueMap>,
    line: &str,
) -> Result<String, DiagnosticReport> {
    if let Some(color) = color_aliases.get(expr) {
        return Ok(color.clone());
    }
    if let Some((name, value_expr)) = parse_visual_table_expr(expr, line).ok() {
        let table = color_tables
            .get(&name)
            .ok_or_else(|| parse_error(line, "unknown visual colors"))?;
        let value = visual_table_key(
            &value_expr,
            &table.axis,
            &table.entries,
            bindings,
            maps,
            line,
        )?;
        return table
            .entries
            .get(&value)
            .cloned()
            .ok_or_else(|| parse_error(line, "visual color value missing"));
    }
    Ok(expr.to_string())
}

#[derive(Clone, Debug)]
struct VisualSelectorTarget {
    object_name: String,
    bindings: HashMap<String, String>,
}

fn expand_visual_selector(
    selector: &str,
    line: &str,
    catalog: &Catalog,
) -> Result<Vec<VisualSelectorTarget>, DiagnosticReport> {
    if !selector.contains(':')
        && let Some(object) = catalog.object_names.get(selector).copied()
    {
        let name = catalog
            .object_labels
            .get(&object)
            .cloned()
            .unwrap_or_else(|| selector.to_string());
        return Ok(vec![VisualSelectorTarget {
            object_name: name,
            bindings: HashMap::new(),
        }]);
    }
    if let Some(objects) = catalog.object_groups.get(selector) {
        return Ok(objects
            .iter()
            .filter_map(|object| catalog.object_labels.get(object).cloned())
            .map(|object_name| VisualSelectorTarget {
                object_name,
                bindings: HashMap::new(),
            })
            .collect());
    }

    let parts = selector.split(':').collect::<Vec<_>>();
    let Some(schema) = catalog.object_schemas.get(parts[0]) else {
        return Err(parse_error(line, "unknown visual object selector"));
    };
    if parts.len() - 1 > schema.axes.len() {
        return Err(parse_error(
            line,
            "visual object selector has too many tags",
        ));
    }

    let constraints = visual_selector_constraints(&parts, schema, catalog, line)?;
    let assignments = visual_selector_assignments(schema, &constraints, &catalog.maps, line)?;
    let mut targets = Vec::new();
    for (target_values, bindings) in assignments {
        let variant = schema
            .variants
            .iter()
            .find(|variant| variant.values == target_values)
            .ok_or_else(|| parse_error(line, "visual object selector target not found"))?;
        let object_name = catalog
            .object_labels
            .get(&variant.object)
            .cloned()
            .ok_or_else(|| parse_error(line, "visual object label missing"))?;
        if targets
            .iter()
            .any(|target: &VisualSelectorTarget| target.object_name == object_name)
        {
            return Err(parse_error(
                line,
                "visual object selector maps multiple bindings to one object",
            ));
        }
        targets.push(VisualSelectorTarget {
            object_name,
            bindings,
        });
    }
    if targets.is_empty() {
        return Err(parse_error(
            line,
            "visual object selector matched no objects",
        ));
    }
    Ok(targets)
}

fn visual_selector_constraints(
    parts: &[&str],
    schema: &ObjectSchema,
    catalog: &Catalog,
    line: &str,
) -> Result<Vec<VisualSelectorConstraint>, DiagnosticReport> {
    let value_sets = catalog_value_sets(catalog);
    schema
        .axes
        .iter()
        .enumerate()
        .map(|(index, axis)| {
            let Some(part) = parts.get(index + 1).copied() else {
                return Ok(VisualSelectorConstraint::Any);
            };
            let expr = parse_value_expr(part, line)?;
            if expr == ValueExpr::Binding(axis.clone()) {
                return Ok(VisualSelectorConstraint::Any);
            }
            if let ValueExpr::MapCall { arg, .. } = &expr {
                if arg != axis {
                    return Err(parse_error(
                        line,
                        "map argument must match selector tag set",
                    ));
                }
                let ValueExpr::MapCall { name, .. } = &expr else {
                    unreachable!("map call branch only handles map calls");
                };
                let map = catalog
                    .maps
                    .get(name)
                    .ok_or_else(|| parse_error(line, "unknown map"))?;
                if map.axis != *axis {
                    return Err(parse_error(line, "map tag set must match argument tag set"));
                }
                return Ok(VisualSelectorConstraint::Mapped(expr));
            }
            let ValueExpr::Binding(name) = expr else {
                unreachable!("value expr is either binding or map call");
            };
            let axis_values = schema_axis_values(schema, index)?;
            if axis_values.contains(&name) && value_sets.contains_key(&name) {
                Err(ambiguous_selector_tag_error(&name, parts[0], axis, line))
            } else if let Some(values) = value_sets.get(&name) {
                validate_selector_subset(&name, values, &axis_values, parts[0], axis, line)?;
                Ok(VisualSelectorConstraint::ValueSet(values.clone()))
            } else if axis_values.contains(&name) {
                Ok(VisualSelectorConstraint::Fixed(name))
            } else {
                Ok(VisualSelectorConstraint::Fixed(name))
            }
        })
        .collect()
}

fn visual_selector_assignments(
    schema: &ObjectSchema,
    constraints: &[VisualSelectorConstraint],
    maps: &HashMap<String, ValueMap>,
    line: &str,
) -> Result<Vec<(Vec<String>, HashMap<String, String>)>, DiagnosticReport> {
    let mut assignments = vec![(Vec::<String>::new(), HashMap::<String, String>::new())];
    for (index, axis) in schema.axes.iter().enumerate() {
        let axis_values = schema_axis_values(schema, index)?;
        let values = match &constraints[index] {
            VisualSelectorConstraint::Any | VisualSelectorConstraint::Mapped(_) => axis_values,
            VisualSelectorConstraint::Fixed(value) => vec![value.clone()],
            VisualSelectorConstraint::ValueSet(values) => values.clone(),
        };
        let mut next = Vec::new();
        for (target_prefix, bindings) in &assignments {
            for value in &values {
                let mut env = visual_value_env(bindings);
                env.bind(axis, axis, value);
                let target_value = match &constraints[index] {
                    VisualSelectorConstraint::Mapped(expr) => {
                        eval_bound_value_expr(expr, &env, maps, line)?
                    }
                    _ => value.clone(),
                };
                if !schema_axis_values(schema, index)?.contains(&target_value) {
                    return Err(parse_error(
                        line,
                        "visual object selector target value is not in tag slot",
                    ));
                }
                let mut target_values = target_prefix.clone();
                target_values.push(target_value);
                let mut next_bindings = bindings.clone();
                next_bindings.insert(axis.clone(), value.clone());
                next.push((target_values, next_bindings));
            }
        }
        assignments = next;
    }
    Ok(assignments)
}

#[derive(Clone, Debug)]
enum VisualSelectorConstraint {
    Any,
    Fixed(String),
    ValueSet(Vec<String>),
    Mapped(ValueExpr),
}

fn sprite_name_for_object(object_name: &str) -> String {
    let mut sprite = String::new();
    for ch in object_name.chars() {
        if ch.is_ascii_alphanumeric() {
            sprite.push(ch);
        } else if !sprite.ends_with('-') {
            sprite.push('-');
        }
    }
    let sprite = sprite.trim_matches('-').to_string();
    if sprite.is_empty() {
        "unknown".to_string()
    } else {
        sprite
    }
}
