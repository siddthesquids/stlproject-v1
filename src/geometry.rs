use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0);

    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn cross(self, other: Self) -> Self {
        Self::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    pub fn length(self) -> f32 {
        self.dot(self).sqrt()
    }

    pub fn normalized(self) -> Self {
        let length = self.length();
        if length > 0.0 {
            self * (1.0 / length)
        } else {
            Self::ZERO
        }
    }

    pub fn distance(self, other: Self) -> f32 {
        (self - other).length()
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }
}

impl std::ops::Add for Vec3 {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }
}

impl std::ops::Sub for Vec3 {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }
}

impl std::ops::Mul<f32> for Vec3 {
    type Output = Self;
    fn mul(self, scalar: f32) -> Self {
        Self::new(self.x * scalar, self.y * scalar, self.z * scalar)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Triangle {
    pub normal: Vec3,
    pub vertices: [Vec3; 3],
}

impl Triangle {
    pub fn new(vertices: [Vec3; 3]) -> Self {
        let mut triangle = Self {
            normal: Vec3::ZERO,
            vertices,
        };
        triangle.recalculate_normal();
        triangle
    }

    pub fn recalculate_normal(&mut self) {
        self.normal = (self.vertices[1] - self.vertices[0])
            .cross(self.vertices[2] - self.vertices[0])
            .normalized();
    }

    pub fn area(&self) -> f32 {
        0.5 * (self.vertices[1] - self.vertices[0])
            .cross(self.vertices[2] - self.vertices[0])
            .length()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexedTriangle {
    pub indices: [usize; 3],
}

#[derive(Debug, Clone)]
pub struct Mesh {
    pub vertices: Vec<Vec3>,
    pub triangles: Vec<IndexedTriangle>,
}

#[derive(Hash, PartialEq, Eq)]
struct VertexKey([u32; 3]);

impl VertexKey {
    fn from_vertex(vertex: Vec3) -> Self {
        fn bits(value: f32) -> u32 {
            if value == 0.0 {
                0
            } else {
                value.to_bits()
            }
        }
        Self([bits(vertex.x), bits(vertex.y), bits(vertex.z)])
    }
}

impl Mesh {
    pub fn from_triangles(triangles: Vec<Triangle>) -> Self {
        let mut vertices = Vec::new();
        let mut lookup = HashMap::with_capacity(triangles.len().saturating_mul(2));
        let mut indexed = Vec::with_capacity(triangles.len());

        for triangle in triangles {
            let mut indices = [0; 3];
            for (slot, vertex) in triangle.vertices.into_iter().enumerate() {
                let key = VertexKey::from_vertex(vertex);
                let next = vertices.len();
                indices[slot] = *lookup.entry(key).or_insert_with(|| {
                    vertices.push(vertex);
                    next
                });
            }
            indexed.push(IndexedTriangle { indices });
        }
        Self {
            vertices,
            triangles: indexed,
        }
    }

    pub fn triangle(&self, index: usize) -> Option<Triangle> {
        let indices = self.triangles.get(index)?.indices;
        Some(Triangle::new([
            *self.vertices.get(indices[0])?,
            *self.vertices.get(indices[1])?,
            *self.vertices.get(indices[2])?,
        ]))
    }
}
