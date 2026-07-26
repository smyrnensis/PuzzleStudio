use crate::authoring_grammar::AuthoringKind;
use crate::surface::{SourceSpan, SurfaceDisplayFact, SurfaceDocument};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "operation", rename_all = "camelCase")]
pub enum LevelSourceRequest {
    Format {
        name: String,
        rows: Vec<String>,
        #[serde(default)]
        local_legends: Vec<LevelLegendDraft>,
    },
    Insert {
        name: String,
        #[serde(default)]
        namespace: String,
        rows: Vec<String>,
        #[serde(default)]
        local_legends: Vec<LevelLegendDraft>,
        #[serde(default)]
        cursor: Option<usize>,
        #[serde(default)]
        create_container: bool,
    },
    Update {
        target_start: usize,
        name: String,
        rows: Vec<String>,
        #[serde(default)]
        local_legends: Vec<LevelLegendDraft>,
    },
    InsertLegend {
        symbol: String,
        selectors: Vec<String>,
    },
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LevelLegendDraft {
    pub symbol: String,
    pub selectors: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LevelSourceResponse {
    pub source: String,
    pub start: usize,
    pub end: usize,
    pub text: String,
}

pub(crate) fn level_source_request(
    source: &str,
    document: &SurfaceDocument,
    request: LevelSourceRequest,
) -> Result<LevelSourceResponse, String> {
    match request {
        LevelSourceRequest::Format {
            name,
            rows,
            local_legends,
        } => {
            let text = format_level(&name, &rows, &local_legends, "", "");
            Ok(LevelSourceResponse {
                source: source.to_string(),
                start: 0,
                end: 0,
                text,
            })
        }
        LevelSourceRequest::Insert {
            name,
            namespace,
            rows,
            local_legends,
            cursor,
            create_container,
        } => insert_level(
            source,
            document,
            &name,
            &namespace,
            &rows,
            &local_legends,
            cursor,
            create_container,
        ),
        LevelSourceRequest::Update {
            target_start,
            name,
            rows,
            local_legends,
        } => update_level(source, document, target_start, &name, &rows, &local_legends),
        LevelSourceRequest::InsertLegend { symbol, selectors } => {
            insert_common_legend(source, document, &symbol, &selectors)
        }
    }
}

fn insert_level(
    source: &str,
    document: &SurfaceDocument,
    name: &str,
    namespace: &str,
    rows: &[String],
    local_legends: &[LevelLegendDraft],
    cursor: Option<usize>,
    create_container: bool,
) -> Result<LevelSourceResponse, String> {
    let requested_namespace = namespace.trim();
    let mut candidates = document
        .structural_blocks
        .iter()
        .filter(|block| block.authoring_kind == Some(AuthoringKind::LevelsConfig))
        .filter(|block| levels_namespace(&block.header).as_deref() == Some(requested_namespace))
        .collect::<Vec<_>>();
    let selected = cursor
        .and_then(|cursor| {
            candidates
                .iter()
                .copied()
                .find(|block| cursor >= block.start && cursor <= block.end)
        })
        .or_else(|| candidates.pop());

    let Some(block) = selected else {
        if !create_container {
            return Err(if requested_namespace.is_empty() {
                "source has no levels block".to_string()
            } else {
                format!("source has no levels block named `{requested_namespace}`")
            });
        }
        let start = source.trim_end().len();
        let separator = if start == 0 { "" } else { "\n\n" };
        let levels_header = if requested_namespace.is_empty() {
            "levels".to_string()
        } else {
            format!("levels {requested_namespace}")
        };
        let level = format_level(name, rows, local_legends, "", "");
        let text = format!("{separator}{levels_header} {{\n{level}\n}}\n");
        let mut next = source[..start].to_string();
        next.push_str(&text);
        return Ok(LevelSourceResponse {
            source: next,
            start: start + separator.len() + levels_header.len() + 3,
            end: start + text.len() - 3,
            text: level,
        });
    };
    let insertion = block
        .close_brace
        .ok_or_else(|| "levels block has no parser-owned closing boundary".to_string())?;
    let level_indent = child_indent(source, document, block.start, insertion);
    let body_indent = level_body_indent(source, document, block.start, insertion, &level_indent);
    let level = format_level(name, rows, local_legends, &level_indent, &body_indent);
    let prefix = if source[..insertion].trim_end().ends_with('{') {
        "\n"
    } else {
        "\n\n"
    };
    let text = format!("{prefix}{level}\n");
    let mut next = source.to_string();
    next.insert_str(insertion, &text);
    let start = insertion + prefix.len();
    Ok(LevelSourceResponse {
        source: next,
        start,
        end: start + level.len(),
        text: level,
    })
}

fn update_level(
    source: &str,
    document: &SurfaceDocument,
    target_start: usize,
    name: &str,
    rows: &[String],
    local_legends: &[LevelLegendDraft],
) -> Result<LevelSourceResponse, String> {
    let product = document
        .level_products
        .iter()
        .find(|product| product.span.start == target_start)
        .ok_or_else(|| format!("no parser-owned level starts at byte {target_start}"))?;
    let (map_span, body_indent) = level_map_line_span(source, document, product.body_span)
        .ok_or_else(|| "level has no parser-owned map rows".to_string())?;
    let mut replacement = format_level_body(rows, local_legends, &body_indent);
    if source[..map_span.end].ends_with('\n') {
        replacement.push('\n');
    }
    let map_delta = replacement.len() as isize - (map_span.end - map_span.start) as isize;
    let mut next = source.to_string();
    next.replace_range(map_span.start..map_span.end, &replacement);

    let name_edit = level_name_edit(source, document, product.span, name)?;
    if let Some((start, end, replacement)) = name_edit {
        let name_delta = replacement.len() as isize - (end - start) as isize;
        next.replace_range(start..end, &replacement);
        return Ok(LevelSourceResponse {
            source: next,
            start: product.span.start,
            end: product
                .span
                .end
                .saturating_add_signed(map_delta + name_delta),
            text: replacement,
        });
    }
    Ok(LevelSourceResponse {
        source: next,
        start: product.span.start,
        end: product.span.end.saturating_add_signed(map_delta),
        text: replacement,
    })
}

fn insert_common_legend(
    source: &str,
    document: &SurfaceDocument,
    symbol: &str,
    selectors: &[String],
) -> Result<LevelSourceResponse, String> {
    if symbol.chars().count() != 1
        || selectors.is_empty()
        || selectors.iter().any(|selector| selector.trim().is_empty())
    {
        return Err("legend insertion requires one symbol and at least one selector".to_string());
    }
    let row = format!("{symbol} = {}", selectors.join(" "));
    if let Some(block) = document.structural_blocks.iter().find(|block| {
        crate::split_header_tokens(&block.header).as_slice() == ["legend"]
            && block.parent.is_none_or(|parent| {
                !matches!(
                    document.structural_blocks[parent].authoring_kind,
                    Some(AuthoringKind::LevelsConfig | AuthoringKind::LevelConfig)
                )
            })
    }) {
        let insertion = block
            .close_brace
            .ok_or_else(|| "legend block has no parser-owned closing boundary".to_string())?;
        let indent = document
            .logical_lines
            .iter()
            .filter_map(|line| line.tokens.first())
            .filter(|token| {
                token.start > block.open_brace.unwrap_or(block.start) && token.start < insertion
            })
            .next_back()
            .map_or_else(
                || format!("{}  ", line_indent_at(source, block.start)),
                |token| line_indent_at(source, token.start),
            );
        let text = format!("{indent}{row}\n");
        let mut next = source.to_string();
        next.insert_str(insertion, &text);
        return Ok(LevelSourceResponse {
            source: next,
            start: insertion,
            end: insertion + text.trim_end().len(),
            text: row,
        });
    }
    let insertion = document
        .structural_blocks
        .iter()
        .filter(|block| block.authoring_kind == Some(AuthoringKind::LevelsConfig))
        .map(|block| block.start)
        .min()
        .unwrap_or_else(|| source.trim_end().len());
    let prefix = if insertion == 0 || source[..insertion].trim_end().is_empty() {
        ""
    } else {
        "\n\n"
    };
    let text = format!("{prefix}legend {{\n  {row}\n}}\n\n");
    let mut next = source.to_string();
    next.insert_str(insertion, &text);
    Ok(LevelSourceResponse {
        source: next,
        start: insertion + prefix.len() + "legend {\n  ".len(),
        end: insertion + prefix.len() + "legend {\n  ".len() + row.len(),
        text: row,
    })
}

fn levels_namespace(header: &str) -> Option<String> {
    let surface = puzzle_authoring::resource_header_surface(header, "levels").ok()?;
    Some(surface.name.unwrap_or("").to_string())
}

fn level_map_line_span(
    source: &str,
    document: &SurfaceDocument,
    body: SourceSpan,
) -> Option<(SourceSpan, String)> {
    let mut spans = document
        .highlight_ranges
        .display_facts
        .iter()
        .filter_map(|fact| match fact {
            SurfaceDisplayFact::LevelCell { span, .. }
            | SurfaceDisplayFact::LevelSeparator { span }
                if span.start >= body.start && span.end <= body.end =>
            {
                Some(*span)
            }
            _ => None,
        });
    let first = spans.next()?;
    let (start, end) = spans.fold((first.start, first.end), |(start, end), span| {
        (start.min(span.start), end.max(span.end))
    });
    let line_start = line_start(source, start);
    Some((
        SourceSpan {
            start: line_start,
            end: line_end_with_newline(source, end),
        },
        source[line_start..start].to_string(),
    ))
}

fn level_name_edit(
    source: &str,
    document: &SurfaceDocument,
    span: SourceSpan,
    name: &str,
) -> Result<Option<(usize, usize, String)>, String> {
    let Some(line) = document.logical_lines.iter().find(|line| {
        line.tokens
            .first()
            .is_some_and(|token| token.start == span.start)
    }) else {
        return Ok(None);
    };
    let tokens = line
        .tokens
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    if tokens.first().copied() != Some("level") && tokens.first().copied() != Some("{") {
        return Ok(None);
    }
    if name.trim().is_empty() {
        return Ok(None);
    }
    let quoted = serde_json::to_string(name)
        .map_err(|error| format!("could not quote level name: {error}"))?;
    if tokens.first().copied() == Some("{") {
        let token = &line.tokens[0];
        return Ok(Some((token.start, token.end, format!("level {quoted} {{"))));
    }
    let token = line
        .tokens
        .get(1)
        .ok_or_else(|| "level header has no parser-owned name token".to_string())?;
    if token.text == "{" {
        return Ok(Some((token.start, token.end, format!("{quoted} {{"))));
    }
    let _ = source;
    Ok(Some((token.start, token.end, quoted)))
}

fn format_level(
    name: &str,
    rows: &[String],
    local_legends: &[LevelLegendDraft],
    level_indent: &str,
    body_indent: &str,
) -> String {
    let quoted = serde_json::to_string(name).expect("level name is serializable");
    let mut lines = vec![format!("{level_indent}level {quoted} {{")];
    lines.extend(
        format_level_body(rows, local_legends, body_indent)
            .lines()
            .map(str::to_string),
    );
    lines.push(format!("{level_indent}}}"));
    lines.join("\n")
}

fn format_level_body(rows: &[String], local_legends: &[LevelLegendDraft], indent: &str) -> String {
    let mut lines = Vec::new();
    if !local_legends.is_empty() {
        lines.push(format!("{indent}legend {{"));
        lines.extend(
            local_legends
                .iter()
                .map(|entry| format!("{indent}{} = {}", entry.symbol, entry.selectors.join(" "))),
        );
        lines.push(format!("{indent}}}"));
    }
    lines.extend(rows.iter().map(|row| {
        if row.is_empty() {
            String::new()
        } else {
            format!("{indent}{row}")
        }
    }));
    lines.join("\n")
}

fn child_indent(source: &str, document: &SurfaceDocument, start: usize, end: usize) -> String {
    document
        .level_products
        .iter()
        .find(|level| level.span.start > start && level.span.end < end)
        .map_or_else(
            || line_indent_at(source, start),
            |level| line_indent_at(source, level.span.start),
        )
}

fn level_body_indent(
    source: &str,
    document: &SurfaceDocument,
    start: usize,
    end: usize,
    fallback: &str,
) -> String {
    document
        .level_products
        .iter()
        .find(|level| level.span.start > start && level.span.end < end)
        .and_then(|level| level_map_line_span(source, document, level.body_span))
        .map_or_else(|| fallback.to_string(), |(_, indent)| indent)
}

fn line_start(source: &str, offset: usize) -> usize {
    source[..offset].rfind('\n').map_or(0, |index| index + 1)
}

fn line_end_with_newline(source: &str, offset: usize) -> usize {
    source[offset..]
        .find('\n')
        .map_or(source.len(), |index| offset + index + 1)
}

fn line_indent_at(source: &str, offset: usize) -> String {
    source[line_start(source, offset)..offset]
        .chars()
        .take_while(|ch| *ch == ' ' || *ch == '\t')
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{LevelLegendDraft, LevelSourceRequest};
    use crate::{ModelDimension, SourceAnalysis};

    const SOURCE: &str = r#"puzzle game {
layers {
base = Player
}
legend {
. = empty
P = Player
}
rules {
[ Player ] -> [ Player ]
}
groups {
Actors = Player
}
}

levels pack {
level "old" {
on_level_start {
message "start"
}
P
on_level_clear {
message "clear"
}
}
}
"#;

    fn analysis() -> SourceAnalysis {
        SourceAnalysis::new_with_owner_dimension(SOURCE, Some(ModelDimension::Two))
    }

    #[test]
    fn update_replaces_only_parser_owned_map_and_name_spans() {
        let analysis = analysis();
        let target_start = SOURCE.find("level \"old\"").unwrap();
        let result = analysis
            .level_source_request(LevelSourceRequest::Update {
                target_start,
                name: "new".to_string(),
                rows: vec!["PP".to_string(), "PP".to_string()],
                local_legends: vec![],
            })
            .unwrap();

        assert!(result.source.contains("level \"new\" {"));
        assert!(
            result
                .source
                .contains("on_level_start {\nmessage \"start\"\n}")
        );
        assert!(
            result
                .source
                .contains("on_level_clear {\nmessage \"clear\"\n}")
        );
        assert!(
            result.source.contains("}\nPP\nPP\non_level_clear"),
            "{}",
            result.source
        );
        assert!(!result.source.contains("\nP\non_level_clear"));
    }

    #[test]
    fn insert_uses_parser_owned_named_levels_boundary() {
        let result = analysis()
            .level_source_request(LevelSourceRequest::Insert {
                name: "second".to_string(),
                namespace: "pack".to_string(),
                rows: vec!["P".to_string()],
                local_legends: vec![LevelLegendDraft {
                    symbol: "Q".to_string(),
                    selectors: vec!["Player".to_string()],
                }],
                cursor: None,
                create_container: false,
            })
            .unwrap();

        assert!(
            result
                .source
                .contains("level \"second\" {\nlegend {\nQ = Player\n}\nP\n}\n}")
        );
    }

    #[test]
    fn common_legend_insertion_is_planned_from_structural_blocks() {
        let result = analysis()
            .level_source_request(LevelSourceRequest::InsertLegend {
                symbol: "Q".to_string(),
                selectors: vec!["Actors".to_string()],
            })
            .unwrap();

        assert!(
            result
                .source
                .contains("legend {\n. = empty\nP = Player\nQ = Actors\n}"),
            "{}",
            result.source
        );
    }
}
