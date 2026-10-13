#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

extern crate alloc;

mod materialize;
mod types;

pub use materialize::{
    evaluate_face_crossing_replays, evaluate_shell_crossing, extract_face_crossing_replay,
    materialize_analytic_brep, prove_face_crossing_replays, validate_closed_oriented_shell,
    FaceCrossingReplay, FaceSeamKind, ShellCrossingRequest,
};
pub use neco_arrangement2d::construct_full_domain_arrangement;
pub use types::{
    AnalyticBrep, ArrangementId, BrepAssignments, BrepCorrespondence, BrepFailure,
    BrepMaterializationRequest, BrepOrientation, BrepWitnesses, ChartArrangement,
    ClosedOrientedShell, CurveRepresentation, CurveRepresentationId, Edge, EdgeAssignment, Face,
    FaceAssignment, FaceOrientation, Incidence, OwnedParameterSpan, Shell, ShellOrientation,
    ShellTopologyWitness, SourceRevision, SurfaceRepresentation, SurfaceRepresentationId, Trim,
    TrimPCurve, Vertex, VertexAssignment,
};
