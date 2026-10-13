#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use neco_mesh::{MeshFailure, SurfaceMesh, ValidatedSurfaceMesh};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StlEncoding {
    Ascii,
    Binary,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StlFailure {
    InvalidHeader,
    TruncatedBinary {
        expected_length: usize,
        actual_length: usize,
    },
    InvalidTriangleCount,
    InvalidAscii,
    EmptyMesh,
    Mesh(MeshFailure),
}

pub fn decode(bytes: &[u8]) -> Result<SurfaceMesh, StlFailure> {
    if bytes.starts_with(b"solid") {
        decode_ascii(bytes)
    } else {
        decode_binary(bytes)
    }
}

pub fn encode(mesh: &ValidatedSurfaceMesh, encoding: StlEncoding) -> Result<Vec<u8>, StlFailure> {
    match encoding {
        StlEncoding::Ascii => encode_ascii(mesh),
        StlEncoding::Binary => encode_binary(mesh),
    }
}

fn decode_binary(bytes: &[u8]) -> Result<SurfaceMesh, StlFailure> {
    if bytes.len() < 84 {
        return Err(StlFailure::InvalidHeader);
    }
    let triangle_count = u32::from_le_bytes([bytes[80], bytes[81], bytes[82], bytes[83]]) as usize;
    let body_length = triangle_count
        .checked_mul(50)
        .ok_or(StlFailure::InvalidTriangleCount)?;
    let expected_length = 84usize
        .checked_add(body_length)
        .ok_or(StlFailure::InvalidTriangleCount)?;
    if bytes.len() != expected_length {
        return Err(StlFailure::TruncatedBinary {
            expected_length,
            actual_length: bytes.len(),
        });
    }
    let mut vertices = Vec::with_capacity(triangle_count * 3);
    let mut triangles = Vec::with_capacity(triangle_count);
    for triangle_index in 0..triangle_count {
        let base = 84 + triangle_index * 50;
        let mut triangle = [0; 3];
        for (vertex_offset, slot) in triangle.iter_mut().enumerate() {
            let coordinate_offset = base + 12 + vertex_offset * 12;
            let vertex = [
                f32_at(bytes, coordinate_offset) as f64,
                f32_at(bytes, coordinate_offset + 4) as f64,
                f32_at(bytes, coordinate_offset + 8) as f64,
            ];
            *slot = vertices.len();
            vertices.push(vertex);
        }
        triangles.push(triangle);
    }
    if triangles.is_empty() {
        return Err(StlFailure::EmptyMesh);
    }
    SurfaceMesh::new(vertices, triangles, Vec::new()).map_err(StlFailure::Mesh)
}

fn decode_ascii(bytes: &[u8]) -> Result<SurfaceMesh, StlFailure> {
    let text = core::str::from_utf8(bytes).map_err(|_| StlFailure::InvalidAscii)?;
    let mut vertices = Vec::new();
    let mut triangles = Vec::new();
    let mut facet_vertices = Vec::new();
    for line in text.lines() {
        let mut terms = line.split_whitespace();
        if terms.next() == Some("vertex") {
            let vertex = [
                parse_coordinate(terms.next())?,
                parse_coordinate(terms.next())?,
                parse_coordinate(terms.next())?,
            ];
            if terms.next().is_some() {
                return Err(StlFailure::InvalidAscii);
            }
            facet_vertices.push(vertex);
        }
        if line.trim() == "endfacet" {
            let values: [[f64; 3]; 3] = facet_vertices
                .as_slice()
                .try_into()
                .map_err(|_| StlFailure::InvalidAscii)?;
            let start = vertices.len();
            vertices.extend(values);
            triangles.push([start, start + 1, start + 2]);
            facet_vertices.clear();
        }
    }
    if !facet_vertices.is_empty() || triangles.is_empty() {
        return Err(StlFailure::InvalidAscii);
    }
    SurfaceMesh::new(vertices, triangles, Vec::new()).map_err(StlFailure::Mesh)
}

fn encode_binary(mesh: &ValidatedSurfaceMesh) -> Result<Vec<u8>, StlFailure> {
    let triangle_count = u32::try_from(mesh.mesh().triangles().len())
        .map_err(|_| StlFailure::InvalidTriangleCount)?;
    let mut bytes = vec![0; 80];
    bytes.extend(triangle_count.to_le_bytes());
    for triangle in mesh.mesh().triangles() {
        bytes.extend([0; 12]);
        for vertex_index in triangle {
            for coordinate in mesh.mesh().vertices()[*vertex_index] {
                bytes.extend((coordinate as f32).to_le_bytes());
            }
        }
        bytes.extend([0; 2]);
    }
    Ok(bytes)
}

fn encode_ascii(mesh: &ValidatedSurfaceMesh) -> Result<Vec<u8>, StlFailure> {
    let mut text = String::from("solid neco-mesh\n");
    for triangle in mesh.mesh().triangles() {
        text.push_str("  facet normal 0 0 0\n    outer loop\n");
        for vertex_index in triangle {
            let [x, y, z] = mesh.mesh().vertices()[*vertex_index];
            text.push_str(&format!("      vertex {x} {y} {z}\n"));
        }
        text.push_str("    endloop\n  endfacet\n");
    }
    text.push_str("endsolid neco-mesh\n");
    Ok(text.into_bytes())
}

fn f32_at(bytes: &[u8], offset: usize) -> f32 {
    f32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

fn parse_coordinate(value: Option<&str>) -> Result<f64, StlFailure> {
    value
        .ok_or(StlFailure::InvalidAscii)?
        .parse()
        .map_err(|_| StlFailure::InvalidAscii)
}
