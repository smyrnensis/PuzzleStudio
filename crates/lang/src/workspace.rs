use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use puzzle_assets::VisualImageAssetManifestEntry;

use crate::{AssetKind, DiagnosticReport, LoadedDocumentModel, VisualKind};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceSourceDocument {
    pub path: String,
    pub source: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct WorkspacePath(String);

impl WorkspacePath {
    pub fn parse(path: &str) -> Result<Self, String> {
        normalize_workspace_path(None, path).map(Self)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn resolve_import(&self, requested: &str) -> Result<Self, String> {
        self.resolve_relative(requested)
    }

    pub(crate) fn resolve_relative(&self, requested: &str) -> Result<Self, String> {
        let base = self.0.rsplit_once('/').map(|(base, _)| base);
        normalize_workspace_path(base, requested).map(Self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceImportStatus {
    Resolved,
    InvalidPath,
    MissingDocument,
    Cycle,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceImportEdge {
    pub alias: String,
    pub raw_path: String,
    pub start: usize,
    pub end: usize,
    pub path_start: usize,
    pub path_end: usize,
    pub resolved_path: Option<String>,
    pub status: WorkspaceImportStatus,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceIndexDocument {
    pub path: String,
    pub imports: Vec<WorkspaceImportEdge>,
    pub direct_importers: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceGraphDiagnostic {
    pub code: String,
    pub message: String,
    pub path: String,
    pub start: Option<usize>,
    pub end: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceIndex {
    pub version: u32,
    pub revision: u64,
    pub documents: Vec<WorkspaceIndexDocument>,
    pub diagnostics: Vec<WorkspaceGraphDiagnostic>,
}

struct WorkspaceAnalyzedDocument {
    path: WorkspacePath,
    analysis: crate::SourceAnalysis,
}

pub struct WorkspaceAnalysis {
    revision: u64,
    documents: BTreeMap<WorkspacePath, WorkspaceAnalyzedDocument>,
    index: WorkspaceIndex,
}

pub(crate) struct WorkspaceModulePlan<'a> {
    pub(crate) path: &'a str,
    pub(crate) namespace: String,
    pub(crate) analysis: &'a crate::SourceAnalysis,
    pub(crate) imports: BTreeMap<String, String>,
}

impl WorkspaceAnalysis {
    pub fn new(documents: &[WorkspaceSourceDocument]) -> Result<Self, DiagnosticReport> {
        Self::from_documents(documents, 1)
    }

    pub fn replace_documents(
        &mut self,
        documents: &[WorkspaceSourceDocument],
    ) -> Result<(), DiagnosticReport> {
        let revision = self
            .revision
            .checked_add(1)
            .ok_or_else(|| DiagnosticReport::error("workspace revision counter exhausted"))?;
        *self = Self::from_documents(documents, revision)?;
        Ok(())
    }

    fn from_documents(
        documents: &[WorkspaceSourceDocument],
        revision: u64,
    ) -> Result<Self, DiagnosticReport> {
        let mut analyzed = BTreeMap::new();
        for document in documents {
            let path = WorkspacePath::parse(&document.path).map_err(|message| {
                DiagnosticReport::error(format!(
                    "invalid workspace document path `{}`: {message}",
                    document.path
                ))
            })?;
            if analyzed.contains_key(&path) {
                return Err(DiagnosticReport::error(format!(
                    "duplicate workspace document path: {}",
                    path.as_str()
                )));
            }
            analyzed.insert(
                path.clone(),
                WorkspaceAnalyzedDocument {
                    path,
                    analysis: crate::analyze_source(&document.source),
                },
            );
        }
        let index = build_workspace_index(revision, &analyzed);
        Ok(Self {
            revision,
            documents: analyzed,
            index,
        })
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn index(&self) -> &WorkspaceIndex {
        &self.index
    }

    pub fn index_json(&self) -> Result<String, String> {
        serde_json::to_string(&self.index)
            .map_err(|error| format!("workspace index could not be encoded: {error}"))
    }

    pub fn source_analysis(&self, path: &str) -> Result<&crate::SourceAnalysis, String> {
        let path = WorkspacePath::parse(path)?;
        self.documents
            .get(&path)
            .map(|document| &document.analysis)
            .ok_or_else(|| format!("workspace document not found: {}", path.as_str()))
    }

    pub(crate) fn module_plan(
        &self,
        entry_path: &str,
    ) -> Result<Vec<WorkspaceModulePlan<'_>>, DiagnosticReport> {
        let entry = WorkspacePath::parse(entry_path).map_err(DiagnosticReport::error)?;
        if !self.documents.contains_key(&entry) {
            return Err(DiagnosticReport::error(format!(
                "workspace puzzle entry not found: {}",
                entry.as_str()
            )));
        }
        let imports_by_path = self
            .index
            .documents
            .iter()
            .map(|document| {
                (
                    WorkspacePath::parse(&document.path)
                        .expect("workspace index contains normalized paths"),
                    document.imports.clone(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let mut reachable = BTreeSet::new();
        collect_reachable(&entry, &imports_by_path, &mut reachable);
        let blocking = self
            .index
            .diagnostics
            .iter()
            .filter(|diagnostic| {
                WorkspacePath::parse(&diagnostic.path)
                    .ok()
                    .is_some_and(|path| reachable.contains(&path))
            })
            .map(|diagnostic| self.graph_diagnostic(diagnostic))
            .collect::<Vec<_>>();
        if !blocking.is_empty() {
            return Err(DiagnosticReport::from_diagnostics(blocking));
        }

        let mut namespaces = BTreeMap::<WorkspacePath, String>::new();
        namespaces.insert(entry.clone(), String::new());
        let mut queue = vec![entry.clone()];
        while let Some(path) = queue.pop() {
            let parent_namespace = namespaces.get(&path).cloned().unwrap_or_default();
            let mut edges = imports_by_path
                .get(&path)
                .into_iter()
                .flat_map(|edges| edges.iter())
                .filter(|edge| edge.status == WorkspaceImportStatus::Resolved)
                .collect::<Vec<_>>();
            edges.sort_by(|left, right| left.alias.cmp(&right.alias));
            for edge in edges {
                let target = WorkspacePath::parse(
                    edge.resolved_path
                        .as_deref()
                        .expect("resolved import has a target"),
                )
                .expect("resolved import target is normalized");
                let candidate = if parent_namespace.is_empty() {
                    edge.alias.clone()
                } else {
                    format!("{parent_namespace}:{}", edge.alias)
                };
                let replace = namespaces
                    .get(&target)
                    .is_none_or(|existing| candidate < *existing);
                if replace {
                    namespaces.insert(target.clone(), candidate);
                    queue.push(target);
                }
            }
        }

        reachable
            .into_iter()
            .map(|path| {
                let document = self
                    .documents
                    .get(&path)
                    .expect("reachable workspace path exists");
                let imports = imports_by_path
                    .get(&path)
                    .into_iter()
                    .flat_map(|edges| edges.iter())
                    .filter(|edge| edge.status == WorkspaceImportStatus::Resolved)
                    .map(|edge| {
                        (
                            edge.alias.clone(),
                            edge.resolved_path
                                .clone()
                                .expect("resolved import has a target"),
                        )
                    })
                    .collect();
                Ok(WorkspaceModulePlan {
                    path: document.path.as_str(),
                    namespace: namespaces
                        .get(&path)
                        .cloned()
                        .expect("reachable module has a namespace"),
                    analysis: &document.analysis,
                    imports,
                })
            })
            .collect()
    }

    fn graph_diagnostic(&self, diagnostic: &WorkspaceGraphDiagnostic) -> crate::Diagnostic {
        let base = crate::Diagnostic::error(&diagnostic.message).with_file(&diagnostic.path);
        let Some(offset) = diagnostic.start else {
            return base;
        };
        let Ok(path) = WorkspacePath::parse(&diagnostic.path) else {
            return base;
        };
        let Some(source) = self
            .documents
            .get(&path)
            .map(|document| document.analysis.source())
        else {
            return base;
        };
        let Some(prefix) = source.get(..offset) else {
            return base;
        };
        let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
        let source_line = source.lines().nth(line - 1).unwrap_or("");
        crate::Diagnostic::error(&diagnostic.message)
            .with_source_line_number(source_line, line)
            .with_file(&diagnostic.path)
    }
}

fn normalize_workspace_path(base: Option<&str>, requested: &str) -> Result<String, String> {
    if requested.is_empty() {
        return Err("path must not be empty".to_string());
    }
    if requested.starts_with('/')
        || requested.starts_with('\\')
        || requested.as_bytes().get(1) == Some(&b':')
    {
        return Err("path must be workspace-relative".to_string());
    }
    if requested.contains('\\') {
        return Err("path must use `/` separators".to_string());
    }
    let mut parts = base
        .into_iter()
        .flat_map(|base| base.split('/'))
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    for part in requested.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return Err("path escapes the workspace root".to_string());
                }
            }
            _ => parts.push(part.to_string()),
        }
    }
    if parts.is_empty() {
        return Err("path must name a document".to_string());
    }
    Ok(parts.join("/"))
}

fn build_workspace_index(
    revision: u64,
    documents: &BTreeMap<WorkspacePath, WorkspaceAnalyzedDocument>,
) -> WorkspaceIndex {
    let mut diagnostics = Vec::new();
    let mut imports = BTreeMap::<WorkspacePath, Vec<WorkspaceImportEdge>>::new();
    let mut adjacency = BTreeMap::<WorkspacePath, Vec<WorkspacePath>>::new();
    for document in documents.values() {
        let mut aliases = BTreeSet::new();
        let mut edges = Vec::new();
        for import in document.analysis.imports() {
            if !aliases.insert(import.alias.clone()) {
                diagnostics.push(WorkspaceGraphDiagnostic {
                    code: "workspace.duplicate-import-alias".to_string(),
                    message: format!("duplicate import alias `{}`", import.alias),
                    path: document.path.as_str().to_string(),
                    start: Some(import.alias_range.start),
                    end: Some(import.alias_range.end),
                });
                continue;
            }
            let resolved = document.path.resolve_import(&import.raw_path);
            let (resolved_path, status) = match resolved {
                Ok(target) if documents.contains_key(&target) => {
                    adjacency
                        .entry(document.path.clone())
                        .or_default()
                        .push(target.clone());
                    (Some(target.0), WorkspaceImportStatus::Resolved)
                }
                Ok(target) => {
                    diagnostics.push(WorkspaceGraphDiagnostic {
                        code: "workspace.import-not-found".to_string(),
                        message: format!("imported document not found: {}", target.as_str()),
                        path: document.path.as_str().to_string(),
                        start: Some(import.path_range.start),
                        end: Some(import.path_range.end),
                    });
                    (Some(target.0), WorkspaceImportStatus::MissingDocument)
                }
                Err(message) => {
                    diagnostics.push(WorkspaceGraphDiagnostic {
                        code: "workspace.invalid-import-path".to_string(),
                        message,
                        path: document.path.as_str().to_string(),
                        start: Some(import.path_range.start),
                        end: Some(import.path_range.end),
                    });
                    (None, WorkspaceImportStatus::InvalidPath)
                }
            };
            edges.push(WorkspaceImportEdge {
                alias: import.alias.clone(),
                raw_path: import.raw_path.clone(),
                start: import.range.start,
                end: import.range.end,
                path_start: import.path_range.start,
                path_end: import.path_range.end,
                resolved_path,
                status,
            });
        }
        imports.insert(document.path.clone(), edges);
    }

    let graph = adjacency.clone();
    for (source, edges) in &mut imports {
        for edge in edges {
            let Some(target) = edge
                .resolved_path
                .as_deref()
                .and_then(|path| WorkspacePath::parse(path).ok())
            else {
                continue;
            };
            if reaches(&graph, &target, source, &mut BTreeSet::new()) {
                edge.status = WorkspaceImportStatus::Cycle;
                diagnostics.push(WorkspaceGraphDiagnostic {
                    code: "workspace.import-cycle".to_string(),
                    message: format!(
                        "cyclic workspace import: {} -> {}",
                        source.as_str(),
                        target.as_str()
                    ),
                    path: source.as_str().to_string(),
                    start: Some(edge.path_start),
                    end: Some(edge.path_end),
                });
            }
        }
    }

    let mut reverse = BTreeMap::<WorkspacePath, BTreeSet<WorkspacePath>>::new();
    for (source, edges) in &imports {
        for edge in edges
            .iter()
            .filter(|edge| edge.status == WorkspaceImportStatus::Resolved)
        {
            if let Some(target) = edge
                .resolved_path
                .as_deref()
                .and_then(|path| WorkspacePath::parse(path).ok())
            {
                reverse.entry(target).or_default().insert(source.clone());
            }
        }
    }
    let index_documents = documents
        .values()
        .map(|document| WorkspaceIndexDocument {
            path: document.path.as_str().to_string(),
            imports: imports.get(&document.path).cloned().unwrap_or_default(),
            direct_importers: reverse
                .get(&document.path)
                .into_iter()
                .flat_map(|paths| paths.iter())
                .map(|path| path.as_str().to_string())
                .collect(),
        })
        .collect();
    WorkspaceIndex {
        version: 1,
        revision,
        documents: index_documents,
        diagnostics,
    }
}

fn reaches(
    graph: &BTreeMap<WorkspacePath, Vec<WorkspacePath>>,
    current: &WorkspacePath,
    goal: &WorkspacePath,
    visited: &mut BTreeSet<WorkspacePath>,
) -> bool {
    if current == goal {
        return true;
    }
    if !visited.insert(current.clone()) {
        return false;
    }
    graph
        .get(current)
        .into_iter()
        .flat_map(|targets| targets.iter())
        .any(|target| reaches(graph, target, goal, visited))
}

fn collect_reachable(
    current: &WorkspacePath,
    imports: &BTreeMap<WorkspacePath, Vec<WorkspaceImportEdge>>,
    reachable: &mut BTreeSet<WorkspacePath>,
) {
    if !reachable.insert(current.clone()) {
        return;
    }
    for edge in imports
        .get(current)
        .into_iter()
        .flat_map(|edges| edges.iter())
        .filter(|edge| edge.status == WorkspaceImportStatus::Resolved)
    {
        if let Some(target) = edge
            .resolved_path
            .as_deref()
            .and_then(|path| WorkspacePath::parse(path).ok())
        {
            collect_reachable(&target, imports, reachable);
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspacePresentationManifest {
    pub theme_name: Option<String>,
    pub css_paths: Vec<String>,
    pub script_paths: Vec<String>,
    pub file_paths: Vec<String>,
    pub visual_image_assets: Vec<VisualImageAssetManifestEntry>,
}

pub fn workspace_presentation_manifest(
    entry_path: &str,
    documents: &[WorkspaceSourceDocument],
) -> Result<WorkspacePresentationManifest, DiagnosticReport> {
    let document = WorkspaceAnalysis::new(documents)?.compile_game(entry_path)?;
    loaded_document_presentation_manifest(&document)
}

pub fn loaded_document_presentation_manifest(
    document: &crate::LoadedDocument,
) -> Result<WorkspacePresentationManifest, DiagnosticReport> {
    Ok(workspace_presentation_manifest_from_document(document))
}

impl WorkspaceAnalysis {
    pub fn presentation_manifest(
        &self,
        entry_path: &str,
    ) -> Result<WorkspacePresentationManifest, DiagnosticReport> {
        let document = self.compile_game(entry_path)?;
        Ok(workspace_presentation_manifest_from_document(&document))
    }
}

pub fn workspace_presentation_manifest_from_document(
    document: &crate::LoadedDocument,
) -> WorkspacePresentationManifest {
    let mut css_paths = Vec::new();
    let mut script_paths = Vec::new();
    let mut file_paths = Vec::new();
    for asset in &document.assets.entries {
        match asset.kind {
            AssetKind::Css => css_paths.push(asset.path.clone()),
            AssetKind::Script => script_paths.push(asset.path.clone()),
            AssetKind::File => file_paths.push(asset.path.clone()),
        }
    }

    let mut visual_image_assets = Vec::new();
    for model in &document.models {
        let visuals = match model {
            LoadedDocumentModel::Puzzle2d { game, .. } => &game.visuals,
            LoadedDocumentModel::Puzzle3d { game, .. } => &game.visuals,
        };
        for visual in &visuals.entries {
            let VisualKind::Image { asset } = &visual.kind else {
                continue;
            };
            if visual_image_assets
                .iter()
                .all(|existing: &VisualImageAssetManifestEntry| existing.id != asset.id)
            {
                visual_image_assets.push(asset.clone());
            }
        }
    }

    WorkspacePresentationManifest {
        theme_name: document.theme.name.clone(),
        css_paths,
        script_paths,
        file_paths,
        visual_image_assets,
    }
}

#[cfg(test)]
mod tests {
    use super::{WorkspaceAnalysis, WorkspaceImportStatus, WorkspacePath, WorkspaceSourceDocument};

    fn document(path: &str, source: &str) -> WorkspaceSourceDocument {
        WorkspaceSourceDocument {
            path: path.to_string(),
            source: source.to_string(),
        }
    }

    fn minimal_model(name: &str) -> String {
        format!(
            "puzzle {name} {{\nlayers {{\nactor = Player\n}}\nrules {{\n}}\nlevels {{\nlegend {{\nP = Player\n}}\nlevel \"start\" {{\nP\n}}\n}}\n}}\n"
        )
    }

    #[test]
    fn workspace_graph_uses_parser_owned_alias_imports() {
        let workspace = WorkspaceAnalysis::new(&[
            document(
                "games/demo/game.puzzle",
                "import shared = \"parts/shared.puzzle\"\npuzzle main {}\n",
            ),
            document("games/demo/parts/shared.puzzle", "scene title {}\n"),
        ])
        .expect("workspace analysis");

        let entry = workspace
            .index()
            .documents
            .iter()
            .find(|document| document.path == "games/demo/game.puzzle")
            .expect("entry document");
        assert_eq!(entry.imports.len(), 1);
        assert_eq!(entry.imports[0].alias, "shared");
        assert_eq!(
            entry.imports[0].resolved_path.as_deref(),
            Some("games/demo/parts/shared.puzzle")
        );
        assert_eq!(entry.imports[0].status, WorkspaceImportStatus::Resolved);
        let index_json = workspace.index_json().expect("workspace index JSON");
        assert!(!index_json.contains("entryCandidate"));
        assert!(!index_json.contains("containingEntries"));
        assert!(!index_json.contains("preferredEntry"));
    }

    #[test]
    fn replacing_workspace_documents_advances_one_authoritative_revision() {
        let mut workspace =
            WorkspaceAnalysis::new(&[document("game.puzzle", "")]).expect("workspace analysis");
        workspace
            .replace_documents(&[document("game.puzzle", "const title = \"Next\"\n")])
            .expect("workspace replacement");

        assert_eq!(workspace.revision(), 2);
        assert_eq!(workspace.index().revision, 2);
        assert_eq!(
            workspace
                .source_analysis("game.puzzle")
                .expect("source analysis")
                .source(),
            "const title = \"Next\"\n"
        );
    }

    #[test]
    fn workspace_paths_reject_root_escape_instead_of_clamping_it() {
        let workspace = WorkspaceAnalysis::new(&[
            document(
                "game.puzzle",
                "import outside = \"../outside.puzzle\"\npuzzle main {}\n",
            ),
            document("outside.puzzle", "scene outside {}\n"),
        ])
        .expect("workspace analysis");

        let entry = &workspace.index().documents[0];
        assert_eq!(entry.imports[0].status, WorkspaceImportStatus::InvalidPath);
        assert!(
            workspace
                .index()
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "workspace.invalid-import-path")
        );
        let report = workspace
            .compile_game("game.puzzle")
            .expect_err("invalid import path must block compilation");
        let span = report.diagnostics()[0]
            .primary_span
            .as_ref()
            .expect("graph diagnostic span");
        assert_eq!(span.file.as_deref(), Some("game.puzzle"));
        assert_eq!(span.line, Some(1));
    }

    #[test]
    fn workspace_graph_marks_every_edge_in_a_cycle() {
        let workspace = WorkspaceAnalysis::new(&[
            document(
                "a.puzzle",
                "import b = \"b.puzzle\"\npuzzle a {\nrules {\n}\n}\n",
            ),
            document("b.puzzle", "import a = \"a.puzzle\"\nscene b {\n}\n"),
        ])
        .expect("workspace analysis");

        assert!(
            workspace.index().documents.iter().all(|document| {
                document.imports.len() == 1
                    && document.imports[0].status == WorkspaceImportStatus::Cycle
            }),
            "{:?}",
            workspace.index().documents
        );
    }

    #[test]
    fn workspace_path_is_platform_independent_and_normalized() {
        assert_eq!(
            WorkspacePath::parse("games/./demo/../game.puzzle")
                .expect("normalized path")
                .as_str(),
            "games/game.puzzle"
        );
        assert!(WorkspacePath::parse("../../game.puzzle").is_err());
        assert!(WorkspacePath::parse("C:/game.puzzle").is_err());
        assert!(WorkspacePath::parse("games\\game.puzzle").is_err());
    }

    #[test]
    fn workspace_compiler_links_typed_modules_through_explicit_aliases() {
        let workspace = WorkspaceAnalysis::new(&[
            document(
                "game.puzzle",
                "import board = \"models/board.puzzle\"\nscene title {\nlayout {\npuzzle board = board:main\n}\n}\n",
            ),
            document("models/board.puzzle", &minimal_model("main")),
        ])
        .expect("workspace analysis");

        let loaded = workspace
            .compile_game("game.puzzle")
            .expect("workspace compile");
        assert!(matches!(
            loaded.models.as_slice(),
            [crate::LoadedDocumentModel::Puzzle2d { name, .. }] if name == "board:main"
        ));
        assert_eq!(loaded.scenes[0].name, "title");
    }

    #[test]
    fn imported_resource_references_are_canonical_workspace_paths() {
        let model = minimal_model("main").replace(
            "rules {",
            "visuals {\nPlayer {\nimage = \"images/player.png\"\n}\n}\nrules {",
        );
        let workspace = WorkspaceAnalysis::new(&[
            document(
                "games/game.puzzle",
                "import board = \"../models/board.puzzle\"\n",
            ),
            document("models/board.puzzle", &model),
        ])
        .expect("workspace analysis");

        let manifest = workspace
            .presentation_manifest("games/game.puzzle")
            .expect("workspace manifest");

        assert_eq!(
            manifest
                .visual_image_assets
                .iter()
                .map(|asset| asset.path.as_str())
                .collect::<Vec<_>>(),
            ["models/images/player.png"]
        );
    }

    #[test]
    fn workspace_compiler_does_not_make_imported_names_implicitly_local() {
        let workspace = WorkspaceAnalysis::new(&[
            document(
                "game.puzzle",
                "import board = \"board.puzzle\"\nscene title {\nlayout {\npuzzle board = main\n}\n}\n",
            ),
            document("board.puzzle", &minimal_model("main")),
        ])
        .expect("workspace analysis");

        let error = workspace
            .compile_game("game.puzzle")
            .expect_err("unqualified imported model must fail");
        assert!(
            error
                .to_string()
                .contains("unknown puzzle model reference `main`"),
            "{error}"
        );
    }

    #[test]
    fn workspace_compiler_rejects_transitive_namespace_access() {
        let workspace = WorkspaceAnalysis::new(&[
            document(
                "game.puzzle",
                "import middle = \"middle.puzzle\"\nscene title {\nlayout {\npuzzle board = leaf:main\n}\n}\n",
            ),
            document("middle.puzzle", "import leaf = \"leaf.puzzle\"\n"),
            document("leaf.puzzle", &minimal_model("main")),
        ])
        .expect("workspace analysis");

        let error = workspace
            .compile_game("game.puzzle")
            .expect_err("transitive import must not be re-exported");
        assert!(
            error.to_string().contains("unknown import alias `leaf`"),
            "{error}"
        );
    }

    #[test]
    fn canonical_import_requires_an_alias_at_the_document_root() {
        let unaliased = WorkspaceAnalysis::new(&[
            document("game.puzzle", "import \"board.puzzle\"\n"),
            document("board.puzzle", &minimal_model("main")),
        ])
        .expect("workspace analysis")
        .compile_game("game.puzzle")
        .expect_err("unaliased import must fail");
        assert!(
            unaliased
                .to_string()
                .contains("import must be: import <alias>"),
            "{unaliased}"
        );

        let nested = WorkspaceAnalysis::new(&[
            document(
                "game.puzzle",
                "puzzle shell {\nimport board = \"board.puzzle\"\n}\n",
            ),
            document("board.puzzle", &minimal_model("main")),
        ])
        .expect("workspace analysis")
        .compile_game("game.puzzle")
        .expect_err("nested import must fail");
        assert!(nested.to_string().contains("document-scoped"), "{nested}");
    }
}
