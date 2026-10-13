#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

extern crate alloc;

mod adaptive_tessellation;
mod analytic_brep;
mod boundary;
mod construction;
mod construction_types;
mod numeric_projection;
mod provenance;
mod types;

pub use adaptive_tessellation::{
    plane_surface_covering, tessellate_adaptive, unit_sphere_stereographic_covering,
    AdaptiveTessellation, AdaptiveTessellationProvenance, AdaptiveTessellationRequest, ChartPatch,
    ChordBoundArcRange, ChordBoundEdgeEvaluation, ChordBoundEdgeShape, ChordBoundEndpoint,
    ChordBoundTriangleWitness, DiskConstraint, FaceChordBoundWitness, FaceExactSurface,
    SurfaceCovering, DEFAULT_MAX_SUBDIVISION_DEPTH,
};
pub use analytic_brep::{derive_analytic_brep, DerivedAnalyticBrep};
pub use boundary::derive_complete_boundary;
pub use construction::{classify_region_membership, construct, region_polynomial_from_exact_terms};
pub use construction_types::{
    BooleanKind, BooleanOperator, BooleanRequest, CadConstructionFailure,
    CadConstructionLowerFailure, ConstructionOperation, ConstructionOutput, ConstructionRequest,
    ConstructionResult, ConstructionRoot, CurveDeclaration, CurveOrSurfaceRepresentation,
    ExactConversionFailure, ExactConversionFailureKind, ExactPoint3, ExactRigidPlacement,
    ExactScalar, ExactVector3, ExplicitSolidFace, ExplicitSolidVertex, ExternalGeometryRequest,
    GeometryAccuracyPolicy, GeometryClosure, GeometryOutputCategory, GeometryOutputExpectation,
    InputRole, LoftMode, LoftSection, LoweringCondition, NurbsSurfaceOrientation,
    NurbsSurfacePatchRequest, NurbsSurfaceTrim, NurbsTrimDirection, NurbsTrimRole,
    OperationOutputIdentity, OperationOutputRef, OperationRequest, OutputSlot,
    ParameterValueSample, PrimitiveDeclaration, ProfileCorner, ProfileRequest, ProfileSegment,
    ProfileSegmentCurve, RevolveExtent, SubentityRef, SweepFrameRule,
};
pub use neco_brep::{
    AnalyticBrep, BrepAssignments, BrepFailure, BrepMaterializationRequest, ChartArrangement,
    CurveRepresentation, CurveRepresentationId, EdgeAssignment, FaceAssignment, SourceRevision,
    SurfaceRepresentation, SurfaceRepresentationId, VertexAssignment,
};
pub use neco_cad_import::{
    region_from_mesh, CertifiedRegion, RegionImportFailure, RegionImportRequest,
};
pub use neco_calculus::{RationalSurfacePolynomials, SurfacePolynomial, SurfaceTerm};
pub use neco_geometry::{
    AlgebraicRegion, BoundaryDerivationRequest, BoundaryLocalEvidence, BoundaryStratification,
    BoundaryStratumDimension, CanonicalAtom, ChartDisplayName, ChartDomainId, Enclosure,
    GeometryFailure, MembershipClassification, NormalizedRegionFormula, PairedSurfaceSeam,
    ParameterInterval, ParametricSurfaceChart2, Point2, Point3, RationalFunction3,
    RationalFunctionDomain3, RayCrossingPolicy, RegionAffineMap3, RegionConstructionRequest,
    RegionFunction, RegionKind, RegionMembershipRequest, RegionPoint, RegionPolynomial3,
    RegionPolynomialTerm, RootIsolationPolicy, RootRefinementMethod, SeamEnd, SignRelation,
    SurfaceChartIdentity, SurfaceOrientation, SurfaceParameterDomain, SurfaceParameterPoint,
    SurfaceTrimLoop2, TrimClassification, TrimLoopRole, TrimmedSurfaceDeclaration,
};
pub use neco_mesh::{ClosedOrientedSurfaceMesh, MeshFailure, ValidatedSurfaceMesh};
pub use neco_tessellation::{
    project_real_algebraic, tessellate, tessellate_view_elements, AdaptivePrecision,
    AdaptivePrecisionWitness, BrepElementCorrespondence, ProjectedCoordinate, TessellatedFace,
    TessellatedViewElements, TessellationConsumer, TessellationRequest, TessellationResult,
};
pub use numeric_projection::{
    project_numeric_coordinates, NumericProjection, NumericProjectionProvenance,
    NumericProjectionRequest, NumericSample, ProjectedPoint, DEFAULT_NUMERIC_PROJECTION_BITS,
};
pub use provenance::{
    BodyComposition, ConstructionDependency, ConstructionGraph, ConstructionGraphFailure,
    ConstructionNode, EntityIdentityBinding, IdentityFailure, IdentityLedger, OperationIdentity,
    OperationIdentityBinding, OperationProvenance, Provenance, SourceIdentity,
};
pub use types::{
    AdaptiveTessellationCondition, BoundaryDerivationCondition, BrepDerivationCondition,
    CadFailure, NumericProjectionCondition, SetBooleanRequest, SolidBooleanRequest,
};

pub fn construct_region(request: RegionConstructionRequest) -> Result<AlgebraicRegion, CadFailure> {
    Ok(neco_geometry::construct_region_types(request)?.into_region())
}

pub fn set_boolean(request: SetBooleanRequest) -> Result<AlgebraicRegion, CadFailure> {
    request
        .left
        .set_boolean(&request.right, request.formula)
        .map_err(Into::into)
}

pub fn solid_boolean(request: SolidBooleanRequest) -> Result<AlgebraicRegion, CadFailure> {
    request
        .left
        .solid_boolean(&request.right, request.formula)
        .map_err(Into::into)
}

pub fn derive_boundary(
    request: BoundaryDerivationRequest,
) -> Result<BoundaryStratification, CadFailure> {
    Ok(neco_geometry::derive_boundary(request)?.into_parts().1)
}

pub fn materialize_brep(request: BrepMaterializationRequest) -> Result<AnalyticBrep, CadFailure> {
    neco_brep::materialize_analytic_brep(request).map_err(Into::into)
}
