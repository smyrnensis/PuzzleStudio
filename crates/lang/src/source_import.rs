#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceImportDeclaration {
    pub range: SourceImportRange,
    pub alias_range: SourceImportRange,
    pub path_range: SourceImportRange,
    pub alias: String,
    pub raw_path: String,
}

impl SourceImportDeclaration {
    pub(crate) fn shift_offsets(&mut self, threshold: usize, delta: i64) {
        self.range.shift_offsets(threshold, delta);
        self.alias_range.shift_offsets(threshold, delta);
        self.path_range.shift_offsets(threshold, delta);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceImportReference {
    pub range: SourceImportRange,
    pub path_range: SourceImportRange,
    pub raw_path: String,
    pub alias: String,
    pub resolved_path: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceImportRange {
    pub start: usize,
    pub end: usize,
}

impl SourceImportRange {
    fn shift_offsets(&mut self, threshold: usize, delta: i64) {
        if self.start >= threshold {
            self.start = usize::try_from(self.start as i64 + delta)
                .expect("incremental import span start underflow");
            self.end = usize::try_from(self.end as i64 + delta)
                .expect("incremental import span end underflow");
        }
    }
}

pub(crate) fn source_import_reference_at(
    declarations: &[SourceImportDeclaration],
    document_path: &str,
    cursor: usize,
) -> Option<SourceImportReference> {
    let declaration = declarations.iter().find(|declaration| {
        declaration.path_range.start <= cursor && cursor <= declaration.path_range.end
    })?;
    let resolved = crate::WorkspacePath::parse(document_path)
        .ok()?
        .resolve_import(&declaration.raw_path)
        .ok()?;
    Some(SourceImportReference {
        range: declaration.range,
        path_range: declaration.path_range,
        raw_path: declaration.raw_path.clone(),
        alias: declaration.alias.clone(),
        resolved_path: resolved.as_str().to_string(),
    })
}
