struct EditorPreviewState {
    standalone_export: StandaloneRuntimeExport<puzzle_lang::LoadedDocument>,
    runtime: puzzle_game_runtime::RuntimeSession,
    source: String,
    puzzle_path: String,
}

impl EditorPreviewState {
    fn new(
        document: puzzle_lang::LoadedDocument,
        source: String,
        puzzle_path: String,
        visual_images: EncodedVisualImageBundle,
    ) -> Result<Self, String> {
        let mut runtime = puzzle_game_runtime::RuntimeSession::from_document(document.clone())?;
        runtime.set_progress_persistence_enabled(false);
        let progress_storage = standalone_progress_storage(&document);
        Ok(Self {
            standalone_export: StandaloneRuntimeExport::new(
                document,
                visual_images,
                progress_storage,
            ),
            runtime,
            source,
            puzzle_path,
        })
    }
}
