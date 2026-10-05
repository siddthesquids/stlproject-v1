use crate::geometry::{Mesh, Vec3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundingBox {
    pub min: Vec3,
    pub max: Vec3,
}

impl BoundingBox {
    pub fn from_vertices(vertices: &[Vec3]) -> Option<Self> {
        let first = *vertices.first()?;
        let mut bounds = Self {
            min: first,
            max: first,
        };
        for &vertex in &vertices[1..] {
            bounds.min.x = bounds.min.x.min(vertex.x);
            bounds.min.y = bounds.min.y.min(vertex.y);
            bounds.min.z = bounds.min.z.min(vertex.z);
            bounds.max.x = bounds.max.x.max(vertex.x);
            bounds.max.y = bounds.max.y.max(vertex.y);
            bounds.max.z = bounds.max.z.max(vertex.z);
        }
        Some(bounds)
    }

    pub fn diagonal_squared(self) -> f32 {
        let delta = self.max - self.min;
        delta.dot(delta)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Thresholds {
    pub sliver_aspect_ratio: f32,
    pub micro_triangle_area_ratio: f32,
    pub large_triangle_area_ratio: f32,
    pub max_incident_edges: usize,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            sliver_aspect_ratio: 20.0,
            micro_triangle_area_ratio: 1.0e-8,
            large_triangle_area_ratio: 0.25,
            max_incident_edges: 12,
        }
    }
}

#[derive(Debug)]
pub struct SliverTriangle {
    pub triangle: usize,
    pub area: f32,
    pub aspect_ratio: f32,
}
#[derive(Debug)]
pub struct LargeTriangle {
    pub triangle: usize,
    pub area: f32,
    pub relative_area: f32,
}
#[derive(Debug)]
pub struct MicroTriangle {
    pub triangle: usize,
    pub area: f32,
    pub relative_area: f32,
}
#[derive(Debug)]
pub struct HighIncidentVertex {
    pub vertex: usize,
    pub position: Vec3,
    pub incident_edges: usize,
}

#[derive(Debug)]
pub struct Analysis {
    pub bounds: Option<BoundingBox>,
    pub slivers: Vec<SliverTriangle>,
    pub micro_triangles: Vec<MicroTriangle>,
    pub large_triangles: Vec<LargeTriangle>,
    pub high_incident_vertices: Vec<HighIncidentVertex>,
}

pub fn analyze(mesh: &Mesh, thresholds: Thresholds) -> Analysis {
    let bounds = BoundingBox::from_vertices(&mesh.vertices);
    let model_area = bounds.map_or(0.0, BoundingBox::diagonal_squared);
    let mut slivers = Vec::new();
    let mut micro_triangles = Vec::new();
    let mut large_triangles = Vec::new();
    let mut edges = Vec::with_capacity(mesh.triangles.len().saturating_mul(3));

    for (index, indexed) in mesh.triangles.iter().enumerate() {
        let [a, b, c] = indexed.indices;
        let vertices = [mesh.vertices[a], mesh.vertices[b], mesh.vertices[c]];
        let area = 0.5
            * (vertices[1] - vertices[0])
                .cross(vertices[2] - vertices[0])
                .length();
        let longest = vertices[0]
            .distance(vertices[1])
            .max(vertices[1].distance(vertices[2]))
            .max(vertices[2].distance(vertices[0]));
        let aspect_ratio = if area > 0.0 {
            longest * longest / (2.0 * area)
        } else {
            f32::INFINITY
        };
        if aspect_ratio >= thresholds.sliver_aspect_ratio {
            slivers.push(SliverTriangle {
                triangle: index,
                area,
                aspect_ratio,
            });
        }
        if area == 0.0
            || (model_area > 0.0 && area / model_area <= thresholds.micro_triangle_area_ratio)
        {
            micro_triangles.push(MicroTriangle {
                triangle: index,
                area,
                relative_area: if model_area > 0.0 {
                    area / model_area
                } else {
                    0.0
                },
            });
        }
        if model_area > 0.0 && area / model_area >= thresholds.large_triangle_area_ratio {
            large_triangles.push(LargeTriangle {
                triangle: index,
                area,
                relative_area: area / model_area,
            });
        }
        for (start, end) in [(a, b), (b, c), (c, a)] {
            if start != end {
                edges.push((start.min(end), start.max(end)));
            }
        }
    }

    // Sorting compact edge pairs avoids a hash set per vertex while counting unique neighbors.
    edges.sort_unstable();
    edges.dedup();
    let mut degrees = vec![0_usize; mesh.vertices.len()];
    for (a, b) in edges {
        degrees[a] += 1;
        degrees[b] += 1;
    }
    let high_incident_vertices = degrees
        .into_iter()
        .enumerate()
        .filter_map(|(vertex, incident_edges)| {
            (incident_edges > thresholds.max_incident_edges).then(|| HighIncidentVertex {
                vertex,
                position: mesh.vertices[vertex],
                incident_edges,
            })
        })
        .collect();
    Analysis {
        bounds,
        slivers,
        micro_triangles,
        large_triangles,
        high_incident_vertices,
    }
}
