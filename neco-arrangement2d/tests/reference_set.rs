use neco_arrangement2d::{
    construct_arrangement, Arrangement, ArrangementConstructionRequest, ArrangementCurve,
    ArrangementFailure, ArrangementIntersectionBinding, ClosedWalk, CurveEventLocator,
    CurveEventLocatorKey, CurveIdentity, CurveParameterAssociation, DirectedIncidence, Endpoint,
    EndpointRole, EventRotation, Face, FiniteCell, Hole, IntersectionComponentIdentity, Overlap,
    OverlapOrientation, PairedSpanAssociation, ParameterSpan, PlanarTopologyDeclaration,
    PlanarVertexDeclaration, PlanarWalkRole, PlanarWinding, RepresentationCategory, Segment,
    SpanEndpointCorrespondence, WalkClassification,
};

#[test]
fn crate_root_exposes_the_declared_arrangement_api() {
    let _: fn(ArrangementConstructionRequest) -> Result<Arrangement, ArrangementFailure> =
        construct_arrangement;
    let _: Option<ArrangementCurve> = None;
    let _: Option<ArrangementIntersectionBinding> = None;
    let _: Option<CurveEventLocator> = None;
    let _: Option<CurveEventLocatorKey> = None;
    let _: Option<CurveIdentity> = None;
    let _: Option<CurveParameterAssociation> = None;
    let _: Option<DirectedIncidence> = None;
    let _: Option<Endpoint> = None;
    let _: Option<EndpointRole> = None;
    let _: Option<EventRotation> = None;
    let _: Option<Face> = None;
    let _: Option<FiniteCell> = None;
    let _: Option<Hole> = None;
    let _: Option<IntersectionComponentIdentity> = None;
    let _: Option<Overlap> = None;
    let _: Option<OverlapOrientation> = None;
    let _: Option<PairedSpanAssociation> = None;
    let _: Option<ParameterSpan> = None;
    let _: Option<PlanarTopologyDeclaration> = None;
    let _: Option<PlanarVertexDeclaration> = None;
    let _: Option<PlanarWalkRole> = None;
    let _: Option<PlanarWinding> = None;
    let _: Option<RepresentationCategory> = None;
    let _: Option<Segment> = None;
    let _: Option<SpanEndpointCorrespondence> = None;
    let _: Option<ClosedWalk> = None;
    let _: Option<WalkClassification> = None;
}
