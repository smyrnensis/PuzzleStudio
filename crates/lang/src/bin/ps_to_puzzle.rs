use std::env;
use std::fs;
use std::path::PathBuf;

use puzzle_lang::translate_puzzlescript_to_canonical;

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let source_path = args.next().map(PathBuf::from).ok_or_else(|| {
        "usage: cargo run -p puzzle-lang --bin ps_to_puzzle -- source.ps output.puzzle".to_string()
    })?;
    let output_path = args.next().map(PathBuf::from).ok_or_else(|| {
        "usage: cargo run -p puzzle-lang --bin ps_to_puzzle -- source.ps output.puzzle".to_string()
    })?;
    if args.next().is_some() {
        return Err("too many arguments".into());
    }

    let source = fs::read_to_string(&source_path)?;
    let canonical = translate_puzzlescript_to_canonical(&source)?;
    fs::write(&output_path, canonical)?;
    println!("translated {}", output_path.display());
    Ok(())
}
