use std::env;
use std::fs;
use std::path::PathBuf;

use puzzle_lang::{export_loaded_document_visual_fixture_json, parse_game};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let source = args.next().map(PathBuf::from).ok_or_else(|| {
        "usage: export_puzzle3_visual_fixture <source.puzzle> <fixture.json> <fixture.js>"
            .to_string()
    })?;
    let json_output = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "missing fixture.json output path".to_string())?;
    let js_output = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "missing fixture.js output path".to_string())?;

    let source_text = fs::read_to_string(&source)
        .map_err(|error| format!("failed to read {}: {error}", source.display()))?;
    let document = parse_game(&source_text)
        .map_err(|error| format!("failed to parse {}: {error}", source.display()))?;
    let json = export_loaded_document_visual_fixture_json(&document)
        .map_err(|error| format!("failed to export fixture: {error}"))?;

    fs::write(&json_output, &json)
        .map_err(|error| format!("failed to write {}: {error}", json_output.display()))?;
    fs::write(&js_output, format!("window.Puzzle3DFixture = {json}"))
        .map_err(|error| format!("failed to write {}: {error}", js_output.display()))?;

    Ok(())
}
