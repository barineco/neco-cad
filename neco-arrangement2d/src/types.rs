use alloc::string::String;
use alloc::vec::Vec;
use core::cmp::Ordering;

use neco_algnum::RealAlgebraic;
use neco_calculus::CalculusFailure;
use neco_geometry::{
    ChartDefinition2, GeometryFailure, IntersectionComponentIdentity, IntersectionSet,
    IntersectionUndeterminedReason, ParameterInterval,
};

fn require_text(value: &str, failure: ArrangementFailure) -> Result<(), ArrangementFailure> {
    if value.is_empty() {
        Err(failure)
    } else {
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct CurveIdentity {
    curve_id: String,
    stable_key: String,
}
impl CurveIdentity {
    pub fn new(curve_id: String, stable_key: String) -> Result<Self, ArrangementFailure> {
        require_text(
            &curve_id,
            ArrangementFailure::InvalidCurveIdentity {
                curve_id: curve_id.clone(),
            },
        )?;
        require_text(
            &stable_key,
            ArrangementFailure::InvalidCurveIdentity {
                curve_id: curve_id.clone(),
            },
        )?;
        Ok(Self {
            curve_id,
            stable_key,
        })
    }
    pub fn curve_id(&self) -> &str {
        &self.curve_id
    }
    pub fn stable_key(&self) -> &str {
        &self.stable_key
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RepresentationCategory {
    PolynomialBezier2D,
    RationalBezier2D,
    PolynomialNurbs2D,
    RationalNurbs2D,
}

#[derive(Debug, Eq, PartialEq)]
pub struct ArrangementCurve {
    identity: CurveIdentity,
    representation_category: RepresentationCategory,
    chart: ChartDefinition2,
    parameter_interval: ParameterInterval,
}
impl ArrangementCurve {
    pub fn new(
        identity: CurveIdentity,
        representation_category: RepresentationCategory,
        chart: ChartDefinition2,
        parameter_interval: ParameterInterval,
    ) -> Result<Self, ArrangementFailure> {
        Ok(Self {
            identity,
            representation_category,
            chart,
            parameter_interval,
        })
    }
    pub fn identity(&self) -> &CurveIdentity {
        &self.identity
    }
    pub fn representation_category(&self) -> RepresentationCategory {
        self.representation_category
    }
    pub fn chart(&self) -> &ChartDefinition2 {
        &self.chart
    }
    pub fn parameter_interval(&self) -> &ParameterInterval {
        &self.parameter_interval
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EndpointRole {
    Point,
    Lower,
    Upper,
    OverlapLower,
    OverlapUpper,
}

#[derive(Debug, Eq, PartialEq)]
pub struct CurveParameterAssociation {
    curve_id: String,
    intersection_id: String,
    component_identity: IntersectionComponentIdentity,
    parameter: RealAlgebraic,
    endpoint_role: Option<EndpointRole>,
}
impl CurveParameterAssociation {
    pub fn new(
        curve_id: String,
        intersection_id: String,
        component_identity: IntersectionComponentIdentity,
        parameter: RealAlgebraic,
        endpoint_role: Option<EndpointRole>,
    ) -> Result<Self, ArrangementFailure> {
        require_text(
            &curve_id,
            ArrangementFailure::UnknownPointAssociation {
                component_id: intersection_id.clone(),
            },
        )?;
        require_text(
            &intersection_id,
            ArrangementFailure::UnknownPointAssociation {
                component_id: intersection_id.clone(),
            },
        )?;
        Ok(Self {
            curve_id,
            intersection_id,
            component_identity,
            parameter,
            endpoint_role,
        })
    }
    pub fn curve_id(&self) -> &str {
        &self.curve_id
    }
    pub fn intersection_id(&self) -> &str {
        &self.intersection_id
    }
    pub fn component_identity(&self) -> IntersectionComponentIdentity {
        self.component_identity
    }
    pub fn parameter(&self) -> &RealAlgebraic {
        &self.parameter
    }
    pub fn endpoint_role(&self) -> Option<EndpointRole> {
        self.endpoint_role
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpanEndpointCorrespondence {
    LowerToLower,
    LowerToUpper,
}

#[derive(Debug, Eq, PartialEq)]
pub struct SpanSide {
    pub curve_id: String,
    pub lower: CurveParameterAssociation,
    pub upper: CurveParameterAssociation,
}

#[derive(Debug, Eq, PartialEq)]
pub struct PairedSpanAssociation {
    span_id: String,
    left_curve_id: String,
    right_curve_id: String,
    intersection_id: String,
    component_identity: IntersectionComponentIdentity,
    left_lower: CurveParameterAssociation,
    left_upper: CurveParameterAssociation,
    right_lower: CurveParameterAssociation,
    right_upper: CurveParameterAssociation,
    correspondence: SpanEndpointCorrespondence,
}
impl PairedSpanAssociation {
    pub fn new(
        span_id: String,
        intersection_id: String,
        component_identity: IntersectionComponentIdentity,
        left: SpanSide,
        right: SpanSide,
        correspondence: SpanEndpointCorrespondence,
    ) -> Result<Self, ArrangementFailure> {
        require_text(
            &span_id,
            ArrangementFailure::IncompleteSpanAssociation {
                span_id: span_id.clone(),
            },
        )?;
        for value in [&left.curve_id, &right.curve_id, &intersection_id] {
            require_text(
                value,
                ArrangementFailure::UnknownSpanAssociation {
                    span_id: span_id.clone(),
                },
            )?;
        }
        for association in [&left.lower, &left.upper, &right.lower, &right.upper] {
            if association.intersection_id() != intersection_id
                || association.component_identity() != component_identity
            {
                return Err(ArrangementFailure::AssociationComponentIdentityMismatch {
                    component_id: intersection_id.clone(),
                });
            }
        }
        Ok(Self {
            span_id,
            left_curve_id: left.curve_id,
            right_curve_id: right.curve_id,
            intersection_id,
            component_identity,
            left_lower: left.lower,
            left_upper: left.upper,
            right_lower: right.lower,
            right_upper: right.upper,
            correspondence,
        })
    }
    pub fn span_id(&self) -> &str {
        &self.span_id
    }
    pub fn left_curve_id(&self) -> &str {
        &self.left_curve_id
    }
    pub fn right_curve_id(&self) -> &str {
        &self.right_curve_id
    }
    pub fn intersection_id(&self) -> &str {
        &self.intersection_id
    }
    pub fn component_identity(&self) -> IntersectionComponentIdentity {
        self.component_identity
    }
    pub fn left_lower(&self) -> &CurveParameterAssociation {
        &self.left_lower
    }
    pub fn left_upper(&self) -> &CurveParameterAssociation {
        &self.left_upper
    }
    pub fn right_lower(&self) -> &CurveParameterAssociation {
        &self.right_lower
    }
    pub fn right_upper(&self) -> &CurveParameterAssociation {
        &self.right_upper
    }
    pub fn correspondence(&self) -> SpanEndpointCorrespondence {
        self.correspondence
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct ArrangementIntersectionBinding {
    left_curve_id: String,
    right_curve_id: String,
    intersection: IntersectionSet,
    point_associations: Vec<CurveParameterAssociation>,
    span_associations: Vec<PairedSpanAssociation>,
}
impl ArrangementIntersectionBinding {
    pub fn new(
        left_curve_id: String,
        right_curve_id: String,
        intersection: IntersectionSet,
        point_associations: Vec<CurveParameterAssociation>,
        span_associations: Vec<PairedSpanAssociation>,
    ) -> Result<Self, ArrangementFailure> {
        require_text(
            &left_curve_id,
            ArrangementFailure::UnknownIntersectionCurve {
                curve_id: left_curve_id.clone(),
            },
        )?;
        require_text(
            &right_curve_id,
            ArrangementFailure::UnknownIntersectionCurve {
                curve_id: right_curve_id.clone(),
            },
        )?;
        Ok(Self {
            left_curve_id,
            right_curve_id,
            intersection,
            point_associations,
            span_associations,
        })
    }
    pub fn left_curve_id(&self) -> &str {
        &self.left_curve_id
    }
    pub fn right_curve_id(&self) -> &str {
        &self.right_curve_id
    }
    pub fn intersection(&self) -> &IntersectionSet {
        &self.intersection
    }
    pub fn point_associations(&self) -> &[CurveParameterAssociation] {
        &self.point_associations
    }
    pub fn span_associations(&self) -> &[PairedSpanAssociation] {
        &self.span_associations
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Endpoint {
    Lower,
    Upper,
}
#[derive(Debug, Eq, PartialEq)]
pub enum CurveEventLocatorKey {
    Endpoint {
        curve_id: String,
        endpoint: Endpoint,
    },
    PointIntersection {
        curve_id: String,
        intersection_id: String,
        component_identity: IntersectionComponentIdentity,
        parameter: RealAlgebraic,
    },
}

#[derive(Debug, Eq, PartialEq)]
pub enum CurveEventLocator {
    Endpoint {
        curve_id: String,
        endpoint: Endpoint,
    },
    PointIntersection {
        curve_id: String,
        intersection_id: String,
        component_identity: IntersectionComponentIdentity,
        parameter: RealAlgebraic,
    },
}
impl CurveEventLocator {
    pub fn endpoint(curve_id: String, endpoint: Endpoint) -> Result<Self, ArrangementFailure> {
        require_text(
            &curve_id,
            ArrangementFailure::UnknownVertexEventLocator {
                vertex_key: String::new(),
                locator: Self::Endpoint {
                    curve_id: String::new(),
                    endpoint,
                },
            },
        )?;
        Ok(Self::Endpoint { curve_id, endpoint })
    }
    pub fn point_intersection(
        curve_id: String,
        intersection_id: String,
        component_identity: IntersectionComponentIdentity,
        parameter: RealAlgebraic,
    ) -> Result<Self, ArrangementFailure> {
        require_text(
            &curve_id,
            ArrangementFailure::UnknownPointAssociation {
                component_id: intersection_id.clone(),
            },
        )?;
        require_text(
            &intersection_id,
            ArrangementFailure::UnknownPointAssociation {
                component_id: intersection_id.clone(),
            },
        )?;
        Ok(Self::PointIntersection {
            curve_id,
            intersection_id,
            component_identity,
            parameter,
        })
    }
    pub fn curve_id(&self) -> &str {
        match self {
            Self::Endpoint { curve_id, .. } | Self::PointIntersection { curve_id, .. } => curve_id,
        }
    }
    pub fn endpoint_role(&self) -> Option<&Endpoint> {
        match self {
            Self::Endpoint { endpoint, .. } => Some(endpoint),
            _ => None,
        }
    }
    pub fn try_clone(&self) -> Result<Self, ArrangementFailure> {
        match self {
            Self::Endpoint { curve_id, endpoint } => Self::endpoint(curve_id.clone(), *endpoint),
            Self::PointIntersection {
                curve_id,
                intersection_id,
                component_identity,
                parameter,
            } => Self::point_intersection(
                curve_id.clone(),
                intersection_id.clone(),
                *component_identity,
                parameter.try_clone()?,
            ),
        }
    }
    pub fn compare(&self, other: &Self) -> Result<Ordering, ArrangementFailure> {
        let order = self.curve_id().cmp(other.curve_id());
        if order != Ordering::Equal {
            return Ok(order);
        }
        match (self, other) {
            (
                Self::Endpoint { endpoint: left, .. },
                Self::Endpoint {
                    endpoint: right, ..
                },
            ) => Ok(left.cmp(right)),
            (
                Self::PointIntersection {
                    intersection_id: left_intersection,
                    component_identity: left_identity,
                    parameter: left_parameter,
                    ..
                },
                Self::PointIntersection {
                    intersection_id: right_intersection,
                    component_identity: right_identity,
                    parameter: right_parameter,
                    ..
                },
            ) => {
                let order = left_intersection.cmp(right_intersection);
                if order != Ordering::Equal {
                    return Ok(order);
                }
                let order = left_identity.cmp(right_identity);
                if order != Ordering::Equal {
                    return Ok(order);
                }
                Ok(left_parameter.compare(right_parameter)?)
            }
            (Self::Endpoint { .. }, Self::PointIntersection { .. }) => Ok(Ordering::Less),
            (Self::PointIntersection { .. }, Self::Endpoint { .. }) => Ok(Ordering::Greater),
        }
    }
    pub fn canonical_key(&self) -> Result<CurveEventLocatorKey, ArrangementFailure> {
        match self {
            Self::Endpoint { curve_id, endpoint } => Ok(CurveEventLocatorKey::Endpoint {
                curve_id: curve_id.clone(),
                endpoint: *endpoint,
            }),
            Self::PointIntersection {
                curve_id,
                intersection_id,
                component_identity,
                parameter,
            } => Ok(CurveEventLocatorKey::PointIntersection {
                curve_id: curve_id.clone(),
                intersection_id: intersection_id.clone(),
                component_identity: *component_identity,
                parameter: parameter.try_clone()?,
            }),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct PlanarVertexDeclaration {
    vertex_key: String,
    incident_event_locators: Vec<CurveEventLocator>,
}
impl PlanarVertexDeclaration {
    pub fn new(
        vertex_key: String,
        incident_event_locators: Vec<CurveEventLocator>,
    ) -> Result<Self, ArrangementFailure> {
        if vertex_key.is_empty() || incident_event_locators.is_empty() {
            return Err(ArrangementFailure::EmptyPlanarVertex { vertex_key });
        }
        Ok(Self {
            vertex_key,
            incident_event_locators,
        })
    }
    pub fn isolated(vertex_key: String) -> Result<Self, ArrangementFailure> {
        if vertex_key.is_empty() {
            return Err(ArrangementFailure::EmptyPlanarVertex { vertex_key });
        }
        Ok(Self {
            vertex_key,
            incident_event_locators: Vec::new(),
        })
    }
    pub fn vertex_key(&self) -> &str {
        &self.vertex_key
    }
    pub fn incident_event_locators(&self) -> &[CurveEventLocator] {
        &self.incident_event_locators
    }
}
#[derive(Debug, Eq, PartialEq)]
pub struct EventRotation {
    vertex_key: String,
    outgoing_incidence_ids_counterclockwise: Vec<String>,
}
impl EventRotation {
    pub fn new(
        vertex_key: String,
        outgoing_incidence_ids_counterclockwise: Vec<String>,
    ) -> Result<Self, ArrangementFailure> {
        if vertex_key.is_empty()
            || outgoing_incidence_ids_counterclockwise.is_empty()
            || outgoing_incidence_ids_counterclockwise
                .iter()
                .any(String::is_empty)
        {
            return Err(ArrangementFailure::EmptyEventRotation { vertex_key });
        }
        Ok(Self {
            vertex_key,
            outgoing_incidence_ids_counterclockwise,
        })
    }
    pub fn vertex_key(&self) -> &str {
        &self.vertex_key
    }
    pub fn outgoing_incidence_ids_counterclockwise(&self) -> &[String] {
        &self.outgoing_incidence_ids_counterclockwise
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlanarWalkRole {
    Exterior,
    FiniteOuter,
    Hole,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlanarWinding {
    Clockwise,
    Counterclockwise,
}
#[derive(Debug, Eq, PartialEq)]
pub struct WalkClassification {
    walk_key: String,
    canonical_incidence_ids: Vec<String>,
    role: PlanarWalkRole,
    winding: Option<PlanarWinding>,
    containing_outer_walk_key: Option<String>,
}
impl WalkClassification {
    pub fn new(
        walk_key: String,
        canonical_incidence_ids: Vec<String>,
        role: PlanarWalkRole,
        winding: Option<PlanarWinding>,
        containing_outer_walk_key: Option<String>,
    ) -> Result<Self, ArrangementFailure> {
        if walk_key.is_empty()
            || canonical_incidence_ids.is_empty()
            || canonical_incidence_ids.iter().any(String::is_empty)
        {
            return Err(ArrangementFailure::EmptyWalkClassification { walk_key });
        }
        Ok(Self {
            walk_key,
            canonical_incidence_ids,
            role,
            winding,
            containing_outer_walk_key,
        })
    }
    pub fn walk_key(&self) -> &str {
        &self.walk_key
    }
    pub fn canonical_incidence_ids(&self) -> &[String] {
        &self.canonical_incidence_ids
    }
    pub fn role(&self) -> PlanarWalkRole {
        self.role
    }
    pub fn winding(&self) -> Option<PlanarWinding> {
        self.winding
    }
    pub fn containing_outer_walk_key(&self) -> Option<&str> {
        self.containing_outer_walk_key.as_deref()
    }
}
#[derive(Debug, Eq, PartialEq)]
pub struct PlanarTopologyDeclaration {
    vertices: Vec<PlanarVertexDeclaration>,
    event_rotations: Vec<EventRotation>,
    walk_classifications: Vec<WalkClassification>,
}
impl PlanarTopologyDeclaration {
    pub fn new(
        vertices: Vec<PlanarVertexDeclaration>,
        event_rotations: Vec<EventRotation>,
        walk_classifications: Vec<WalkClassification>,
    ) -> Result<Self, ArrangementFailure> {
        Ok(Self {
            vertices,
            event_rotations,
            walk_classifications,
        })
    }
    pub fn vertices(&self) -> &[PlanarVertexDeclaration] {
        &self.vertices
    }
    pub fn event_rotations(&self) -> &[EventRotation] {
        &self.event_rotations
    }
    pub fn walk_classifications(&self) -> &[WalkClassification] {
        &self.walk_classifications
    }
    pub(crate) fn into_vertices(self) -> Vec<PlanarVertexDeclaration> {
        self.vertices
    }
}
#[derive(Debug, Eq, PartialEq)]
pub struct ArrangementConstructionRequest {
    curves: Vec<ArrangementCurve>,
    intersections: Vec<ArrangementIntersectionBinding>,
    planar_topology: PlanarTopologyDeclaration,
}
impl ArrangementConstructionRequest {
    pub fn new(
        curves: Vec<ArrangementCurve>,
        intersections: Vec<ArrangementIntersectionBinding>,
        planar_topology: PlanarTopologyDeclaration,
    ) -> Result<Self, ArrangementFailure> {
        Ok(Self {
            curves,
            intersections,
            planar_topology,
        })
    }
    pub fn curves(&self) -> &[ArrangementCurve] {
        &self.curves
    }
    pub fn intersections(&self) -> &[ArrangementIntersectionBinding] {
        &self.intersections
    }
    pub fn planar_topology(&self) -> &PlanarTopologyDeclaration {
        &self.planar_topology
    }
    pub(crate) fn into_parts(
        self,
    ) -> (
        Vec<ArrangementCurve>,
        Vec<ArrangementIntersectionBinding>,
        PlanarTopologyDeclaration,
    ) {
        (self.curves, self.intersections, self.planar_topology)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct ParameterSpan {
    lower: RealAlgebraic,
    upper: RealAlgebraic,
    lower_closed: bool,
    upper_closed: bool,
}
impl ParameterSpan {
    pub fn new(
        lower: RealAlgebraic,
        upper: RealAlgebraic,
        lower_closed: bool,
        upper_closed: bool,
    ) -> Result<Self, ArrangementFailure> {
        ParameterInterval::new(
            lower.try_clone()?,
            upper.try_clone()?,
            lower_closed,
            upper_closed,
        )?;
        Ok(Self {
            lower,
            upper,
            lower_closed,
            upper_closed,
        })
    }
    pub fn lower(&self) -> &RealAlgebraic {
        &self.lower
    }
    pub fn upper(&self) -> &RealAlgebraic {
        &self.upper
    }
    pub fn lower_closed(&self) -> bool {
        self.lower_closed
    }
    pub fn upper_closed(&self) -> bool {
        self.upper_closed
    }
}
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct SplitEvent {
    pub(crate) curve_id: String,
    pub(crate) stable_key: String,
    pub(crate) parameter: RealAlgebraic,
    pub(crate) locator: CurveEventLocator,
    pub(crate) source_rank: u8,
}
#[derive(Debug, Eq, PartialEq)]
pub struct Segment {
    segment_id: String,
    curve_id: String,
    parameter_span: ParameterSpan,
    pub(crate) start_event: usize,
    pub(crate) end_event: usize,
}
impl Segment {
    pub(crate) fn new(
        segment_id: String,
        curve_id: String,
        parameter_span: ParameterSpan,
        start_event: usize,
        end_event: usize,
    ) -> Self {
        Self {
            segment_id,
            curve_id,
            parameter_span,
            start_event,
            end_event,
        }
    }
    pub fn segment_id(&self) -> &str {
        &self.segment_id
    }
    pub fn curve_id(&self) -> &str {
        &self.curve_id
    }
    pub fn parameter_span(&self) -> &ParameterSpan {
        &self.parameter_span
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OverlapOrientation {
    SameDirection,
    OppositeDirection,
}
#[derive(Debug, Eq, PartialEq)]
pub struct Overlap {
    pub(crate) overlap_id: String,
    pub(crate) left_curve_id: String,
    pub(crate) right_curve_id: String,
    pub(crate) left_span: ParameterSpan,
    pub(crate) right_span: ParameterSpan,
    pub(crate) orientation: OverlapOrientation,
}
impl Overlap {
    pub fn overlap_id(&self) -> &str {
        &self.overlap_id
    }
    pub fn left_curve_id(&self) -> &str {
        &self.left_curve_id
    }
    pub fn right_curve_id(&self) -> &str {
        &self.right_curve_id
    }
    pub fn left_span(&self) -> &ParameterSpan {
        &self.left_span
    }
    pub fn right_span(&self) -> &ParameterSpan {
        &self.right_span
    }
    pub fn orientation(&self) -> OverlapOrientation {
        self.orientation
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DirectedIncidenceDirection {
    Forward,
    Reverse,
}
#[derive(Debug, Eq, PartialEq)]
pub struct DirectedIncidence {
    incidence_id: String,
    segment_id: String,
    from_vertex_key: String,
    to_vertex_key: String,
    direction: DirectedIncidenceDirection,
}
impl DirectedIncidence {
    pub(crate) fn new(
        incidence_id: String,
        segment_id: String,
        from_vertex_key: String,
        to_vertex_key: String,
        direction: DirectedIncidenceDirection,
    ) -> Self {
        Self {
            incidence_id,
            segment_id,
            from_vertex_key,
            to_vertex_key,
            direction,
        }
    }
    pub fn incidence_id(&self) -> &str {
        &self.incidence_id
    }
    pub fn segment_id(&self) -> &str {
        &self.segment_id
    }
    pub fn from_vertex_key(&self) -> &str {
        &self.from_vertex_key
    }
    pub fn to_vertex_key(&self) -> &str {
        &self.to_vertex_key
    }
    pub fn direction(&self) -> DirectedIncidenceDirection {
        self.direction
    }
}
#[derive(Debug, Eq, PartialEq)]
pub struct ClosedWalk {
    walk_id: String,
    incidence_ids: Vec<String>,
    vertex_keys: Vec<String>,
    role: PlanarWalkRole,
    winding: Option<PlanarWinding>,
}
impl ClosedWalk {
    pub(crate) fn new(
        walk_id: String,
        incidence_ids: Vec<String>,
        vertex_keys: Vec<String>,
        role: PlanarWalkRole,
        winding: Option<PlanarWinding>,
    ) -> Self {
        Self {
            walk_id,
            incidence_ids,
            vertex_keys,
            role,
            winding,
        }
    }
    pub fn walk_id(&self) -> &str {
        &self.walk_id
    }
    pub fn incidence_ids(&self) -> &[String] {
        &self.incidence_ids
    }
    pub fn vertex_keys(&self) -> &[String] {
        &self.vertex_keys
    }
    pub fn role(&self) -> PlanarWalkRole {
        self.role
    }
    pub fn winding(&self) -> Option<PlanarWinding> {
        self.winding
    }
}
#[derive(Debug, Eq, PartialEq)]
pub struct Face {
    pub(crate) face_id: String,
    pub(crate) outer_walk_id: String,
    pub(crate) hole_ids: Vec<String>,
}
impl Face {
    pub fn face_id(&self) -> &str {
        &self.face_id
    }
    pub fn outer_walk_id(&self) -> &str {
        &self.outer_walk_id
    }
    pub fn hole_ids(&self) -> &[String] {
        &self.hole_ids
    }
}
#[derive(Debug, Eq, PartialEq)]
pub struct Hole {
    pub(crate) hole_id: String,
    pub(crate) walk_id: String,
    pub(crate) containing_face_id: String,
}
impl Hole {
    pub fn hole_id(&self) -> &str {
        &self.hole_id
    }
    pub fn walk_id(&self) -> &str {
        &self.walk_id
    }
    pub fn containing_face_id(&self) -> &str {
        &self.containing_face_id
    }
}
#[derive(Debug, Eq, PartialEq)]
pub struct FiniteCell {
    cell_id: String,
    face_ids: Vec<String>,
    hole_ids: Vec<String>,
}
impl FiniteCell {
    pub(crate) fn new(cell_id: String, face_ids: Vec<String>, hole_ids: Vec<String>) -> Self {
        Self {
            cell_id,
            face_ids,
            hole_ids,
        }
    }
    pub fn cell_id(&self) -> &str {
        &self.cell_id
    }
    pub fn face_ids(&self) -> &[String] {
        &self.face_ids
    }
    pub fn hole_ids(&self) -> &[String] {
        &self.hole_ids
    }
}
#[derive(Debug, Eq, PartialEq)]
pub struct Arrangement {
    pub(crate) vertices: Vec<PlanarVertexDeclaration>,
    pub(crate) curves: Vec<ArrangementCurve>,
    pub(crate) segments: Vec<Segment>,
    pub(crate) overlaps: Vec<Overlap>,
    pub(crate) directed_incidences: Vec<DirectedIncidence>,
    pub(crate) closed_walks: Vec<ClosedWalk>,
    pub(crate) faces: Vec<Face>,
    pub(crate) holes: Vec<Hole>,
    pub(crate) finite_cells: Vec<FiniteCell>,
}
impl Arrangement {
    pub fn vertices(&self) -> &[PlanarVertexDeclaration] {
        &self.vertices
    }
    pub fn curves(&self) -> &[ArrangementCurve] {
        &self.curves
    }
    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }
    pub fn overlaps(&self) -> &[Overlap] {
        &self.overlaps
    }
    pub fn directed_incidences(&self) -> &[DirectedIncidence] {
        &self.directed_incidences
    }
    pub fn closed_walks(&self) -> &[ClosedWalk] {
        &self.closed_walks
    }
    pub fn faces(&self) -> &[Face] {
        &self.faces
    }
    pub fn holes(&self) -> &[Hole] {
        &self.holes
    }
    pub fn finite_cells(&self) -> &[FiniteCell] {
        &self.finite_cells
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum ArrangementFailure {
    EmptyCurveInput,
    DuplicateCurveIdentity {
        curve_id: String,
    },
    InvalidCurveIdentity {
        curve_id: String,
    },
    UnsupportedRepresentationCategory {
        curve_id: String,
    },
    InvalidParameterInterval {
        curve_id: String,
    },
    ChartDimensionMismatch {
        curve_id: String,
    },
    MissingIntersectionBinding {
        left_curve_id: String,
        right_curve_id: String,
    },
    DuplicateIntersectionBinding {
        left_curve_id: String,
        right_curve_id: String,
    },
    UnknownIntersectionCurve {
        curve_id: String,
    },
    IntersectionBindingSelfPair {
        curve_id: String,
    },
    IntersectionSetWitnessNotProved {
        left_curve_id: String,
        right_curve_id: String,
    },
    IntersectionUndetermined {
        reason: IntersectionUndeterminedReason,
    },
    MissingPointAssociation {
        component_id: String,
    },
    IncompletePointAssociation {
        component_id: String,
    },
    DuplicatePointAssociation {
        component_id: String,
        curve_id: String,
    },
    UnknownPointAssociation {
        component_id: String,
    },
    MissingSpanAssociation {
        component_id: String,
    },
    IncompleteSpanAssociation {
        span_id: String,
    },
    DuplicateSpanAssociation {
        span_id: String,
    },
    UnknownSpanAssociation {
        span_id: String,
    },
    AssociationCurveMismatch {
        span_id: String,
    },
    AssociationComponentIdentityMismatch {
        component_id: String,
    },
    AssociationParameterOutOfDomain {
        curve_id: String,
    },
    AssociationEndpointRoleMismatch {
        span_id: String,
    },
    DuplicateSplitEvent {
        locator: CurveEventLocator,
    },
    OverlapParameterMismatch {
        overlap_id: String,
    },
    DuplicateOverlap {
        overlap_id: String,
    },
    SegmentCoverageFailure {
        curve_id: String,
    },
    IncidenceConstructionFailure {
        incidence_id: String,
    },
    EmptyPlanarVertex {
        vertex_key: String,
    },
    DuplicatePlanarVertex {
        vertex_key: String,
    },
    UnknownPlanarVertex {
        vertex_key: String,
    },
    UnknownVertexEventLocator {
        vertex_key: String,
        locator: CurveEventLocator,
    },
    AmbiguousVertexEventLocator {
        vertex_key: String,
        locator: CurveEventLocator,
    },
    DuplicateVertexEventLocator {
        vertex_key: String,
        locator: CurveEventLocator,
    },
    UncoveredVertexEventLocator {
        locator: CurveEventLocator,
    },
    EmptyEventRotation {
        vertex_key: String,
    },
    DuplicateEventRotation {
        vertex_key: String,
    },
    UnknownEventRotation {
        vertex_key: String,
    },
    RotationUnknownIncidence {
        vertex_key: String,
        incidence_id: String,
    },
    RotationDuplicateIncidence {
        vertex_key: String,
        incidence_id: String,
    },
    RotationIncidenceCompletenessFailure {
        vertex_key: String,
    },
    RotationIncidenceMismatch {
        vertex_key: String,
        incidence_id: String,
    },
    VertexIncidenceMismatch {
        vertex_key: String,
        incidence_id: String,
    },
    UnknownWalkClassification {
        walk_key: String,
    },
    DuplicateWalkClassification {
        walk_key: String,
    },
    DuplicateWalkClassificationSequence {
        canonical_incidence_ids: Vec<String>,
    },
    EmptyWalkClassification {
        walk_key: String,
    },
    WalkClassificationUnknownIncidence {
        walk_key: String,
        incidence_id: String,
    },
    WalkSequenceMismatch {
        walk_key: String,
    },
    WalkClassificationRoleMismatch {
        walk_key: String,
    },
    WalkClassificationWindingMismatch {
        walk_key: String,
    },
    WalkClassificationContainmentMismatch {
        walk_key: String,
    },
    ContainingOuterWalkUnknown {
        walk_key: String,
        containing_outer_walk_key: String,
    },
    ContainingOuterWalkSelfReference {
        walk_key: String,
    },
    NonClosedWalk {
        incidence_id: String,
    },
    FaceClassificationFailure {
        walk_key: String,
    },
    HoleContainmentFailure {
        walk_key: String,
    },
    FiniteCellConstructionFailure {
        face_id: String,
    },
    GeometryFailure(GeometryFailure),
    CalculusFailure(CalculusFailure),
}
impl From<GeometryFailure> for ArrangementFailure {
    fn from(value: GeometryFailure) -> Self {
        Self::GeometryFailure(value)
    }
}
impl From<CalculusFailure> for ArrangementFailure {
    fn from(value: CalculusFailure) -> Self {
        Self::CalculusFailure(value)
    }
}
impl From<neco_algnum::AlgnumError> for ArrangementFailure {
    fn from(value: neco_algnum::AlgnumError) -> Self {
        Self::CalculusFailure(value.into())
    }
}
