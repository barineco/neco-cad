#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]

extern crate alloc;

mod construction;
mod types;

pub use construction::{construct_arrangement, construct_full_domain_arrangement};
pub use types::{
    Arrangement, ArrangementConstructionRequest, ArrangementCurve, ArrangementFailure,
    ArrangementIntersectionBinding, ClosedWalk, CurveEventLocator, CurveEventLocatorKey,
    CurveIdentity, CurveParameterAssociation, DirectedIncidence, DirectedIncidenceDirection,
    Endpoint, EndpointRole, EventRotation, Face, FiniteCell, Hole, Overlap, OverlapOrientation,
    PairedSpanAssociation, ParameterSpan, PlanarTopologyDeclaration, PlanarVertexDeclaration,
    PlanarWalkRole, PlanarWinding, RepresentationCategory, Segment, SpanEndpointCorrespondence,
    SpanSide, WalkClassification,
};

pub use neco_geometry::IntersectionComponentIdentity;
