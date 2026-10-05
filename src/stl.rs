use crate::geometry::{Triangle, Vec3};
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StlFormat {
    Ascii,
    Binary,
}

impl fmt::Display for StlFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Ascii => "ASCII",
                Self::Binary => "binary",
            }
        )
    }
}

#[derive(Debug)]
pub enum StlError {
    Io(io::Error),
    Invalid(String),
}

impl fmt::Display for StlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::Invalid(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for StlError {}
impl From<io::Error> for StlError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

#[derive(Debug)]
pub struct ParsedStl {
    pub format: StlFormat,
    pub triangles: Vec<Triangle>,
}

pub fn parse_file(path: &Path) -> Result<ParsedStl, StlError> {
    parse_bytes(&fs::read(path)?)
}

pub fn parse_bytes(bytes: &[u8]) -> Result<ParsedStl, StlError> {
    let format = detect_format(bytes)?;
    let triangles = match format {
        StlFormat::Ascii => parse_ascii(bytes)?,
        StlFormat::Binary => parse_binary(bytes)?,
    };
    Ok(ParsedStl { format, triangles })
}

pub fn detect_format(bytes: &[u8]) -> Result<StlFormat, StlError> {
    if bytes.len() >= 84 {
        let count = u32::from_le_bytes(bytes[80..84].try_into().unwrap()) as usize;
        if count.checked_mul(50).and_then(|size| size.checked_add(84)) == Some(bytes.len()) {
            return Ok(StlFormat::Binary);
        }
    }
    if std::str::from_utf8(bytes).is_ok() {
        Ok(StlFormat::Ascii)
    } else {
        Err(StlError::Invalid(
            "not a structurally valid binary STL or UTF-8 ASCII STL".into(),
        ))
    }
}

fn parse_binary(bytes: &[u8]) -> Result<Vec<Triangle>, StlError> {
    if bytes.len() < 84 {
        return Err(StlError::Invalid(
            "binary STL is shorter than its header".into(),
        ));
    }
    let count = u32::from_le_bytes(bytes[80..84].try_into().unwrap()) as usize;
    let expected = count
        .checked_mul(50)
        .and_then(|size| size.checked_add(84))
        .ok_or_else(|| StlError::Invalid("binary triangle count overflows file size".into()))?;
    if expected != bytes.len() {
        return Err(StlError::Invalid(
            "binary STL length does not match triangle count".into(),
        ));
    }
    let mut triangles = Vec::with_capacity(count);
    for number in 0..count {
        let offset = 84 + number * 50;
        let normal = read_vec3(&bytes[offset..offset + 12])?;
        let vertices = [
            read_vec3(&bytes[offset + 12..offset + 24])?,
            read_vec3(&bytes[offset + 24..offset + 36])?,
            read_vec3(&bytes[offset + 36..offset + 48])?,
        ];
        validate_triangle(vertices, number)?;
        triangles.push(Triangle { normal, vertices });
    }
    Ok(triangles)
}

fn read_vec3(bytes: &[u8]) -> Result<Vec3, StlError> {
    if bytes.len() != 12 {
        return Err(StlError::Invalid("truncated binary vector".into()));
    }
    let value = Vec3::new(
        f32::from_le_bytes(bytes[0..4].try_into().unwrap()),
        f32::from_le_bytes(bytes[4..8].try_into().unwrap()),
        f32::from_le_bytes(bytes[8..12].try_into().unwrap()),
    );
    if value.is_finite() {
        Ok(value)
    } else {
        Err(StlError::Invalid(
            "STL contains a non-finite coordinate".into(),
        ))
    }
}

fn parse_ascii(bytes: &[u8]) -> Result<Vec<Triangle>, StlError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| StlError::Invalid("ASCII STL is not UTF-8".into()))?;
    let mut lines = text.lines().map(str::trim).filter(|line| !line.is_empty());
    let first = lines
        .next()
        .ok_or_else(|| StlError::Invalid("ASCII STL is empty".into()))?;
    if !first.starts_with("solid") {
        return Err(StlError::Invalid(
            "ASCII STL must start with 'solid'".into(),
        ));
    }
    let mut triangles = Vec::new();
    loop {
        let Some(line) = lines.next() else {
            return Err(StlError::Invalid("ASCII STL is missing 'endsolid'".into()));
        };
        if line.starts_with("endsolid") {
            break;
        }
        let parts: Vec<_> = line.split_whitespace().collect();
        if parts.len() != 5 || parts[0] != "facet" || parts[1] != "normal" {
            return Err(StlError::Invalid(format!(
                "expected 'facet normal' near triangle {}",
                triangles.len()
            )));
        }
        let normal = parse_vec3(&parts[2..], triangles.len())?;
        expect_line(&mut lines, "outer loop", triangles.len())?;
        let mut vertices = [Vec3::ZERO; 3];
        for vertex in &mut vertices {
            let line = lines
                .next()
                .ok_or_else(|| StlError::Invalid("unexpected end while reading vertices".into()))?;
            let parts: Vec<_> = line.split_whitespace().collect();
            if parts.len() != 4 || parts[0] != "vertex" {
                return Err(StlError::Invalid("expected 'vertex x y z'".into()));
            }
            *vertex = parse_vec3(&parts[1..], triangles.len())?;
        }
        expect_line(&mut lines, "endloop", triangles.len())?;
        expect_line(&mut lines, "endfacet", triangles.len())?;
        validate_triangle(vertices, triangles.len())?;
        triangles.push(Triangle { normal, vertices });
    }
    Ok(triangles)
}

fn parse_vec3(parts: &[&str], triangle: usize) -> Result<Vec3, StlError> {
    let parse = |value: &str| {
        value
            .parse::<f32>()
            .map_err(|_| StlError::Invalid(format!("invalid number near triangle {triangle}")))
    };
    let value = Vec3::new(parse(parts[0])?, parse(parts[1])?, parse(parts[2])?);
    if value.is_finite() {
        Ok(value)
    } else {
        Err(StlError::Invalid(
            "STL contains a non-finite coordinate".into(),
        ))
    }
}

fn expect_line<'a>(
    lines: &mut impl Iterator<Item = &'a str>,
    expected: &str,
    triangle: usize,
) -> Result<(), StlError> {
    match lines.next() {
        Some(line) if line == expected => Ok(()),
        _ => Err(StlError::Invalid(format!(
            "expected '{expected}' near triangle {triangle}"
        ))),
    }
}

fn validate_triangle(vertices: [Vec3; 3], number: usize) -> Result<(), StlError> {
    if vertices.into_iter().all(Vec3::is_finite) {
        Ok(())
    } else {
        Err(StlError::Invalid(format!(
            "triangle {number} has non-finite vertices"
        )))
    }
}

pub fn write_ascii<W: Write>(
    writer: &mut W,
    name: &str,
    triangles: &[Triangle],
) -> Result<(), StlError> {
    writeln!(writer, "solid {name}")?;
    for triangle in triangles {
        writeln!(
            writer,
            "  facet normal {} {} {}",
            triangle.normal.x, triangle.normal.y, triangle.normal.z
        )?;
        writeln!(writer, "    outer loop")?;
        for vertex in triangle.vertices {
            writeln!(
                writer,
                "      vertex {} {} {}",
                vertex.x, vertex.y, vertex.z
            )?;
        }
        writeln!(writer, "    endloop\n  endfacet")?;
    }
    writeln!(writer, "endsolid {name}")?;
    Ok(())
}

pub fn write_binary<W: Write>(
    writer: &mut W,
    name: &str,
    triangles: &[Triangle],
) -> Result<(), StlError> {
    let mut header = [0_u8; 80];
    let label = name.as_bytes();
    header[..label.len().min(80)].copy_from_slice(&label[..label.len().min(80)]);
    writer.write_all(&header)?;
    writer.write_all(&(triangles.len() as u32).to_le_bytes())?;
    for triangle in triangles {
        write_binary_triangle(writer, triangle)?;
    }
    Ok(())
}

pub fn write_binary_triangle<W: Write>(
    writer: &mut W,
    triangle: &Triangle,
) -> Result<(), StlError> {
    for vector in [
        triangle.normal,
        triangle.vertices[0],
        triangle.vertices[1],
        triangle.vertices[2],
    ] {
        for value in [vector.x, vector.y, vector.z] {
            writer.write_all(&value.to_le_bytes())?;
        }
    }
    writer.write_all(&0_u16.to_le_bytes())?;
    Ok(())
}
