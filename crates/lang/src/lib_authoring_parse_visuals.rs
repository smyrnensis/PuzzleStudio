fn define_object_spec(
    spec: &str,
    layer: u16,
    render_spec: Option<&str>,
    line: &str,
    value_sets: &HashMap<String, Vec<String>>,
    axis_types: &HashMap<String, ValueType>,
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
    let schema_axis_types = validate_object_schema_axes(&axes, axis_types, line)?;
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
            axis_types: schema_axis_types,
            variants,
        },
    );
    Ok(created)
}

fn validate_object_schema_axes(
    axes: &[String],
    axis_types: &HashMap<String, ValueType>,
    line: &str,
) -> Result<Vec<Option<ValueType>>, DiagnosticReport> {
    let mut seen = Vec::<String>::new();
    let mut has_angle = false;
    let mut has_vec2 = false;
    let mut kinds = Vec::with_capacity(axes.len());
    for axis in axes {
        if seen.contains(axis) {
            return Err(parse_error(
                line,
                "object schema cannot repeat the same tag slot",
            ));
        }
        seen.push(axis.clone());
        let value_type = axis_types.get(axis).copied();
        match value_type {
            Some(ValueType::Angle) if has_angle => {
                return Err(parse_error(
                    line,
                    "object schema can have at most one angle tag",
                ));
            }
            Some(ValueType::Vec2) if has_vec2 => {
                return Err(parse_error(
                    line,
                    "object schema can have at most one vec2 tag",
                ));
            }
            Some(ValueType::Angle) => has_angle = true,
            Some(ValueType::Vec2) => has_vec2 = true,
            Some(_) => {}
            None => {}
        }
        kinds.push(value_type);
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
type VisualShapeFrames = Vec<crate::visual_authoring::VisualFrameSyntax>;

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
struct VisualEntrySpec {
    source_line: String,
    selector: Option<String>,
    color_exprs: Option<Vec<(char, String)>>,
    transforms: Vec<(crate::visual_authoring::VisualPropertySyntax, String)>,
    sampling: Option<VisualSampling>,
    loop_duration_ms: Option<u64>,
    loop_frame_duration_ms: Option<u64>,
    image_source: Option<puzzle_assets::VisualImageAssetManifestEntry>,
    shape_ref: Option<(String, ValueExpr)>,
    frames: Option<Vec<VisualFrameDef>>,
    animation_duration_ms: Option<u64>,
    rotation: Option<VisualShapeRotation>,
}

impl VisualEntrySpec {
    fn new(source_line: &str, rotation: Option<VisualShapeRotation>) -> Self {
        Self {
            source_line: source_line.to_string(),
            selector: None,
            color_exprs: None,
            transforms: Vec::new(),
            sampling: None,
            loop_duration_ms: None,
            loop_frame_duration_ms: None,
            image_source: None,
            shape_ref: None,
            frames: None,
            animation_duration_ms: None,
            rotation,
        }
    }

    fn selector(&self) -> Result<&str, DiagnosticReport> {
        self.selector
            .as_deref()
            .ok_or_else(|| parse_error(&self.source_line, "visual entry missing selector"))
    }

    fn set_image(&mut self, source: &str, line: &str) -> Result<(), DiagnosticReport> {
        if self.image_source.is_some() {
            return Err(parse_error(line, "duplicate visual image"));
        }
        self.image_source = Some(parse_visual_image_path(source, line)?);
        Ok(())
    }

    fn set_sampling(&mut self, value: &str, line: &str) -> Result<(), DiagnosticReport> {
        if self.sampling.is_some() {
            return Err(parse_error(line, "duplicate visual sampling"));
        }
        self.sampling = Some(parse_visual_sampling(value, line)?);
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct VisualColorTable {
    axis: String,
    entries: HashMap<String, String>,
}

fn parse_visuals_block(
    lines: &[source::LogicalLine],
    start: usize,
    catalog: &Catalog,
    visuals: &mut VisualsDef,
) -> crate::surface::ParseProduct<Result<usize, DiagnosticReport>> {
    let mut recognition = crate::surface::ParserRecognition::default();
    let value = parse_visuals_block_inner(lines, start, catalog, visuals, &mut recognition);
    crate::surface::ParseProduct::new(value, recognition)
}

fn parse_visuals_entry(
    entry: &crate::model_syntax::PuzzleEntrySyntax,
    catalog: &Catalog,
    visuals: &mut VisualsDef,
) -> crate::surface::ParseProduct<Result<usize, DiagnosticReport>> {
    let mut lines = Vec::with_capacity(entry.body.len() + 2);
    lines.push(entry.header.clone());
    lines.extend(entry.body.iter().cloned());
    if let Some(closing) = &entry.closing {
        lines.push(closing.clone());
    }
    parse_visuals_block(&lines, 0, catalog, visuals)
}

fn parse_visuals_block_inner(
    lines: &[source::LogicalLine],
    start: usize,
    catalog: &Catalog,
    visuals: &mut VisualsDef,
    recognition: &mut crate::surface::ParserRecognition,
) -> Result<usize, DiagnosticReport> {
    let owner = split_header_tokens(&lines[start])
        .first()
        .copied()
        .unwrap_or("visuals");
    let resource = puzzle_authoring::collect_resource_block_surface(lines, start, owner)
        .map_err(|error| parse_error(&lines[start], error.message()))?;
    if let Some(product) =
        visual_resource_product(lines, start, resource.next_index, catalog.dimension)
    {
        recognition.visual_resources.push(product);
    }
    let mut shapes = HashMap::<String, VisualShapeTable>::new();
    let mut plain_shapes = HashMap::<String, VisualShapeFrames>::new();
    let mut color_aliases = HashMap::<String, String>::new();
    let mut colors = HashMap::<String, VisualColorTable>::new();
    let declared_color_names =
        predeclare_visual_color_names(lines, resource.body_start, resource.body_end);
    let mut visual_entries =
        Vec::<crate::visual_authoring::VisualAttachmentSyntax<source::LogicalLine>>::new();
    let mut i = resource.body_start;

    while i < resource.body_end {
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        if tokens.first() == Some(&"shape") && tokens.iter().any(|token| *token == "rotate") {
            return Err(parse_error(
                line,
                "shape rotation derivation syntax was removed; use visual rotate",
            ));
        }
        match tokens.as_slice() {
            [] => i += 1,
            ["palette"] => {
                i = parse_visual_palette_block(
                    lines,
                    i,
                    catalog,
                    &mut color_aliases,
                    &mut colors,
                    recognition,
                )?;
            }
            ["colors"] => {
                return Err(parse_error(
                    line,
                    "colors block was renamed to palette; visual color rows still use colors",
                ));
            }
            ["palettes"] => {
                return Err(parse_error(line, "palettes block was renamed to palette"));
            }
            ["shapes"] => {
                i = parse_visual_shapes_block(
                    lines,
                    i,
                    catalog,
                    &mut plain_shapes,
                    &mut shapes,
                    recognition,
                )?;
            }
            ["shape", table_ref] => {
                if !table_ref.contains(':') {
                    if plain_shapes.contains_key(*table_ref) {
                        return Err(parse_error(line, "duplicate visual shape"));
                    }
                    let (pattern, next_i) = parse_visual_plain_shape(lines, i)?;
                    plain_shapes.insert((*table_ref).to_string(), pattern);
                    if let Some(product) =
                        visual_shape_definition_product(lines, i, next_i, table_ref)
                    {
                        recognition.visual_shape_definitions.push(product);
                    }
                    i = next_i;
                    continue;
                }
                let (name, axis) = parse_visual_table_ref(table_ref, line)?;
                if shapes.contains_key(&name) {
                    return Err(parse_error(line, "duplicate visual shape"));
                }
                let (table, next_i) =
                    parse_visual_shape_table(lines, i, &name, &axis, None, catalog, recognition)?;
                shapes.insert(name, table);
                i = next_i;
            }
            ["shape", table_ref, "rotate", "from", from] => {
                let (name, axis) = parse_visual_table_ref(table_ref, line)?;
                if shapes.contains_key(&name) {
                    return Err(parse_error(line, "duplicate visual shape"));
                }
                let rotation = VisualShapeRotation::intrinsic(from);
                let (table, next_i) = parse_visual_shape_table(
                    lines,
                    i,
                    &name,
                    &axis,
                    Some(rotation),
                    catalog,
                    recognition,
                )?;
                shapes.insert(name, table);
                i = next_i;
            }
            ["shape", table_ref, "rotate", "using", map, "from", from] => {
                let (name, axis) = parse_visual_table_ref(table_ref, line)?;
                if shapes.contains_key(&name) {
                    return Err(parse_error(line, "duplicate visual shape"));
                }
                let rotation = VisualShapeRotation::using(map, from);
                let (table, next_i) = parse_visual_shape_table(
                    lines,
                    i,
                    &name,
                    &axis,
                    Some(rotation),
                    catalog,
                    recognition,
                )?;
                shapes.insert(name, table);
                i = next_i;
            }
            ["shape", table_ref, "rotate", map, "from", from] => {
                let (name, axis) = parse_visual_table_ref(table_ref, line)?;
                if shapes.contains_key(&name) {
                    return Err(parse_error(line, "duplicate visual shape"));
                }
                let rotation = VisualShapeRotation::using(map, from);
                let (table, next_i) = parse_visual_shape_table(
                    lines,
                    i,
                    &name,
                    &axis,
                    Some(rotation),
                    catalog,
                    recognition,
                )?;
                shapes.insert(name, table);
                i = next_i;
            }
            ["palette", table_ref] => {
                let (name, axis) = parse_visual_table_ref(table_ref, line)?;
                if colors.contains_key(&name) {
                    return Err(parse_error(line, "duplicate visual colors"));
                }
                mark_visual_color_table_ref(recognition, line, table_ref);
                let (table, next_i) =
                    parse_visual_color_table(lines, i, &name, &axis, catalog, recognition)?;
                colors.insert(name, table);
                i = next_i;
            }
            ["colors", ..] => {
                return Err(parse_error(
                    line,
                    "colors table was renamed to palette; visual color rows still use colors",
                ));
            }
            ["visual"] | ["visual", _] if is_block_header_line(line) => {
                let entry = collect_visual_attachment_entry(lines, i, &declared_color_names)?;
                i = entry.next_index;
                visual_entries.push(entry);
            }
            [other, ..] => {
                if crate::authoring_grammar::authoring_kind_content_attachment(
                    crate::authoring_grammar::AuthoringKind::VisualsConfig,
                ) == Some(crate::authoring_grammar::ContentAttachment::VisualEntries)
                {
                    let entry = collect_visual_attachment_entry(lines, i, &declared_color_names)?;
                    i = entry.next_index;
                    visual_entries.push(entry);
                    continue;
                }
                return Err(parse_error(
                    line,
                    &format!("unknown visuals directive {other}"),
                ));
            }
        }
    }
    recognition
        .visual_refs
        .color_names
        .extend(color_aliases.keys().cloned());
    recognition
        .visual_refs
        .color_names
        .extend(colors.keys().cloned());
    recognition.visual_refs.color_assets.extend(
        color_aliases
            .iter()
            .map(|(name, color)| (name.clone(), color.clone())),
    );
    recognition
        .visual_refs
        .shape_names
        .extend(plain_shapes.keys().cloned());
    recognition
        .visual_refs
        .shape_names
        .extend(shapes.keys().cloned());
    recognition
        .visual_refs
        .shape_assets
        .extend(plain_shapes.iter().map(|(name, frames)| {
            (
                name.clone(),
                crate::surface::SurfaceVisualShapeAsset::Plain {
                    frames: frames.clone(),
                },
            )
        }));
    recognition
        .visual_refs
        .shape_assets
        .extend(shapes.iter().map(|(name, table)| {
            (
                name.clone(),
                crate::surface::SurfaceVisualShapeAsset::Table {
                    axis: table.axis.clone(),
                    variants: table
                        .entries
                        .iter()
                        .map(|(value, rows)| (value.clone(), visual_frame_from_rows(rows)))
                        .collect(),
                },
            )
        }));
    let visual_entries = visual_entries
        .into_iter()
        .map(|attachment| {
            let analyzed = analyze_visual_attachment_entry(
                &attachment,
                &plain_shapes,
                &shapes,
                &color_aliases,
                &colors,
                catalog,
                recognition,
            );
            (attachment, analyzed)
        })
        .collect::<Vec<_>>();
    for (attachment, analyzed) in visual_entries {
        lower_visual_attachment_entry(
            attachment,
            analyzed,
            &plain_shapes,
            &shapes,
            &color_aliases,
            &colors,
            catalog,
            visuals,
        )?;
    }
    visuals.order = catalog.visual_order.clone();
    Ok(resource.next_index)
}

fn parse_visual_palette_block(
    lines: &[source::LogicalLine],
    start: usize,
    catalog: &Catalog,
    color_aliases: &mut HashMap<String, String>,
    colors: &mut HashMap<String, VisualColorTable>,
    recognition: &mut crate::surface::ParserRecognition,
) -> Result<usize, DiagnosticReport> {
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        match tokens.as_slice() {
            [] => i += 1,
            [name, "=", color] => {
                let color_token = *color;
                let color = crate::syntax::canonical_visual_color_literal(color_token).ok_or_else(
                    || parse_error(line, "palette color must be a named color or hex color"),
                )?;
                mark_line_token(
                    recognition,
                    line,
                    Some(name),
                    crate::surface::SurfaceSemanticKind::Color,
                );
                mark_line_token(
                    recognition,
                    line,
                    Some(color_token),
                    crate::surface::SurfaceSemanticKind::Color,
                );
                if let Some(value_span) = line
                    .tokens
                    .iter()
                    .rev()
                    .find(|token| token.text == color_token)
                    .map(|token| crate::surface::SourceSpan {
                        start: token.start,
                        end: token.end,
                    })
                {
                    recognition.visual_color_definitions.push(
                        crate::surface::SurfaceVisualColorDefinitionProduct {
                            name: (*name).to_string(),
                            value_span,
                        },
                    );
                }
                color_aliases.insert((*name).to_string(), color);
                i += 1;
            }
            [table_ref] => {
                let (name, axis) = parse_visual_table_ref(table_ref, line)?;
                if colors.contains_key(&name) {
                    return Err(parse_error(line, "duplicate visual colors"));
                }
                mark_visual_color_table_ref(recognition, line, table_ref);
                let (table, next_i) =
                    parse_visual_color_table(lines, i, &name, &axis, catalog, recognition)?;
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
    if let Some(product) = visual_asset_block_product(
        lines,
        start,
        i + 1,
        crate::surface::SurfaceVisualAssetBlockKind::Palette,
    ) {
        recognition.visual_asset_blocks.push(product);
    }
    Ok(i + 1)
}

fn parse_visual_shapes_block(
    lines: &[source::LogicalLine],
    start: usize,
    catalog: &Catalog,
    plain_shapes: &mut HashMap<String, VisualShapeFrames>,
    shapes: &mut HashMap<String, VisualShapeTable>,
    recognition: &mut crate::surface::ParserRecognition,
) -> Result<usize, DiagnosticReport> {
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        let line = &lines[i];
        let tokens = split_header_tokens(line);
        if tokens.iter().any(|token| *token == "rotate") {
            return Err(parse_error(
                line,
                "shape rotation derivation syntax was removed; use visual rotate",
            ));
        }
        match tokens.as_slice() {
            [] => i += 1,
            [name] if !name.contains(':') => {
                let (pattern, next_i) = parse_visual_plain_shape(lines, i)?;
                plain_shapes.insert((*name).to_string(), pattern);
                mark_visual_shape_ref(recognition, line, name, false);
                if let Some(product) = visual_shape_definition_product(lines, i, next_i, name) {
                    recognition.visual_shape_definitions.push(product);
                }
                i = next_i;
            }
            [table_ref] => {
                if let Some((name, axis, value)) =
                    parse_visual_shape_value_ref(table_ref, line, catalog)?
                {
                    let asset_name = format!("{name}:{value}");
                    let (pattern, next_i) = parse_visual_shape_value_pattern(lines, i, &[], false)?;
                    insert_visual_shape_value(shapes, name, axis, value, pattern, line)?;
                    mark_visual_shape_ref(recognition, line, table_ref, true);
                    if let Some(product) =
                        visual_shape_definition_product(lines, i, next_i, &asset_name)
                    {
                        recognition.visual_shape_definitions.push(product);
                    }
                    i = next_i;
                } else {
                    let (name, axis) = parse_visual_table_ref(table_ref, line)?;
                    let (table, next_i) = parse_visual_shape_table(
                        lines,
                        i,
                        &name,
                        &axis,
                        None,
                        catalog,
                        recognition,
                    )?;
                    shapes.insert(name, table);
                    mark_visual_shape_ref(recognition, line, table_ref, false);
                    i = next_i;
                }
            }
            [table_ref, "rotate", "from", from] => {
                let (name, axis) = parse_visual_table_ref(table_ref, line)?;
                let rotation = VisualShapeRotation::intrinsic(from);
                let (table, next_i) = parse_visual_shape_table(
                    lines,
                    i,
                    &name,
                    &axis,
                    Some(rotation),
                    catalog,
                    recognition,
                )?;
                shapes.insert(name, table);
                i = next_i;
            }
            [table_ref, "rotate", "using", map, "from", from] => {
                let (name, axis) = parse_visual_table_ref(table_ref, line)?;
                let rotation = VisualShapeRotation::using(map, from);
                let (table, next_i) = parse_visual_shape_table(
                    lines,
                    i,
                    &name,
                    &axis,
                    Some(rotation),
                    catalog,
                    recognition,
                )?;
                shapes.insert(name, table);
                i = next_i;
            }
            [table_ref, "rotate", map, "from", from] => {
                let (name, axis) = parse_visual_table_ref(table_ref, line)?;
                let rotation = VisualShapeRotation::using(map, from);
                let (table, next_i) = parse_visual_shape_table(
                    lines,
                    i,
                    &name,
                    &axis,
                    Some(rotation),
                    catalog,
                    recognition,
                )?;
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
    if let Some(product) = visual_asset_block_product(
        lines,
        start,
        i + 1,
        crate::surface::SurfaceVisualAssetBlockKind::Shapes,
    ) {
        recognition.visual_asset_blocks.push(product);
    }
    Ok(i + 1)
}

fn mark_visual_shape_ref(
    recognition: &mut crate::surface::ParserRecognition,
    line: &source::LogicalLine,
    value: &str,
    concrete_value: bool,
) {
    for token in &line.tokens {
        if token.text != value {
            continue;
        }
        let parts = value.split(':').collect::<Vec<_>>();
        let mut offset = 0usize;
        for (index, part) in parts.iter().enumerate() {
            let kind = if index == 0 {
                crate::surface::SurfaceSemanticKind::Asset
            } else if concrete_value && index + 1 == parts.len() {
                crate::surface::SurfaceSemanticKind::Variant
            } else {
                crate::surface::SurfaceSemanticKind::Group
            };
            recognition.mark(
                crate::surface::SourceSpan {
                    start: token.start + offset,
                    end: token.start + offset + part.len(),
                },
                kind,
            );
            offset += part.len() + 1;
        }
    }
}

fn mark_visual_color_table_ref(
    recognition: &mut crate::surface::ParserRecognition,
    line: &source::LogicalLine,
    value: &str,
) {
    for token in &line.tokens {
        if token.text != value {
            continue;
        }
        let mut offset = 0usize;
        for (index, part) in value.split(':').enumerate() {
            recognition.mark(
                crate::surface::SourceSpan {
                    start: token.start + offset,
                    end: token.start + offset + part.len(),
                },
                if index == 0 {
                    crate::surface::SurfaceSemanticKind::Color
                } else {
                    crate::surface::SurfaceSemanticKind::Group
                },
            );
            offset += part.len() + 1;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn lower_visual_attachment_entry(
    attachment: crate::visual_authoring::VisualAttachmentSyntax<source::LogicalLine>,
    analyzed: AnalyzedVisualAttachment,
    plain_shapes: &HashMap<String, VisualShapeFrames>,
    shapes: &HashMap<String, VisualShapeTable>,
    color_aliases: &HashMap<String, String>,
    color_tables: &HashMap<String, VisualColorTable>,
    catalog: &Catalog,
    visuals: &mut VisualsDef,
) -> Result<(), DiagnosticReport> {
    let mut entry = VisualEntrySpec::new(&attachment.header, None);
    apply_visual_attachment_body(&mut entry, analyzed, plain_shapes)?;
    lower_visual_entry(
        entry,
        plain_shapes,
        shapes,
        color_aliases,
        color_tables,
        catalog,
        visuals,
    )?;
    Ok(())
}

struct AnalyzedVisualAttachment {
    body: crate::visual_authoring::VisualBodyProduct,
    shape_ref: Result<Option<(String, ValueExpr)>, DiagnosticReport>,
}

fn analyze_visual_attachment_entry(
    attachment: &crate::visual_authoring::VisualAttachmentSyntax<source::LogicalLine>,
    plain_shapes: &HashMap<String, VisualShapeFrames>,
    shapes: &HashMap<String, VisualShapeTable>,
    color_aliases: &HashMap<String, String>,
    color_tables: &HashMap<String, VisualColorTable>,
    catalog: &Catalog,
    recognition: &mut crate::surface::ParserRecognition,
) -> AnalyzedVisualAttachment {
    let empty_bindings = HashMap::new();
    let analyzed = crate::visual_authoring::analyze_visual_body_product(
        Some(&attachment.header),
        &attachment.body_lines,
        |name| plain_shapes.contains_key(name) || shapes.contains_key(name),
        |expr| {
            resolve_visual_color_expr(
                expr,
                &empty_bindings,
                color_aliases,
                color_tables,
                &catalog.maps,
                &attachment.header,
            )
            .ok()
            .and_then(crate::SourceHighlightColor::parse)
        },
    );
    recognition.merge(analyzed.recognition);
    let shape_ref = match &analyzed.value.shape {
        crate::visual_authoring::ResolvedVisualShape::Reference(reference) => {
            parse_visual_shape_ref(reference, &attachment.header).map(Some)
        }
        _ => Ok(None),
    };
    let shape_asset_name = shape_ref
        .as_ref()
        .ok()
        .and_then(|shape_ref| shape_ref.as_ref().map(|(name, _)| name.clone()));
    recognition
        .visual_products
        .push(crate::surface::SurfaceVisualProduct {
            span: visual_attachment_span(attachment),
            body_span: visual_attachment_body_span(attachment),
            name: analyzed
                .value
                .syntax
                .selector
                .clone()
                .or_else(|| {
                    crate::split_header_tokens(&attachment.header)
                        .into_iter()
                        .find(|token| *token != "visual")
                        .map(str::to_string)
                })
                .unwrap_or_default(),
            dimension: catalog.dimension,
            body: analyzed.value.clone(),
            shape_asset_name,
        });
    AnalyzedVisualAttachment {
        body: analyzed.value,
        shape_ref,
    }
}

fn collect_visual_attachment_entry(
    lines: &[source::LogicalLine],
    start: usize,
    known_colors: &HashSet<String>,
) -> Result<crate::visual_authoring::VisualAttachmentSyntax<source::LogicalLine>, DiagnosticReport>
{
    crate::visual_authoring::collect_visual_attachment(lines, start, known_colors)
        .map_err(|message| parse_error(&lines[start], message))
}

fn predeclare_visual_color_names(
    lines: &[source::LogicalLine],
    start: usize,
    end: usize,
) -> HashSet<String> {
    let mut names = HashSet::new();
    let mut index = start;
    while index < end {
        let tokens = split_header_tokens(&lines[index]);
        match tokens.as_slice() {
            ["palette"] if is_block_header_line(&lines[index]) => {
                let Ok(block) =
                    puzzle_authoring::collect_container_block_surface(lines, index + 1, "palette")
                else {
                    break;
                };
                let mut row = block.body_start;
                while row < block.body_end {
                    let row_tokens = split_header_tokens(&lines[row]);
                    match row_tokens.as_slice() {
                        [name, "=", _] => {
                            names.insert((*name).to_string());
                            row += 1;
                        }
                        [table] if is_block_header_line(&lines[row]) => {
                            names.insert(table.split(':').next().unwrap_or(table).to_string());
                            row = puzzle_authoring::collect_container_block_surface(
                                lines,
                                row + 1,
                                "palette table",
                            )
                            .map_or(row + 1, |block| block.next_index);
                        }
                        _ => row += 1,
                    }
                }
                index = block.next_index;
            }
            ["palette", table] => {
                names.insert(table.split(':').next().unwrap_or(table).to_string());
                index += 1;
            }
            _ => index += 1,
        }
    }
    names
}

fn apply_visual_attachment_body(
    entry: &mut VisualEntrySpec,
    analyzed: AnalyzedVisualAttachment,
    plain_shapes: &HashMap<String, VisualShapeFrames>,
) -> Result<(), DiagnosticReport> {
    entry.selector = None;
    if let Some(error) = &analyzed.body.error {
        return Err(parse_error(&error.line, &error.message));
    }
    let shape_ref = analyzed.shape_ref?;
    let syntax = analyzed.body.syntax;
    let inline_frame_count = match &analyzed.body.shape {
        crate::visual_authoring::ResolvedVisualShape::Inline(frames) => frames.len(),
        crate::visual_authoring::ResolvedVisualShape::Reference(_) => shape_ref
            .as_ref()
            .and_then(|(name, _)| plain_shapes.get(name))
            .map_or(1, Vec::len),
        _ => 1,
    };
    let timing = crate::visual_authoring::resolve_visual_timing(
        inline_frame_count,
        syntax.duration.as_deref(),
        syntax.frame_duration.as_deref(),
    )
    .map_err(|message| parse_error(&entry.source_line, &message))?;
    entry.loop_duration_ms = timing.duration_ms;
    entry.loop_frame_duration_ms = timing.frame_duration_ms;
    if let Some(selector) = syntax.selector {
        if entry.selector.replace(selector).is_some() {
            return Err(parse_error(&entry.source_line, "duplicate visual selector"));
        }
    }
    if let Some(colors) = syntax.colors {
        let values = colors.iter().map(String::as_str).collect::<Vec<_>>();
        entry.color_exprs = Some(visual_colors_from_tokens(&values, &entry.source_line)?);
    }
    for (property, property_line) in syntax.properties {
        match property {
            crate::visual_authoring::VisualPropertySyntax::Image(source) => {
                entry.set_image(&source, &property_line)?;
            }
            crate::visual_authoring::VisualPropertySyntax::Sampling(value) => {
                entry.set_sampling(&value, &property_line)?;
            }
            crate::visual_authoring::VisualPropertySyntax::Translate { value, space } => {
                entry.transforms.push((
                    crate::visual_authoring::VisualPropertySyntax::Translate { value, space },
                    property_line,
                ));
            }
            crate::visual_authoring::VisualPropertySyntax::Rotate {
                angle,
                from,
                axis,
                space,
            } => {
                entry.transforms.push((
                    crate::visual_authoring::VisualPropertySyntax::Rotate {
                        angle,
                        from,
                        axis,
                        space,
                    },
                    property_line,
                ));
            }
            crate::visual_authoring::VisualPropertySyntax::Flip(value) => {
                entry.transforms.push((
                    crate::visual_authoring::VisualPropertySyntax::Flip(value),
                    property_line,
                ));
            }
            crate::visual_authoring::VisualPropertySyntax::RemovedOffset => {
                return Err(parse_error(
                    &property_line,
                    "visual offset was replaced by translate (<x>, <y>)",
                ));
            }
            crate::visual_authoring::VisualPropertySyntax::Unknown(property) => {
                if property == "rotate" {
                    return Err(parse_error(
                        &property_line,
                        "removed visual rotation syntax; use rotate [world|local] <angle>",
                    ));
                }
                return Err(parse_error(
                    &property_line,
                    &format!("unknown visual property {property}"),
                ));
            }
        }
    }
    match analyzed.body.shape {
        crate::visual_authoring::ResolvedVisualShape::Reference(_) => {
            entry.shape_ref = shape_ref;
        }
        crate::visual_authoring::ResolvedVisualShape::Inline(frames) => {
            crate::visual_authoring::validate_visual_frame_geometry(&frames)
                .map_err(|message| parse_error(&entry.source_line, message))?;
            let frames = frames
                .iter()
                .map(|frame| VisualFrameDef {
                    planes: frame
                        .layers
                        .iter()
                        .map(|layer| {
                            layer
                                .rows
                                .iter()
                                .map(|row| row.text.clone())
                                .collect::<Vec<_>>()
                        })
                        .collect::<Vec<_>>(),
                })
                .collect::<Vec<_>>();
            if frames.len() > 1 {
                entry.animation_duration_ms = timing.total_duration_ms;
            }
            entry.frames = Some(frames);
        }
        crate::visual_authoring::ResolvedVisualShape::UnknownBareReference(reference) => {
            return Err(parse_error(
                &reference,
                &format!("unknown visual shape `{reference}`"),
            ));
        }
        crate::visual_authoring::ResolvedVisualShape::AmbiguousBareRow(value) => {
            return Err(parse_error(
                &value,
                "bare visual row is both a declared shape name and valid ASCII; use `shape = <name>` for a reference or `shape = { ... }` for inline ASCII",
            ));
        }
        crate::visual_authoring::ResolvedVisualShape::None => {}
    }
    Ok(())
}

fn visual_attachment_span(
    attachment: &crate::visual_authoring::VisualAttachmentSyntax<source::LogicalLine>,
) -> crate::surface::SourceSpan {
    let start = attachment
        .header_line
        .source_start()
        .or_else(|| {
            attachment
                .header_line
                .tokens
                .first()
                .map(|token| token.start)
        })
        .unwrap_or(0);
    let end = attachment
        .closing_line
        .as_ref()
        .and_then(|line| {
            line.source_end()
                .or_else(|| line.tokens.last().map(|token| token.end))
        })
        .or_else(|| {
            attachment
                .body_lines
                .iter()
                .rev()
                .find_map(|line| line.tokens.last().map(|token| token.end))
        })
        .or_else(|| attachment.header_line.tokens.last().map(|token| token.end))
        .unwrap_or(start);
    crate::surface::SourceSpan { start, end }
}

fn visual_attachment_body_span(
    attachment: &crate::visual_authoring::VisualAttachmentSyntax<source::LogicalLine>,
) -> crate::surface::SourceSpan {
    let header_end = attachment
        .header_line
        .source_end()
        .or_else(|| attachment.header_line.tokens.last().map(|token| token.end))
        .unwrap_or(0);
    let start = if attachment.closing_line.is_some() {
        header_end
    } else if attachment.body_lines.is_empty() {
        attachment
            .header_line
            .tokens
            .get(1)
            .map_or(header_end, |token| token.start)
    } else {
        header_end
    };
    let end = attachment
        .closing_line
        .as_ref()
        .and_then(|line| {
            line.source_start()
                .or_else(|| line.tokens.first().map(|token| token.start))
        })
        .or_else(|| {
            attachment
                .body_lines
                .iter()
                .rev()
                .find_map(|line| line.tokens.last().map(|token| token.end))
        })
        .unwrap_or_else(|| {
            attachment
                .header_line
                .tokens
                .last()
                .map_or(start, |token| token.end)
        });
    crate::surface::SourceSpan { start, end }
}

fn visual_resource_product(
    lines: &[source::LogicalLine],
    start: usize,
    next: usize,
    dimension: crate::ModelDimension,
) -> Option<crate::surface::SurfaceVisualResourceProduct> {
    let header = lines.get(start)?;
    let closing = lines.get(next.checked_sub(1)?)?;
    let header_start = header.source_start()?;
    let closing_start = closing.source_start()?;
    let first = header_start + header.text.len() - header.text.trim_start().len();
    let open = header_start + header.text.rfind('{')?;
    let close = closing_start + closing.text.find('}')?;
    Some(crate::surface::SurfaceVisualResourceProduct {
        span: crate::surface::SourceSpan {
            start: first,
            end: close + 1,
        },
        open_brace: open,
        close_brace: close,
        dimension,
    })
}

fn visual_asset_block_product(
    lines: &[source::LogicalLine],
    start: usize,
    next: usize,
    kind: crate::surface::SurfaceVisualAssetBlockKind,
) -> Option<crate::surface::SurfaceVisualAssetBlockProduct> {
    let header = lines.get(start)?;
    let closing = lines.get(next.checked_sub(1)?)?;
    let header_start = header.source_start()?;
    let closing_start = closing.source_start()?;
    let first = header_start + header.text.len() - header.text.trim_start().len();
    let open = header_start + header.text.rfind('{')?;
    let close = closing_start + closing.text.find('}')?;
    Some(crate::surface::SurfaceVisualAssetBlockProduct {
        span: crate::surface::SourceSpan {
            start: first,
            end: close + 1,
        },
        open_brace: open,
        close_brace: close,
        kind,
    })
}

fn visual_shape_definition_product(
    lines: &[source::LogicalLine],
    start: usize,
    next: usize,
    name: &str,
) -> Option<crate::surface::SurfaceVisualShapeDefinitionProduct> {
    let header = lines.get(start)?;
    let last = lines.get(next.checked_sub(1)?)?;
    let start_offset = header.source_start()?;
    let end_offset = last.source_end()?;
    let braced = is_block_header_line(header);
    Some(crate::surface::SurfaceVisualShapeDefinitionProduct {
        name: name.to_string(),
        span: crate::surface::SourceSpan {
            start: start_offset,
            end: end_offset,
        },
        header: block_header_text(header).trim().to_string(),
        braced,
    })
}

#[allow(clippy::too_many_arguments)]
fn lower_visual_entry(
    entry: VisualEntrySpec,
    plain_shapes: &HashMap<String, VisualShapeFrames>,
    shapes: &HashMap<String, VisualShapeTable>,
    color_aliases: &HashMap<String, String>,
    color_tables: &HashMap<String, VisualColorTable>,
    catalog: &Catalog,
    visuals: &mut VisualsDef,
) -> Result<(), DiagnosticReport> {
    let selector = entry.selector()?.to_string();
    let line = entry.source_line.as_str();
    if let Some(source) = entry.image_source {
        if entry.color_exprs.is_some()
            || entry.frames.is_some()
            || entry.shape_ref.is_some()
            || entry.animation_duration_ms.is_some()
            || entry.rotation.is_some()
        {
            return Err(parse_error(
                line,
                "image visual cannot also define ASCII colors, shape, loop, or rotate",
            ));
        }
        add_image_visuals(
            &selector,
            line,
            &source,
            &entry.transforms,
            entry.sampling,
            catalog,
            visuals,
        )?;
    } else if let Some(rotation) = entry.rotation {
        let color_exprs = entry
            .color_exprs
            .ok_or_else(|| parse_error(line, "visual entry missing colors"))?;
        let Some(frames) = entry.frames else {
            return Err(parse_error(
                line,
                "visual rotation requires inline ASCII rows",
            ));
        };
        let [frame] = frames.as_slice() else {
            return Err(parse_error(
                line,
                "visual rotation derivation requires one frame",
            ));
        };
        let [pattern] = frame.planes.as_slice() else {
            return Err(parse_error(
                line,
                "visual rotation derivation requires one plane",
            ));
        };
        validate_visual_pattern_palette(&pattern, &color_exprs, line)?;
        let targets = expand_visual_selector(&selector, line, catalog)?;
        let axis = visual_rotation_axis_for_targets(&targets, catalog, &rotation, line)?;
        let mut entries = HashMap::new();
        entries.insert(rotation.from.clone(), pattern.clone());
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
            &entry.transforms,
            entry.sampling,
            color_aliases,
            color_tables,
            catalog,
            visuals,
        )?;
    } else if let Some((shape_name, shape_value)) = entry.shape_ref {
        let color_exprs = entry
            .color_exprs
            .ok_or_else(|| parse_error(line, "visual entry missing colors"))?;
        if let Some(shape) = shapes.get(&shape_name) {
            add_ascii_visuals(
                &selector,
                line,
                shape,
                &shape_value,
                &color_exprs,
                &entry.transforms,
                entry.sampling,
                color_aliases,
                color_tables,
                catalog,
                visuals,
            )?;
        } else {
            let shape_frames = plain_shapes
                .get(&shape_name)
                .ok_or_else(|| parse_error(line, "unknown visual shape"))?;
            let frames = shape_frames
                .iter()
                .map(|frame| VisualFrameDef {
                    planes: frame
                        .layers
                        .iter()
                        .map(|layer| layer.rows.iter().map(|row| row.text.clone()).collect())
                        .collect(),
                })
                .collect::<Vec<_>>();
            add_inline_ascii_visuals(
                &selector,
                line,
                &frames,
                &color_exprs,
                &entry.transforms,
                entry.sampling,
                entry.animation_duration_ms,
                color_aliases,
                color_tables,
                catalog,
                visuals,
            )?;
        }
    } else if let Some(frames) = entry.frames {
        let color_exprs = entry
            .color_exprs
            .ok_or_else(|| parse_error(line, "visual entry missing colors"))?;
        add_inline_ascii_visuals(
            &selector,
            line,
            &frames,
            &color_exprs,
            &entry.transforms,
            entry.sampling,
            entry.animation_duration_ms,
            color_aliases,
            color_tables,
            catalog,
            visuals,
        )?;
    } else {
        let color_exprs = entry
            .color_exprs
            .ok_or_else(|| parse_error(line, "visual entry missing colors"))?;
        let [(_, color)] = color_exprs.as_slice() else {
            return Err(parse_error(line, "solid visual requires exactly one color"));
        };
        add_solid_visuals(
            &selector,
            line,
            color,
            &entry.transforms,
            entry.sampling,
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
                .ok_or_else(|| parse_error(line, "visual supports at most 62 colors"))?;
            Ok((token, (*color).to_string()))
        })
        .collect()
}

fn parse_visual_sampling(
    value: &str,
    line: &str,
) -> Result<VisualSampling, DiagnosticReport> {
    match value {
        "pixelated" => Ok(VisualSampling::Pixelated),
        "smooth" => Ok(VisualSampling::Smooth),
        _ => Err(parse_error(
            line,
            "visual sampling must be pixelated or smooth",
        )),
    }
}

fn parse_visual_image_path(
    value: &str,
    line: &str,
) -> Result<puzzle_assets::VisualImageAssetManifestEntry, DiagnosticReport> {
    let path = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .ok_or_else(|| parse_error(line, "visual image path must be quoted"))?;
    puzzle_assets::VisualImageAssetManifestEntry::from_path(path)
        .map_err(|error| parse_error(line, &error.to_string()))
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
            "visual rotation requires exactly one matching selector tag set",
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
    lines: &[source::LogicalLine],
    start: usize,
) -> Result<(VisualShapeFrames, usize), DiagnosticReport> {
    let is_braced = is_block_header_line(&lines[start]);
    let mut i = start + 1;
    while i < lines.len() && !is_block_close_line(&lines[i]) {
        if lines[i].is_empty() {
            if is_braced {
                i += 1;
                continue;
            }
            break;
        }
        i += 1;
    }
    if is_braced && (i >= lines.len() || !is_block_close_line(&lines[i])) {
        return Err(parse_error(
            &lines[start],
            "visual shape missing closing brace",
        ));
    }
    let frames = crate::visual_authoring::parse_visual_shape_rows(&lines[start + 1..i])
        .map_err(|error| parse_error(&error.line, &error.message))?;
    for frame in &frames {
        for layer in &frame.layers {
            let rows = layer
                .rows
                .iter()
                .map(|row| row.text.clone())
                .collect::<Vec<_>>();
            validate_visual_pattern(&rows, &lines[start])?;
        }
    }
    let next_i = if is_braced { i + 1 } else { i };
    Ok((frames, next_i))
}

fn visual_frame_from_rows(rows: &[String]) -> crate::visual_authoring::VisualFrameSyntax {
    crate::visual_authoring::VisualFrameSyntax {
        layers: vec![crate::visual_authoring::VisualLayerSyntax {
            rows: rows
                .iter()
                .enumerate()
                .map(
                    |(body_line, text)| crate::visual_authoring::VisualShapeRow {
                        text: text.clone(),
                        body_line,
                    },
                )
                .collect(),
        }],
    }
}

fn parse_visual_shape_ref(
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
                "visual pattern references a color outside the color row",
            ));
        }
    }
    Ok(())
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
    lines: &[source::LogicalLine],
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
    lines: &[source::LogicalLine],
    start: usize,
    table_name: &str,
    axis: &str,
    rotation: Option<VisualShapeRotation>,
    catalog: &Catalog,
    recognition: &mut crate::surface::ParserRecognition,
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
        let value_start = i;
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
        if let Some(product) = visual_shape_definition_product(
            lines,
            value_start,
            next_i,
            &format!("{table_name}:{value}"),
        ) {
            recognition.visual_shape_definitions.push(product);
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
    lines: &[source::LogicalLine],
    start: usize,
    table_name: &str,
    axis: &str,
    catalog: &Catalog,
    recognition: &mut crate::surface::ParserRecognition,
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
        mark_line_token(
            recognition,
            &lines[i],
            Some(value),
            crate::surface::SurfaceSemanticKind::Variant,
        );
        mark_line_token(
            recognition,
            &lines[i],
            Some(color),
            crate::surface::SurfaceSemanticKind::Color,
        );
        if let Some(value_span) = lines[i]
            .tokens
            .iter()
            .rev()
            .find(|token| token.text == *color)
            .map(|token| crate::surface::SourceSpan {
                start: token.start,
                end: token.end,
            })
        {
            recognition.visual_color_definitions.push(
                crate::surface::SurfaceVisualColorDefinitionProduct {
                    name: format!("{table_name}:{value}"),
                    value_span,
                },
            );
        }
        let color = crate::syntax::canonical_visual_color_literal(color).ok_or_else(|| {
            parse_error(
                &lines[i],
                "visual color table value must be a named color or hex color",
            )
        })?;
        if entries.insert((*value).to_string(), color).is_some() {
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

pub(crate) fn eval_visual_transforms(
    expressions: &[(crate::visual_authoring::VisualPropertySyntax, String)],
    bindings: &HashMap<String, String>,
    line: &str,
) -> Result<Vec<VisualTransform>, DiagnosticReport> {
    expressions
        .iter()
        .map(|(expression, expression_line)| match expression {
            crate::visual_authoring::VisualPropertySyntax::Rotate {
                angle,
                from,
                axis,
                space,
            } => {
                let mut degrees = eval_visual_angle_expr(angle, bindings, expression_line)?;
                if let Some(from) = from {
                    degrees = degrees.sub(eval_visual_angle_expr(from, bindings, expression_line)?);
                }
                Ok(VisualTransform::Rotate {
                    degrees: degrees.as_f64(),
                    axis: eval_visual_axis_expr(
                        axis.as_deref().unwrap_or("up"),
                        bindings,
                        expression_line,
                    )?,
                    space: visual_space(*space),
                })
            }
            crate::visual_authoring::VisualPropertySyntax::Translate { value, space } => {
                let value = eval_visual_vector_expr(value, bindings, expression_line)?;
                Ok(VisualTransform::Translate {
                    value: value.map(Rational::as_f64),
                    space: visual_space(*space),
                })
            }
            crate::visual_authoring::VisualPropertySyntax::Flip(value) => {
                Ok(VisualTransform::Flip {
                    enabled: eval_visual_bool_expr(value, bindings, expression_line)?,
                })
            }
            _ => Err(parse_error(
                line,
                "non-transform visual property reached transform lowering",
            )),
        })
        .collect()
}

fn visual_space(
    space: crate::visual_authoring::VisualSpaceSyntax,
) -> crate::VisualSpace {
    match space {
        crate::visual_authoring::VisualSpaceSyntax::World => crate::VisualSpace::World,
        crate::visual_authoring::VisualSpaceSyntax::Local => crate::VisualSpace::Local,
    }
}

fn eval_visual_bool_expr(
    expression: &str,
    bindings: &HashMap<String, String>,
    line: &str,
) -> Result<bool, DiagnosticReport> {
    let expression = strip_expression_parentheses(expression.trim());
    let value = bindings
        .get(expression)
        .map(String::as_str)
        .unwrap_or(expression);
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(parse_error(
            line,
            "visual flip expression must resolve to true or false",
        )),
    }
}

fn eval_visual_angle_expr(
    expression: &str,
    bindings: &HashMap<String, String>,
    line: &str,
) -> Result<Rational, DiagnosticReport> {
    let expression = strip_expression_parentheses(expression.trim());
    if let Some((left, operator, right)) = split_top_level_arithmetic(expression) {
        let left = eval_visual_angle_expr(left, bindings, line)?;
        let right = eval_visual_angle_expr(right, bindings, line)?;
        return Ok(if operator == '+' {
            left.add(right)
        } else {
            left.sub(right)
        });
    }
    let value = bindings
        .get(expression)
        .map(String::as_str)
        .unwrap_or(expression);
    if let Some(degrees) = visual_direction_degrees(value) {
        return Ok(degrees);
    }
    parse_degree_value(value, line).map_err(|_| {
        parse_error(
            line,
            "visual rotate expression must resolve to an angle or direction",
        )
    })
}

fn visual_direction_degrees(value: &str) -> Option<Rational> {
    Some(match value {
        "right" => Rational::ZERO,
        "up" | "front" => Rational::integer(90),
        "left" => Rational::integer(180),
        "down" | "back" => Rational::integer(-90),
        _ => return None,
    })
}

fn eval_visual_vector_expr(
    expression: &str,
    bindings: &HashMap<String, String>,
    line: &str,
) -> Result<[Rational; 3], DiagnosticReport> {
    let expression = strip_expression_parentheses(expression.trim());
    if let Some((left, operator, right)) = split_top_level_arithmetic(expression) {
        let left = eval_visual_vector_expr(left, bindings, line)?;
        let right = eval_visual_vector_expr(right, bindings, line)?;
        return Ok(if operator == '+' {
            std::array::from_fn(|axis| left[axis].add(right[axis]))
        } else {
            std::array::from_fn(|axis| left[axis].sub(right[axis]))
        });
    }
    if let Some(components) = split_visual_vector_components(expression) {
        return match components.as_slice() {
            [x, y] => Ok([
                eval_visual_number_expr(x, bindings, line)?,
                eval_visual_number_expr(y, bindings, line)?,
                Rational::ZERO,
            ]),
            [x, y, z] => Ok([
                eval_visual_number_expr(x, bindings, line)?,
                eval_visual_number_expr(y, bindings, line)?,
                eval_visual_number_expr(z, bindings, line)?,
            ]),
            _ => Err(parse_error(
                line,
                "visual translate expression must resolve to a vec2 or vec3",
            )),
        };
    }
    let value = bindings
        .get(expression)
        .map(String::as_str)
        .unwrap_or(expression);
    if let Some(direction) = visual_direction_vector(value) {
        return Ok(direction);
    }
    Err(parse_error(
        line,
        "visual translate expression must resolve to a vec2 or vec3",
    ))
}

fn split_visual_vector_components(expression: &str) -> Option<Vec<&str>> {
    let inner = expression.trim();
    if !inner.contains(',') {
        return None;
    }
    let components = inner.split(',').map(str::trim).collect::<Vec<_>>();
    components
        .iter()
        .all(|value| !value.is_empty())
        .then_some(components)
}

fn eval_visual_axis_expr(
    expression: &str,
    bindings: &HashMap<String, String>,
    line: &str,
) -> Result<[f64; 3], DiagnosticReport> {
    let expression = bindings
        .get(expression.trim())
        .map(String::as_str)
        .unwrap_or(expression.trim());
    let value = match expression {
        "right" => [1.0, 0.0, 0.0],
        "left" => [-1.0, 0.0, 0.0],
        "front" => [0.0, 1.0, 0.0],
        "back" => [0.0, -1.0, 0.0],
        "up" => [0.0, 0.0, 1.0],
        "down" => [0.0, 0.0, -1.0],
        _ => eval_visual_vector_expr(expression, bindings, line)?.map(Rational::as_f64),
    };
    let length = value
        .iter()
        .map(|component| component * component)
        .sum::<f64>()
        .sqrt();
    if length == 0.0 {
        return Err(parse_error(line, "visual rotate axis cannot be zero"));
    }
    Ok(value.map(|component| component / length))
}

fn eval_visual_number_expr(
    expression: &str,
    bindings: &HashMap<String, String>,
    line: &str,
) -> Result<Rational, DiagnosticReport> {
    let expression = strip_expression_parentheses(expression.trim());
    if let Some((left, operator, right)) = split_top_level_arithmetic(expression) {
        let left = eval_visual_number_expr(left, bindings, line)?;
        let right = eval_visual_number_expr(right, bindings, line)?;
        return Ok(if operator == '+' {
            left.add(right)
        } else {
            left.sub(right)
        });
    }
    let value = bindings
        .get(expression)
        .map(String::as_str)
        .unwrap_or(expression);
    parse_rational_value(value, line)
        .map_err(|_| parse_error(line, "visual vec2 component must resolve to a number"))
}

fn visual_direction_vector(value: &str) -> Option<[Rational; 3]> {
    Some(match value {
        "right" => [Rational::integer(1), Rational::ZERO, Rational::ZERO],
        "up" => [Rational::ZERO, Rational::integer(-1), Rational::ZERO],
        "left" => [Rational::integer(-1), Rational::ZERO, Rational::ZERO],
        "down" => [Rational::ZERO, Rational::integer(1), Rational::ZERO],
        _ => return None,
    })
}

fn strip_expression_parentheses(mut expression: &str) -> &str {
    loop {
        let Some(inner) = expression
            .strip_prefix('(')
            .and_then(|value| value.strip_suffix(')'))
        else {
            return expression;
        };
        let mut depth = 0usize;
        let mut encloses_whole_expression = true;
        for (index, ch) in expression.char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 && index + ch.len_utf8() != expression.len() {
                        encloses_whole_expression = false;
                        break;
                    }
                }
                _ => {}
            }
        }
        if !encloses_whole_expression {
            return expression;
        }
        expression = inner.trim();
    }
}

fn split_top_level_arithmetic(expression: &str) -> Option<(&str, char, &str)> {
    let mut depth = 0usize;
    let mut found = None;
    for (index, ch) in expression.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            '+' | '-' if depth == 0 && index > 0 => {
                let previous = expression[..index]
                    .chars()
                    .rev()
                    .find(|candidate| !candidate.is_whitespace());
                if !matches!(previous, None | Some(',' | '(' | '+' | '-')) {
                    found = Some((index, ch));
                }
            }
            _ => {}
        }
    }
    let (index, operator) = found?;
    let left = expression[..index].trim();
    let right = expression[index + operator.len_utf8()..].trim();
    if left.is_empty() || right.is_empty() {
        return None;
    }
    Some((left, operator, right))
}

fn add_ascii_visuals(
    selector: &str,
    line: &str,
    shape: &VisualShapeTable,
    shape_value_expr: &ValueExpr,
    color_exprs: &[(char, String)],
    transform_exprs: &[(crate::visual_authoring::VisualPropertySyntax, String)],
    sampling: Option<VisualSampling>,
    color_aliases: &HashMap<String, String>,
    color_tables: &HashMap<String, VisualColorTable>,
    catalog: &Catalog,
    visuals: &mut VisualsDef,
) -> Result<(), DiagnosticReport> {
    for target in expand_visual_selector(selector, line, catalog)? {
        let transforms = eval_visual_transforms(transform_exprs, &target.bindings, line)?;
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
        let visual = visual_name_for_object(&target.object_name);
        if target.bind_object {
            visuals.aliases.push(VisualAliasDef {
                object: target.object_name,
                visual: visual.clone(),
            });
        }
        visuals.entries.push(VisualDef {
            name: visual,
            frames: vec![VisualFrameDef {
                planes: vec![pattern],
            }],
            transforms,
            fit: VisualFit::default(),
            sampling,
            animation_duration_ms: None,
            pixels_per_cell: None,
            kind: VisualKind::Ascii { colors },
        });
    }
    Ok(())
}

fn add_inline_ascii_visuals(
    selector: &str,
    line: &str,
    frames: &[VisualFrameDef],
    color_exprs: &[(char, String)],
    transform_exprs: &[(crate::visual_authoring::VisualPropertySyntax, String)],
    sampling: Option<VisualSampling>,
    animation_duration_ms: Option<u64>,
    color_aliases: &HashMap<String, String>,
    color_tables: &HashMap<String, VisualColorTable>,
    catalog: &Catalog,
    visuals: &mut VisualsDef,
) -> Result<(), DiagnosticReport> {
    for frame in frames {
        for plane in &frame.planes {
            validate_visual_pattern_palette(plane, color_exprs, line)?;
        }
    }
    for target in expand_visual_selector(selector, line, catalog)? {
        let transforms = eval_visual_transforms(transform_exprs, &target.bindings, line)?;
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
        let visual = visual_name_for_object(&target.object_name);
        if target.bind_object {
            visuals.aliases.push(VisualAliasDef {
                object: target.object_name,
                visual: visual.clone(),
            });
        }
        visuals.entries.push(VisualDef {
            name: visual,
            frames: frames.to_vec(),
            transforms,
            fit: VisualFit::default(),
            sampling,
            animation_duration_ms,
            pixels_per_cell: None,
            kind: VisualKind::Ascii { colors },
        });
    }
    Ok(())
}

fn add_solid_visuals(
    selector: &str,
    line: &str,
    color_expr: &str,
    transform_exprs: &[(crate::visual_authoring::VisualPropertySyntax, String)],
    sampling: Option<VisualSampling>,
    color_aliases: &HashMap<String, String>,
    color_tables: &HashMap<String, VisualColorTable>,
    catalog: &Catalog,
    visuals: &mut VisualsDef,
) -> Result<(), DiagnosticReport> {
    for target in expand_visual_selector(selector, line, catalog)? {
        let transforms = eval_visual_transforms(transform_exprs, &target.bindings, line)?;
        let visual = visual_name_for_object(&target.object_name);
        let color = resolve_visual_color_expr(
            color_expr,
            &target.bindings,
            color_aliases,
            color_tables,
            &catalog.maps,
            line,
        )?;
        if target.bind_object {
            visuals.aliases.push(VisualAliasDef {
                object: target.object_name,
                visual: visual.clone(),
            });
        }
        visuals.entries.push(VisualDef {
            name: visual,
            frames: Vec::new(),
            transforms,
            fit: VisualFit::default(),
            sampling,
            animation_duration_ms: None,
            pixels_per_cell: None,
            kind: VisualKind::Solid(color),
        });
    }
    Ok(())
}

fn add_image_visuals(
    selector: &str,
    line: &str,
    asset: &puzzle_assets::VisualImageAssetManifestEntry,
    transform_exprs: &[(crate::visual_authoring::VisualPropertySyntax, String)],
    sampling: Option<VisualSampling>,
    catalog: &Catalog,
    visuals: &mut VisualsDef,
) -> Result<(), DiagnosticReport> {
    for target in expand_visual_selector(selector, line, catalog)? {
        let transforms = eval_visual_transforms(transform_exprs, &target.bindings, line)?;
        let visual = visual_name_for_object(&target.object_name);
        if target.bind_object {
            visuals.aliases.push(VisualAliasDef {
                object: target.object_name,
                visual: visual.clone(),
            });
        }
        visuals.entries.push(VisualDef {
            name: visual,
            frames: Vec::new(),
            transforms,
            fit: VisualFit::default(),
            sampling,
            animation_duration_ms: None,
            pixels_per_cell: None,
            kind: VisualKind::Image {
                asset: asset.clone(),
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
    crate::syntax::canonical_visual_color_literal(expr).ok_or_else(|| {
        parse_error(
            line,
            "visual color must resolve to a named color or hex color",
        )
    })
}

#[derive(Clone, Debug)]
struct VisualSelectorTarget {
    object_name: String,
    bindings: HashMap<String, String>,
    bind_object: bool,
}

fn expand_visual_selector(
    selector: &str,
    line: &str,
    catalog: &Catalog,
) -> Result<Vec<VisualSelectorTarget>, DiagnosticReport> {
    if let Some(name) = selector.strip_prefix('!') {
        if !puzzle_authoring::is_qualified_identifier(name) {
            return Err(parse_error(line, "invalid named visual"));
        }
        return Ok(vec![VisualSelectorTarget {
            object_name: name.to_string(),
            bindings: HashMap::new(),
            bind_object: false,
        }]);
    }
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
            bind_object: true,
        }]);
    }
    if let Some(objects) = catalog.object_groups.get(selector) {
        return Ok(objects
            .iter()
            .filter_map(|object| catalog.object_labels.get(object).cloned())
            .map(|object_name| VisualSelectorTarget {
                object_name,
                bindings: HashMap::new(),
                bind_object: true,
            })
            .collect());
    }

    if !selector.contains(':') && puzzle_authoring::is_qualified_identifier(selector) {
        return Ok(vec![VisualSelectorTarget {
            object_name: selector.to_string(),
            bindings: HashMap::new(),
            bind_object: false,
        }]);
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
            bind_object: true,
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
            } else {
                Ok(VisualSelectorConstraint::Fixed(normalize_axis_literal(
                    &name, schema, index, line,
                )?))
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

fn visual_name_for_object(object_name: &str) -> String {
    let mut visual = String::new();
    for ch in object_name.chars() {
        if ch == '@' || ch.is_ascii_alphanumeric() {
            visual.push(ch);
        } else if !visual.ends_with('-') {
            visual.push('-');
        }
    }
    let visual = visual.trim_matches('-').to_string();
    if visual.is_empty() {
        "unknown".to_string()
    } else {
        visual
    }
}
