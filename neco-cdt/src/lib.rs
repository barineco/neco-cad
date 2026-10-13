#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

extern crate alloc;

use alloc::vec::Vec;

pub use neco_mesh::TriangleMesh2d;

#[derive(Clone, Debug, PartialEq)]
pub struct CdtRequest {
    outer_loop: Vec<[f64; 2]>,
}

impl CdtRequest {
    pub fn new(outer_loop: Vec<[f64; 2]>) -> Result<Self, CdtFailure> {
        if outer_loop.len() < 3 {
            return Err(CdtFailure::TooFewVertices {
                vertex_count: outer_loop.len(),
            });
        }
        if !outer_loop
            .iter()
            .all(|vertex| vertex.iter().all(|coordinate| coordinate.is_finite()))
        {
            return Err(CdtFailure::NonFiniteVertex);
        }
        if signed_double_area(&outer_loop) == 0.0 {
            return Err(CdtFailure::DegenerateConstraint);
        }
        Ok(Self { outer_loop })
    }

    pub fn outer_loop(&self) -> &[[f64; 2]] {
        &self.outer_loop
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CdtFailure {
    TooFewVertices { vertex_count: usize },
    NonFiniteVertex,
    DegenerateConstraint,
    Mesh(neco_mesh::MeshFailure),
}

pub fn triangulate(request: CdtRequest) -> Result<TriangleMesh2d, CdtFailure> {
    let vertices = request.outer_loop;
    let mut triangles = Vec::new();
    for index in 1..vertices.len() - 1 {
        triangles.push([0, index, index + 1]);
    }
    TriangleMesh2d::new(vertices, triangles).map_err(CdtFailure::Mesh)
}

fn signed_double_area(vertices: &[[f64; 2]]) -> f64 {
    vertices
        .iter()
        .zip(vertices.iter().cycle().skip(1))
        .take(vertices.len())
        .map(|(left, right)| left[0] * right[1] - left[1] * right[0])
        .sum()
}
