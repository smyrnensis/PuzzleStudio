fn main() {
    if let Err(error) = html_editor::run_cli() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
