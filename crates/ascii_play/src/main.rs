fn main() {
    if let Err(error) = ascii_play::run_terminal_from_env() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
