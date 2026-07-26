use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
}

impl DiagnosticSeverity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticSpan {
    pub file: Option<String>,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub source_line: Option<String>,
}

impl DiagnosticSpan {
    pub fn source_line(source_line: impl Into<String>) -> Self {
        Self {
            file: None,
            line: None,
            column: None,
            source_line: Some(source_line.into()),
        }
    }

    pub fn source_line_number(source_line: impl Into<String>, line: usize) -> Self {
        Self {
            file: None,
            line: Some(line),
            column: None,
            source_line: Some(source_line.into()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    pub code: &'static str,
    pub message: String,
    pub primary_span: Option<DiagnosticSpan>,
    pub notes: Vec<String>,
    pub help: Option<String>,
}

impl Diagnostic {
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            severity: DiagnosticSeverity::Error,
            code: "compile-error",
            message: message.into(),
            primary_span: None,
            notes: Vec::new(),
            help: None,
        }
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: DiagnosticSeverity::Warning,
            code: "warning",
            message: message.into(),
            primary_span: None,
            notes: Vec::new(),
            help: None,
        }
    }

    pub fn with_source_line(mut self, source_line: impl Into<String>) -> Self {
        self.primary_span = Some(DiagnosticSpan::source_line(source_line));
        self
    }

    pub fn with_source_line_number(mut self, source_line: impl Into<String>, line: usize) -> Self {
        self.primary_span = Some(DiagnosticSpan::source_line_number(source_line, line));
        self
    }

    pub fn with_file(mut self, file: impl Into<String>) -> Self {
        let span = self.primary_span.get_or_insert_with(|| DiagnosticSpan {
            file: None,
            line: None,
            column: None,
            source_line: None,
        });
        span.file = Some(file.into());
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticReport {
    diagnostics: Vec<Diagnostic>,
}

impl DiagnosticReport {
    pub fn from_diagnostic(diagnostic: Diagnostic) -> Self {
        Self {
            diagnostics: vec![diagnostic],
        }
    }

    pub fn from_diagnostics(diagnostics: Vec<Diagnostic>) -> Self {
        debug_assert!(
            !diagnostics.is_empty(),
            "diagnostic reports must contain at least one diagnostic"
        );
        Self { diagnostics }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self::from_diagnostic(Diagnostic::error(message))
    }

    pub fn error_at_line(message: impl Into<String>, source_line: impl Into<String>) -> Self {
        Self::from_diagnostic(Diagnostic::error(message).with_source_line(source_line))
    }

    pub fn error_at_source_line_number(
        message: impl Into<String>,
        source_line: impl Into<String>,
        line: usize,
    ) -> Self {
        Self::from_diagnostic(Diagnostic::error(message).with_source_line_number(source_line, line))
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub fn into_diagnostics(self) -> Vec<Diagnostic> {
        self.diagnostics
    }

    pub fn with_file(self, file: impl Into<String>) -> Self {
        let file = file.into();
        Self::from_diagnostics(
            self.diagnostics
                .into_iter()
                .map(|diagnostic| diagnostic.with_file(&file))
                .collect(),
        )
    }
}

impl From<puzzle_core::StateError> for DiagnosticReport {
    fn from(value: puzzle_core::StateError) -> Self {
        Self::error(format!("{value:?}"))
    }
}

impl From<puzzle_scene::SceneBlockParseError> for DiagnosticReport {
    fn from(value: puzzle_scene::SceneBlockParseError) -> Self {
        Self::error(value.to_string())
    }
}

impl fmt::Display for DiagnosticReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, diagnostic) in self.diagnostics.iter().enumerate() {
            if index > 0 {
                writeln!(f)?;
            }
            write!(f, "{}", diagnostic.message)?;
            if let Some(source_line) = diagnostic
                .primary_span
                .as_ref()
                .and_then(|span| span.source_line.as_deref())
            {
                write!(f, ": {source_line}")?;
            }
        }
        Ok(())
    }
}

impl std::error::Error for DiagnosticReport {}
