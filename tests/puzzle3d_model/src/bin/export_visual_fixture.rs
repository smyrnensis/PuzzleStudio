use std::env;
use std::fs;
use std::path::PathBuf;

use puzzle3d_model::{export_visual_fixture_json, parse_puzzle3d};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let source = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("games/spec_3d.puzzle"));
    let json_output = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("visual/fixture.json"));
    let js_output = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("visual/fixture.js"));

    let source_text = fs::read_to_string(&source)
        .map_err(|error| format!("failed to read {}: {error}", source.display()))?;
    let parsed = parse_puzzle3d(&source_text)
        .map_err(|error| format!("failed to parse {}: {error:?}", source.display()))?;
    let json = export_visual_fixture_json(&parsed)
        .map_err(|error| format!("failed to export visual fixture: {error:?}"))?;

    fs::write(&json_output, &json)
        .map_err(|error| format!("failed to write {}: {error}", json_output.display()))?;
    fs::write(&js_output, format!("window.Puzzle3DFixture = {json}"))
        .map_err(|error| format!("failed to write {}: {error}", js_output.display()))?;

    Ok(())
}
