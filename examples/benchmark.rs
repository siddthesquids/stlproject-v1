use std::env;
use std::time::Instant;

use stl_analyzer::analysis::{analyze, Thresholds};
use stl_analyzer::geometry::Mesh;
use stl_analyzer::stl::parse_file;

fn main() {
    let Some(path) = env::args().nth(1) else {
        eprintln!("Usage: cargo run --release --example benchmark -- <file.stl> [iterations]");
        std::process::exit(2);
    };
    let iterations = env::args()
        .nth(2)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(1);
    let start = Instant::now();
    let mut triangle_count = 0;
    for _ in 0..iterations {
        let parsed = parse_file(std::path::Path::new(&path)).expect("failed to parse STL file");
        let mesh = Mesh::from_triangles(parsed.triangles);
        triangle_count = mesh.triangles.len();
        let _ = analyze(&mesh, Thresholds::default());
    }
    let elapsed = start.elapsed();
    let throughput = triangle_count as f64 * iterations as f64 / elapsed.as_secs_f64();
    println!(
        "Loaded and analyzed {triangle_count} triangles {iterations} times in {elapsed:.3?} ({throughput:.0} triangles/s)"
    );
}
