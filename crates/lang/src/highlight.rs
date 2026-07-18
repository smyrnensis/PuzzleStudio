use crate::{SourceOutlineItem, SurfaceDocument};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HighlightedSource {
    pub spans: Vec<SourceHighlightSpan>,
    pub parsed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HighlightedSourceWithOutline {
    pub highlighted: HighlightedSource,
    pub outline: Vec<SourceOutlineItem>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceHighlightSpan {
    pub start: usize,
    pub end: usize,
    pub kind: SourceHighlightKind,
    pub(crate) color: Option<SourceHighlightColor>,
    pub(crate) transparent: bool,
}

impl SourceHighlightSpan {
    pub fn color(&self) -> Option<&SourceHighlightColor> {
        self.color.as_ref()
    }

    pub fn is_transparent(&self) -> bool {
        self.transparent
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceHighlightColor(String);

impl SourceHighlightColor {
    pub fn parse(value: impl Into<String>) -> Option<Self> {
        let value = value.into();
        crate::syntax::canonical_visual_color_literal(&value).map(Self)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_transparent(&self) -> bool {
        self.0.eq_ignore_ascii_case("transparent")
    }
}

impl AsRef<str> for SourceHighlightColor {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceHighlightKind {
    Keyword,
    Literal,
    Binding,
    Effect,
    Emission,
    Object,
    Input,
    State,
    Group,
    Mark,
    Variant,
    Condition,
    Scene,
    Theme,
    Asset,
    Color,
    Number,
    String,
    Comment,
    Operator,
    Arrow,
    Brace0,
    Brace1,
    Brace2,
    Brace3,
    Brace4,
    Brace5,
    InvalidSyntax,
    InvalidBrace,
    LevelCell,
    InvalidLevelCell,
    SpritePixel,
}

impl SourceHighlightKind {
    pub fn as_str(self) -> &'static str {
        match self {
            SourceHighlightKind::Keyword => "keyword",
            SourceHighlightKind::Literal => "literal",
            SourceHighlightKind::Binding => "binding",
            SourceHighlightKind::Effect => "effect",
            SourceHighlightKind::Emission => "emission",
            SourceHighlightKind::Object => "object",
            SourceHighlightKind::Input => "input",
            SourceHighlightKind::State => "state",
            SourceHighlightKind::Group => "group",
            SourceHighlightKind::Mark => "mark",
            SourceHighlightKind::Variant => "variant",
            SourceHighlightKind::Condition => "condition",
            SourceHighlightKind::Scene => "scene",
            SourceHighlightKind::Theme => "theme",
            SourceHighlightKind::Asset => "asset",
            SourceHighlightKind::Color => "color",
            SourceHighlightKind::Number => "number",
            SourceHighlightKind::String => "string",
            SourceHighlightKind::Comment => "comment",
            SourceHighlightKind::Operator => "operator",
            SourceHighlightKind::Arrow => "arrow",
            SourceHighlightKind::Brace0 => "brace-depth-0",
            SourceHighlightKind::Brace1 => "brace-depth-1",
            SourceHighlightKind::Brace2 => "brace-depth-2",
            SourceHighlightKind::Brace3 => "brace-depth-3",
            SourceHighlightKind::Brace4 => "brace-depth-4",
            SourceHighlightKind::Brace5 => "brace-depth-5",
            SourceHighlightKind::InvalidSyntax => "syntax-invalid",
            SourceHighlightKind::InvalidBrace => "brace-invalid",
            SourceHighlightKind::LevelCell => "level-cell",
            SourceHighlightKind::InvalidLevelCell => "level-cell-invalid",
            SourceHighlightKind::SpritePixel => "sprite-pixel",
        }
    }
}

/// Builds a profile-aware canonical analysis and returns its parser-owned highlight product.
pub fn highlight_source(source: &str, profile: crate::PuzzleSourceProfile) -> HighlightedSource {
    crate::analyze_source_for_profile(source, profile).highlight()
}

/// Builds one canonical surface document and projects highlighting and outline from it.
pub fn highlight_source_with_outline(
    source: &str,
    profile: crate::PuzzleSourceProfile,
) -> HighlightedSourceWithOutline {
    crate::analyze_source_for_profile(source, profile).highlighted()
}

pub(crate) fn highlight_source_with_document(document: &SurfaceDocument) -> HighlightedSource {
    HighlightedSource {
        spans: crate::source::lexical_product::build_surface_highlight_spans(document),
        parsed: true,
    }
}

pub(crate) fn highlight_source_range_with_document(
    document: &SurfaceDocument,
    range_start: usize,
    range_end: usize,
) -> HighlightedSource {
    if range_start >= range_end {
        return HighlightedSource {
            spans: Vec::new(),
            parsed: true,
        };
    }
    HighlightedSource {
        spans: crate::source::lexical_product::build_surface_highlight_spans_in_range(
            document,
            range_start,
            range_end,
        ),
        parsed: true,
    }
}

#[cfg(test)]
mod tests {
    use crate::{SourceHighlightColor, SurfaceDocument};

    use super::highlight_source_range_with_document;

    #[test]
    fn empty_highlight_range_returns_no_spans() {
        let document = SurfaceDocument::default();

        assert!(
            highlight_source_range_with_document(&document, 3, 3)
                .spans
                .is_empty()
        );
    }

    #[test]
    fn every_named_highlight_color_resolves_to_a_browser_color() {
        for (name, _) in crate::syntax::VISUAL_NAMED_COLORS {
            let color = SourceHighlightColor::parse(*name).expect("registered color must resolve");
            let value = color.as_str();
            assert!(
                value == "transparent"
                    || value == "currentcolor"
                    || value.strip_prefix('#').is_some_and(|hex| {
                        matches!(hex.len(), 3 | 4 | 6 | 8)
                            && hex.chars().all(|ch| ch.is_ascii_hexdigit())
                    }),
                "registered highlight color {name:?} resolved to invalid CSS color {value:?}"
            );
        }
    }

    #[test]
    fn highlight_projection_contains_no_source_recognizer() {
        let source = include_str!("highlight.rs");
        let production = source
            .split("#[cfg(test)]")
            .next()
            .expect("highlight production source");
        assert_eq!(
            production
                .matches("profile: crate::PuzzleSourceProfile")
                .count(),
            2,
            "public highlight entrypoints must require an explicit source profile"
        );
        for forbidden in [
            ".chars(",
            ".char_indices(",
            "split_whitespace",
            "split_inclusive",
            "starts_with",
            "ends_with",
            "scan_",
            "consume_",
            "scan_brace",
            "brace_ranges",
            "line_code_braces",
            "inline_selector",
            "split_highlight_word",
            "hex_color_end",
            "semantic_kind_at",
        ] {
            assert!(
                !production.contains(forbidden),
                "highlight projection must consume parser-owned spans, not recognize source via {forbidden}"
            );
        }
    }
}
