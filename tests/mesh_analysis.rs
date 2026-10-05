use std::fs::{self, File};
use std::time::{SystemTime, UNIX_EPOCH};
use stl_analyzer::analysis::{analyze, BoundingBox, Thresholds};
use stl_analyzer::app::analyze_path;
use stl_analyzer::geometry::{Mesh, Triangle, Vec3};
use stl_analyzer::stl::write_ascii;

fn mesh(triangles: Vec<[Vec3; 3]>) -> Mesh {
    Mesh::from_triangles(triangles.into_iter().map(Triangle::new).collect())
}

#[test]
fn reports_counts_and_bounds() {
    let mesh = mesh(vec![[
        Vec3::new(-1.0, 2.0, 3.0),
        Vec3::new(4.0, -2.0, 8.0),
        Vec3::new(0.0, 1.0, -4.0),
    ]]);
    assert_eq!(mesh.triangles.len(), 1);
    assert_eq!(mesh.vertices.len(), 3);
    let bounds = BoundingBox::from_vertices(&mesh.vertices).unwrap();
    assert_eq!(bounds.min, Vec3::new(-1.0, -2.0, -4.0));
    assert_eq!(bounds.max, Vec3::new(4.0, 2.0, 8.0));
}

#[test]
fn finds_sliver_and_large_triangles() {
    let mesh = mesh(vec![
        [
            Vec3::ZERO,
            Vec3::new(10.0, 0.0, 0.0),
            Vec3::new(5.0, 0.001, 0.0),
        ],
        [
            Vec3::ZERO,
            Vec3::new(10.0, 0.0, 0.0),
            Vec3::new(0.0, 10.0, 0.0),
        ],
        [
            Vec3::new(0.0, 0.0, 10.0),
            Vec3::new(1.0, 0.0, 10.0),
            Vec3::new(0.0, 1.0, 10.0),
        ],
    ]);
    let thresholds = Thresholds {
        large_triangle_area_ratio: 0.1,
        ..Thresholds::default()
    };
    let report = analyze(&mesh, thresholds);
    assert_eq!(report.slivers.len(), 1);
    assert_eq!(report.large_triangles.len(), 1);
}

#[test]
fn reports_exact_unique_vertices_and_incident_edges() {
    let a = Vec3::new(0.0, 0.0, 0.0);
    let b = Vec3::new(1.0, 0.0, 0.0);
    let c = Vec3::new(0.0, 1.0, 0.0);
    let mesh = mesh(vec![[a, b, c], [a, b, c], [a, a, b]]);
    let report = analyze(
        &mesh,
        Thresholds {
            max_incident_edges: 1,
            ..Thresholds::default()
        },
    );
    assert_eq!(mesh.vertices.len(), 3);
    assert!(report
        .high_incident_vertices
        .iter()
        .any(|vertex| vertex.position == a && vertex.incident_edges == 2));
}

#[test]
fn finds_micro_triangles_relative_to_model_size() {
    let mesh = mesh(vec![
        [
            Vec3::ZERO,
            Vec3::new(10.0, 0.0, 0.0),
            Vec3::new(0.0, 10.0, 0.0),
        ],
        [
            Vec3::new(0.0, 0.0, 10.0),
            Vec3::new(0.01, 0.0, 10.0),
            Vec3::new(0.0, 0.01, 10.0),
        ],
    ]);
    let report = analyze(
        &mesh,
        Thresholds {
            micro_triangle_area_ratio: 1.0e-6,
            ..Thresholds::default()
        },
    );
    assert_eq!(report.micro_triangles.len(), 1);
    assert_eq!(report.micro_triangles[0].triangle, 1);
}

#[test]
fn treats_zero_area_triangles_as_micro_even_for_zero_size_meshes() {
    let mesh = mesh(vec![[Vec3::ZERO, Vec3::ZERO, Vec3::ZERO]]);
    let report = analyze(&mesh, Thresholds::default());
    assert_eq!(report.micro_triangles.len(), 1);
    assert_eq!(report.micro_triangles[0].relative_area, 0.0);
}

#[test]
fn report_includes_micro_triangle_findings() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("stl-micro-triangle-{stamp}.stl"));
    let triangles = [
        Triangle::new([
            Vec3::ZERO,
            Vec3::new(10.0, 0.0, 0.0),
            Vec3::new(0.0, 10.0, 0.0),
        ]),
        Triangle::new([
            Vec3::new(0.0, 0.0, 10.0),
            Vec3::new(0.01, 0.0, 10.0),
            Vec3::new(0.0, 0.01, 10.0),
        ]),
    ];
    write_ascii(&mut File::create(&path).unwrap(), "micro", &triangles).unwrap();
    let report = analyze_path(
        &path,
        Thresholds {
            micro_triangle_area_ratio: 1.0e-6,
            ..Thresholds::default()
        },
    )
    .unwrap();
    fs::remove_file(path).unwrap();
    assert!(report.contains("Micro triangle #1"));
}
