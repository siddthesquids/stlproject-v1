use std::env;
use std::fs::{self, File};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use stl_analyzer::app::Config;
use stl_analyzer::samples::{generate_with_dimensions, SampleKind, ShapeDimensions};
use stl_analyzer::stl::{write_ascii, write_binary};

fn main() -> ExitCode {
    let arguments: Vec<_> = env::args().skip(1).collect();
    let result = if arguments.is_empty() {
        let stdin = io::stdin();
        let mut input = stdin.lock();
        let mut output = io::stdout().lock();
        run_interactive(&mut input, &mut output)
    } else {
        run(arguments)
    };
    match result {
        Ok(message) => {
            println!("{message}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("Error: {message}\n\n{}", usage());
            ExitCode::from(2)
        }
    }
}

fn run(args: Vec<String>) -> Result<String, String> {
    if args
        .first()
        .is_some_and(|value| value == "--help" || value == "-h")
    {
        return Ok(usage().to_owned());
    }
    let shape = args
        .first()
        .and_then(|value| SampleKind::parse(value))
        .ok_or(
            "first argument must be cuboid, cube, cylinder, cone, sphere, torus, or tetrahedron",
        )?;
    let config = Config::load_default().map_err(|error| error.to_string())?;
    let mut dimensions = ShapeDimensions::defaults(shape);
    let mut density = 24_usize;
    let mut binary = true;
    let mut output: Option<PathBuf> = None;
    let mut index = 1;
    while index < args.len() {
        let option = &args[index];
        let value = |name: &str| {
            args.get(index + 1)
                .ok_or_else(|| format!("{name} needs a value"))
        };
        match option.as_str() {
            "--output" => {
                output = Some(PathBuf::from(value("--output")?));
                index += 2;
            }
            "--density" => {
                density = value("--density")?
                    .parse()
                    .map_err(|_| "density must be a positive integer")?;
                if density == 0 {
                    return Err("density must be a positive integer".into());
                }
                index += 2;
            }
            "--format" => {
                binary = match value("--format")? {
                    value if value.eq_ignore_ascii_case("binary") => true,
                    value if value.eq_ignore_ascii_case("ascii") => false,
                    _ => return Err("format must be ascii or binary".into()),
                };
                index += 2;
            }
            "--size" => {
                let size = number(value("--size")?, "size")?;
                dimensions.width = size;
                dimensions.height = size;
                dimensions.depth = size;
                index += 2;
            }
            "--width" => {
                dimensions.width = number(value("--width")?, "width")?;
                index += 2;
            }
            "--height" => {
                dimensions.height = number(value("--height")?, "height")?;
                index += 2;
            }
            "--depth" => {
                dimensions.depth = number(value("--depth")?, "depth")?;
                index += 2;
            }
            "--radius" => {
                dimensions.radius = number(value("--radius")?, "radius")?;
                index += 2;
            }
            "--major-radius" => {
                dimensions.major_radius = number(value("--major-radius")?, "major radius")?;
                index += 2;
            }
            "--minor-radius" => {
                dimensions.minor_radius = number(value("--minor-radius")?, "minor radius")?;
                index += 2;
            }
            _ => return Err(format!("unknown option: {option}")),
        }
    }
    let file_name = format!("{}.stl", args[0]);
    let path = output.unwrap_or_else(|| config.stl_folder.join(file_name));
    write_model(shape, density, dimensions, binary, path)
}

fn run_interactive(input: &mut dyn BufRead, output: &mut dyn Write) -> Result<String, String> {
    writeln!(
        output,
        "STL Generator\n1. Cube\n2. Cuboid\n3. Cylinder\n4. Cone\n5. Sphere\n6. Torus\n7. Tetrahedron"
    )
        .map_err(|error| error.to_string())?;
    let shape = match prompt_text(input, output, "Choose a shape", None)?.as_str() {
        "1" | "cube" => SampleKind::Cube,
        "2" | "cuboid" | "box" => SampleKind::Prism,
        "3" | "cylinder" => SampleKind::Cylinder,
        "4" | "cone" => SampleKind::Cone,
        "5" | "sphere" => SampleKind::Sphere,
        "6" | "torus" => SampleKind::Torus,
        "7" | "tetrahedron" | "tetra" => SampleKind::Tetrahedron,
        _ => return Err("choose a number from 1 to 7 or enter a listed shape name".into()),
    };
    let mut dimensions = ShapeDimensions::defaults(shape);
    match shape {
        SampleKind::Cube => {
            dimensions.width = prompt_number(input, output, "Cube size", dimensions.width)?
        }
        SampleKind::Prism => {
            dimensions.width = prompt_number(input, output, "Cuboid width", dimensions.width)?;
            dimensions.height = prompt_number(input, output, "Cuboid height", dimensions.height)?;
            dimensions.depth = prompt_number(input, output, "Cuboid depth", dimensions.depth)?;
        }
        SampleKind::Cylinder => {
            dimensions.radius = prompt_number(input, output, "Cylinder radius", dimensions.radius)?;
            dimensions.height = prompt_number(input, output, "Cylinder height", dimensions.height)?;
        }
        SampleKind::Torus => {
            dimensions.major_radius =
                prompt_number(input, output, "Torus major radius", dimensions.major_radius)?;
            dimensions.minor_radius =
                prompt_number(input, output, "Torus minor radius", dimensions.minor_radius)?;
        }
        SampleKind::Cone => {
            dimensions.radius = prompt_number(input, output, "Cone radius", dimensions.radius)?;
            dimensions.height = prompt_number(input, output, "Cone height", dimensions.height)?;
        }
        SampleKind::Sphere => {
            dimensions.radius = prompt_number(input, output, "Sphere radius", dimensions.radius)?;
        }
        SampleKind::Tetrahedron => {
            dimensions.width =
                prompt_number(input, output, "Tetrahedron edge size", dimensions.width)?;
        }
    }
    let density = prompt_usize(input, output, "Triangle density", 24)?;
    let format = prompt_text(input, output, "Format: binary or ascii", Some("binary"))?;
    let binary = match format.to_ascii_lowercase().as_str() {
        "binary" => true,
        "ascii" => false,
        _ => return Err("format must be binary or ascii".into()),
    };
    let name = prompt_text(
        input,
        output,
        "Output file name",
        Some(&format!("{}.stl", shape_name(shape))),
    )?;
    let config = Config::load_default().map_err(|error| error.to_string())?;
    write_model(
        shape,
        density,
        dimensions,
        binary,
        config.stl_folder.join(name),
    )
}

fn prompt_text(
    input: &mut dyn BufRead,
    output: &mut dyn Write,
    label: &str,
    default: Option<&str>,
) -> Result<String, String> {
    match default {
        Some(value) => write!(output, "{label} [{value}]: "),
        None => write!(output, "{label}: "),
    }
    .map_err(|error| error.to_string())?;
    output.flush().map_err(|error| error.to_string())?;
    let mut value = String::new();
    if input
        .read_line(&mut value)
        .map_err(|error| error.to_string())?
        == 0
    {
        return Err("input closed".into());
    }
    let value = value.trim();
    if value.is_empty() {
        default
            .map(str::to_owned)
            .ok_or_else(|| "a value is required".into())
    } else {
        Ok(value.to_owned())
    }
}

fn prompt_number(
    input: &mut dyn BufRead,
    output: &mut dyn Write,
    label: &str,
    default: f32,
) -> Result<f32, String> {
    number(
        &prompt_text(input, output, label, Some(&default.to_string()))?,
        label,
    )
}

fn prompt_usize(
    input: &mut dyn BufRead,
    output: &mut dyn Write,
    label: &str,
    default: usize,
) -> Result<usize, String> {
    let value = prompt_text(input, output, label, Some(&default.to_string()))?;
    let value = value
        .parse::<usize>()
        .map_err(|_| format!("{label} must be a positive integer"))?;
    if value == 0 {
        Err(format!("{label} must be a positive integer"))
    } else {
        Ok(value)
    }
}

fn shape_name(shape: SampleKind) -> &'static str {
    match shape {
        SampleKind::Cube => "cube",
        SampleKind::Prism => "cuboid",
        SampleKind::Cylinder => "cylinder",
        SampleKind::Cone => "cone",
        SampleKind::Sphere => "sphere",
        SampleKind::Torus => "torus",
        SampleKind::Tetrahedron => "tetrahedron",
    }
}

fn write_model(
    shape: SampleKind,
    density: usize,
    dimensions: ShapeDimensions,
    binary: bool,
    path: PathBuf,
) -> Result<String, String> {
    let triangles = generate_with_dimensions(shape, density, dimensions)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let mut file = File::create(&path).map_err(|error| error.to_string())?;
    if binary {
        write_binary(&mut file, shape_name(shape), &triangles)
    } else {
        write_ascii(&mut file, shape_name(shape), &triangles)
    }
    .map_err(|error| error.to_string())?;
    Ok(format!(
        "Generated {} {} triangles at {}",
        triangles.len(),
        if binary { "binary" } else { "ASCII" },
        path.display()
    ))
}

fn number(value: &str, name: &str) -> Result<f32, String> {
    value
        .parse()
        .map_err(|_| format!("{name} must be a number"))
}

fn usage() -> &'static str {
    "Run without arguments for the interactive seven-shape generator. Press Enter at a dimension prompt to use its default.\n\nCommand mode: stl-generator <shape> [options]\nDefaults: density 24, binary output, configured stl_folder/<shape>.stl\nShapes and dimensions:\n  cuboid   --width 2 --height 1 --depth 1 (prism is an alias)\n  cube     --size 1\n  cylinder --radius 1 --height 2\n  cone     --radius 1 --height 2\n  sphere   --radius 1\n  torus    --major-radius 1 --minor-radius 0.35\n  tetrahedron --size 1 (regular tetrahedron edge length)\n\nCommon options: --density N --format ascii|binary --output PATH"
}
