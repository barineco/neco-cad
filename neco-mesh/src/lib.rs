#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

extern crate alloc;

mod surface;

pub use surface::{
    validate_closed_oriented, validate_surface_mesh, AllowedSharedIndexIntersectionKind,
    ClosedOrientedSurfaceMesh, ClosureWitness, ManifoldWitness, MeshFailure,
    MeshPairIntersectionKind, MeshSelfIntersectionWitness, MeshSource, MeshTrianglePairWitness,
    OrientationWitness, SurfaceMesh, SurfaceMeshValidationWitness, TriangleMesh2d,
    ValidatedSurfaceMesh,
};
