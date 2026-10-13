#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use neco_algnum::RealAlgebraic;
use neco_arrangement2d::DirectedIncidenceDirection;
use neco_bigint::{BigInt, Dyadic, DyadicEnclosure, RawRational, ReducedRational, Sign};
use neco_brep::{AnalyticBrep, OwnedParameterSpan, SourceRevision, SurfaceRepresentation};
use neco_mesh::{validate_surface_mesh, MeshFailure, SurfaceMesh, ValidatedSurfaceMesh};
use neco_nurbs::{evaluate_curve_nurbs, BezierPoint, NurbsTypes, RationalNurbsSurface};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TessellationRequest {
    maximum_chord_error: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TessellationConsumer {
    Display,
    Analysis,
    Picking,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConsumerPrecision {
    consumer: TessellationConsumer,
    maximum_chord_error: f64,
}

impl ConsumerPrecision {
    pub fn new(
        consumer: TessellationConsumer,
        maximum_chord_error: f64,
    ) -> Result<Self, TessellationFailure> {
        let request = TessellationRequest::new(maximum_chord_error)?;
        Ok(Self {
            consumer,
            maximum_chord_error: request.maximum_chord_error(),
        })
    }

    pub fn consumer(self) -> TessellationConsumer {
        self.consumer
    }

    pub fn maximum_chord_error(self) -> f64 {
        self.maximum_chord_error
    }

    pub fn request(self) -> TessellationRequest {
        TessellationRequest {
            maximum_chord_error: self.maximum_chord_error,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AdaptivePrecision {
    consumer: TessellationConsumer,
    maximum_chord_error: f64,
    maximum_surface_error: f64,
}

impl AdaptivePrecision {
    pub fn new(
        consumer: TessellationConsumer,
        maximum_chord_error: f64,
        maximum_surface_error: f64,
    ) -> Result<Self, TessellationFailure> {
        let chord = TessellationRequest::new(maximum_chord_error)?;
        if !maximum_surface_error.is_finite() || maximum_surface_error <= 0.0 {
            return Err(TessellationFailure::InvalidPrecision);
        }
        Ok(Self {
            consumer,
            maximum_chord_error: chord.maximum_chord_error(),
            maximum_surface_error,
        })
    }

    pub fn consumer(self) -> TessellationConsumer {
        self.consumer
    }

    pub fn maximum_chord_error(self) -> f64 {
        self.maximum_chord_error
    }

    pub fn maximum_surface_error(self) -> f64 {
        self.maximum_surface_error
    }
}

#[derive(Debug, PartialEq)]
pub struct AdaptivePrecisionWitness {
    consumer: TessellationConsumer,
    requested_maximum_chord_error: f64,
    established_maximum_chord_error: f64,
    requested_maximum_surface_error: f64,
    established_maximum_surface_error: Dyadic,
}

impl AdaptivePrecisionWitness {
    pub fn new(
        consumer: TessellationConsumer,
        requested_maximum_chord_error: f64,
        established_maximum_chord_error: f64,
        requested_maximum_surface_error: f64,
        established_maximum_surface_error: Dyadic,
    ) -> Self {
        Self {
            consumer,
            requested_maximum_chord_error,
            established_maximum_chord_error,
            requested_maximum_surface_error,
            established_maximum_surface_error,
        }
    }

    pub fn consumer(&self) -> TessellationConsumer {
        self.consumer
    }

    pub fn requested_maximum_chord_error(&self) -> f64 {
        self.requested_maximum_chord_error
    }

    pub fn established_maximum_chord_error(&self) -> f64 {
        self.established_maximum_chord_error
    }

    pub fn requested_maximum_surface_error(&self) -> f64 {
        self.requested_maximum_surface_error
    }

    pub fn established_maximum_surface_error(&self) -> &Dyadic {
        &self.established_maximum_surface_error
    }
}
impl TessellationRequest {
    pub fn new(maximum_chord_error: f64) -> Result<Self, TessellationFailure> {
        if !maximum_chord_error.is_finite() || maximum_chord_error <= 0.0 {
            return Err(TessellationFailure::InvalidPrecision);
        }
        Ok(Self {
            maximum_chord_error,
        })
    }
    pub fn maximum_chord_error(&self) -> f64 {
        self.maximum_chord_error
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PrecisionWitness {
    requested_maximum_chord_error: f64,
}
impl PrecisionWitness {
    pub fn requested_maximum_chord_error(&self) -> f64 {
        self.requested_maximum_chord_error
    }
    pub fn established_maximum_chord_error(&self) -> Option<f64> {
        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConsumerPrecisionWitness {
    consumer: TessellationConsumer,
    requested_maximum_chord_error: f64,
}

impl ConsumerPrecisionWitness {
    pub fn consumer(self) -> TessellationConsumer {
        self.consumer
    }
    pub fn requested_maximum_chord_error(self) -> f64 {
        self.requested_maximum_chord_error
    }
    pub fn established_maximum_chord_error(self) -> Option<f64> {
        None
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BrepElementCorrespondence {
    face_ids: Vec<String>,
}
impl BrepElementCorrespondence {
    pub fn new(face_ids: Vec<String>) -> Self {
        Self { face_ids }
    }

    pub fn face_ids(&self) -> &[String] {
        &self.face_ids
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TessellatedFace {
    face_id: String,
    triangle_indices: Vec<usize>,
}
impl TessellatedFace {
    pub fn new(face_id: String, triangle_indices: Vec<usize>) -> Self {
        Self {
            face_id,
            triangle_indices,
        }
    }

    pub fn face_id(&self) -> &str {
        &self.face_id
    }
    pub fn triangle_indices(&self) -> &[usize] {
        &self.triangle_indices
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct TessellatedCurve {
    identity: String,
    points: Vec<[f64; 3]>,
}
impl TessellatedCurve {
    pub fn identity(&self) -> &str {
        &self.identity
    }
    pub fn points(&self) -> &[[f64; 3]] {
        &self.points
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct TessellatedVertex {
    vertex_id: String,
    position: [f64; 3],
}
impl TessellatedVertex {
    pub fn vertex_id(&self) -> &str {
        &self.vertex_id
    }
    pub fn position(&self) -> [f64; 3] {
        self.position
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct TessellatedControlPoint {
    surface_representation_id: String,
    row: usize,
    column: usize,
    position: [f64; 3],
}
impl TessellatedControlPoint {
    pub fn surface_representation_id(&self) -> &str {
        &self.surface_representation_id
    }
    pub fn row(&self) -> usize {
        self.row
    }
    pub fn column(&self) -> usize {
        self.column
    }
    pub fn position(&self) -> [f64; 3] {
        self.position
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TessellatedViewElements {
    edge_samples: Vec<TessellatedCurve>,
    trim_samples: Vec<TessellatedCurve>,
    vertex_positions: Vec<TessellatedVertex>,
    control_points: Vec<TessellatedControlPoint>,
}
impl TessellatedViewElements {
    pub fn edge_samples(&self) -> &[TessellatedCurve] {
        &self.edge_samples
    }
    pub fn trim_samples(&self) -> &[TessellatedCurve] {
        &self.trim_samples
    }
    pub fn vertex_positions(&self) -> &[TessellatedVertex] {
        &self.vertex_positions
    }
    pub fn control_points(&self) -> &[TessellatedControlPoint] {
        &self.control_points
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TessellationResult {
    validated_surface_mesh: ValidatedSurfaceMesh,
    precision_witness: PrecisionWitness,
    brep_element_correspondence: BrepElementCorrespondence,
    source_brep_revision: SourceRevision,
    face_triangle_correspondence: Vec<TessellatedFace>,
    view_elements: TessellatedViewElements,
}
impl TessellationResult {
    pub fn validated_surface_mesh(&self) -> &ValidatedSurfaceMesh {
        &self.validated_surface_mesh
    }
    pub fn into_validated_surface_mesh(self) -> ValidatedSurfaceMesh {
        self.validated_surface_mesh
    }
    pub fn precision_witness(&self) -> PrecisionWitness {
        self.precision_witness
    }

    pub fn consumer_precision_witness(
        &self,
        precision: ConsumerPrecision,
    ) -> Result<ConsumerPrecisionWitness, TessellationFailure> {
        let requested = precision.maximum_chord_error();
        if requested != self.precision_witness.requested_maximum_chord_error() {
            return Err(TessellationFailure::RequestedPrecisionMismatch);
        }
        Ok(ConsumerPrecisionWitness {
            consumer: precision.consumer(),
            requested_maximum_chord_error: requested,
        })
    }
    pub fn brep_element_correspondence(&self) -> &BrepElementCorrespondence {
        &self.brep_element_correspondence
    }
    pub fn source_brep_revision(&self) -> &SourceRevision {
        &self.source_brep_revision
    }
    pub fn face_triangle_correspondence(&self) -> &[TessellatedFace] {
        &self.face_triangle_correspondence
    }
    pub fn edge_samples(&self) -> &[TessellatedCurve] {
        self.view_elements.edge_samples()
    }
    pub fn trim_samples(&self) -> &[TessellatedCurve] {
        self.view_elements.trim_samples()
    }
    pub fn vertex_positions(&self) -> &[TessellatedVertex] {
        self.view_elements.vertex_positions()
    }
    pub fn control_points(&self) -> &[TessellatedControlPoint] {
        self.view_elements.control_points()
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TessellationFailure {
    InvalidPrecision,
    SurfaceRepresentationMissing { face_id: String },
    CurveRepresentationMissing { identity: String },
    VertexPositionMissing { vertex_id: String },
    SurfaceControlNet,
    CurveEvaluation,
    AlgebraicValue,
    NonFiniteCoordinate,
    InconsistentVertexPosition { vertex_id: String },
    Mesh(MeshFailure),
    RequestedPrecisionMismatch,
}

#[derive(Debug)]
pub struct ProjectedCoordinate {
    value: f64,
    enclosure: DyadicEnclosure,
    absolute_error: Dyadic,
}

impl PartialEq for ProjectedCoordinate {
    fn eq(&self, other: &Self) -> bool {
        self.value.to_bits() == other.value.to_bits()
            && self.enclosure == other.enclosure
            && self.absolute_error == other.absolute_error
    }
}

impl Eq for ProjectedCoordinate {}

impl ProjectedCoordinate {
    pub fn value(&self) -> f64 {
        self.value
    }

    pub fn enclosure(&self) -> &DyadicEnclosure {
        &self.enclosure
    }

    pub fn absolute_error(&self) -> &Dyadic {
        &self.absolute_error
    }
}

fn degree_one_rational(
    value: &RealAlgebraic,
) -> Result<Option<ReducedRational>, TessellationFailure> {
    let coefficients = value.minimal_polynomial().polynomial().coefficients();
    if coefficients.len() != 2 {
        return Ok(None);
    }
    if coefficients[1].sign() == Sign::Zero {
        return Err(TessellationFailure::AlgebraicValue);
    }
    let mut numerator = coefficients[0]
        .negated()
        .map_err(|_| TessellationFailure::AlgebraicValue)?;
    if coefficients[1].sign() == Sign::Negative {
        numerator = numerator
            .negated()
            .map_err(|_| TessellationFailure::AlgebraicValue)?;
    }
    let denominator = coefficients[1]
        .magnitude()
        .try_clone()
        .map_err(|_| TessellationFailure::AlgebraicValue)?;
    Ok(Some(
        RawRational::new(numerator, denominator)
            .reduce()
            .map_err(|_| TessellationFailure::AlgebraicValue)?
            .into_reduced(),
    ))
}

fn project_reduced_rational(
    value: &ReducedRational,
    enclosure_bits: u32,
) -> Result<ProjectedCoordinate, TessellationFailure> {
    let lower = value
        .dyadic_floor(enclosure_bits)
        .map_err(|_| TessellationFailure::AlgebraicValue)?;
    let upper = value
        .dyadic_ceil(enclosure_bits)
        .map_err(|_| TessellationFailure::AlgebraicValue)?;
    let enclosure =
        DyadicEnclosure::new(lower, upper).map_err(|_| TessellationFailure::AlgebraicValue)?;
    let midpoint = enclosure
        .midpoint()
        .map_err(|_| TessellationFailure::AlgebraicValue)?;
    let rounded = midpoint
        .round_to_f64_ties_even()
        .map_err(|_| TessellationFailure::AlgebraicValue)?;
    if !rounded.is_finite() {
        return Err(TessellationFailure::NonFiniteCoordinate);
    }
    let exact_float =
        Dyadic::from_f64_exact(rounded).map_err(|_| TessellationFailure::AlgebraicValue)?;
    let absolute_error = coordinate_absolute_error(&exact_float, &enclosure)
        .map_err(|_| TessellationFailure::AlgebraicValue)?;
    Ok(ProjectedCoordinate {
        value: rounded,
        enclosure,
        absolute_error,
    })
}

pub fn project_real_algebraic(
    value: &RealAlgebraic,
    enclosure_bits: u32,
) -> Result<ProjectedCoordinate, TessellationFailure> {
    if let Some(rational) = degree_one_rational(value)? {
        return project_reduced_rational(&rational, enclosure_bits);
    }
    let interval = value
        .enclose(enclosure_bits)
        .map_err(|_| TessellationFailure::AlgebraicValue)?;
    let midpoint = interval
        .enclosure()
        .midpoint()
        .map_err(|_| TessellationFailure::AlgebraicValue)?;
    let rounded = midpoint
        .round_to_f64_ties_even()
        .map_err(|_| TessellationFailure::AlgebraicValue)?;
    if !rounded.is_finite() {
        return Err(TessellationFailure::NonFiniteCoordinate);
    }
    let exact_float =
        Dyadic::from_f64_exact(rounded).map_err(|_| TessellationFailure::AlgebraicValue)?;
    let enclosure = interval
        .enclosure()
        .try_clone()
        .map_err(|_| TessellationFailure::AlgebraicValue)?;
    let absolute_error = coordinate_absolute_error(&exact_float, &enclosure)
        .map_err(|_| TessellationFailure::AlgebraicValue)?;
    Ok(ProjectedCoordinate {
        value: rounded,
        enclosure,
        absolute_error,
    })
}

fn coordinate_absolute_error(
    value: &Dyadic,
    enclosure: &DyadicEnclosure,
) -> Result<Dyadic, TessellationFailure> {
    let lower = dyadic_abs(
        &value
            .sub(enclosure.lower())
            .map_err(|_| TessellationFailure::AlgebraicValue)?,
    )?;
    let upper = dyadic_abs(
        &enclosure
            .upper()
            .sub(value)
            .map_err(|_| TessellationFailure::AlgebraicValue)?,
    )?;
    if lower >= upper {
        Ok(lower)
    } else {
        Ok(upper)
    }
}

fn dyadic_abs(value: &Dyadic) -> Result<Dyadic, TessellationFailure> {
    if value.integer().sign() == Sign::Negative {
        Dyadic::new(
            BigInt::try_from(0_i32).map_err(|_| TessellationFailure::AlgebraicValue)?,
            0,
        )
        .sub(value)
        .map_err(|_| TessellationFailure::AlgebraicValue)
    } else {
        value
            .try_clone()
            .map_err(|_| TessellationFailure::AlgebraicValue)
    }
}

pub fn tessellate(
    brep: &AnalyticBrep,
    request: TessellationRequest,
) -> Result<TessellationResult, TessellationFailure> {
    let mut vertices = Vec::new();
    let mut triangles = Vec::new();
    let mut face_ids = Vec::new();
    let mut face_triangle_correspondence = Vec::new();
    for face in brep.faces() {
        let representation = brep
            .surface_representations()
            .iter()
            .find(|x| x.id() == face.surface_representation_id())
            .ok_or_else(|| TessellationFailure::SurfaceRepresentationMissing {
                face_id: String::from(face.id()),
            })?;
        let corners = surface_corners(representation)?;
        let first_vertex = vertices.len();
        let first_triangle = triangles.len();
        vertices.extend(corners);
        triangles.push([first_vertex, first_vertex + 1, first_vertex + 2]);
        triangles.push([first_vertex, first_vertex + 2, first_vertex + 3]);
        face_ids.push(String::from(face.id()));
        face_triangle_correspondence.push(TessellatedFace {
            face_id: String::from(face.id()),
            triangle_indices: vec![first_triangle, first_triangle + 1],
        });
    }
    let mesh =
        SurfaceMesh::new(vertices, triangles, Vec::new()).map_err(TessellationFailure::Mesh)?;
    Ok(TessellationResult {
        validated_surface_mesh: validate_surface_mesh(mesh).map_err(TessellationFailure::Mesh)?,
        precision_witness: PrecisionWitness {
            requested_maximum_chord_error: request.maximum_chord_error,
        },
        brep_element_correspondence: BrepElementCorrespondence { face_ids },
        source_brep_revision: brep.witnesses().source_revision().clone(),
        face_triangle_correspondence,
        view_elements: tessellate_view_elements(brep)?,
    })
}

pub fn tessellate_view_elements(
    brep: &AnalyticBrep,
) -> Result<TessellatedViewElements, TessellationFailure> {
    let mut control_points = Vec::new();
    for face in brep.faces() {
        let representation = brep
            .surface_representations()
            .iter()
            .find(|x| x.id() == face.surface_representation_id())
            .ok_or_else(|| TessellationFailure::SurfaceRepresentationMissing {
                face_id: String::from(face.id()),
            })?;
        append_control_points(representation, &mut control_points)?;
    }
    let mut edge_samples = Vec::new();
    for edge in brep.edges() {
        let representation =
            curve_representation(brep, edge.curve_representation_id().as_str(), edge.id())?;
        edge_samples.push(TessellatedCurve {
            identity: String::from(edge.id()),
            points: curve_samples(representation, edge.parameter_span())?,
        });
    }
    let edge_by_id: BTreeMap<&str, &TessellatedCurve> = edge_samples
        .iter()
        .map(|x| (x.identity.as_str(), x))
        .collect();
    let mut trim_samples = Vec::new();
    for trim in brep.trims() {
        let representation =
            curve_representation(brep, trim.curve_representation_id().as_str(), trim.id())?;
        trim_samples.push(TessellatedCurve {
            identity: String::from(trim.id()),
            points: curve_samples(representation, trim.parameter_span())?,
        });
    }
    let mut positions: BTreeMap<&str, [f64; 3]> = BTreeMap::new();
    for incidence in brep.incidences() {
        let sample = edge_by_id.get(incidence.edge_id()).ok_or_else(|| {
            TessellationFailure::CurveRepresentationMissing {
                identity: String::from(incidence.edge_id()),
            }
        })?;
        let (from, to) = match incidence.direction() {
            DirectedIncidenceDirection::Forward => {
                (sample.points[0], sample.points[sample.points.len() - 1])
            }
            DirectedIncidenceDirection::Reverse => {
                (sample.points[sample.points.len() - 1], sample.points[0])
            }
        };
        for (id, point) in [
            (incidence.from_vertex_id(), from),
            (incidence.to_vertex_id(), to),
        ] {
            insert_vertex_position(&mut positions, id, point)?;
        }
    }
    let mut vertex_positions = Vec::new();
    for vertex in brep.vertices() {
        let position = positions.get(vertex.id()).ok_or_else(|| {
            TessellationFailure::VertexPositionMissing {
                vertex_id: String::from(vertex.id()),
            }
        })?;
        vertex_positions.push(TessellatedVertex {
            vertex_id: String::from(vertex.id()),
            position: *position,
        });
    }
    Ok(TessellatedViewElements {
        edge_samples,
        trim_samples,
        vertex_positions,
        control_points,
    })
}
fn insert_vertex_position<'a>(
    positions: &mut BTreeMap<&'a str, [f64; 3]>,
    id: &'a str,
    point: [f64; 3],
) -> Result<(), TessellationFailure> {
    if let Some(existing) = positions.get(id) {
        if *existing != point {
            return Err(TessellationFailure::InconsistentVertexPosition {
                vertex_id: String::from(id),
            });
        }
    } else {
        positions.insert(id, point);
    }
    Ok(())
}

fn append_control_points(
    r: &SurfaceRepresentation,
    out: &mut Vec<TessellatedControlPoint>,
) -> Result<(), TessellationFailure> {
    let net = match r.value() {
        NurbsTypes::PolynomialSurface(x) => x.control_net(),
        NurbsTypes::RationalSurface(x) => x.control_net(),
        _ => return Err(TessellationFailure::SurfaceControlNet),
    };
    for (row, values) in net.iter().enumerate() {
        for (column, value) in values.iter().enumerate() {
            out.push(TessellatedControlPoint {
                surface_representation_id: String::from(r.id().as_str()),
                row,
                column,
                position: point3(value)?,
            });
        }
    }
    Ok(())
}
fn curve_representation<'a>(
    brep: &'a AnalyticBrep,
    representation_id: &str,
    identity: &str,
) -> Result<&'a NurbsTypes, TessellationFailure> {
    brep.curve_representations()
        .iter()
        .find(|x| x.id().as_str() == representation_id)
        .map(|x| x.value())
        .ok_or_else(|| TessellationFailure::CurveRepresentationMissing {
            identity: String::from(identity),
        })
}
fn curve_samples(
    value: &NurbsTypes,
    parameter_span: &OwnedParameterSpan,
) -> Result<Vec<[f64; 3]>, TessellationFailure> {
    let lower = parameter_span.lower();
    let upper = parameter_span.upper();
    let delta = upper
        .sub(lower)
        .map_err(|_| TessellationFailure::CurveEvaluation)?;
    let denominator = RealAlgebraic::from_integer(
        BigInt::try_from(4_i32).map_err(|_| TessellationFailure::AlgebraicValue)?,
    )
    .map_err(|_| TessellationFailure::AlgebraicValue)?;
    let step = delta
        .div(&denominator)
        .map_err(|_| TessellationFailure::CurveEvaluation)?;
    let mut points = Vec::with_capacity(5);
    for multiple in 0_i32..=4 {
        let factor = RealAlgebraic::from_integer(
            BigInt::try_from(multiple).map_err(|_| TessellationFailure::AlgebraicValue)?,
        )
        .map_err(|_| TessellationFailure::AlgebraicValue)?;
        let parameter = lower
            .add(
                &step
                    .mul(&factor)
                    .map_err(|_| TessellationFailure::CurveEvaluation)?,
            )
            .map_err(|_| TessellationFailure::CurveEvaluation)?;
        let point = evaluate_curve_nurbs(value, &parameter)
            .map_err(|_| TessellationFailure::CurveEvaluation)?
            .into_point();
        let position = match point3(&point) {
            Ok(position) => position,
            Err(TessellationFailure::SurfaceControlNet) => match point {
                BezierPoint::Point2(p) => [
                    project_coordinate_f64(&p.x)?,
                    project_coordinate_f64(&p.y)?,
                    0.0,
                ],
                BezierPoint::Point3(_) => return Err(TessellationFailure::SurfaceControlNet),
            },
            Err(error) => return Err(error),
        };
        if points.last().copied() != Some(position) {
            points.push(position);
        }
    }
    if points.len() < 2 {
        return Err(TessellationFailure::CurveEvaluation);
    }
    Ok(points)
}
fn surface_corners(r: &SurfaceRepresentation) -> Result<[[f64; 3]; 4], TessellationFailure> {
    match r.value() {
        NurbsTypes::PolynomialSurface(x) => corners_from_control_net(x.control_net()),
        NurbsTypes::RationalSurface(x) => corners_from_rational_surface(x),
        _ => Err(TessellationFailure::SurfaceControlNet),
    }
}
fn corners_from_rational_surface(
    s: &RationalNurbsSurface,
) -> Result<[[f64; 3]; 4], TessellationFailure> {
    corners_from_control_net(s.control_net())
}
fn corners_from_control_net(
    net: &[Vec<BezierPoint>],
) -> Result<[[f64; 3]; 4], TessellationFailure> {
    let a = net
        .first()
        .and_then(|x| x.first())
        .ok_or(TessellationFailure::SurfaceControlNet)?;
    let b = net
        .first()
        .and_then(|x| x.last())
        .ok_or(TessellationFailure::SurfaceControlNet)?;
    let c = net
        .last()
        .and_then(|x| x.last())
        .ok_or(TessellationFailure::SurfaceControlNet)?;
    let d = net
        .last()
        .and_then(|x| x.first())
        .ok_or(TessellationFailure::SurfaceControlNet)?;
    Ok([point3(a)?, point3(b)?, point3(c)?, point3(d)?])
}
fn point3(point: &BezierPoint) -> Result<[f64; 3], TessellationFailure> {
    match point {
        BezierPoint::Point3(p) => Ok([
            project_coordinate_f64(&p.x)?,
            project_coordinate_f64(&p.y)?,
            project_coordinate_f64(&p.z)?,
        ]),
        BezierPoint::Point2(_) => Err(TessellationFailure::SurfaceControlNet),
    }
}
#[cfg(feature = "test-fixtures")]
#[derive(Clone, Debug)]
pub struct TessellationResultFixture {
    result: TessellationResult,
}

#[cfg(feature = "test-fixtures")]
impl TessellationResultFixture {
    pub fn from_result(result: TessellationResult) -> Self {
        Self { result }
    }

    pub fn source_revision(mut self, revision: SourceRevision) -> Self {
        self.result.source_brep_revision = revision;
        self
    }

    pub fn precision(mut self, maximum_chord_error: f64) -> Self {
        self.result.precision_witness.requested_maximum_chord_error = maximum_chord_error;
        self
    }

    pub fn clear_face_triangle_correspondence(mut self) -> Self {
        self.result.face_triangle_correspondence.clear();
        self
    }

    pub fn duplicate_face_triangle_correspondence(mut self) -> Self {
        if let Some(entry) = self.result.face_triangle_correspondence.first().cloned() {
            self.result.face_triangle_correspondence.push(entry);
        }
        self
    }

    pub fn unknown_face_triangle_correspondence(mut self, face_id: impl Into<String>) -> Self {
        self.result
            .face_triangle_correspondence
            .push(TessellatedFace {
                face_id: face_id.into(),
                triangle_indices: vec![0],
            });
        self
    }

    pub fn invalid_face_triangle_correspondence(mut self) -> Self {
        if let Some(entry) = self.result.face_triangle_correspondence.first_mut() {
            entry.triangle_indices.clear();
        }
        self
    }

    pub fn uncovered_triangle(mut self) -> Self {
        if let Some(entry) = self.result.face_triangle_correspondence.first_mut() {
            entry.triangle_indices.truncate(1);
        }
        self
    }

    pub fn clear_edges(mut self) -> Self {
        self.result.view_elements.edge_samples.clear();
        self
    }

    pub fn duplicate_edges(mut self) -> Self {
        if let Some(entry) = self.result.view_elements.edge_samples.first().cloned() {
            self.result.view_elements.edge_samples.push(entry);
        }
        self
    }

    pub fn unknown_edge(mut self, identity: impl Into<String>) -> Self {
        self.result
            .view_elements
            .edge_samples
            .push(TessellatedCurve {
                identity: identity.into(),
                points: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
            });
        self
    }

    pub fn invalid_edges(mut self) -> Self {
        if let Some(entry) = self.result.view_elements.edge_samples.first_mut() {
            entry.points.clear();
        }
        self
    }

    pub fn clear_trims(mut self) -> Self {
        self.result.view_elements.trim_samples.clear();
        self
    }

    pub fn duplicate_trims(mut self) -> Self {
        if let Some(entry) = self.result.view_elements.trim_samples.first().cloned() {
            self.result.view_elements.trim_samples.push(entry);
        }
        self
    }

    pub fn unknown_trim(mut self, identity: impl Into<String>) -> Self {
        self.result
            .view_elements
            .trim_samples
            .push(TessellatedCurve {
                identity: identity.into(),
                points: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
            });
        self
    }

    pub fn invalid_trims(mut self) -> Self {
        if let Some(entry) = self.result.view_elements.trim_samples.first_mut() {
            entry.points.clear();
        }
        self
    }

    pub fn clear_vertices(mut self) -> Self {
        self.result.view_elements.vertex_positions.clear();
        self
    }

    pub fn duplicate_vertices(mut self) -> Self {
        if let Some(entry) = self.result.view_elements.vertex_positions.first().cloned() {
            self.result.view_elements.vertex_positions.push(entry);
        }
        self
    }

    pub fn unknown_vertex(mut self, identity: impl Into<String>) -> Self {
        self.result
            .view_elements
            .vertex_positions
            .push(TessellatedVertex {
                vertex_id: identity.into(),
                position: [0.0, 0.0, 0.0],
            });
        self
    }

    pub fn invalid_vertices(mut self) -> Self {
        if let Some(entry) = self.result.view_elements.vertex_positions.first_mut() {
            entry.position = [f64::NAN, 0.0, 0.0];
        }
        self
    }

    pub fn clear_control_points(mut self) -> Self {
        self.result.view_elements.control_points.clear();
        self
    }

    pub fn duplicate_control_points(mut self) -> Self {
        if let Some(entry) = self.result.view_elements.control_points.first().cloned() {
            self.result.view_elements.control_points.push(entry);
        }
        self
    }

    pub fn unknown_control_point(mut self, surface_representation_id: impl Into<String>) -> Self {
        self.result
            .view_elements
            .control_points
            .push(TessellatedControlPoint {
                surface_representation_id: surface_representation_id.into(),
                row: 0,
                column: 0,
                position: [0.0, 0.0, 0.0],
            });
        self
    }

    pub fn invalid_control_points(mut self) -> Self {
        if let Some(entry) = self.result.view_elements.control_points.first_mut() {
            entry.position = [f64::NAN, 0.0, 0.0];
        }
        self
    }

    pub fn into_result(self) -> TessellationResult {
        self.result
    }
}

fn project_coordinate_f64(value: &RealAlgebraic) -> Result<f64, TessellationFailure> {
    Ok(project_real_algebraic(value, 48)?.value())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_rejects_inconsistent_vertex_endpoints() {
        let mut positions = BTreeMap::new();
        insert_vertex_position(&mut positions, "vertex", [0.0, 0.0, 0.0]).unwrap();
        assert_eq!(
            insert_vertex_position(&mut positions, "vertex", [1.0, 0.0, 0.0]),
            Err(TessellationFailure::InconsistentVertexPosition {
                vertex_id: String::from("vertex"),
            })
        );
    }
}
