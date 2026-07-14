use std::env;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use puzzle_lang::{Diagnostic, DiagnosticReport, LoadedDocument, LoadedDocumentModel};

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
        "export-editor" => export_editor_command(&args),
        "import-puzzlescript" => import_puzzlescript_command(&args),
        "inspect" | "list" => inspect_command(&args),
        "agent" => agent_command(&args),
        "screenshot" => screenshot_command(&args),
        "preview" => preview_command(&args),
        "editor" | "edit" => editor_command(&args),
        "--help" | "-h" | "help" => {
            print_usage();
            Ok(())
        }
        other => Err(CliError::Usage(format!("unknown command: {other}"))),
    }
}

fn agent_command(args: &[String]) -> Result<(), CliError> {
    match args {
        [arg] if arg == "--help" || arg == "-h" || arg == "help" => {
            print_agent_usage();
            Ok(())
        }
        [arg] if arg == "--stdio" || arg == "request" => {
            agent_json_lines(io::stdin().lock(), io::stdout().lock())
        }
        [] => Err(CliError::Usage(
            "agent requires --stdio or request".to_string(),
        )),
        _ => Err(CliError::Usage(format!(
            "unknown agent arguments: {}",
            args.join(" ")
        ))),
    }
}

fn agent_json_lines(reader: impl BufRead, mut writer: impl Write) -> Result<(), CliError> {
    let mut server = puzzle_agent_runtime::AgentServer::new();
    for line in reader.lines() {
        let response = server.handle_line(&line?);
        writeln!(writer, "{response}")?;
        writer.flush()?;
    }
    Ok(())
}

fn preview_command(args: &[String]) -> Result<(), CliError> {
    #[cfg(feature = "preview")]
    {
        if args.iter().any(|arg| arg == "--help" || arg == "-h") {
            print_preview_usage();
            return Ok(());
        }
        let mut forwarded = args.to_vec();
        if !forwarded.iter().any(|arg| arg == "--serve") {
            forwarded.push("--serve".to_string());
        }
        html_play::run_cli_with_args(forwarded).map_err(CliError::Config)
    }
    #[cfg(not(feature = "preview"))]
    {
        let _ = args;
        Err(disabled_adapter_command("preview", "preview"))
    }
}

fn editor_command(args: &[String]) -> Result<(), CliError> {
    #[cfg(feature = "editor")]
    {
        if args.iter().any(|arg| arg == "--help" || arg == "-h") {
            print_editor_usage();
            return Ok(());
        }
        let mut forwarded = args.to_vec();
        if !forwarded.iter().any(|arg| arg == "--serve") {
            forwarded.push("--serve".to_string());
        }
        html_editor::run_cli_with_args(forwarded)
            .map_err(|error| CliError::Config(error.to_string()))
    }
    #[cfg(not(feature = "editor"))]
    {
        let _ = args;
        Err(disabled_adapter_command("editor", "editor"))
    }
}

fn screenshot_command(args: &[String]) -> Result<(), CliError> {
    if args.iter().any(|arg| arg == "--list") {
        return inspect_command(&screenshot_inspect_args(args)?);
    }

    #[cfg(feature = "screenshot")]
    {
        if args.iter().any(|arg| arg == "--help" || arg == "-h") {
            print_screenshot_usage();
            return Ok(());
        }
        html_play::run_cli_with_args(screenshot_forwarded_args(args)?).map_err(CliError::Config)
    }
    #[cfg(not(feature = "screenshot"))]
    {
        let _ = args;
        Err(disabled_adapter_command("screenshot", "screenshot"))
    }
}

fn screenshot_inspect_args(args: &[String]) -> Result<Vec<String>, CliError> {
    let mut inspect_args = Vec::new();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--list" => {}
            "-o"
            | "--output"
            | "--scene"
            | "--level"
            | "--input"
            | "--inputs"
            | "--width"
            | "--height"
            | "--screenshot-timeout-ms"
            | "--browser" => {
                index += 1;
                if index >= args.len() {
                    return Err(CliError::Usage(format!(
                        "{} requires a value",
                        args[index - 1]
                    )));
                }
            }
            "--help" | "-h" => inspect_args.push(args[index].clone()),
            value if value.starts_with('-') => {
                return Err(CliError::Usage(format!(
                    "unknown screenshot list option: {value}"
                )));
            }
            value => {
                if inspect_args.iter().any(|arg| !arg.starts_with('-')) {
                    return Err(CliError::Usage(
                        "screenshot --list accepts exactly one input path".to_string(),
                    ));
                }
                inspect_args.push(value.to_string());
            }
        }
        index += 1;
    }
    Ok(inspect_args)
}

#[cfg(feature = "screenshot")]
fn screenshot_forwarded_args(args: &[String]) -> Result<Vec<String>, CliError> {
    let mut forwarded = Vec::new();
    let mut output_path = None::<String>;
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
                output_path = Some(value.clone());
            }
            value => forwarded.push(value.to_string()),
        }
        index += 1;
    }
    let Some(output_path) = output_path else {
        return Err(CliError::Usage(
            "screenshot requires -o/--output <output.png>".to_string(),
        ));
    };
    forwarded.push("--screenshot".to_string());
    forwarded.push(output_path);
    Ok(forwarded)
}

fn inspect_command(args: &[String]) -> Result<(), CliError> {
    let mut path = None::<PathBuf>;

    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--help" | "-h" => {
                print_inspect_usage();
                return Ok(());
            }
            value if value.starts_with('-') => {
                return Err(CliError::Usage(format!("unknown inspect option: {value}")));
            }
            value => {
                if path.is_some() {
                    return Err(CliError::Usage(
                        "inspect accepts exactly one path".to_string(),
                    ));
                }
                path = Some(PathBuf::from(value));
            }
        }
        index += 1;
    }

    let Some(input_path) = path else {
        return Err(CliError::Usage("inspect requires a path".to_string()));
    };
    let entry = puzzle_lang::resolve_game_entry(&input_path)?;
    let document = puzzle_lang::parse_game_file(&entry)?;
    print!("{}", inspect_document_text(&document));
    Ok(())
}

fn inspect_document_text(document: &LoadedDocument) -> String {
    let mut scenes = Vec::<String>::new();
    for scene in &document.scenes {
        push_unique(&mut scenes, scene.name.clone());
    }
    for model in &document.models {
        if let LoadedDocumentModel::Puzzle2d { game, .. } = model {
            for scene in &game.scenes {
                push_unique(&mut scenes, scene.name.clone());
            }
        }
    }

    let mut out = String::new();
    out.push_str("scenes:\n");
    if scenes.is_empty() {
        out.push_str("  (none)\n");
    } else {
        for (index, scene) in scenes.iter().enumerate() {
            out.push_str(&format!("  {index}: {scene}\n"));
        }
    }

    out.push_str("levels:\n");
    let multi_model = document.models.len() > 1;
    let mut any_levels = false;
    for model in &document.models {
        match model {
            LoadedDocumentModel::Puzzle2d { name, game } => {
                if multi_model {
                    out.push_str(&format!("  {name}:\n"));
                }
                if game.levels.is_empty() {
                    if multi_model {
                        out.push_str("    (none)\n");
                    }
                    continue;
                }
                any_levels = true;
                for (index, level) in game.levels.iter().enumerate() {
                    if multi_model {
                        out.push_str(&format!("    {index}: {}\n", level.name));
                    } else {
                        out.push_str(&format!("  {index}: {}\n", level.name));
                    }
                }
            }
            LoadedDocumentModel::Puzzle3d { name, puzzle } => {
                if multi_model {
                    out.push_str(&format!("  {name}:\n"));
                }
                let Some(bundle) = puzzle.level_bundle.as_ref() else {
                    if multi_model {
                        out.push_str("    (none)\n");
                    }
                    continue;
                };
                any_levels = true;
                for (index, level) in bundle.levels.iter().enumerate() {
                    if multi_model {
                        out.push_str(&format!("    {index}: {}\n", level.name));
                    } else {
                        out.push_str(&format!("  {index}: {}\n", level.name));
                    }
                }
            }
        }
    }
    if !any_levels {
        out.push_str("  (none)\n");
    }

    out.push_str("inputs:\n");
    let mut any_inputs = false;
    for model in &document.models {
        match model {
            LoadedDocumentModel::Puzzle2d { name, game } => {
                if multi_model {
                    out.push_str(&format!("  {name}:\n"));
                }
                let mut inputs = game
                    .input_labels
                    .iter()
                    .map(|(id, label)| (id.0, label.as_str()))
                    .collect::<Vec<_>>();
                inputs.sort_by_key(|(id, _)| *id);
                if inputs.is_empty() {
                    if multi_model {
                        out.push_str("    (none)\n");
                    }
                    continue;
                }
                any_inputs = true;
                for (_, input) in inputs {
                    if multi_model {
                        out.push_str(&format!("    {input}\n"));
                    } else {
                        out.push_str(&format!("  {input}\n"));
                    }
                }
            }
            LoadedDocumentModel::Puzzle3d { name, puzzle } => {
                if multi_model {
                    out.push_str(&format!("  {name}:\n"));
                }
                let mut inputs = puzzle
                    .inputs
                    .iter()
                    .map(|input| (input.id.0, input.name.as_str()))
                    .collect::<Vec<_>>();
                inputs.sort_by_key(|(id, _)| *id);
                if inputs.is_empty() {
                    if multi_model {
                        out.push_str("    (none)\n");
                    }
                    continue;
                }
                any_inputs = true;
                for (_, input) in inputs {
                    if multi_model {
                        out.push_str(&format!("    {input}\n"));
                    } else {
                        out.push_str(&format!("  {input}\n"));
                    }
                }
            }
        }
    }
    if !any_inputs {
        out.push_str("  (none)\n");
    }
    out
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

#[cfg(not(all(
    feature = "preview",
    feature = "editor",
    feature = "export-html",
    feature = "export-editor",
    feature = "screenshot"
)))]
fn disabled_adapter_command(command: &str, feature: &str) -> CliError {
    CliError::Config(format!(
        "puzzlestudio {command} is not included in this build; rebuild with --features {feature} or --features adapters"
    ))
}

#[cfg(any(
    feature = "preview",
    feature = "editor",
    feature = "export-html",
    feature = "export-editor",
    feature = "screenshot"
))]
fn adapter_feature_note() -> &'static str {
    ""
}

#[cfg(not(any(
    feature = "preview",
    feature = "editor",
    feature = "export-html",
    feature = "export-editor",
    feature = "screenshot"
)))]
fn adapter_feature_note() -> &'static str {
    "\n\nadapter commands require --features adapters when building from source"
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
            let diagnostics = diagnostics_with_file(&input_path, error);
            write_check_failure(json, &diagnostics);
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
            let source = fs::read_to_string(&entry).ok();
            let diagnostics = diagnostics_with_file_and_source(&entry, error, source.as_deref());
            write_check_failure(json, &diagnostics);
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
                diagnostic.severity.as_str(),
                diagnostic.message,
                diagnostic_location(diagnostic)
            );
        }
    }
}

fn export_html_command(args: &[String]) -> Result<(), CliError> {
    #[cfg(not(feature = "export-html"))]
    {
        let _ = args;
        return Err(disabled_adapter_command("export-html", "export-html"));
    }

    #[cfg(feature = "export-html")]
    {
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
}

fn export_editor_command(args: &[String]) -> Result<(), CliError> {
    #[cfg(not(feature = "export-editor"))]
    {
        let _ = args;
        return Err(disabled_adapter_command("export-editor", "export-editor"));
    }

    #[cfg(feature = "export-editor")]
    {
        if args.iter().any(|arg| arg == "--help" || arg == "-h") {
            print_export_editor_usage();
            return Ok(());
        }
        html_editor::run_cli_with_args(args.iter().cloned())
            .map_err(|error| CliError::Config(error.to_string()))
    }
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
        .map(|warning| Diagnostic::warning(warning.clone()).with_file(entry.display().to_string()))
        .collect()
}

fn diagnostics_with_file(path: &Path, report: DiagnosticReport) -> Vec<Diagnostic> {
    let file = path.display().to_string();
    report
        .into_diagnostics()
        .into_iter()
        .map(|diagnostic| {
            if diagnostic
                .primary_span
                .as_ref()
                .and_then(|span| span.file.as_ref())
                .is_some()
            {
                diagnostic
            } else {
                diagnostic.with_file(file.clone())
            }
        })
        .collect()
}

fn diagnostics_with_file_and_source(
    path: &Path,
    report: DiagnosticReport,
    _source: Option<&str>,
) -> Vec<Diagnostic> {
    diagnostics_with_file(path, report)
}

fn diagnostic_location(diagnostic: &Diagnostic) -> String {
    let span = diagnostic.primary_span.as_ref();
    let file = span
        .and_then(|span| span.file.as_deref())
        .unwrap_or("<source>");
    match (
        span.and_then(|span| span.line),
        span.and_then(|span| span.column),
    ) {
        (Some(line), Some(column)) => format!("{file}:{line}:{column}"),
        (Some(line), None) => format!("{file}:{line}"),
        _ => file.to_string(),
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
        let span = diagnostic.primary_span.as_ref();
        print!(
            "{{\"severity\":\"{}\",\"code\":\"{}\",\"file\":\"{}\",\"line\":{},\"column\":{},\"sourceLine\":{},\"message\":\"{}\"}}",
            diagnostic.severity.as_str(),
            escape_json(diagnostic.code),
            escape_json(span.and_then(|span| span.file.as_deref()).unwrap_or("")),
            option_number_json(span.and_then(|span| span.line)),
            option_number_json(span.and_then(|span| span.column)),
            option_string_json(span.and_then(|span| span.source_line.as_deref())),
            escape_json(&diagnostic.message)
        );
    }
    println!("]}}");
}

fn option_number_json(value: Option<usize>) -> String {
    value.map_or_else(|| "null".to_string(), |value| value.to_string())
}

fn option_string_json(value: Option<&str>) -> String {
    value.map_or_else(
        || "null".to_string(),
        |value| format!("\"{}\"", escape_json(value)),
    )
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
        "usage:\n  puzzlestudio check <path> [--json]\n  puzzlestudio inspect <path>\n  puzzlestudio agent --stdio\n  puzzlestudio agent request < requests.jsonl\n  puzzlestudio preview [path] [--port 7878] [--solver-depth N] [--solver-nodes N] [--solver-ms N]\n  puzzlestudio editor [path] [--port 8787]\n  puzzlestudio export-html <path> -o <output.html>\n  puzzlestudio export-editor [path] -o <docs/index.html>\n  puzzlestudio screenshot <path> -o <output.png> [--scene name] [--level name-or-index] [--input name] [--inputs a,b,c] [--width 1280] [--height 720] [--browser path]\n  puzzlestudio screenshot <path> --list\n  puzzlestudio import-puzzlescript <source.txt> -o <game.puzzle>{}",
        adapter_feature_note()
    );
}

fn print_agent_usage() {
    eprintln!(
        "usage:\n  puzzlestudio agent --stdio\n  puzzlestudio agent request < requests.jsonl"
    );
}

fn print_check_usage() {
    eprintln!(
        "usage: puzzlestudio check <path/to/game-folder-or-game.puzzle-or-game.puzzle3> [--json]"
    );
}

fn print_inspect_usage() {
    eprintln!("usage: puzzlestudio inspect <path/to/game-folder-or-game.puzzle-or-game.puzzle3>");
}

#[cfg(feature = "export-html")]
fn print_export_html_usage() {
    eprintln!(
        "usage: puzzlestudio export-html <path/to/game-folder-or-game.puzzle-or-game.puzzle3> -o <output.html>"
    );
}

#[cfg(feature = "preview")]
fn print_preview_usage() {
    eprintln!(
        "usage: puzzlestudio preview [path/to/game-folder-or-game.puzzle-or-game.puzzle3] [--port 7878] [--solver-depth N] [--solver-nodes N] [--solver-ms N]"
    );
}

#[cfg(feature = "editor")]
fn print_editor_usage() {
    eprintln!(
        "usage: puzzlestudio editor [path/to/game-folder-or-game.puzzle-or-game.puzzle3] [--port 8787]"
    );
}

#[cfg(feature = "export-editor")]
fn print_export_editor_usage() {
    eprintln!(
        "usage: puzzlestudio export-editor [path/to/game-folder-or-game.puzzle-or-game.puzzle3] -o <docs/index.html>"
    );
}

#[cfg(feature = "screenshot")]
fn print_screenshot_usage() {
    eprintln!(
        "usage: puzzlestudio screenshot <path/to/game-folder-or-game.puzzle-or-game.puzzle3> -o <output.png> [--scene name] [--level name-or-index] [--input name] [--inputs a,b,c] [--width 1280] [--height 720] [--screenshot-timeout-ms 5000] [--browser path]\n       puzzlestudio screenshot <path/to/game-folder-or-game.puzzle-or-game.puzzle3> --list"
    );
}

fn print_import_puzzlescript_usage() {
    eprintln!("usage: puzzlestudio import-puzzlescript <source.txt> -o <game.puzzle>");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inspect_document_lists_scenes_levels_and_inputs() {
        let document = puzzle_lang::parse_game(include_str!(
            "../../../crates/lang/tests/fixtures/spec_2d_microban_basic.puzzle"
        ))
        .expect("parse sample game");
        let text = inspect_document_text(&document);

        assert!(text.contains("scenes:\n"));
        assert!(text.contains("  0: title\n"));
        assert!(text.contains("  1: playing\n"));
        assert!(text.contains("levels:\n"));
        assert!(text.contains("  0: microban.1\n"));
        assert!(text.contains("  1: microban.2\n"));
        assert!(text.contains("inputs:\n"));
        assert!(text.contains("  up\n"));
        assert!(text.contains("  right\n"));
    }

    #[test]
    fn screenshot_list_args_keep_only_the_input_path() {
        let args = vec![
            "games/spec_2d.puzzle".to_string(),
            "-o".to_string(),
            "/tmp/out.png".to_string(),
            "--scene".to_string(),
            "playing".to_string(),
            "--list".to_string(),
        ];

        assert_eq!(
            screenshot_inspect_args(&args).expect("parse screenshot list args"),
            vec!["games/spec_2d.puzzle".to_string()]
        );
    }

    #[test]
    fn agent_json_lines_keeps_the_session_alive_after_request_errors() {
        let input = b"{\n{\"version\":1,\"op\":\"manifest\",\"sessionId\":\"missing\"}\n";
        let mut output = Vec::new();

        agent_json_lines(std::io::Cursor::new(input), &mut output).unwrap();

        let lines = String::from_utf8(output).unwrap();
        let responses = lines.lines().collect::<Vec<_>>();
        assert_eq!(responses.len(), 2);
        assert!(responses[0].contains(r#""code":"invalid_request""#));
        assert!(responses[1].contains(r#""code":"unknown_session""#));
    }
}

#[derive(Debug)]
enum CliError {
    Io(std::io::Error),
    Lang(DiagnosticReport),
    Config(String),
    Usage(String),
    CommandFailed,
}

impl From<std::io::Error> for CliError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<DiagnosticReport> for CliError {
    fn from(value: DiagnosticReport) -> Self {
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
