use std::fs;
use std::path::{Path, PathBuf};

use puzzle_lang::{DiagnosticReport, LoadedDocument, WorkspaceAnalysis, WorkspaceSourceDocument};

pub struct FileWorkspace {
    root: PathBuf,
    entry_path: String,
    documents: Vec<WorkspaceSourceDocument>,
}

impl FileWorkspace {
    pub fn load(entry: impl AsRef<Path>, root: impl AsRef<Path>) -> Result<Self, String> {
        Self::load_with_entry_source(entry, root, None)
    }

    pub fn load_with_entry_source(
        entry: impl AsRef<Path>,
        root: impl AsRef<Path>,
        entry_source: Option<&str>,
    ) -> Result<Self, String> {
        let root = canonical_directory(root.as_ref())?;
        let entry = canonical_file(entry.as_ref())?;
        if !entry.starts_with(&root) {
            return Err(format!(
                "workspace entry is outside root {}: {}",
                root.display(),
                entry.display()
            ));
        }
        let entry_path = workspace_path(&root, &entry)?;
        let mut paths = Vec::new();
        collect_puzzle_files(&root, &mut paths)?;
        paths.sort();
        let mut documents = Vec::with_capacity(paths.len());
        for path in paths {
            let source = if path == entry {
                match entry_source {
                    Some(source) => source.to_string(),
                    None => fs::read_to_string(&path)
                        .map_err(|error| format!("failed to read {}: {error}", path.display()))?,
                }
            } else {
                fs::read_to_string(&path)
                    .map_err(|error| format!("failed to read {}: {error}", path.display()))?
            };
            documents.push(WorkspaceSourceDocument {
                path: workspace_path(&root, &path)?,
                source,
            });
        }
        if !documents.iter().any(|document| document.path == entry_path) {
            return Err(format!(
                "workspace entry is not a .puzzle file: {}",
                entry.display()
            ));
        }
        Ok(Self {
            root,
            entry_path,
            documents,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn entry_path(&self) -> &str {
        &self.entry_path
    }

    pub fn documents(&self) -> &[WorkspaceSourceDocument] {
        &self.documents
    }

    pub fn entry_source(&self) -> &str {
        self.documents
            .iter()
            .find(|document| document.path == self.entry_path)
            .map(|document| document.source.as_str())
            .expect("loaded workspace contains its validated entry")
    }

    pub fn analysis(&self) -> Result<WorkspaceAnalysis, DiagnosticReport> {
        WorkspaceAnalysis::new(&self.documents)
    }

    pub fn compile(&self) -> Result<LoadedDocument, DiagnosticReport> {
        self.analysis()?.compile_game(&self.entry_path)
    }
}

fn canonical_directory(path: &Path) -> Result<PathBuf, String> {
    let path = fs::canonicalize(path).map_err(|error| {
        format!(
            "failed to resolve workspace root {}: {error}",
            path.display()
        )
    })?;
    if !path.is_dir() {
        return Err(format!(
            "workspace root is not a directory: {}",
            path.display()
        ));
    }
    Ok(path)
}

fn canonical_file(path: &Path) -> Result<PathBuf, String> {
    let path = fs::canonicalize(path).map_err(|error| {
        format!(
            "failed to resolve workspace entry {}: {error}",
            path.display()
        )
    })?;
    if !path.is_file() {
        return Err(format!("workspace entry is not a file: {}", path.display()));
    }
    Ok(path)
}

fn workspace_path(root: &Path, path: &Path) -> Result<String, String> {
    path.strip_prefix(root)
        .map_err(|_| format!("workspace path is outside root: {}", path.display()))?
        .to_str()
        .map(|path| path.replace('\\', "/"))
        .ok_or_else(|| format!("workspace path is not valid UTF-8: {}", path.display()))
}

fn collect_puzzle_files(directory: &Path, output: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("failed to read {}: {error}", directory.display()))?
    {
        let entry =
            entry.map_err(|error| format!("failed to read {}: {error}", directory.display()))?;
        let file_type = entry
            .file_type()
            .map_err(|error| format!("failed to inspect {}: {error}", entry.path().display()))?;
        let path = entry.path();
        if file_type.is_dir() {
            collect_puzzle_files(&path, output)?;
        } else if file_type.is_file() && puzzle_lang::is_puzzle_source_path(&path) {
            output.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::FileWorkspace;
    use puzzle_lang::LoadedDocumentModel;

    #[test]
    fn file_host_supplies_documents_without_interpreting_imports() {
        let root = std::env::temp_dir().join(format!(
            "puzzle-workspace-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        ));
        std::fs::create_dir_all(root.join("models")).expect("workspace directories");
        let entry = root.join("entry.puzzle");
        std::fs::write(&entry, "invalid disk snapshot").expect("entry file");
        std::fs::write(
            root.join("models/board.puzzle"),
            "puzzle main {\nlayers {\nactor = Player\n}\nrules {\n}\nlevels {\nlegend {\nP = Player\n}\nlevel \"start\" {\nP\n}\n}\n}\n",
        )
        .expect("model file");

        let workspace = FileWorkspace::load_with_entry_source(
            &entry,
            &root,
            Some("import board = \"models/board.puzzle\"\n"),
        )
        .expect("file workspace");
        let document = workspace.compile().expect("workspace compile");

        assert_eq!(workspace.entry_path(), "entry.puzzle");
        assert!(matches!(
            document.models.as_slice(),
            [LoadedDocumentModel::Puzzle2d { name, .. }] if name == "board:main"
        ));
        std::fs::remove_dir_all(&root).expect("remove test workspace");
    }
}
