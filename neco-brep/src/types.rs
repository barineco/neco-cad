use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;

use neco_arrangement2d::{Arrangement, DirectedIncidenceDirection, ParameterSpan};
use neco_geometry::{BoundaryStratification, RealAlgebraic, TrimLoopRole};
use neco_nurbs::NurbsTypes;

fn require_identifier(value: String) -> Result<String, BrepFailure> {
    if value.is_empty() {
        Err(BrepFailure::EmptyIdentifier)
    } else {
        Ok(value)
    }
}

macro_rules! identifier {
    ($name:ident) => {
        #[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);
        impl $name {
            pub fn new(value: String) -> Result<Self, BrepFailure> {
                Ok(Self(require_identifier(value)?))
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

identifier!(ArrangementId);
identifier!(CurveRepresentationId);
identifier!(SurfaceRepresentationId);
identifier!(SourceRevision);

#[derive(Debug, Eq, PartialEq)]
pub struct ChartArrangement {
    id: ArrangementId,
    arrangement: Arrangement,
}
impl ChartArrangement {
    pub fn new(id: ArrangementId, arrangement: Arrangement) -> Self {
        Self { id, arrangement }
    }
    pub fn id(&self) -> &ArrangementId {
        &self.id
    }
    pub fn arrangement(&self) -> &Arrangement {
        &self.arrangement
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct CurveRepresentation {
    id: CurveRepresentationId,
    value: NurbsTypes,
}
impl CurveRepresentation {
    pub fn new(id: CurveRepresentationId, value: NurbsTypes) -> Result<Self, BrepFailure> {
        if !matches!(
            value,
            NurbsTypes::PolynomialCurve(_) | NurbsTypes::RationalCurve(_)
        ) {
            return Err(BrepFailure::CurveRepresentationKind {
                representation_id: id.as_str().into(),
            });
        }
        Ok(Self { id, value })
    }
    pub fn id(&self) -> &CurveRepresentationId {
        &self.id
    }
    pub fn value(&self) -> &NurbsTypes {
        &self.value
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct SurfaceRepresentation {
    id: SurfaceRepresentationId,
    value: NurbsTypes,
}
impl SurfaceRepresentation {
    pub fn new(id: SurfaceRepresentationId, value: NurbsTypes) -> Result<Self, BrepFailure> {
        if !matches!(
            value,
            NurbsTypes::PolynomialSurface(_) | NurbsTypes::RationalSurface(_)
        ) {
            return Err(BrepFailure::SurfaceRepresentationKind {
                representation_id: id.as_str().into(),
            });
        }
        Ok(Self { id, value })
    }
    pub fn id(&self) -> &SurfaceRepresentationId {
        &self.id
    }
    pub fn value(&self) -> &NurbsTypes {
        &self.value
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VertexAssignment {
    stratum_id: String,
    chart_arrangement_id: ArrangementId,
    arrangement_vertex_key: String,
}
impl VertexAssignment {
    pub fn new(
        stratum_id: String,
        chart_arrangement_id: ArrangementId,
        arrangement_vertex_key: String,
    ) -> Result<Self, BrepFailure> {
        Ok(Self {
            stratum_id: require_identifier(stratum_id)?,
            chart_arrangement_id,
            arrangement_vertex_key: require_identifier(arrangement_vertex_key)?,
        })
    }
    pub fn stratum_id(&self) -> &str {
        &self.stratum_id
    }
    pub fn chart_arrangement_id(&self) -> &ArrangementId {
        &self.chart_arrangement_id
    }
    pub fn arrangement_vertex_key(&self) -> &str {
        &self.arrangement_vertex_key
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EdgeAssignment {
    stratum_id: String,
    chart_arrangement_id: ArrangementId,
    segment_id: String,
    curve_representation_id: CurveRepresentationId,
}
impl EdgeAssignment {
    pub fn new(
        stratum_id: String,
        chart_arrangement_id: ArrangementId,
        segment_id: String,
        curve_representation_id: CurveRepresentationId,
    ) -> Result<Self, BrepFailure> {
        Ok(Self {
            stratum_id: require_identifier(stratum_id)?,
            chart_arrangement_id,
            segment_id: require_identifier(segment_id)?,
            curve_representation_id,
        })
    }
    pub fn stratum_id(&self) -> &str {
        &self.stratum_id
    }
    pub fn chart_arrangement_id(&self) -> &ArrangementId {
        &self.chart_arrangement_id
    }
    pub fn segment_id(&self) -> &str {
        &self.segment_id
    }
    pub fn curve_representation_id(&self) -> &CurveRepresentationId {
        &self.curve_representation_id
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FaceAssignment {
    stratum_id: String,
    chart_arrangement_id: ArrangementId,
    arrangement_face_id: String,
    surface_representation_id: SurfaceRepresentationId,
}
impl FaceAssignment {
    pub fn new(
        stratum_id: String,
        chart_arrangement_id: ArrangementId,
        arrangement_face_id: String,
        surface_representation_id: SurfaceRepresentationId,
    ) -> Result<Self, BrepFailure> {
        Ok(Self {
            stratum_id: require_identifier(stratum_id)?,
            chart_arrangement_id,
            arrangement_face_id: require_identifier(arrangement_face_id)?,
            surface_representation_id,
        })
    }
    pub fn stratum_id(&self) -> &str {
        &self.stratum_id
    }
    pub fn chart_arrangement_id(&self) -> &ArrangementId {
        &self.chart_arrangement_id
    }
    pub fn arrangement_face_id(&self) -> &str {
        &self.arrangement_face_id
    }
    pub fn surface_representation_id(&self) -> &SurfaceRepresentationId {
        &self.surface_representation_id
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BrepAssignments {
    vertices: Vec<VertexAssignment>,
    edges: Vec<EdgeAssignment>,
    faces: Vec<FaceAssignment>,
}
impl BrepAssignments {
    pub fn new(
        vertices: Vec<VertexAssignment>,
        edges: Vec<EdgeAssignment>,
        faces: Vec<FaceAssignment>,
    ) -> Result<Self, BrepFailure> {
        let mut strata = BTreeSet::new();
        for id in vertices
            .iter()
            .map(VertexAssignment::stratum_id)
            .chain(edges.iter().map(EdgeAssignment::stratum_id))
            .chain(faces.iter().map(FaceAssignment::stratum_id))
        {
            if !strata.insert(id) {
                return Err(BrepFailure::DuplicateAssignmentStratum {
                    stratum_id: id.into(),
                });
            }
        }
        Ok(Self {
            vertices,
            edges,
            faces,
        })
    }
    pub fn vertices(&self) -> &[VertexAssignment] {
        &self.vertices
    }
    pub fn edges(&self) -> &[EdgeAssignment] {
        &self.edges
    }
    pub fn faces(&self) -> &[FaceAssignment] {
        &self.faces
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct BrepMaterializationRequest {
    boundary_stratification: BoundaryStratification,
    chart_arrangements: Vec<ChartArrangement>,
    curve_representations: Vec<CurveRepresentation>,
    surface_representations: Vec<SurfaceRepresentation>,
    assignments: BrepAssignments,
    source_revision: SourceRevision,
}
impl BrepMaterializationRequest {
    pub fn new(
        boundary_stratification: BoundaryStratification,
        chart_arrangements: Vec<ChartArrangement>,
        curve_representations: Vec<CurveRepresentation>,
        surface_representations: Vec<SurfaceRepresentation>,
        assignments: BrepAssignments,
        source_revision: SourceRevision,
    ) -> Result<Self, BrepFailure> {
        let mut arrangements = BTreeSet::new();
        for entry in &chart_arrangements {
            if !arrangements.insert(entry.id().as_str()) {
                return Err(BrepFailure::DuplicateArrangement {
                    arrangement_id: entry.id().as_str().into(),
                });
            }
        }
        let mut curves = BTreeSet::new();
        for entry in &curve_representations {
            if !curves.insert(entry.id().as_str()) {
                return Err(BrepFailure::DuplicateCurveRepresentation {
                    representation_id: entry.id().as_str().into(),
                });
            }
        }
        let mut surfaces = BTreeSet::new();
        for entry in &surface_representations {
            if !surfaces.insert(entry.id().as_str()) {
                return Err(BrepFailure::DuplicateSurfaceRepresentation {
                    representation_id: entry.id().as_str().into(),
                });
            }
        }
        for assignment in assignments.vertices() {
            if !arrangements.contains(assignment.chart_arrangement_id().as_str()) {
                return Err(BrepFailure::UnknownArrangement {
                    arrangement_id: assignment.chart_arrangement_id().as_str().into(),
                });
            }
        }
        for assignment in assignments.edges() {
            if !arrangements.contains(assignment.chart_arrangement_id().as_str()) {
                return Err(BrepFailure::UnknownArrangement {
                    arrangement_id: assignment.chart_arrangement_id().as_str().into(),
                });
            }
            if !curves.contains(assignment.curve_representation_id().as_str()) {
                return Err(BrepFailure::UnknownCurveRepresentation {
                    representation_id: assignment.curve_representation_id().as_str().into(),
                });
            }
        }
        for assignment in assignments.faces() {
            if !arrangements.contains(assignment.chart_arrangement_id().as_str()) {
                return Err(BrepFailure::UnknownArrangement {
                    arrangement_id: assignment.chart_arrangement_id().as_str().into(),
                });
            }
            if !surfaces.contains(assignment.surface_representation_id().as_str()) {
                return Err(BrepFailure::UnknownSurfaceRepresentation {
                    representation_id: assignment.surface_representation_id().as_str().into(),
                });
            }
        }
        Ok(Self {
            boundary_stratification,
            chart_arrangements,
            curve_representations,
            surface_representations,
            assignments,
            source_revision,
        })
    }
    pub fn boundary_stratification(&self) -> &BoundaryStratification {
        &self.boundary_stratification
    }
    pub fn chart_arrangements(&self) -> &[ChartArrangement] {
        &self.chart_arrangements
    }
    pub fn curve_representations(&self) -> &[CurveRepresentation] {
        &self.curve_representations
    }
    pub fn surface_representations(&self) -> &[SurfaceRepresentation] {
        &self.surface_representations
    }
    pub fn assignments(&self) -> &BrepAssignments {
        &self.assignments
    }
    pub fn source_revision(&self) -> &SourceRevision {
        &self.source_revision
    }
    pub(crate) fn into_parts(
        self,
    ) -> (
        BoundaryStratification,
        Vec<ChartArrangement>,
        Vec<CurveRepresentation>,
        Vec<SurfaceRepresentation>,
        BrepAssignments,
        SourceRevision,
    ) {
        (
            self.boundary_stratification,
            self.chart_arrangements,
            self.curve_representations,
            self.surface_representations,
            self.assignments,
            self.source_revision,
        )
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct OwnedParameterSpan {
    lower: RealAlgebraic,
    upper: RealAlgebraic,
    lower_closed: bool,
    upper_closed: bool,
}
impl OwnedParameterSpan {
    pub(crate) fn from_arrangement(span: &ParameterSpan) -> Result<Self, BrepFailure> {
        Ok(Self {
            lower: span
                .lower()
                .try_clone()
                .map_err(|_| BrepFailure::ParameterClone)?,
            upper: span
                .upper()
                .try_clone()
                .map_err(|_| BrepFailure::ParameterClone)?,
            lower_closed: span.lower_closed(),
            upper_closed: span.upper_closed(),
        })
    }
    pub(crate) fn from_curve_domain(
        lower: &RealAlgebraic,
        upper: &RealAlgebraic,
    ) -> Result<Self, BrepFailure> {
        Ok(Self {
            lower: lower.try_clone().map_err(|_| BrepFailure::ParameterClone)?,
            upper: upper.try_clone().map_err(|_| BrepFailure::ParameterClone)?,
            lower_closed: true,
            upper_closed: true,
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
pub struct Vertex {
    id: String,
    stratum_id: String,
    chart_arrangement_id: ArrangementId,
    arrangement_vertex_key: String,
}
impl Vertex {
    pub(crate) fn new(id: String, assignment: &VertexAssignment) -> Self {
        Self {
            id,
            stratum_id: assignment.stratum_id().into(),
            chart_arrangement_id: assignment.chart_arrangement_id().clone(),
            arrangement_vertex_key: assignment.arrangement_vertex_key().into(),
        }
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn stratum_id(&self) -> &str {
        &self.stratum_id
    }
    pub fn chart_arrangement_id(&self) -> &ArrangementId {
        &self.chart_arrangement_id
    }
    pub fn arrangement_vertex_key(&self) -> &str {
        &self.arrangement_vertex_key
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Edge {
    id: String,
    stratum_id: String,
    chart_arrangement_id: ArrangementId,
    segment_id: String,
    curve_representation_id: CurveRepresentationId,
    parameter_span: OwnedParameterSpan,
}
impl Edge {
    pub(crate) fn new(
        id: String,
        assignment: &EdgeAssignment,
        parameter_span: OwnedParameterSpan,
    ) -> Self {
        Self {
            id,
            stratum_id: assignment.stratum_id().into(),
            chart_arrangement_id: assignment.chart_arrangement_id().clone(),
            segment_id: assignment.segment_id().into(),
            curve_representation_id: assignment.curve_representation_id().clone(),
            parameter_span,
        }
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn stratum_id(&self) -> &str {
        &self.stratum_id
    }
    pub fn chart_arrangement_id(&self) -> &ArrangementId {
        &self.chart_arrangement_id
    }
    pub fn segment_id(&self) -> &str {
        &self.segment_id
    }
    pub fn curve_representation_id(&self) -> &CurveRepresentationId {
        &self.curve_representation_id
    }
    pub fn parameter_span(&self) -> &OwnedParameterSpan {
        &self.parameter_span
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FaceOrientation {
    Forward,
    Reverse,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShellOrientation {
    Outward,
    Inward,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Incidence {
    id: String,
    edge_id: String,
    from_vertex_id: String,
    to_vertex_id: String,
    direction: DirectedIncidenceDirection,
}
impl Incidence {
    pub(crate) fn new(
        id: String,
        edge_id: String,
        from_vertex_id: String,
        to_vertex_id: String,
        direction: DirectedIncidenceDirection,
    ) -> Self {
        Self {
            id,
            edge_id,
            from_vertex_id,
            to_vertex_id,
            direction,
        }
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn edge_id(&self) -> &str {
        &self.edge_id
    }
    pub fn from_vertex_id(&self) -> &str {
        &self.from_vertex_id
    }
    pub fn to_vertex_id(&self) -> &str {
        &self.to_vertex_id
    }
    pub fn direction(&self) -> DirectedIncidenceDirection {
        self.direction
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct TrimPCurve<'a> {
    curve_representation_id: &'a CurveRepresentationId,
    parameter_span: &'a OwnedParameterSpan,
}
impl<'a> TrimPCurve<'a> {
    pub fn curve_representation_id(&self) -> &'a CurveRepresentationId {
        self.curve_representation_id
    }
    pub fn parameter_span(&self) -> &'a OwnedParameterSpan {
        self.parameter_span
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Trim {
    id: String,
    incidence_id: String,
    face_id: String,
    curve_representation_id: CurveRepresentationId,
    parameter_span: OwnedParameterSpan,
    loop_role: TrimLoopRole,
}
impl Trim {
    pub(crate) fn new(
        id: String,
        incidence_id: String,
        face_id: String,
        curve_representation_id: CurveRepresentationId,
        parameter_span: OwnedParameterSpan,
        loop_role: TrimLoopRole,
    ) -> Self {
        Self {
            id,
            incidence_id,
            face_id,
            curve_representation_id,
            parameter_span,
            loop_role,
        }
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn incidence_id(&self) -> &str {
        &self.incidence_id
    }
    pub fn face_id(&self) -> &str {
        &self.face_id
    }
    pub fn curve_representation_id(&self) -> &CurveRepresentationId {
        &self.curve_representation_id
    }
    pub fn parameter_span(&self) -> &OwnedParameterSpan {
        &self.parameter_span
    }
    pub fn loop_role(&self) -> TrimLoopRole {
        self.loop_role
    }
    pub fn p_curve(&self) -> TrimPCurve<'_> {
        TrimPCurve {
            curve_representation_id: &self.curve_representation_id,
            parameter_span: &self.parameter_span,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Face {
    id: String,
    stratum_id: String,
    chart_arrangement_id: ArrangementId,
    arrangement_face_id: String,
    surface_representation_id: SurfaceRepresentationId,
    orientation: FaceOrientation,
    outer_trim_ids: Vec<String>,
    hole_trim_ids: Vec<String>,
}
impl Face {
    pub(crate) fn new(
        id: String,
        assignment: &FaceAssignment,
        orientation: FaceOrientation,
        outer_trim_ids: Vec<String>,
        hole_trim_ids: Vec<String>,
    ) -> Self {
        Self {
            id,
            stratum_id: assignment.stratum_id().into(),
            chart_arrangement_id: assignment.chart_arrangement_id().clone(),
            arrangement_face_id: assignment.arrangement_face_id().into(),
            surface_representation_id: assignment.surface_representation_id().clone(),
            orientation,
            outer_trim_ids,
            hole_trim_ids,
        }
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn stratum_id(&self) -> &str {
        &self.stratum_id
    }
    pub fn chart_arrangement_id(&self) -> &ArrangementId {
        &self.chart_arrangement_id
    }
    pub fn arrangement_face_id(&self) -> &str {
        &self.arrangement_face_id
    }
    pub fn surface_representation_id(&self) -> &SurfaceRepresentationId {
        &self.surface_representation_id
    }
    pub fn orientation(&self) -> FaceOrientation {
        self.orientation
    }
    pub fn outer_trim_ids(&self) -> &[String] {
        &self.outer_trim_ids
    }
    pub fn hole_trim_ids(&self) -> &[String] {
        &self.hole_trim_ids
    }
    pub fn trim_ids(&self) -> Vec<String> {
        self.outer_trim_ids
            .iter()
            .chain(self.hole_trim_ids.iter())
            .cloned()
            .collect()
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Shell {
    id: String,
    chart_arrangement_id: ArrangementId,
    face_ids: Vec<String>,
    orientation: ShellOrientation,
}
impl Shell {
    pub(crate) fn new(
        id: String,
        chart_arrangement_id: ArrangementId,
        face_ids: Vec<String>,
        orientation: ShellOrientation,
    ) -> Self {
        Self {
            id,
            chart_arrangement_id,
            face_ids,
            orientation,
        }
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn chart_arrangement_id(&self) -> &ArrangementId {
        &self.chart_arrangement_id
    }
    pub fn face_ids(&self) -> &[String] {
        &self.face_ids
    }
    pub fn orientation(&self) -> ShellOrientation {
        self.orientation
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct BrepCorrespondence {
    assignments: BrepAssignments,
}
impl BrepCorrespondence {
    pub(crate) fn new(assignments: BrepAssignments) -> Self {
        Self { assignments }
    }
    pub fn assignments(&self) -> &BrepAssignments {
        &self.assignments
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct BrepOrientation {
    face_orientations: Vec<(String, FaceOrientation)>,
    shell_orientations: Vec<(String, ShellOrientation)>,
}
impl BrepOrientation {
    pub(crate) fn new(faces: &[Face], shells: &[Shell]) -> Self {
        Self {
            face_orientations: faces
                .iter()
                .map(|face| (face.id().into(), face.orientation()))
                .collect(),
            shell_orientations: shells
                .iter()
                .map(|shell| (shell.id().into(), shell.orientation()))
                .collect(),
        }
    }
    pub fn face_orientations(&self) -> &[(String, FaceOrientation)] {
        &self.face_orientations
    }
    pub fn shell_orientations(&self) -> &[(String, ShellOrientation)] {
        &self.shell_orientations
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct ShellTopologyWitness {
    shell_id: String,
    edge_count: usize,
    closed: bool,
    orientation: ShellOrientation,
    manifold: bool,
}
impl ShellTopologyWitness {
    pub(crate) fn new(
        shell_id: String,
        edge_count: usize,
        closed: bool,
        orientation: ShellOrientation,
        manifold: bool,
    ) -> Self {
        Self {
            shell_id,
            edge_count,
            closed,
            orientation,
            manifold,
        }
    }
    pub fn shell_id(&self) -> &str {
        &self.shell_id
    }
    pub fn edge_count(&self) -> usize {
        self.edge_count
    }
    pub fn closed(&self) -> bool {
        self.closed
    }
    pub fn orientation(&self) -> ShellOrientation {
        self.orientation
    }
    pub fn manifold(&self) -> bool {
        self.manifold
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct BrepWitnesses {
    incidence_count: usize,
    closed_shell_ids: Vec<String>,
    source_revision: SourceRevision,
    shell_witnesses: Vec<ShellTopologyWitness>,
}
impl BrepWitnesses {
    pub(crate) fn new(
        incidence_count: usize,
        closed_shell_ids: Vec<String>,
        source_revision: SourceRevision,
        shell_witnesses: Vec<ShellTopologyWitness>,
    ) -> Self {
        Self {
            incidence_count,
            closed_shell_ids,
            source_revision,
            shell_witnesses,
        }
    }
    pub fn incidence_count(&self) -> usize {
        self.incidence_count
    }
    pub fn closed_shell_ids(&self) -> &[String] {
        &self.closed_shell_ids
    }
    pub fn source_revision(&self) -> &SourceRevision {
        &self.source_revision
    }
    pub fn shell_witnesses(&self) -> &[ShellTopologyWitness] {
        &self.shell_witnesses
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct ClosedOrientedShell {
    shell_id: String,
    chart_arrangement_id: ArrangementId,
    face_ids: Vec<String>,
    orientation: ShellOrientation,
    edge_count: usize,
    source_revision: SourceRevision,
}
impl ClosedOrientedShell {
    pub(crate) fn new(
        shell_id: String,
        chart_arrangement_id: ArrangementId,
        face_ids: Vec<String>,
        orientation: ShellOrientation,
        edge_count: usize,
        source_revision: SourceRevision,
    ) -> Self {
        Self {
            shell_id,
            chart_arrangement_id,
            face_ids,
            orientation,
            edge_count,
            source_revision,
        }
    }
    pub fn shell_id(&self) -> &str {
        &self.shell_id
    }
    pub fn chart_arrangement_id(&self) -> &ArrangementId {
        &self.chart_arrangement_id
    }
    pub fn face_ids(&self) -> &[String] {
        &self.face_ids
    }
    pub fn orientation(&self) -> ShellOrientation {
        self.orientation
    }
    pub fn edge_count(&self) -> usize {
        self.edge_count
    }
    pub fn source_revision(&self) -> &SourceRevision {
        &self.source_revision
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct AnalyticBrep {
    pub(crate) vertices: Vec<Vertex>,
    pub(crate) edges: Vec<Edge>,
    pub(crate) faces: Vec<Face>,
    pub(crate) shells: Vec<Shell>,
    pub(crate) trims: Vec<Trim>,
    pub(crate) incidences: Vec<Incidence>,
    pub(crate) curve_representations: Vec<CurveRepresentation>,
    pub(crate) surface_representations: Vec<SurfaceRepresentation>,
    pub(crate) correspondence: BrepCorrespondence,
    pub(crate) orientation: BrepOrientation,
    pub(crate) witnesses: BrepWitnesses,
}
impl AnalyticBrep {
    pub fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }
    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }
    pub fn faces(&self) -> &[Face] {
        &self.faces
    }
    pub fn shells(&self) -> &[Shell] {
        &self.shells
    }
    pub fn trims(&self) -> &[Trim] {
        &self.trims
    }
    pub fn incidences(&self) -> &[Incidence] {
        &self.incidences
    }
    pub fn curve_representations(&self) -> &[CurveRepresentation] {
        &self.curve_representations
    }
    pub fn surface_representations(&self) -> &[SurfaceRepresentation] {
        &self.surface_representations
    }
    pub fn correspondence(&self) -> &BrepCorrespondence {
        &self.correspondence
    }
    pub fn orientation(&self) -> &BrepOrientation {
        &self.orientation
    }
    pub fn witnesses(&self) -> &BrepWitnesses {
        &self.witnesses
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum BrepFailure {
    EmptyIdentifier,
    DuplicateArrangement {
        arrangement_id: String,
    },
    DuplicateCurveRepresentation {
        representation_id: String,
    },
    DuplicateSurfaceRepresentation {
        representation_id: String,
    },
    DuplicateAssignmentStratum {
        stratum_id: String,
    },
    UnknownArrangement {
        arrangement_id: String,
    },
    UnknownCurveRepresentation {
        representation_id: String,
    },
    UnknownSurfaceRepresentation {
        representation_id: String,
    },
    CurveRepresentationKind {
        representation_id: String,
    },
    SurfaceRepresentationKind {
        representation_id: String,
    },
    UnknownStratum {
        stratum_id: String,
    },
    UndeterminedStratum {
        stratum_id: String,
    },
    StratumDimensionMismatch {
        stratum_id: String,
        expected_dimension: i64,
    },
    UnknownArrangementVertex {
        arrangement_id: String,
        vertex_key: String,
    },
    UnknownArrangementSegment {
        arrangement_id: String,
        segment_id: String,
    },
    UnknownArrangementFace {
        arrangement_id: String,
        face_id: String,
    },
    ParameterClone,
    IncidenceVertexMissing {
        incidence_id: String,
    },
    FaceSupport {
        face_id: String,
    },
    ShellClosure {
        shell_id: String,
    },
    UnknownShellIdentity {
        shell_id: String,
    },
    ShellMembershipEvidenceUnavailable {
        shell_id: String,
    },
    ShellSelfIntersection {
        first_face_id: String,
        second_face_id: String,
    },
}
