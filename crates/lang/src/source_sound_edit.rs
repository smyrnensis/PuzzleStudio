use serde::{Deserialize, Serialize};

use crate::authoring_grammar::AuthoringKind;
use crate::surface::{
    ParserRecognition, ParserTokenResolution, SourceSpan, SurfaceDocument, SurfaceSoundKind,
    SurfaceSoundProduct,
};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SoundDefinitionKind {
    Sfx,
    Music,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SoundDefinitionDraft {
    Sfx {
        name: String,
        seed: String,
        #[serde(rename = "type")]
        type_target: String,
        volume: f64,
    },
    Music {
        name: String,
        seed: String,
        height: f64,
        bars: u16,
        bpm: u16,
        volume: f64,
    },
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "operation", rename_all = "camelCase")]
pub enum SoundSourceRequest {
    Inspect {
        cursor: usize,
    },
    Format {
        definition: SoundDefinitionDraft,
    },
    Insert {
        definition: SoundDefinitionDraft,
    },
    Update {
        target_start: usize,
        original_name: String,
        definition: SoundDefinitionDraft,
    },
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SoundDefinitionInspection {
    pub definition: SoundDefinitionDraft,
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SoundSourceMutationResult {
    pub source: String,
    pub selection_start: usize,
    pub selection_end: usize,
    pub definition_start: usize,
    pub definition_end: usize,
    pub definition: SoundDefinitionDraft,
    pub renamed_reference_count: usize,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SoundSourceResponse {
    Inspection {
        definition: Option<SoundDefinitionInspection>,
    },
    Formatted {
        line: String,
        definition: SoundDefinitionDraft,
    },
    Mutation {
        result: SoundSourceMutationResult,
    },
}

pub(crate) fn sound_source_request(
    source: &str,
    document: &SurfaceDocument,
    recognition: &ParserRecognition,
    request: SoundSourceRequest,
) -> Result<SoundSourceResponse, String> {
    match request {
        SoundSourceRequest::Inspect { cursor } => {
            let definition = document
                .sound_products
                .iter()
                .find(|product| cursor >= product.span.start && cursor <= product.span.end)
                .map(sound_inspection_from_product)
                .transpose()?;
            Ok(SoundSourceResponse::Inspection { definition })
        }
        SoundSourceRequest::Format { definition } => {
            let definition = normalize_definition(definition)?;
            Ok(SoundSourceResponse::Formatted {
                line: format_definition(&definition),
                definition,
            })
        }
        SoundSourceRequest::Insert { definition } => {
            let definition = unique_definition(document, normalize_definition(definition)?, None);
            let result = insert_definition(source, document, definition)?;
            Ok(SoundSourceResponse::Mutation { result })
        }
        SoundSourceRequest::Update {
            target_start,
            original_name,
            definition,
        } => {
            let definition = normalize_definition(definition)?;
            let target = find_update_target(
                document,
                target_start,
                definition_kind(&definition),
                &original_name,
            )?;
            if document.sound_products.iter().any(|product| {
                product.span != target.span
                    && product_kind(product) == definition_kind(&definition)
                    && product.name == definition_name(&definition)
            }) {
                return Err(format!(
                    "{} {} already exists",
                    kind_name(definition_kind(&definition)),
                    definition_name(&definition)
                ));
            }
            let result = update_definition(source, recognition, target, definition)?;
            Ok(SoundSourceResponse::Mutation { result })
        }
    }
}

fn sound_inspection_from_product(
    product: &SurfaceSoundProduct,
) -> Result<SoundDefinitionInspection, String> {
    let param = |name: &str| {
        product
            .params
            .iter()
            .find_map(|(key, value)| (key == name).then_some(value.as_str()))
    };
    let definition = match product.kind {
        SurfaceSoundKind::Sfx => SoundDefinitionDraft::Sfx {
            name: product.name.clone(),
            seed: param("seed").unwrap_or("123456").to_string(),
            type_target: param("type").unwrap_or("random").to_string(),
            volume: parse_param(param("volume").unwrap_or("1"), "sfx volume")?,
        },
        SurfaceSoundKind::Music => SoundDefinitionDraft::Music {
            name: product.name.clone(),
            seed: param("seed").unwrap_or("123456").to_string(),
            height: parse_param(
                param("height").or_else(|| param("tone")).unwrap_or("0.5"),
                "music height",
            )?,
            bars: parse_param(param("bars").unwrap_or("8"), "music bars")?,
            bpm: parse_param(param("bpm").unwrap_or("110"), "music bpm")?,
            volume: parse_param(param("volume").unwrap_or("0.5"), "music volume")?,
        },
    };
    Ok(SoundDefinitionInspection {
        definition,
        start: product.span.start,
        end: product.span.end,
    })
}

fn parse_param<T: std::str::FromStr>(value: &str, label: &str) -> Result<T, String> {
    value
        .parse()
        .map_err(|_| format!("{label} is not a valid number"))
}

fn normalize_definition(definition: SoundDefinitionDraft) -> Result<SoundDefinitionDraft, String> {
    let normalized = match definition {
        SoundDefinitionDraft::Sfx {
            name,
            seed,
            type_target,
            volume,
        } => {
            if !volume.is_finite() || volume < 0.0 {
                return Err("sfx volume must be a finite non-negative number".to_string());
            }
            SoundDefinitionDraft::Sfx {
                name: normalize_identifier(&name, "sfx"),
                seed: normalize_atom(&seed, "123456"),
                type_target: normalize_atom(&type_target, "random"),
                volume,
            }
        }
        SoundDefinitionDraft::Music {
            name,
            seed,
            height,
            bars,
            bpm,
            volume,
        } => {
            if !height.is_finite() || !(0.0..=1.0).contains(&height) {
                return Err("music height must be between 0 and 1".to_string());
            }
            if !matches!(bars, 8 | 16 | 32 | 64) {
                return Err("music bars must be one of 8, 16, 32, or 64".to_string());
            }
            if !(40..=180).contains(&bpm) {
                return Err("music bpm must be between 40 and 180".to_string());
            }
            if !volume.is_finite() || volume < 0.0 {
                return Err("music volume must be a finite non-negative number".to_string());
            }
            SoundDefinitionDraft::Music {
                name: normalize_identifier(&name, "music"),
                seed: normalize_atom(&seed, "123456"),
                height,
                bars,
                bpm,
                volume,
            }
        }
    };
    Ok(normalized)
}

fn normalize_identifier(value: &str, fallback: &str) -> String {
    let mut out = String::new();
    for character in value.trim().chars() {
        if character.is_ascii_alphanumeric() || character == '_' {
            out.push(character);
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    let out = out.trim_matches('_');
    if out.is_empty() || out.starts_with(|character: char| character.is_ascii_digit()) {
        fallback.to_string()
    } else {
        out.to_string()
    }
}

fn normalize_atom(value: &str, fallback: &str) -> String {
    let mut out = String::new();
    for character in value.trim().chars() {
        if character.is_alphanumeric() || matches!(character, '_' | '.' | '-') {
            out.push(character);
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    let out = out.trim_matches('_');
    if out.is_empty() {
        fallback.to_string()
    } else {
        out.to_string()
    }
}

fn unique_definition(
    document: &SurfaceDocument,
    mut definition: SoundDefinitionDraft,
    except: Option<SourceSpan>,
) -> SoundDefinitionDraft {
    let kind = definition_kind(&definition);
    let requested = definition_name(&definition).to_string();
    let exists = |candidate: &str| {
        document.sound_products.iter().any(|product| {
            Some(product.span) != except
                && product_kind(product) == kind
                && product.name == candidate
        })
    };
    if !exists(&requested) {
        return definition;
    }
    let (root, mut index) = name_sequence(&requested);
    while index < 100_000 {
        let candidate = format!("{root}_{index}");
        if !exists(&candidate) {
            *definition_name_mut(&mut definition) = candidate;
            return definition;
        }
        index += 1;
    }
    unreachable!("finite source cannot contain every generated sound name")
}

fn name_sequence(name: &str) -> (&str, usize) {
    let Some((root, suffix)) = name.rsplit_once('_') else {
        return (name, 2);
    };
    match suffix.parse::<usize>() {
        Ok(index) if index > 0 => (root, index.saturating_add(1)),
        _ => (name, 2),
    }
}

fn format_definition(definition: &SoundDefinitionDraft) -> String {
    match definition {
        SoundDefinitionDraft::Sfx {
            name,
            seed,
            type_target,
            volume,
        } => format!("sfx {name} {{ seed = {seed}; type = {type_target}; volume = {volume:.2} }}"),
        SoundDefinitionDraft::Music {
            name,
            seed,
            height,
            bars,
            bpm,
            volume,
        } => format!(
            "music {name} {{ seed = {seed}; bars = {bars}; height = {height:.2}; bpm = {bpm}; volume = {volume:.2} }}"
        ),
    }
}

fn insert_definition(
    source: &str,
    document: &SurfaceDocument,
    definition: SoundDefinitionDraft,
) -> Result<SoundSourceMutationResult, String> {
    let line = format_definition(&definition);
    if let Some(block) = document.structural_blocks.iter().find(|block| {
        block.authoring_kind == Some(AuthoringKind::SoundsConfig) && block.close_brace.is_some()
    }) {
        let close = block.close_brace.expect("sounds block close checked");
        let indent = document
            .sound_products
            .iter()
            .find_map(|product| line_indent_at(source, product.span.start))
            .or_else(|| line_indent_at(source, block.start))
            .unwrap_or_default();
        let prefix = if close > 0 && source.as_bytes()[close - 1] == b'\n' {
            String::new()
        } else {
            "\n".to_string()
        };
        let inserted = format!("{prefix}{indent}{line}\n");
        let definition_start = close + prefix.len() + indent.len();
        let definition_end = definition_start + line.len();
        let next_source = format!("{}{}{}", &source[..close], inserted, &source[close..]);
        let selection = close + inserted.len();
        return Ok(SoundSourceMutationResult {
            source: next_source,
            selection_start: selection,
            selection_end: selection,
            definition_start,
            definition_end,
            definition,
            renamed_reference_count: 0,
        });
    }

    let insertion = top_level_name_line_end(source, document).unwrap_or(source.len());
    let before = &source[..insertion];
    let after = &source[insertion..];
    let leading = if before.is_empty() {
        String::new()
    } else if before.ends_with("\n\n") {
        String::new()
    } else if before.ends_with('\n') {
        "\n".to_string()
    } else {
        "\n\n".to_string()
    };
    let block = format!("{leading}sounds {{\n{line}\n}}\n");
    let definition_start = insertion + leading.len() + "sounds {\n".len();
    let definition_end = definition_start + line.len();
    let selection = insertion + block.len();
    Ok(SoundSourceMutationResult {
        source: format!("{before}{block}{after}"),
        selection_start: selection,
        selection_end: selection,
        definition_start,
        definition_end,
        definition,
        renamed_reference_count: 0,
    })
}

fn top_level_name_line_end(source: &str, document: &SurfaceDocument) -> Option<usize> {
    let token = document.logical_lines.iter().find_map(|line| {
        let first = line.tokens.first()?;
        (first.text == "name"
            && !document
                .structural_blocks
                .iter()
                .any(|block| first.start > block.start && first.start < block.end))
        .then_some(first)
    })?;
    Some(
        source[token.end..]
            .find('\n')
            .map(|relative| token.end + relative + 1)
            .unwrap_or(source.len()),
    )
}

fn line_indent_at(source: &str, offset: usize) -> Option<String> {
    if offset > source.len() || !source.is_char_boundary(offset) {
        return None;
    }
    let line_start = source[..offset].rfind('\n').map_or(0, |index| index + 1);
    let indent = source[line_start..offset]
        .chars()
        .take_while(|character| character.is_whitespace())
        .collect::<String>();
    Some(indent)
}

fn find_update_target<'a>(
    document: &'a SurfaceDocument,
    target_start: usize,
    kind: SoundDefinitionKind,
    original_name: &str,
) -> Result<&'a SurfaceSoundProduct, String> {
    document
        .sound_products
        .iter()
        .find(|product| {
            product_kind(product) == kind
                && product.name == original_name
                && target_start >= product.span.start
                && target_start <= product.span.end
        })
        .ok_or_else(|| format!("No {} named {original_name}", kind_name(kind)))
}

fn update_definition(
    source: &str,
    recognition: &ParserRecognition,
    target: &SurfaceSoundProduct,
    definition: SoundDefinitionDraft,
) -> Result<SoundSourceMutationResult, String> {
    let replacement = format_definition(&definition);
    let old_name = target.name.as_str();
    let new_name = definition_name(&definition);
    let mut edits = vec![(target.span, replacement)];
    let mut renamed_reference_count = 0;
    if old_name != new_name {
        for disposition in &recognition.token_dispositions {
            let matches = match &disposition.resolution {
                Some(ParserTokenResolution::Sfx(name)) => {
                    definition_kind(&definition) == SoundDefinitionKind::Sfx && name == old_name
                }
                Some(ParserTokenResolution::Music(name)) => {
                    definition_kind(&definition) == SoundDefinitionKind::Music && name == old_name
                }
                _ => false,
            };
            if matches && disposition.span != target.span {
                edits.push((disposition.span, new_name.to_string()));
                renamed_reference_count += 1;
            }
        }
    }
    edits.sort_by_key(|(span, _)| std::cmp::Reverse(span.start));
    let definition_start_before_references = target.span.start;
    let definition_end_before_references = definition_start_before_references
        + edits
            .iter()
            .find_map(|(span, replacement)| (span == &target.span).then_some(replacement.len()))
            .expect("definition edit exists");
    let mut next_source = source.to_string();
    for (span, replacement) in edits {
        if span.end > next_source.len()
            || !next_source.is_char_boundary(span.start)
            || !next_source.is_char_boundary(span.end)
        {
            return Err("parser returned an invalid sound edit span".to_string());
        }
        next_source.replace_range(span.start..span.end, &replacement);
    }
    let prior_reference_shift = reference_selection_shift(
        recognition,
        target.span.start,
        old_name,
        new_name,
        definition_kind(&definition),
    );
    let definition_start =
        definition_start_before_references.saturating_add_signed(prior_reference_shift);
    let definition_end =
        definition_end_before_references.saturating_add_signed(prior_reference_shift);
    let selection = definition_end;
    Ok(SoundSourceMutationResult {
        source: next_source,
        selection_start: selection,
        selection_end: selection,
        definition_start,
        definition_end,
        definition,
        renamed_reference_count,
    })
}

fn reference_selection_shift(
    recognition: &ParserRecognition,
    before: usize,
    old_name: &str,
    new_name: &str,
    kind: SoundDefinitionKind,
) -> isize {
    let count = recognition
        .token_dispositions
        .iter()
        .filter(|disposition| {
            disposition.span.end <= before
                && match &disposition.resolution {
                    Some(ParserTokenResolution::Sfx(name)) => {
                        kind == SoundDefinitionKind::Sfx && name == old_name
                    }
                    Some(ParserTokenResolution::Music(name)) => {
                        kind == SoundDefinitionKind::Music && name == old_name
                    }
                    _ => false,
                }
        })
        .count();
    (new_name.len() as isize - old_name.len() as isize) * count as isize
}

fn product_kind(product: &SurfaceSoundProduct) -> SoundDefinitionKind {
    match product.kind {
        SurfaceSoundKind::Sfx => SoundDefinitionKind::Sfx,
        SurfaceSoundKind::Music => SoundDefinitionKind::Music,
    }
}

fn definition_kind(definition: &SoundDefinitionDraft) -> SoundDefinitionKind {
    match definition {
        SoundDefinitionDraft::Sfx { .. } => SoundDefinitionKind::Sfx,
        SoundDefinitionDraft::Music { .. } => SoundDefinitionKind::Music,
    }
}

fn definition_name(definition: &SoundDefinitionDraft) -> &str {
    match definition {
        SoundDefinitionDraft::Sfx { name, .. } | SoundDefinitionDraft::Music { name, .. } => name,
    }
}

fn definition_name_mut(definition: &mut SoundDefinitionDraft) -> &mut String {
    match definition {
        SoundDefinitionDraft::Sfx { name, .. } | SoundDefinitionDraft::Music { name, .. } => name,
    }
}

fn kind_name(kind: SoundDefinitionKind) -> &'static str {
    match kind {
        SoundDefinitionKind::Sfx => "sfx",
        SoundDefinitionKind::Music => "music",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SourceAnalysis;

    fn request(source: &str, request: SoundSourceRequest) -> SoundSourceResponse {
        let analysis = SourceAnalysis::new(source);
        analysis.sound_source_request(request).unwrap()
    }

    #[test]
    fn inspection_uses_parser_owned_products_with_comments_quotes_and_braces() {
        let source = "name 😀\nsounds {\n// music fake { seed = no }\nmusic theme { seed = \"a//b\"; bars = 8; height = 0.25; bpm = 90; volume = 0.4 }\n}\n";
        let cursor = source.find("theme").unwrap();
        let SoundSourceResponse::Inspection {
            definition: Some(inspection),
        } = request(source, SoundSourceRequest::Inspect { cursor })
        else {
            panic!("inspection");
        };
        assert_eq!(inspection.start, source.find("music theme").unwrap());
        assert_eq!(
            inspection.definition,
            SoundDefinitionDraft::Music {
                name: "theme".to_string(),
                seed: "a//b".to_string(),
                height: 0.25,
                bars: 8,
                bpm: 90,
                volume: 0.4,
            }
        );
    }

    #[test]
    fn insertion_uses_first_parser_owned_sounds_block_and_unique_name() {
        let source = "name game\nsounds {\nsfx click { seed = one; type = hit }\n}\nscene other {\ntext \"sounds { }\"\n}\n";
        let SoundSourceResponse::Mutation { result } = request(
            source,
            SoundSourceRequest::Insert {
                definition: SoundDefinitionDraft::Sfx {
                    name: "click".to_string(),
                    seed: "two".to_string(),
                    type_target: "jump".to_string(),
                    volume: 0.5,
                },
            },
        ) else {
            panic!("mutation");
        };
        assert!(
            result
                .source
                .contains("sfx click_2 { seed = two; type = jump; volume = 0.50 }")
        );
        assert_eq!(definition_name(&result.definition), "click_2");
        assert_eq!(result.source.matches("sounds {").count(), 2);
    }

    #[test]
    fn insertion_without_sounds_follows_top_level_name() {
        let source = "name game\nscene intro {\ntext \"hello\"\n}\n";
        let SoundSourceResponse::Mutation { result } = request(
            source,
            SoundSourceRequest::Insert {
                definition: SoundDefinitionDraft::Sfx {
                    name: "step".to_string(),
                    seed: "step".to_string(),
                    type_target: "step".to_string(),
                    volume: 1.0,
                },
            },
        ) else {
            panic!("mutation");
        };
        assert!(result.source.starts_with("name game\n\nsounds {\nsfx step"));
    }

    #[test]
    fn update_renames_only_parser_accepted_sound_references() {
        let source = "puzzle game {\nlayers {\nbase = Player\n}\nlegend P = Player\nlegend {\n. = empty\nP = Player\n}\nrules {\n[ Player ] -> sfx old\n}\nlevel \"start\" {\nP\n}\n}\nsounds {\nsfx old { seed = old; type = hit }\n}\n// sfx old\n";
        let target_start = source.find("sfx old {").unwrap();
        let SoundSourceResponse::Mutation { result } = request(
            source,
            SoundSourceRequest::Update {
                target_start,
                original_name: "old".to_string(),
                definition: SoundDefinitionDraft::Sfx {
                    name: "new".to_string(),
                    seed: "new".to_string(),
                    type_target: "jump".to_string(),
                    volume: 0.75,
                },
            },
        ) else {
            panic!("mutation");
        };
        assert!(
            result
                .source
                .contains("sfx new { seed = new; type = jump; volume = 0.75 }")
        );
        assert!(result.source.contains("[ Player ] -> sfx new"));
        assert!(result.source.contains("// sfx old"));
        assert_eq!(result.renamed_reference_count, 1);
        assert_eq!(
            &result.source[result.definition_start..result.definition_end],
            "sfx new { seed = new; type = jump; volume = 0.75 }"
        );
    }

    #[test]
    fn update_renames_scene_and_rewrite_references_through_one_typed_resolution() {
        let source = "sounds {\nsfx old_sfx { seed = old; type = hit }\nmusic old_music { seed = old; bars = 8; height = 0.5; bpm = 100 }\n}\npuzzle game {\nlayers {\nbase = Player\n}\nlegend P = Player\nlegend {\n. = empty\nP = Player\n}\nrules {\n[ Player ] -> sfx old_sfx\n}\nlevel \"start\" {\nP\n}\n}\nscene menu {\non_scene_start {\nsfx old_sfx\nplay_music old_music\npause_music old_music\nresume_music old_music\nstop_music old_music\n}\n}\n";
        let analysis = SourceAnalysis::new(source);
        let SoundSourceResponse::Mutation { result: sfx } = analysis
            .sound_source_request(SoundSourceRequest::Update {
                target_start: source.find("sfx old_sfx {").unwrap(),
                original_name: "old_sfx".to_string(),
                definition: SoundDefinitionDraft::Sfx {
                    name: "new_sfx".to_string(),
                    seed: "new".to_string(),
                    type_target: "jump".to_string(),
                    volume: 1.0,
                },
            })
            .unwrap()
        else {
            panic!("sfx mutation");
        };
        assert!(sfx.source.contains("[ Player ] -> sfx new_sfx"));
        assert!(sfx.source.contains("\nsfx new_sfx\nplay_music"));
        assert_eq!(sfx.renamed_reference_count, 2);

        let music_analysis = SourceAnalysis::new(&sfx.source);
        let SoundSourceResponse::Mutation { result: music } = music_analysis
            .sound_source_request(SoundSourceRequest::Update {
                target_start: sfx.source.find("music old_music {").unwrap(),
                original_name: "old_music".to_string(),
                definition: SoundDefinitionDraft::Music {
                    name: "new_music".to_string(),
                    seed: "new".to_string(),
                    height: 0.5,
                    bars: 8,
                    bpm: 100,
                    volume: 0.5,
                },
            })
            .unwrap()
        else {
            panic!("music mutation");
        };
        for command in ["play_music", "pause_music", "resume_music", "stop_music"] {
            assert!(music.source.contains(&format!("{command} new_music")));
        }
        assert_eq!(music.renamed_reference_count, 4);
    }

    #[test]
    fn full_document_recognition_retains_typed_scene_sound_resolutions() {
        let source = "sounds {\nsfx click { seed = click; type = hit }\nmusic theme { seed = theme; bars = 8; height = 0.5; bpm = 100 }\n}\nscene menu {\non_scene_start {\nsfx click\nplay_music theme\npause_music theme\nresume_music theme\nstop_music theme\n}\n}\n";
        let snapshot = crate::ParseSnapshot::parse(source, None);
        let resolutions = snapshot
            .parser_recognition()
            .token_dispositions
            .iter()
            .filter_map(|token| token.resolution.as_ref())
            .collect::<Vec<_>>();
        assert_eq!(
            resolutions
                .iter()
                .filter(|resolution| {
                    matches!(resolution, ParserTokenResolution::Sfx(name) if name == "click")
                })
                .count(),
            1
        );
        assert_eq!(
            resolutions
                .iter()
                .filter(|resolution| {
                    matches!(resolution, ParserTokenResolution::Music(name) if name == "theme")
                })
                .count(),
            4
        );
    }

    #[test]
    fn update_rejects_duplicate_definition_name() {
        let source =
            "sounds {\nsfx one { seed = one; type = hit }\nsfx two { seed = two; type = hit }\n}\n";
        let analysis = SourceAnalysis::new(source);
        let error = analysis
            .sound_source_request(SoundSourceRequest::Update {
                target_start: source.find("sfx one").unwrap(),
                original_name: "one".to_string(),
                definition: SoundDefinitionDraft::Sfx {
                    name: "two".to_string(),
                    seed: "one".to_string(),
                    type_target: "hit".to_string(),
                    volume: 1.0,
                },
            })
            .unwrap_err();
        assert_eq!(error, "sfx two already exists");
    }
}
