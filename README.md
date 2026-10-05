# STL Analyzer

Rust console application for parsing and checking ASCII and binary STL meshes. It uses no external crates.

## Run

Build with stable Rust: `cargo build --release`.

Place `config.json` beside the executable when distributing it. For `cargo run`, the checked-in project copy is used as a development fallback. `stl_folder` is relative to that config file.

Run the numbered menu with `cargo run --release`. It lists `.stl` files in the configured folder and reports the detected STL format, total faces/triangles, exact-coordinate unique vertices, bounds, and mesh quality findings. ASCII and binary STL files are both supported.

The report identifies:

- High-incidence vertices by their number of unique incident edges.
- Sliver triangles by aspect ratio.
- Micro triangles by area relative to the model bounding-box diagonal squared.
- Extremely large triangles by the same model-relative area measure.

Thresholds are configurable in `config.json`. Defaults are a sliver aspect ratio of `20`, micro-triangle area ratio of `1e-8`, large-triangle area ratio of `0.25`, and a high-incidence threshold of more than `12` unique edges. These are review heuristics, not manufacturing tolerances. Vertex deduplication uses exact `f32` coordinates; positive and negative zero are treated as equal.

Generate a sample with configurable density:

```sh
cargo run --release -- generate cube 12 ascii cube.stl
cargo run --release -- generate torus 48 binary torus.stl
```

Supported shapes are `cuboid` (also accepted as `prism` or `box`), `cube`, `cylinder`, `cone`, `sphere`, `torus`, and `tetrahedron`. The tetrahedron is regular; `--size` controls its edge length. Density subdivides its faces.

## Dedicated Generator

`stl-generator` is a separate binary for dimensioned models. It writes binary STL to the configured `stl_folder` by default; use `--format ascii` or `--output` to override either choice.

Run it without arguments for the interactive generator. It offers all seven supported shapes, asks for their dimensions, and accepts displayed defaults when you press Enter.

```sh
cargo run --release --bin stl-generator
```

```sh
cargo run --release --bin stl-generator -- cube --size 40 --density 16
cargo run --release --bin stl-generator -- prism --width 60 --height 25 --depth 10 --format ascii
cargo run --release --bin stl-generator -- cuboid --width 60 --height 25 --depth 10
cargo run --release --bin stl-generator -- cylinder --radius 12.5 --height 80
cargo run --release --bin stl-generator -- cone --radius 15 --height 50
cargo run --release --bin stl-generator -- sphere --radius 18
cargo run --release --bin stl-generator -- torus --major-radius 30 --minor-radius 8
cargo run --release --bin stl-generator -- tetrahedron --size 20 --density 8
```

Defaults: cuboid `2 x 1 x 1`, cube size `1`, cylinder/cone radius `1` and height `2`, sphere radius `1`, torus major radius `1` and minor radius `0.35`, tetrahedron edge length `1`, density `24`. This is a documented built-in catalog of common parametric solids; it is not an exhaustive list of every possible 3D shape.

Create and measure the required fixture:

```sh
cargo run --release -- benchmark 1000000 benchmark_1000000.stl
```

The benchmark streams the binary fixture to disk, then reports parsing, deduplication, analysis, total time, and estimated indexed-mesh storage. Run checks with `cargo test`. Integration tests live in `tests/` and use distinct descriptive filenames (for example, `mesh_analysis.rs`); production modules remain under `src/`.

## Components

- `geometry.rs`: vectors, raw triangles, and exact-coordinate indexed mesh deduplication.
- `stl.rs`: structural binary detection, strict ASCII parsing, and STL writers.
- `analysis.rs`: bounds, sliver, micro/large relative-area, and unique incident-edge checks.
- `samples.rs`: parameterized primitives and the streaming grid benchmark writer.
- `app.rs`: config loading, menu, reports, commands, and benchmark timings.


The core console analyzer is implemented without external crates. Sample generators and the benchmark are included as optional demonstration tools.
The core analysis application is 934 Rust lines, excluding tests and the optional sample/benchmark generators.

Regenerate the PDF after editing the HTML guide with:

```sh
cargo run --bin render-guide -- docs/stl-analyzer-guide.html docs/STL-Analyzer-Rust-Technical-Guide.pdf
```
