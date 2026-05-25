use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use puzzle_lang::{LoadedDocument, LoadedDocumentModel};

fn main() {
    let code = match run() {
        Ok(()) => 0,
        Err(CliError::CommandFailed) => 1,
        Err(error) => {
            eprintln!("error: {error}");
            1
        }
    };
    std::process::exit(code);
}

fn run() -> Result<(), CliError> {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        print_usage();
        return Err(CliError::Usage("missing command".to_string()));
    };
    let args = args.collect::<Vec<_>>();

    match command.as_str() {
        "check" => check_command(&args),
        "export-html" => export_html_command(&args),
        "import-puzzlescript" => import_puzzlescript_command(&args),
        "--help" | "-h" | "help" => {
            print_usage();
            Ok(())
        }
        other => Err(CliError::Usage(format!("unknown command: {other}"))),
    }
}

fn check_command(args: &[String]) -> Result<(), CliError> {
    let mut path = None::<PathBuf>;
    let mut json = false;

    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--json" => json = true,
            "--help" | "-h" => {
                print_check_usage();
                return Ok(());
            }
            value if value.starts_with('-') => {
                return Err(CliError::Usage(format!("unknown check option: {value}")));
            }
            value => {
                if path.is_some() {
                    return Err(CliError::Usage(
                        "check accepts exactly one path".to_string(),
                    ));
                }
                path = Some(PathBuf::from(value));
            }
        }
        index += 1;
    }

    let Some(input_path) = path else {
        return Err(CliError::Usage("check requires a path".to_string()));
    };

    let entry = match puzzle_lang::resolve_game_entry(&input_path) {
        Ok(entry) => entry,
        Err(error) => {
            let diagnostic = Diagnostic::error(input_path.clone(), error.to_string());
            write_check_failure(json, &[diagnostic]);
            return Err(CliError::CommandFailed);
        }
    };

    match puzzle_lang::parse_game_file(&entry) {
        Ok(document) => {
            let diagnostics = warning_diagnostics(&entry, &document);
            if json {
                print_json_result(true, &diagnostics);
            } else {
                for diagnostic in &diagnostics {
                    eprintln!("warning: {}", diagnostic.message);
                }
                println!("ok: {}", entry.display());
            }
            Ok(())
        }
        Err(error) => {
            let diagnostic = diagnostic_from_error(&entry, error.to_string());
            write_check_failure(json, &[diagnostic]);
            Err(CliError::CommandFailed)
        }
    }
}

fn write_check_failure(json: bool, diagnostics: &[Diagnostic]) {
    if json {
        print_json_result(false, diagnostics);
    } else {
        for diagnostic in diagnostics {
            eprintln!(
                "{}: {}\n  --> {}",
                diagnostic.severity,
                diagnostic.message,
                diagnostic.location()
            );
        }
    }
}

fn export_html_command(args: &[String]) -> Result<(), CliError> {
    let mut input_path = None::<PathBuf>;
    let mut output_path = None::<PathBuf>;

    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "-o" | "--output" => {
                index += 1;
                let Some(value) = args.get(index) else {
                    return Err(CliError::Usage(format!(
                        "{} requires a value",
                        args[index - 1]
                    )));
                };
                output_path = Some(PathBuf::from(value));
            }
            "--help" | "-h" => {
                print_export_html_usage();
                return Ok(());
            }
            value if value.starts_with('-') => {
                return Err(CliError::Usage(format!(
                    "unknown export-html option: {value}"
                )));
            }
            value => {
                if input_path.is_some() {
                    return Err(CliError::Usage(
                        "export-html accepts exactly one input path".to_string(),
                    ));
                }
                input_path = Some(PathBuf::from(value));
            }
        }
        index += 1;
    }

    let Some(input_path) = input_path else {
        return Err(CliError::Usage(
            "export-html requires an input path".to_string(),
        ));
    };
    let Some(output_path) = output_path else {
        return Err(CliError::Usage(
            "export-html requires -o/--output to avoid accidental writes".to_string(),
        ));
    };

    let entry = puzzle_lang::resolve_game_entry(&input_path)?;
    let html = html_play::export_html_file(&entry).map_err(CliError::Config)?;
    if let Some(parent) = output_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output_path, html)?;
    println!("exported {}", output_path.display());
    Ok(())
}

fn import_puzzlescript_command(args: &[String]) -> Result<(), CliError> {
    let mut input_path = None::<PathBuf>;
    let mut output_path = None::<PathBuf>;

    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "-o" | "--output" => {
                index += 1;
                let Some(value) = args.get(index) else {
                    return Err(CliError::Usage(format!(
                        "{} requires a value",
                        args[index - 1]
                    )));
                };
                output_path = Some(PathBuf::from(value));
            }
            "--help" | "-h" => {
                print_import_puzzlescript_usage();
                return Ok(());
            }
            value if value.starts_with('-') => {
                return Err(CliError::Usage(format!(
                    "unknown import-puzzlescript option: {value}"
                )));
            }
            value => {
                if input_path.is_some() {
                    return Err(CliError::Usage(
                        "import-puzzlescript accepts exactly one input path".to_string(),
                    ));
                }
                input_path = Some(PathBuf::from(value));
            }
        }
        index += 1;
    }

    let Some(input_path) = input_path else {
        return Err(CliError::Usage(
            "import-puzzlescript requires an input path".to_string(),
        ));
    };
    let Some(output_path) = output_path else {
        return Err(CliError::Usage(
            "import-puzzlescript requires -o/--output".to_string(),
        ));
    };

    let source = fs::read_to_string(&input_path)?;
    let translated = puzzle_lang::translate_puzzlescript_to_canonical(&source)?;
    if let Some(parent) = output_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output_path, translated)?;
    println!("imported {}", output_path.display());
    Ok(())
}

fn warning_diagnostics(entry: &Path, document: &LoadedDocument) -> Vec<Diagnostic> {
    document
        .models
        .iter()
        .flat_map(|model| match model {
            LoadedDocumentModel::Puzzle2d { game, .. } => game.warnings.as_slice(),
            LoadedDocumentModel::Puzzle3d { .. } => &[],
        })
        .map(|warning| Diagnostic::warning(entry.to_path_buf(), warning.clone()))
        .collect()
}

fn diagnostic_from_error(path: &Path, message: String) -> Diagnostic {
    let mut diagnostic = Diagnostic::error(path.to_path_buf(), message.clone());
    if let Some((line, column)) = find_error_location(path, &message) {
        diagnostic.line = Some(line);
        diagnostic.column = Some(column);
    }
    diagnostic
}

fn find_error_location(path: &Path, message: &str) -> Option<(usize, usize)> {
    let (_, source_line) = message.rsplit_once(": ")?;
    if source_line.is_empty() {
        return None;
    }
    let source = fs::read_to_string(path).ok()?;
    source.lines().enumerate().find_map(|(index, line)| {
        (line.trim() == source_line.trim()).then_some((index + 1, first_non_space_column(line)))
    })
}

fn first_non_space_column(line: &str) -> usize {
    line.char_indices()
        .find_map(|(index, ch)| (!ch.is_whitespace()).then_some(index + 1))
        .unwrap_or(1)
}

#[derive(Clone, Debug)]
struct Diagnostic {
    severity: &'static str,
    file: PathBuf,
    line: Option<usize>,
    column: Option<usize>,
    message: String,
}

impl Diagnostic {
    fn error(file: PathBuf, message: String) -> Self {
        Self {
            severity: "error",
            file,
            line: None,
            column: None,
            message,
        }
    }

    fn warning(file: PathBuf, message: String) -> Self {
        Self {
            severity: "warning",
            file,
            line: None,
            column: None,
            message,
        }
    }

    fn location(&self) -> String {
        match (self.line, self.column) {
            (Some(line), Some(column)) => format!("{}:{line}:{column}", self.file.display()),
            _ => self.file.display().to_string(),
        }
    }
}

fn print_json_result(ok: bool, diagnostics: &[Diagnostic]) {
    print!(
        "{{\"ok\":{},\"diagnostics\":[",
        if ok { "true" } else { "false" }
    );
    for (index, diagnostic) in diagnostics.iter().enumerate() {
        if index > 0 {
            print!(",");
        }
        print!(
            "{{\"severity\":\"{}\",\"file\":\"{}\",\"line\":{},\"column\":{},\"message\":\"{}\"}}",
            diagnostic.severity,
            escape_json(&diagnostic.file.display().to_string()),
            option_number_json(diagnostic.line),
            option_number_json(diagnostic.column),
            escape_json(&diagnostic.message)
        );
    }
    println!("]}}");
}

fn option_number_json(value: Option<usize>) -> String {
    value.map_or_else(|| "null".to_string(), |value| value.to_string())
}

fn escape_json(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out
}

fn print_usage() {
    eprintln!(
        "usage:\n  puzzlestudio check <path> [--json]\n  puzzlestudio export-html <path> -o <output.html>\n  puzzlestudio import-puzzlescript <source.txt> -o <game.puzzle>"
    );
}

fn print_check_usage() {
    eprintln!("usage: puzzlestudio check <path/to/game-folder-or-game.puzzle> [--json]");
}

fn print_export_html_usage() {
    eprintln!(
        "usage: puzzlestudio export-html <path/to/game-folder-or-game.puzzle> -o <output.html>"
    );
}

fn print_import_puzzlescript_usage() {
    eprintln!("usage: puzzlestudio import-puzzlescript <source.txt> -o <game.puzzle>");
}

#[derive(Debug)]
enum CliError {
    Io(std::io::Error),
    Lang(puzzle_lang::AppError),
    Config(String),
    Usage(String),
    CommandFailed,
}

impl From<std::io::Error> for CliError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<puzzle_lang::AppError> for CliError {
    fn from(value: puzzle_lang::AppError) -> Self {
        Self::Lang(value)
    }
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::Lang(error) => write!(f, "{error}"),
            Self::Config(error) => write!(f, "{error}"),
            Self::Usage(error) => write!(f, "{error}"),
            Self::CommandFailed => write!(f, "command failed"),
        }
    }
}

impl std::error::Error for CliError {}
