use std::env;
use std::process::ExitCode;

use stl_analyzer::samples::{generate, SampleKind};
use stl_analyzer::stl::write_binary;

fn main() -> ExitCode {
    let path = env::args().nth(1).unwrap_or_else(|| "sample.stl".into());
    let density = match env::args().nth(2) {
        Some(value) => match value.parse::<u32>() {
            Ok(count) => count,
            Err(error) => {
                eprintln!("Invalid triangle count: {error}");
                return ExitCode::from(2);
            }
        },
        None => 24,
    };

    let triangles = generate(SampleKind::Torus, density as usize);
    match std::fs::File::create(&path)
        .map_err(stl_analyzer::stl::StlError::from)
        .and_then(|mut file| write_binary(&mut file, "torus", &triangles))
    {
        Ok(()) => {
            println!("Wrote {path} with {} triangles", triangles.len());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("Could not write {path}: {error}");
            ExitCode::FAILURE
        }
    }
}
