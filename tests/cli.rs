use std::fs;
use std::io::Write;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use stl_analyzer::stl::{parse_file, StlFormat};

#[test]
fn cli_generates_a_sample() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("stl-analyzer-{stamp}"));
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("config.json"), r#"{"stl_folder":"models"}"#).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_stl-analyzer"))
        .current_dir(&root)
        .args(["generate", "cube", "1", "ascii", "cube.stl"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Generated sample"));
    assert!(root.join("models").join("cube.stl").exists());
}

#[test]
fn cli_shows_menu_without_a_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_stl-analyzer"))
        .stdin(std::process::Stdio::piped())
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("STL Analyzer"));
}

#[test]
fn interactive_generator_uses_cube_defaults_for_blank_dimensions() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("stl-generator-{stamp}"));
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("config.json"), r#"{"stl_folder":"models"}"#).unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_stl-generator"))
        .current_dir(&root)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"1\n\n\n\n\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();

    assert!(output.status.success());
    assert!(String::from_utf8(output.stdout)
        .unwrap()
        .contains("Generated 6912 binary triangles"));
    let parsed = parse_file(&root.join("models").join("cube.stl")).unwrap();
    assert_eq!(parsed.format, StlFormat::Binary);
    assert_eq!(parsed.triangles.len(), 6912);
}

#[test]
fn generator_cli_creates_tetrahedron_and_cuboid_stls() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("stl-generator-shapes-{stamp}"));
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("config.json"), r#"{"stl_folder":"models"}"#).unwrap();

    for (shape, args, expected_triangles) in [
        (
            "tetrahedron",
            vec!["tetrahedron", "--size", "2", "--density", "2"],
            16,
        ),
        (
            "cuboid",
            vec![
                "cuboid",
                "--width",
                "6",
                "--height",
                "4",
                "--depth",
                "2",
                "--density",
                "1",
            ],
            12,
        ),
        (
            "sphere",
            vec!["sphere", "--radius", "2", "--density", "4"],
            48,
        ),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_stl-generator"))
            .current_dir(&root)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let parsed = parse_file(&root.join("models").join(format!("{shape}.stl"))).unwrap();
        assert_eq!(parsed.format, StlFormat::Binary);
        assert_eq!(parsed.triangles.len(), expected_triangles);
    }
}
