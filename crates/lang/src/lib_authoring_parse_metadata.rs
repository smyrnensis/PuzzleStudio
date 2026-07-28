fn parse_theme_block(
    lines: &[source::LogicalLine],
    start: usize,
    theme: &mut ThemeDef,
) -> Result<usize, DiagnosticReport> {
    let (node, next_i) = authoring_grammar::parse_placed_authoring_node(
        lines,
        start,
        authoring_grammar::AuthoringKind::Root,
        "theme missing closing brace",
    )?;
    if node.kind != authoring_grammar::AuthoringKind::ThemeConfig {
        return Err(parse_error(&lines[start], "theme header must be: theme"));
    }
    for definition in &node.definition_rows {
        if definition.op != Some(authoring_grammar::AuthoringDefinitionOp::Equals) {
            return Err(parse_error(
                &definition.source_line,
                "theme entry must use `=`",
            ));
        }
        let Some(value) = definition.single_value() else {
            return Err(parse_error(
                &definition.source_line,
                "theme entry must have one value",
            ));
        };
        let name = normalize_theme_setting_name(&definition.key, &definition.source_line)?;
        validate_theme_value(value, &definition.source_line)?;
        upsert_theme_variable(theme, name, value.to_string());
    }
    Ok(next_i)
}

fn parse_theme_statement(
    lines: &[source::LogicalLine],
    start: usize,
    theme: &mut ThemeDef,
) -> Result<usize, DiagnosticReport> {
    if is_block_header_line(&lines[start]) {
        return parse_theme_block(lines, start, theme);
    }
    let Some(definition) = authoring_grammar::parse_authoring_definition_row(
        authoring_grammar::AuthoringKind::Root,
        &lines[start],
    )?
    else {
        return Err(parse_error(
            &lines[start],
            "theme must be: theme = <preset> or theme { ... }",
        ));
    };
    if definition.key != "theme" {
        return Err(parse_error(
            &lines[start],
            "theme directive has the wrong keyword",
        ));
    }
    if definition.op != Some(authoring_grammar::AuthoringDefinitionOp::Equals) {
        return Err(parse_error(&lines[start], "theme must use `=`"));
    }
    let Some(value) = definition.single_value() else {
        return Err(parse_error(&lines[start], "theme must have one value"));
    };
    let spec = authoring_grammar::authoring_definition_spec(
        authoring_grammar::AuthoringKind::Root,
        &definition.key,
    )
    .expect("root theme shortcut definition exists");
    set_theme_preset_from_spec(theme, spec, value, &lines[start])?;
    Ok(start + 1)
}

fn set_theme_preset_from_spec(
    theme: &mut ThemeDef,
    spec: &authoring_grammar::DefinitionSpec,
    value: &str,
    line: &str,
) -> Result<(), DiagnosticReport> {
    authoring_grammar::validate_definition_value_domain(spec, value, line)?;
    let preset = authoring_grammar::definition_value_literal(spec, value, line)?;
    theme.name = Some(preset.to_string());
    Ok(())
}

fn parse_assets_block(
    lines: &[source::LogicalLine],
    start: usize,
    assets: &mut AssetsDef,
) -> Result<usize, DiagnosticReport> {
    let (node, next_i) = authoring_grammar::parse_placed_authoring_node(
        lines,
        start,
        authoring_grammar::AuthoringKind::Root,
        "assets missing closing brace",
    )?;
    if node.kind != authoring_grammar::AuthoringKind::AssetsConfig {
        return Err(parse_error(&lines[start], "assets header must be: assets"));
    }

    for row in &node.content_rows {
        let path = authoring_grammar::authoring_capture_first(&row.captures, "path")
            .ok_or_else(|| parse_error(&row.source_line, "assets entry must include a path"))?;
        let path = parse_asset_path(path, &row.source_line)?;
        reject_executable_asset_path(&path, &row.source_line)?;
        assets.files.push(path);
    }
    Ok(next_i)
}

fn reject_executable_asset_path(path: &str, line: &str) -> Result<(), DiagnosticReport> {
    let extension = path
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase());
    if let Some(extension @ ("css" | "js" | "mjs")) = extension.as_deref() {
        return Err(parse_error(
            line,
            &format!(
                "assets does not support authored .{extension} CSS/JavaScript assets"
            ),
        ));
    }
    Ok(())
}

fn parse_asset_path(token: &str, line: &str) -> Result<String, DiagnosticReport> {
    let path = token
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .ok_or_else(|| parse_error(line, "asset path must be quoted"))?;
    if path.is_empty() {
        return Err(parse_error(line, "asset path must not be empty"));
    }
    if path.starts_with('/') || path.contains('\\') || path.split('/').any(|part| part == "..") {
        return Err(parse_error(
            line,
            "asset path must be a workspace-relative path",
        ));
    }
    Ok(path.to_string())
}

fn normalize_theme_setting_name(name: &str, line: &str) -> Result<String, DiagnosticReport> {
    let normalized = name
        .trim_start_matches("--")
        .replace('_', "-")
        .to_ascii_lowercase();
    for spec in THEME_SETTING_SPECS {
        if normalized == spec.canonical.replace('_', "-")
            || spec.aliases.iter().any(|alias| normalized == *alias)
        {
            return Ok(spec.css_variable.to_string());
        }
    }
    Err(parse_error(
        line,
        &format!(
            "theme setting must be one of: {}",
            THEME_SETTING_SPECS
                .iter()
                .map(|spec| spec.canonical)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    ))
}

fn validate_theme_value(value: &str, line: &str) -> Result<(), DiagnosticReport> {
    let is_safe_css_token = !value.is_empty()
        && value.chars().all(|ch| {
            ch.is_ascii_alphanumeric()
                || matches!(
                    ch,
                    '#' | '.' | ',' | '%' | '(' | ')' | '-' | '_' | '/' | ':' | '+'
                )
        });
    if is_safe_css_token {
        Ok(())
    } else {
        Err(parse_error(
            line,
            "theme setting value must be a compact CSS token without spaces",
        ))
    }
}

fn upsert_theme_variable(theme: &mut ThemeDef, name: String, value: String) {
    if let Some(existing) = theme
        .variables
        .iter_mut()
        .find(|variable| variable.name == name)
    {
        existing.value = value;
    } else {
        theme.variables.push(ThemeVariableDef { name, value });
    }
}

fn validate_sound_atom(value: &str, line: &str, label: &str) -> Result<(), DiagnosticReport> {
    if !value.is_empty()
        && value
            .chars()
            .all(|ch| ch == '_' || ch == '-' || ch == '.' || ch.is_ascii_alphanumeric())
    {
        Ok(())
    } else {
        Err(parse_error(
            line,
            &format!("{label} must be a compact atom"),
        ))
    }
}

fn parse_sound_f64(value: &str, line: &str, label: &str) -> Result<f64, DiagnosticReport> {
    let parsed = value
        .parse::<f64>()
        .map_err(|_| parse_error(line, &format!("{label} must be a number")))?;
    if parsed.is_finite() {
        Ok(parsed)
    } else {
        Err(parse_error(line, &format!("{label} must be finite")))
    }
}

fn parse_sound_u16(value: &str, line: &str, label: &str) -> Result<u16, DiagnosticReport> {
    value
        .parse::<u16>()
        .map_err(|_| parse_error(line, &format!("{label} must be u16")))
}
