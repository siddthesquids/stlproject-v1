use std::fs::{self, File};
use std::io::Cursor;
use stl_analyzer::analysis::Thresholds;
use stl_analyzer::app::{run_menu, Config};
use stl_analyzer::geometry::{Mesh, Triangle, Vec3};
use stl_analyzer::samples::{generate, generate_with_dimensions, SampleKind, ShapeDimensions};
use stl_analyzer::stl::{parse_bytes, write_ascii, write_binary, StlFormat};

fn one_triangle() -> Triangle {
    Triangle::new([
        Vec3::ZERO,
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ])
}

#[test]
fn deduplicates_vertices_and_keeps_triangle_indices() {
    let a = Vec3::new(0.0, 0.0, 0.0);
    let b = Vec3::new(1.0, 0.0, 0.0);
    let c = Vec3::new(0.0, 1.0, 0.0);
    let d = Vec3::new(1.0, 1.0, 0.0);
    let mesh = Mesh::from_triangles(vec![Triangle::new([a, b, c]), Triangle::new([b, d, c])]);
    assert_eq!(mesh.vertices.len(), 4);
    assert_eq!(mesh.triangles.len(), 2);
    assert_eq!(mesh.triangles[0].indices, [0, 1, 2]);
    assert_eq!(mesh.triangles[1].indices, [1, 3, 2]);
}

#[test]
fn parses_ascii_and_binary_stl() {
    let mut ascii = Vec::new();
    write_ascii(&mut ascii, "test", &[one_triangle()]).unwrap();
    let parsed_ascii = parse_bytes(&ascii).unwrap();
    assert_eq!(parsed_ascii.format, StlFormat::Ascii);
    assert_eq!(parsed_ascii.triangles.len(), 1);

    let mut binary = Vec::new();
    write_binary(&mut binary, "solid but binary", &[one_triangle()]).unwrap();
    let parsed_binary = parse_bytes(&binary).unwrap();
    assert_eq!(parsed_binary.format, StlFormat::Binary);
    assert_eq!(parsed_binary.triangles.len(), 1);
}

#[test]
fn rejects_malformed_stl() {
    assert!(parse_bytes(b"solid broken\nfacet normal 0 0 1\nendsolid").is_err());
    assert!(parse_bytes(&[0xff, 0xfe, 0xfd]).is_err());
}

#[test]
fn config_reads_custom_thresholds() {
    let root = std::env::temp_dir().join(format!("stl-analyzer-config-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let path = root.join("config.json");
    fs::write(
        &path,
        r#"{"stl_folder":"models","max_incident_edges":7,"micro_triangle_area_ratio":0.000001}"#,
    )
    .unwrap();
    let config = Config::load_from(&path).unwrap();
    assert_eq!(config.stl_folder, root.join("models"));
    assert_eq!(config.thresholds.max_incident_edges, 7);
    assert_eq!(config.thresholds.micro_triangle_area_ratio, 0.000001);
}

#[test]
fn menu_analyzes_ascii_and_binary_files_then_exits() {
    let folder = std::env::temp_dir().join(format!("stl-analyzer-menu-{}", std::process::id()));
    fs::create_dir_all(&folder).unwrap();
    let triangle = one_triangle();
    write_ascii(
        &mut File::create(folder.join("a_ascii.stl")).unwrap(),
        "ascii",
        &[triangle],
    )
    .unwrap();
    write_binary(
        &mut File::create(folder.join("b_binary.stl")).unwrap(),
        "binary",
        &[triangle],
    )
    .unwrap();
    let config = Config {
        stl_folder: folder,
        thresholds: Thresholds::default(),
    };
    let mut input = Cursor::new(b"1\n1\n1\n2\n2\n");
    let mut output = Vec::new();
    run_menu(&mut input, &mut output, &config).unwrap();
    let output = String::from_utf8(output).unwrap();
    assert!(output.contains("Format: ASCII"));
    assert!(output.contains("Format: binary"));
    assert!(output.contains("Goodbye."));
}

#[test]
fn generates_each_sample_and_cube_dimensions() {
    for kind in [
        SampleKind::Prism,
        SampleKind::Torus,
        SampleKind::Cylinder,
        SampleKind::Cube,
        SampleKind::Cone,
        SampleKind::Sphere,
        SampleKind::Tetrahedron,
    ] {
        assert!(!generate(kind, 4).is_empty());
    }
    assert_eq!(SampleKind::parse("cuboid"), Some(SampleKind::Prism));
    assert_eq!(
        SampleKind::parse("tetrahedron"),
        Some(SampleKind::Tetrahedron)
    );
    assert!(generate(SampleKind::Cube, 3).len() > generate(SampleKind::Cube, 1).len());

    let triangles = generate_with_dimensions(
        SampleKind::Cube,
        1,
        ShapeDimensions {
            width: 6.0,
            ..ShapeDimensions::defaults(SampleKind::Cube)
        },
    )
    .unwrap();
    let min_x = triangles
        .iter()
        .flat_map(|triangle| triangle.vertices)
        .map(|vertex| vertex.x)
        .fold(f32::INFINITY, f32::min);
    let max_x = triangles
        .iter()
        .flat_map(|triangle| triangle.vertices)
        .map(|vertex| vertex.x)
        .fold(f32::NEG_INFINITY, f32::max);
    assert_eq!((min_x, max_x), (-3.0, 3.0));
}

#[test]
fn generates_regular_tetrahedron_with_subdivided_faces() {
    let triangles = generate_with_dimensions(
        SampleKind::Tetrahedron,
        3,
        ShapeDimensions {
            width: 2.0,
            ..ShapeDimensions::defaults(SampleKind::Tetrahedron)
        },
    )
    .unwrap();
    assert_eq!(triangles.len(), 4 * 3 * 3);
    for triangle in triangles {
        let center =
            (triangle.vertices[0] + triangle.vertices[1] + triangle.vertices[2]) * (1.0 / 3.0);
        assert!(triangle.normal.dot(center) > 0.0);
    }
}

#[test]
fn cuboid_alias_generates_requested_dimensions() {
    let dimensions = ShapeDimensions {
        width: 6.0,
        height: 4.0,
        depth: 2.0,
        ..ShapeDimensions::defaults(SampleKind::Prism)
    };
    let triangles =
        generate_with_dimensions(SampleKind::parse("cuboid").unwrap(), 1, dimensions).unwrap();
    assert_eq!(triangles.len(), 12);
    let mesh = Mesh::from_triangles(triangles);
    let bounds = stl_analyzer::analysis::BoundingBox::from_vertices(&mesh.vertices).unwrap();
    assert_eq!(bounds.min, Vec3::new(-3.0, -2.0, -1.0));
    assert_eq!(bounds.max, Vec3::new(3.0, 2.0, 1.0));
}
