use stl_analyzer::geometry::{Triangle, Vec3};
use stl_analyzer::stl::{parse_bytes, write_ascii, write_binary, StlFormat};

fn triangle() -> Triangle {
    Triangle::new([
        Vec3::ZERO,
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ])
}

#[test]
fn loads_ascii_stl() {
    let mut bytes = Vec::new();
    write_ascii(&mut bytes, "test", &[triangle()]).unwrap();
    let parsed = parse_bytes(&bytes).unwrap();
    assert_eq!(parsed.format, StlFormat::Ascii);
    assert_eq!(parsed.triangles[0].area(), 0.5);
}

#[test]
fn loads_binary_stl() {
    let mut bytes = Vec::new();
    write_binary(&mut bytes, "solid-looking binary", &[triangle()]).unwrap();
    let parsed = parse_bytes(&bytes).unwrap();
    assert_eq!(parsed.format, StlFormat::Binary);
    assert_eq!(parsed.triangles[0].area(), 0.5);
}

#[test]
fn rejects_incomplete_ascii_triangle() {
    let result = parse_bytes(b"solid bad\nvertex 0 0 0\nendsolid bad");
    assert!(result.is_err());
}

#[test]
fn rejects_non_finite_binary_coordinates() {
    let mut bytes = vec![0; 84 + 50];
    bytes[80..84].copy_from_slice(&1u32.to_le_bytes());
    bytes[84 + 12..84 + 16].copy_from_slice(&f32::NAN.to_le_bytes());

    let error = parse_bytes(&bytes).unwrap_err();
    assert!(error.to_string().contains("non-finite coordinate"));
}
