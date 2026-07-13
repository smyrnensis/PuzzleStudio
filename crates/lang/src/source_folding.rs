use crate::surface::SurfaceDocument;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SourceFoldRange {
    pub from: usize,
    pub to: usize,
}

pub(crate) fn source_fold_ranges_from_document(
    source: &str,
    document: &SurfaceDocument,
) -> Vec<SourceFoldRange> {
    document
        .structural_blocks
        .iter()
        .filter_map(|block| {
            let open = block.open_brace?;
            let close = block.close_brace?;
            let from = open + 1;
            if from >= close || !source[from..close].contains('\n') {
                return None;
            }
            Some(SourceFoldRange { from, to: close })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::source_fold_ranges_from_document;

    fn ranges(source: &str) -> Vec<String> {
        let document = crate::parse_surface_structure_document(source);
        source_fold_ranges_from_document(source, &document)
            .into_iter()
            .map(|range| source[range.from..range.to].to_string())
            .collect()
    }

    #[test]
    fn folding_uses_parser_owned_nested_structural_blocks() {
        let source = "puzzle demo {\n  rules {\n    move\n  }\n}\n";
        assert_eq!(
            ranges(source),
            vec!["\n  rules {\n    move\n  }\n", "\n    move\n  "]
        );
    }

    #[test]
    fn folding_excludes_single_line_and_unclosed_blocks() {
        assert!(ranges("puzzle demo { rules { } }\n").is_empty());
        assert!(ranges("puzzle demo {\n  rules {\n").is_empty());
    }

    #[test]
    fn folding_does_not_treat_quoted_or_commented_braces_as_structure() {
        let source = "puzzle demo {\n  title = \"{ not a block }\"\n  // { neither }\n}\n";
        assert_eq!(
            ranges(source),
            vec!["\n  title = \"{ not a block }\"\n  // { neither }\n"]
        );
    }
}
