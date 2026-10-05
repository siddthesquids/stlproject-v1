use std::io::{self, BufReader};

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let stdin = io::stdin();
    let mut input = BufReader::new(stdin.lock());
    let mut output = io::stdout().lock();
    if let Err(error) = stl_analyzer::app::run_command(&arguments, &mut input, &mut output) {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}
