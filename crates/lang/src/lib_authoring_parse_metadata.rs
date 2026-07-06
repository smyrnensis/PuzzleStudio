fn parse_theme_block(
    lines: &[String],
    start: usize,
    theme: &mut ThemeDef,
) -> Result<usize, DiagnosticReport> {
    let header = split_header_tokens(&lines[start]);
    match header.as_slice() {
        ["theme", name] => {
            parse_theme_name_directive(&lines[start], name, theme)?;
        }
        _ => {
            return Err(parse_error(
                &lines[start],
                "theme header must be: theme <theme> or theme <theme> {",
            ));
        }
    }

    let mut i = start + 1;
    while i < lines.len() {
        let line = &lines[i];
        if is_block_close_line(line) {
            return Ok(i + 1);
        }
        let tokens = split_header_tokens(line);
        match parse_theme_setting_tokens(tokens.as_slice(), line) {
            Ok(Some((name, value))) => upsert_theme_variable(theme, name, value),
            Ok(None) => {}
            Err(error) => return Err(error),
        }
        i += 1;
    }

    Err(parse_error(&lines[start], "theme missing closing brace"))
}

fn parse_theme_setting_tokens(
    tokens: &[&str],
    line: &str,
) -> Result<Option<(String, String)>, DiagnosticReport> {
    match tokens {
        [name, value] => {
            let name = normalize_theme_setting_name(name, line)?;
            validate_theme_value(value, line)?;
            Ok(Some((name, (*value).to_string())))
        }
        [name, "=", value] => {
            let name = normalize_theme_setting_name(name, line)?;
            validate_theme_value(value, line)?;
            Ok(Some((name, (*value).to_string())))
        }
        _ => Err(parse_error(
            line,
            "theme entry must be: <setting> <value> or <setting> = <value>",
        )),
    }
}

fn parse_theme_statement(
    lines: &[String],
    start: usize,
    theme: &mut ThemeDef,
) -> Result<usize, DiagnosticReport> {
    let tokens = split_header_tokens(&lines[start]);
    let ["theme", name] = tokens.as_slice() else {
        return Err(parse_error(
            &lines[start],
            "theme header must be: theme <theme> or theme <theme> {",
        ));
    };
    if lines[start].trim_end().ends_with('{')
        || lines
            .get(start + 1)
            .is_some_and(|line| is_block_close_line(line) || is_theme_setting_line(line))
    {
        return parse_theme_block(lines, start, theme);
    }
    parse_theme_name_directive(&lines[start], name, theme)?;
    Ok(start + 1)
}

fn parse_theme_name_directive(
    line: &str,
    name: &str,
    theme: &mut ThemeDef,
) -> Result<(), DiagnosticReport> {
    validate_qualified_identifier(name, line, "theme name")?;
    theme.name = Some(name.to_string());
    Ok(())
}

fn is_theme_setting_line(line: &str) -> bool {
    let tokens = split_header_tokens(line);
    parse_theme_setting_tokens(tokens.as_slice(), line).is_ok()
}

fn parse_assets_block(
    lines: &[String],
    start: usize,
    assets: &mut AssetsDef,
) -> Result<usize, DiagnosticReport> {
    let header = split_header_tokens(&lines[start]);
    if !matches!(header.as_slice(), ["assets"]) {
        return Err(parse_error(&lines[start], "assets header must be: assets"));
    }

    let mut i = start + 1;
    while i < lines.len() {
        let line = &lines[i];
        if is_block_close_line(line) {
            return Ok(i + 1);
        }
        let tokens = split_header_tokens(line);
        match tokens.as_slice() {
            ["css", path] => assets.entries.push(AssetDef {
                kind: AssetKind::Css,
                path: parse_asset_path(path, line)?,
            }),
            ["script", path] => assets.entries.push(AssetDef {
                kind: AssetKind::Script,
                path: parse_asset_path(path, line)?,
            }),
            ["file", path] => assets.entries.push(AssetDef {
                kind: AssetKind::File,
                path: parse_asset_path(path, line)?,
            }),
            _ => {
                return Err(parse_error(
                    line,
                    "assets entry must be: css \"path\" | script \"path\" | file \"path\"",
                ));
            }
        }
        i += 1;
    }
    Err(parse_error(&lines[start], "assets missing closing brace"))
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
            "asset path must be a game-folder relative path",
        ));
    }
    Ok(path.to_string())
}

fn parse_metadata_text(line: &str, keyword: &str) -> Result<String, DiagnosticReport> {
    let Some(rest) = line.strip_prefix(keyword) else {
        return Err(parse_error(
            line,
            "metadata directive has the wrong keyword",
        ));
    };
    let value = rest.trim();
    if value.is_empty() {
        return Err(parse_error(line, "metadata value must not be empty"));
    }
    Ok(parse_quoted_text(value).unwrap_or_else(|| value.to_string()))
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

fn required_sound_setting<'a>(
    settings: &'a [&'a str],
    key: &str,
    line: &str,
) -> Result<&'a str, DiagnosticReport> {
    optional_sound_setting(settings, key)
        .ok_or_else(|| parse_error(line, &format!("sounds setting `{key}` is required")))
}

fn optional_sound_setting<'a>(settings: &'a [&'a str], key: &str) -> Option<&'a str> {
    settings.iter().find_map(|setting| {
        let (name, value) = parse_assignment_row(setting)?;
        (name == key).then_some(value)
    })
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
