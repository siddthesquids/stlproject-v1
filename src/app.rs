use crate::analysis::{analyze, Analysis, Thresholds};
use crate::geometry::Mesh;
use crate::samples::{generate, write_benchmark_binary, SampleKind};
use crate::stl::{parse_file, write_ascii, write_binary, StlError};
use std::env;
use std::fs::{self, File};
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

pub struct Config {
    pub stl_folder: PathBuf,
    pub thresholds: Thresholds,
}

impl Config {
    pub fn load_default() -> io::Result<Self> {
        let executable_config = env::current_exe()?
            .parent()
            .map(|path| path.join("config.json"));
        let path = executable_config
            .filter(|path| path.exists())
            .unwrap_or(env::current_dir()?.join("config.json"));
        Self::load_from(&path)
    }

    pub fn load_from(path: &Path) -> io::Result<Self> {
        let text = fs::read_to_string(path)?;
        let folder =
            json_string(&text, "stl_folder").ok_or_else(|| invalid_config("missing stl_folder"))?;
        let base = path.parent().unwrap_or_else(|| Path::new("."));
        let thresholds = Thresholds {
            sliver_aspect_ratio: json_number(&text, "sliver_aspect_ratio").unwrap_or(20.0),
            micro_triangle_area_ratio: json_number(&text, "micro_triangle_area_ratio")
                .unwrap_or(1.0e-8),
            large_triangle_area_ratio: json_number(&text, "large_triangle_area_ratio")
                .unwrap_or(0.25),
            max_incident_edges: json_number(&text, "max_incident_edges").unwrap_or(12.0) as usize,
        };
        if !thresholds.sliver_aspect_ratio.is_finite()
            || thresholds.sliver_aspect_ratio <= 0.0
            || !thresholds.micro_triangle_area_ratio.is_finite()
            || thresholds.micro_triangle_area_ratio <= 0.0
            || !thresholds.large_triangle_area_ratio.is_finite()
            || thresholds.large_triangle_area_ratio <= 0.0
        {
            return Err(invalid_config(
                "area and aspect-ratio thresholds must be positive and finite",
            ));
        }
        Ok(Self {
            stl_folder: base.join(folder),
            thresholds,
        })
    }
}

fn invalid_config(message: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("invalid config.json: {message}"),
    )
}

fn value_after_key<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let start = text.find(&format!("\"{key}\""))? + key.len() + 2;
    let colon = text[start..].find(':')? + start;
    Some(text[colon + 1..].trim_start())
}

fn json_string(text: &str, key: &str) -> Option<String> {
    let value = value_after_key(text, key)?;
    let rest = value.strip_prefix('"')?;
    Some(rest.split('"').next()?.to_owned())
}

fn json_number(text: &str, key: &str) -> Option<f32> {
    value_after_key(text, key)?
        .split(|character: char| !matches!(character, '0'..='9' | '.' | '-' | '+' | 'e' | 'E'))
        .next()?
        .parse()
        .ok()
}

pub fn run_menu(
    input: &mut dyn BufRead,
    output: &mut dyn Write,
    config: &Config,
) -> io::Result<()> {
    loop {
        writeln!(
            output,
            "\nSTL Analyzer\n1. Open and analyze an STL file\n2. Close the application"
        )?;
        write!(output, "Choose an option: ")?;
        output.flush()?;
        let Some(choice) = read_line(input)? else {
            return Ok(());
        };
        match choice.trim() {
            "1" => open_from_menu(input, output, config)?,
            "2" => {
                writeln!(output, "Goodbye.")?;
                return Ok(());
            }
            _ => writeln!(output, "Please enter 1 or 2.")?,
        }
    }
}

fn open_from_menu(
    input: &mut dyn BufRead,
    output: &mut dyn Write,
    config: &Config,
) -> io::Result<()> {
    let files = list_stl_files(&config.stl_folder)?;
    if files.is_empty() {
        writeln!(
            output,
            "No .stl files found in {}.",
            config.stl_folder.display()
        )?;
        return Ok(());
    }
    for (index, path) in files.iter().enumerate() {
        writeln!(
            output,
            "{}. {}",
            index + 1,
            path.file_name().unwrap_or_default().to_string_lossy()
        )?;
    }
    write!(output, "Select a file number: ")?;
    output.flush()?;
    let Some(selection) = read_line(input)? else {
        return Ok(());
    };
    match selection
        .trim()
        .parse::<usize>()
        .ok()
        .and_then(|number| number.checked_sub(1))
        .and_then(|index| files.get(index))
    {
        Some(path) => match analyze_path(path, config.thresholds) {
            Ok(report) => write!(output, "{report}")?,
            Err(error) => writeln!(output, "Could not analyze {}: {error}", path.display())?,
        },
        None => writeln!(output, "Invalid file number.")?,
    }
    Ok(())
}

fn read_line(input: &mut dyn BufRead) -> io::Result<Option<String>> {
    let mut line = String::new();
    if input.read_line(&mut line)? == 0 {
        Ok(None)
    } else {
        Ok(Some(line))
    }
}

fn list_stl_files(folder: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if !folder.exists() {
        return Ok(files);
    }
    for entry in fs::read_dir(folder)? {
        let path = entry?.path();
        if path.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("stl"))
        {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

pub fn analyze_path(path: &Path, thresholds: Thresholds) -> Result<String, StlError> {
    let parsed = parse_file(path)?;
    let mesh = Mesh::from_triangles(parsed.triangles);
    let analysis = analyze(&mesh, thresholds);
    Ok(format_report(
        path,
        &parsed.format.to_string(),
        &mesh,
        &analysis,
    ))
}

fn format_report(path: &Path, format: &str, mesh: &Mesh, analysis: &Analysis) -> String {
    let mut output = format!(
        "\nFile: {}\nFormat: {format}\nTriangles/faces: {}\nUnique vertices: {}\n",
        path.display(),
        mesh.triangles.len(),
        mesh.vertices.len()
    );
    match analysis.bounds {
        Some(bounds) => output.push_str(&format!(
            "Bounding box: min ({:.6}, {:.6}, {:.6}), max ({:.6}, {:.6}, {:.6})\n",
            bounds.min.x, bounds.min.y, bounds.min.z, bounds.max.x, bounds.max.y, bounds.max.z
        )),
        None => output.push_str("Bounding box: unavailable for an empty mesh\n"),
    }
    if analysis.slivers.is_empty()
        && analysis.micro_triangles.is_empty()
        && analysis.large_triangles.is_empty()
        && analysis.high_incident_vertices.is_empty()
    {
        output.push_str("Quality warnings: none\n");
        return output;
    }
    output.push_str("Quality warnings:\n");
    for item in &analysis.slivers {
        output.push_str(&format!(
            "- Sliver triangle #{}: area {:.6}, aspect ratio {:.3}\n",
            item.triangle, item.area, item.aspect_ratio
        ));
    }
    for item in &analysis.micro_triangles {
        output.push_str(&format!(
            "- Micro triangle #{}: area {:.6}, relative area {:.3e}\n",
            item.triangle, item.area, item.relative_area
        ));
    }
    for item in &analysis.large_triangles {
        output.push_str(&format!(
            "- Large triangle #{}: area {:.6}, relative area {:.3}\n",
            item.triangle, item.area, item.relative_area
        ));
    }
    for item in &analysis.high_incident_vertices {
        output.push_str(&format!(
            "- Vertex #{} at ({:.6}, {:.6}, {:.6}) has {} incident edges\n",
            item.vertex, item.position.x, item.position.y, item.position.z, item.incident_edges
        ));
    }
    output
}

pub fn generate_sample(
    config: &Config,
    kind: SampleKind,
    density: usize,
    binary: bool,
    name: &str,
) -> Result<PathBuf, StlError> {
    fs::create_dir_all(&config.stl_folder)?;
    let path = config.stl_folder.join(name);
    let triangles = generate(kind, density);
    let mut file = File::create(&path)?;
    if binary {
        write_binary(&mut file, name, &triangles)?;
    } else {
        write_ascii(&mut file, name, &triangles)?;
    }
    Ok(path)
}

pub fn benchmark(config: &Config, triangle_count: u32, name: &str) -> Result<String, StlError> {
    fs::create_dir_all(&config.stl_folder)?;
    let path = config.stl_folder.join(name);
    if !path.exists() || fs::metadata(&path)?.len() != 84 + triangle_count as u64 * 50 {
        let mut file = File::create(&path)?;
        write_benchmark_binary(&mut file, triangle_count)?;
    }
    let total = Instant::now();
    let parsing = Instant::now();
    let parsed = parse_file(&path)?;
    let parsing_time = parsing.elapsed();
    let deduplication = Instant::now();
    let mesh = Mesh::from_triangles(parsed.triangles);
    let deduplication_time = deduplication.elapsed();
    let analysis_start = Instant::now();
    let result = analyze(&mesh, config.thresholds);
    let analysis_time = analysis_start.elapsed();
    let estimated_memory = mesh.vertices.capacity() * std::mem::size_of::<crate::geometry::Vec3>()
        + mesh.triangles.capacity() * std::mem::size_of::<crate::geometry::IndexedTriangle>();
    Ok(format!("Benchmark file: {}\nTriangles: {}\nParsing: {:.3?}\nDeduplication: {:.3?}\nAnalysis: {:.3?}\nTotal: {:.3?}\nEstimated indexed mesh storage: {:.2} MiB\nWarnings: {}\n", path.display(), mesh.triangles.len(), parsing_time, deduplication_time, analysis_time, total.elapsed(), estimated_memory as f64 / 1_048_576.0, result.slivers.len() + result.micro_triangles.len() + result.large_triangles.len() + result.high_incident_vertices.len()))
}

pub fn run_command(
    args: &[String],
    input: &mut dyn BufRead,
    output: &mut dyn Write,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load_default()?;
    match args {
        [] => run_menu(input, output, &config)?,
        [command, shape, density, encoding, name] if command == "generate" => {
            let kind = SampleKind::parse(shape)
                .ok_or("shape must be cuboid, prism, box, torus, cylinder, cube, cone, sphere, or tetrahedron")?;
            let density = density.parse()?; let binary = match encoding.as_str() { "binary" => true, "ascii" => false, _ => return Err("encoding must be ascii or binary".into()) };
            writeln!(
                output,
                "Generated sample at {}",
                generate_sample(&config, kind, density, binary, name)?.display()
            )?;
        }
        [command, count, name] if command == "benchmark" => writeln!(output, "{}", benchmark(&config, count.parse()?, name)?)?,
        [command] if command == "benchmark" => writeln!(output, "{}", benchmark(&config, 1_000_000, "benchmark_1000000.stl")?)?,
        _ => writeln!(output, "Usage: stl_analyzer [generate <shape> <density> <ascii|binary> <name.stl> | benchmark [triangles] [name.stl]]")?,
    }
    Ok(())
}
