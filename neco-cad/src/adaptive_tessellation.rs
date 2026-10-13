use crate::numeric_projection::{
    project_numeric_coordinates, NumericProjectionRequest, NumericSample,
    DEFAULT_NUMERIC_PROJECTION_BITS,
};
use crate::provenance::SourceIdentity;
use crate::types::{AdaptiveTessellationCondition, CadFailure};
use alloc::boxed::Box;
use alloc::collections::{btree_map::Entry, BTreeMap};
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::cmp::Ordering;
use neco_algnum::RealAlgebraic;
use neco_bigint::{BigInt, BigUint, Dyadic, RawRational, ReducedRational, Sign};
use neco_brep::AnalyticBrep;
use neco_calculus::{RationalSurfacePolynomials, SurfacePolynomial, SurfaceTerm};
use neco_geometry::{
    CanonicalAtom, ChartDisplayName, ChartDomainId, PairedSurfaceSeam, ParameterInterval,
    ParametricSurfaceChart2, Point3, SeamEnd, SurfaceChartIdentity, SurfaceOrientation,
    SurfaceParameterDomain, SurfaceParameterPoint, SurfaceTrimLoop2, TrimClassification,
    TrimmedSurfaceDeclaration,
};
use neco_mesh::{
    validate_closed_oriented, validate_surface_mesh, ClosedOrientedSurfaceMesh, MeshFailure,
    MeshSource, SurfaceMesh, ValidatedSurfaceMesh,
};
use neco_nurbs::{define_nurbs, BezierPoint, NurbsDefinitionRequest, NurbsTypes};
use neco_tessellation::{
    tessellate_view_elements, AdaptivePrecision, AdaptivePrecisionWitness,
    BrepElementCorrespondence, TessellatedFace, TessellatedViewElements,
};

pub const DEFAULT_MAX_SUBDIVISION_DEPTH: u32 = 8;

fn failure(condition: AdaptiveTessellationCondition, identity: Option<String>) -> CadFailure {
    CadFailure::AdaptiveTessellation {
        condition,
        identity,
    }
}

fn algebraic(value: i32) -> Result<RealAlgebraic, CadFailure> {
    RealAlgebraic::from_integer(
        BigInt::try_from(value)
            .map_err(|_| failure(AdaptiveTessellationCondition::SurfaceEvaluation, None))?,
    )
    .map_err(|_| failure(AdaptiveTessellationCondition::SurfaceEvaluation, None))
}

fn ratio(numerator: i32, denominator: i32) -> Result<RealAlgebraic, CadFailure> {
    let numerator = BigInt::try_from(numerator)
        .map_err(|_| failure(AdaptiveTessellationCondition::SurfaceEvaluation, None))?;
    let denominator = BigInt::try_from(denominator)
        .map_err(|_| failure(AdaptiveTessellationCondition::SurfaceEvaluation, None))?;
    if denominator.sign() == Sign::Zero {
        return Err(failure(
            AdaptiveTessellationCondition::SurfaceEvaluation,
            None,
        ));
    }
    let (numerator, denominator) = if denominator.sign() == Sign::Negative {
        (
            numerator
                .negated()
                .map_err(|_| failure(AdaptiveTessellationCondition::SurfaceEvaluation, None))?,
            denominator
                .magnitude()
                .try_clone()
                .map_err(|_| failure(AdaptiveTessellationCondition::SurfaceEvaluation, None))?,
        )
    } else {
        (
            numerator,
            denominator
                .magnitude()
                .try_clone()
                .map_err(|_| failure(AdaptiveTessellationCondition::SurfaceEvaluation, None))?,
        )
    };
    from_rational(
        &RawRational::new(numerator, denominator)
            .reduce()
            .map_err(|_| failure(AdaptiveTessellationCondition::SurfaceEvaluation, None))?
            .into_reduced(),
        "",
    )
}

fn as_rational(value: &RealAlgebraic, identity: &str) -> Result<ReducedRational, CadFailure> {
    let coefficients = value.minimal_polynomial().polynomial().coefficients();
    if coefficients.len() != 2 {
        return Err(failure(
            AdaptiveTessellationCondition::SurfaceEvaluation,
            Some(String::from(identity)),
        ));
    }
    let mut numerator = coefficients[0].negated().map_err(|_| {
        failure(
            AdaptiveTessellationCondition::SurfaceEvaluation,
            Some(String::from(identity)),
        )
    })?;
    if coefficients[1].sign() == Sign::Negative {
        numerator = numerator.negated().map_err(|_| {
            failure(
                AdaptiveTessellationCondition::SurfaceEvaluation,
                Some(String::from(identity)),
            )
        })?;
    }
    if coefficients[1].sign() == Sign::Zero {
        return Err(failure(
            AdaptiveTessellationCondition::SurfaceEvaluation,
            Some(String::from(identity)),
        ));
    }
    let denominator = coefficients[1].magnitude().try_clone().map_err(|_| {
        failure(
            AdaptiveTessellationCondition::SurfaceEvaluation,
            Some(String::from(identity)),
        )
    })?;
    RawRational::new(numerator, denominator)
        .reduce()
        .map_err(|_| {
            failure(
                AdaptiveTessellationCondition::SurfaceEvaluation,
                Some(String::from(identity)),
            )
        })
        .map(neco_bigint::RationalReduction::into_reduced)
}

fn from_rational(value: &ReducedRational, identity: &str) -> Result<RealAlgebraic, CadFailure> {
    RealAlgebraic::from_reduced_rational(value).map_err(|_| {
        failure(
            AdaptiveTessellationCondition::SurfaceEvaluation,
            Some(String::from(identity)),
        )
    })
}

fn integer_rational(value: i32, identity: &str) -> Result<ReducedRational, CadFailure> {
    ReducedRational::from_bigint(BigInt::try_from(value).map_err(|_| {
        failure(
            AdaptiveTessellationCondition::SurfaceEvaluation,
            Some(String::from(identity)),
        )
    })?)
    .map_err(|_| {
        failure(
            AdaptiveTessellationCondition::SurfaceEvaluation,
            Some(String::from(identity)),
        )
    })
}

fn clone_algebraic(value: &RealAlgebraic, identity: &str) -> Result<RealAlgebraic, CadFailure> {
    value.try_clone().map_err(|_| {
        failure(
            AdaptiveTessellationCondition::SurfaceEvaluation,
            Some(String::from(identity)),
        )
    })
}

fn clone_point(point: &Point3, identity: &str) -> Result<Point3, CadFailure> {
    Ok(Point3 {
        x: clone_algebraic(&point.x, identity)?,
        y: clone_algebraic(&point.y, identity)?,
        z: clone_algebraic(&point.z, identity)?,
    })
}

fn rational(value: i32) -> Result<neco_bigint::ReducedRational, CadFailure> {
    Ok(RawRational::new(
        BigInt::try_from(value)
            .map_err(|_| failure(AdaptiveTessellationCondition::SurfaceEvaluation, None))?,
        BigUint::one()
            .map_err(|_| failure(AdaptiveTessellationCondition::SurfaceEvaluation, None))?,
    )
    .reduce()
    .map_err(|_| failure(AdaptiveTessellationCondition::SurfaceEvaluation, None))?
    .into_reduced())
}

fn surface_term(
    coefficient: i32,
    exponent_u: u32,
    exponent_v: u32,
) -> Result<SurfaceTerm, CadFailure> {
    Ok(SurfaceTerm {
        coefficient: rational(coefficient)?,
        exponent_u,
        exponent_v,
    })
}

fn surface_polynomial(terms: &[(i32, u32, u32)]) -> Result<SurfacePolynomial, CadFailure> {
    let mut values = Vec::new();
    for (coefficient, exponent_u, exponent_v) in terms {
        values.push(surface_term(*coefficient, *exponent_u, *exponent_v)?);
    }
    SurfacePolynomial::try_from_terms(values)
        .map_err(|_| failure(AdaptiveTessellationCondition::SurfaceEvaluation, None))
}

fn clone_rational(value: &ReducedRational, identity: &str) -> Result<ReducedRational, CadFailure> {
    value.try_clone().map_err(|_| {
        failure(
            AdaptiveTessellationCondition::SurfaceEvaluation,
            Some(String::from(identity)),
        )
    })
}

fn surface_polynomial_rational(
    terms: Vec<(ReducedRational, u32, u32)>,
    identity: &str,
) -> Result<SurfacePolynomial, CadFailure> {
    SurfacePolynomial::try_from_terms(
        terms
            .into_iter()
            .map(|(coefficient, exponent_u, exponent_v)| SurfaceTerm {
                coefficient,
                exponent_u,
                exponent_v,
            })
            .collect(),
    )
    .map_err(|_| {
        failure(
            AdaptiveTessellationCondition::SurfaceEvaluation,
            Some(String::from(identity)),
        )
    })
}

fn identity_chart(domain_id: &str) -> Result<ParametricSurfaceChart2, CadFailure> {
    ParametricSurfaceChart2::try_new(
        ChartDomainId::new(String::from(domain_id))
            .map_err(|_| failure(AdaptiveTessellationCondition::SurfaceEvaluation, None))?,
        ChartDisplayName::new(String::from(domain_id))
            .map_err(|_| failure(AdaptiveTessellationCondition::SurfaceEvaluation, None))?,
        1,
        SurfaceOrientation::Preserve,
        RationalSurfacePolynomials {
            x: surface_polynomial(&[(1, 1, 0)])?,
            y: surface_polynomial(&[(1, 0, 1)])?,
            z: surface_polynomial(&[(0, 0, 0)])?,
            denominator: surface_polynomial(&[(1, 0, 0)])?,
        },
    )
    .map_err(CadFailure::from)
}

fn rational_add(
    left: &ReducedRational,
    right: &ReducedRational,
    identity: &str,
) -> Result<ReducedRational, CadFailure> {
    left.add(right).map_err(|_| {
        failure(
            AdaptiveTessellationCondition::SurfaceEvaluation,
            Some(String::from(identity)),
        )
    })
}

fn rational_sub(
    left: &ReducedRational,
    right: &ReducedRational,
    identity: &str,
) -> Result<ReducedRational, CadFailure> {
    left.sub(right).map_err(|_| {
        failure(
            AdaptiveTessellationCondition::SurfaceEvaluation,
            Some(String::from(identity)),
        )
    })
}

fn rational_mul(
    left: &ReducedRational,
    right: &ReducedRational,
    identity: &str,
) -> Result<ReducedRational, CadFailure> {
    left.mul(right).map_err(|_| {
        failure(
            AdaptiveTessellationCondition::SurfaceEvaluation,
            Some(String::from(identity)),
        )
    })
}

fn rational_div(
    left: &ReducedRational,
    right: &ReducedRational,
    identity: &str,
) -> Result<ReducedRational, CadFailure> {
    left.div(right).map_err(|_| {
        failure(
            AdaptiveTessellationCondition::SurfaceEvaluation,
            Some(String::from(identity)),
        )
    })
}

fn rational_negate(value: &ReducedRational, identity: &str) -> Result<ReducedRational, CadFailure> {
    rational_sub(&integer_rational(0, identity)?, value, identity)
}

fn affine_plane_chart(
    domain_id: &str,
    normal: &[ReducedRational; 3],
    constant: &ReducedRational,
    pivot_axis: usize,
    orientation: SurfaceOrientation,
) -> Result<ParametricSurfaceChart2, CadFailure> {
    let identity = domain_id;
    let one = integer_rational(1, identity)?;
    let parameter_axes: Vec<usize> = (0..3).filter(|axis| *axis != pivot_axis).collect();
    if parameter_axes.len() != 2 || normal[pivot_axis].is_zero() {
        return Err(failure(
            AdaptiveTessellationCondition::SurfaceEvaluation,
            Some(String::from(identity)),
        ));
    }
    let coordinate_polynomial = |axis: usize| -> Result<SurfacePolynomial, CadFailure> {
        if axis == parameter_axes[0] {
            return surface_polynomial_rational(
                vec![(clone_rational(&one, identity)?, 1, 0)],
                identity,
            );
        }
        if axis == parameter_axes[1] {
            return surface_polynomial_rational(
                vec![(clone_rational(&one, identity)?, 0, 1)],
                identity,
            );
        }
        surface_polynomial_rational(
            vec![
                (
                    rational_div(
                        &rational_negate(constant, identity)?,
                        &normal[pivot_axis],
                        identity,
                    )?,
                    0,
                    0,
                ),
                (
                    rational_div(
                        &rational_negate(&normal[parameter_axes[0]], identity)?,
                        &normal[pivot_axis],
                        identity,
                    )?,
                    1,
                    0,
                ),
                (
                    rational_div(
                        &rational_negate(&normal[parameter_axes[1]], identity)?,
                        &normal[pivot_axis],
                        identity,
                    )?,
                    0,
                    1,
                ),
            ],
            identity,
        )
    };
    ParametricSurfaceChart2::try_new(
        ChartDomainId::new(String::from(domain_id))
            .map_err(|_| failure(AdaptiveTessellationCondition::SurfaceEvaluation, None))?,
        ChartDisplayName::new(String::from(domain_id))
            .map_err(|_| failure(AdaptiveTessellationCondition::SurfaceEvaluation, None))?,
        1,
        orientation,
        RationalSurfacePolynomials {
            x: coordinate_polynomial(0)?,
            y: coordinate_polynomial(1)?,
            z: coordinate_polynomial(2)?,
            denominator: surface_polynomial_rational(vec![(one, 0, 0)], identity)?,
        },
    )
    .map_err(CadFailure::from)
}

fn stereographic_chart_rational(
    domain_id: &str,
    south_from: bool,
    center: &[ReducedRational; 3],
    radius: &ReducedRational,
) -> Result<ParametricSurfaceChart2, CadFailure> {
    let identity = domain_id;
    let one = integer_rational(1, identity)?;
    let two = integer_rational(2, identity)?;
    let twice_radius = rational_mul(radius, &two, identity)?;
    let x = surface_polynomial_rational(
        vec![
            (clone_rational(&center[0], identity)?, 0, 0),
            (clone_rational(&center[0], identity)?, 2, 0),
            (clone_rational(&center[0], identity)?, 0, 2),
            (twice_radius, 1, 0),
        ],
        identity,
    )?;
    let y = surface_polynomial_rational(
        vec![
            (clone_rational(&center[1], identity)?, 0, 0),
            (clone_rational(&center[1], identity)?, 2, 0),
            (clone_rational(&center[1], identity)?, 0, 2),
            (rational_mul(radius, &two, identity)?, 0, 1),
        ],
        identity,
    )?;
    let (constant, quadratic) = if south_from {
        (
            rational_add(&center[2], radius, identity)?,
            rational_sub(&center[2], radius, identity)?,
        )
    } else {
        (
            rational_sub(&center[2], radius, identity)?,
            rational_add(&center[2], radius, identity)?,
        )
    };
    let z = surface_polynomial_rational(
        vec![
            (constant, 0, 0),
            (clone_rational(&quadratic, identity)?, 2, 0),
            (quadratic, 0, 2),
        ],
        identity,
    )?;
    ParametricSurfaceChart2::try_new(
        ChartDomainId::new(String::from(domain_id))
            .map_err(|_| failure(AdaptiveTessellationCondition::SurfaceEvaluation, None))?,
        ChartDisplayName::new(String::from(domain_id))
            .map_err(|_| failure(AdaptiveTessellationCondition::SurfaceEvaluation, None))?,
        1,
        if south_from {
            SurfaceOrientation::Preserve
        } else {
            SurfaceOrientation::Reverse
        },
        RationalSurfacePolynomials {
            x,
            y,
            z,
            denominator: surface_polynomial_rational(
                vec![
                    (one, 0, 0),
                    (integer_rational(1, identity)?, 2, 0),
                    (integer_rational(1, identity)?, 0, 2),
                ],
                identity,
            )?,
        },
    )
    .map_err(CadFailure::from)
}

fn unit_square_domain() -> Result<SurfaceParameterDomain, CadFailure> {
    Ok(SurfaceParameterDomain::new(
        ParameterInterval::new(algebraic(-1)?, algebraic(1)?, true, true)
            .map_err(CadFailure::from)?,
        ParameterInterval::new(algebraic(-1)?, algebraic(1)?, true, true)
            .map_err(CadFailure::from)?,
    ))
}

fn parameter_point(
    u: RealAlgebraic,
    v: RealAlgebraic,
) -> Result<SurfaceParameterPoint, CadFailure> {
    Ok(SurfaceParameterPoint::new(u, v))
}

fn chart_identity(chart: &ParametricSurfaceChart2) -> Result<SurfaceChartIdentity, CadFailure> {
    Ok(SurfaceChartIdentity::new(
        chart
            .identity()
            .domain_id()
            .try_clone()
            .map_err(|_| failure(AdaptiveTessellationCondition::SeamUnpaired, None))?,
        chart.identity().version(),
    ))
}

fn equator_seam(
    first: &ParametricSurfaceChart2,
    second: &ParametricSurfaceChart2,
    u: RealAlgebraic,
    v: RealAlgebraic,
) -> Result<PairedSurfaceSeam, CadFailure> {
    Ok(PairedSurfaceSeam::new(
        SeamEnd::new(
            chart_identity(first)?,
            parameter_point(clone_algebraic(&u, "")?, clone_algebraic(&v, "")?)?,
            SurfaceOrientation::Preserve,
        ),
        SeamEnd::new(
            chart_identity(second)?,
            parameter_point(u, v)?,
            SurfaceOrientation::Preserve,
        ),
    ))
}

fn equator_points() -> Result<Vec<(RealAlgebraic, RealAlgebraic)>, CadFailure> {
    Ok(vec![
        (ratio(3, 5)?, ratio(4, 5)?),
        (ratio(-4, 5)?, ratio(3, 5)?),
        (ratio(-3, 5)?, ratio(-4, 5)?),
        (ratio(4, 5)?, ratio(-3, 5)?),
    ])
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiskConstraint;

pub enum ChartPatch {
    Full {
        chart: ParametricSurfaceChart2,
        domain: SurfaceParameterDomain,
        disk: Option<DiskConstraint>,
    },
    Trimmed {
        declaration: TrimmedSurfaceDeclaration,
    },
}

impl ChartPatch {
    pub fn chart(&self) -> &ParametricSurfaceChart2 {
        match self {
            Self::Full { chart, .. } => chart,
            Self::Trimmed { declaration } => declaration.chart(),
        }
    }

    pub fn domain(&self) -> &SurfaceParameterDomain {
        match self {
            Self::Full { domain, .. } => domain,
            Self::Trimmed { declaration } => declaration.domain(),
        }
    }

    fn disk(&self) -> bool {
        matches!(self, Self::Full { disk: Some(_), .. })
    }

    fn trimmed(&self) -> Option<&TrimmedSurfaceDeclaration> {
        match self {
            Self::Trimmed { declaration } => Some(declaration),
            Self::Full { .. } => None,
        }
    }
}

pub struct SurfaceCovering {
    patches: Vec<ChartPatch>,
    seams: Vec<PairedSurfaceSeam>,
}

impl SurfaceCovering {
    pub fn new(patches: Vec<ChartPatch>, seams: Vec<PairedSurfaceSeam>) -> Self {
        Self { patches, seams }
    }

    pub fn patches(&self) -> &[ChartPatch] {
        &self.patches
    }

    pub fn seams(&self) -> &[PairedSurfaceSeam] {
        &self.seams
    }
}

pub(crate) struct SurfaceWeldVertex {
    identity: Vec<String>,
    point: Point3,
}

impl SurfaceWeldVertex {
    pub(crate) fn new(identity: Vec<String>, point: Point3) -> Self {
        Self { identity, point }
    }
}

pub struct FaceExactSurface {
    face_id: String,
    exact_surface: CanonicalAtom,
    covering: SurfaceCovering,
    weld_vertices: Vec<SurfaceWeldVertex>,
}

impl FaceExactSurface {
    pub fn new(
        face_id: impl Into<String>,
        exact_surface: CanonicalAtom,
        covering: SurfaceCovering,
    ) -> Self {
        Self {
            face_id: face_id.into(),
            exact_surface,
            covering,
            weld_vertices: Vec::new(),
        }
    }

    pub(crate) fn with_weld_vertices(mut self, weld_vertices: Vec<SurfaceWeldVertex>) -> Self {
        self.weld_vertices = weld_vertices;
        self
    }

    pub fn face_id(&self) -> &str {
        &self.face_id
    }

    pub fn exact_surface(&self) -> &CanonicalAtom {
        &self.exact_surface
    }

    pub fn covering(&self) -> &SurfaceCovering {
        &self.covering
    }
}

pub struct AdaptiveTessellationRequest {
    identity: SourceIdentity,
    brep: AnalyticBrep,
    surfaces: Vec<FaceExactSurface>,
    precision: AdaptivePrecision,
    enclosure_bits: u32,
}

impl AdaptiveTessellationRequest {
    pub fn new(
        identity: impl Into<String>,
        brep: AnalyticBrep,
        surfaces: Vec<FaceExactSurface>,
        precision: AdaptivePrecision,
        enclosure_bits: u32,
    ) -> Result<Self, CadFailure> {
        let identity = SourceIdentity::new(identity)
            .map_err(|_| failure(AdaptiveTessellationCondition::EmptyIdentity, None))?;
        Ok(Self {
            identity,
            brep,
            surfaces,
            precision,
            enclosure_bits,
        })
    }

    pub fn identity(&self) -> &SourceIdentity {
        &self.identity
    }

    pub fn brep(&self) -> &AnalyticBrep {
        &self.brep
    }

    pub fn surfaces(&self) -> &[FaceExactSurface] {
        &self.surfaces
    }

    pub fn precision(&self) -> AdaptivePrecision {
        self.precision
    }

    pub fn enclosure_bits(&self) -> u32 {
        self.enclosure_bits
    }
}

pub struct AdaptiveTessellationProvenance {
    enclosure_bits: u32,
    face_atom_ids: Vec<(String, String)>,
}

impl AdaptiveTessellationProvenance {
    pub fn enclosure_bits(&self) -> u32 {
        self.enclosure_bits
    }

    pub fn face_atom_ids(&self) -> &[(String, String)] {
        &self.face_atom_ids
    }
}

pub struct AdaptiveTessellation {
    identity: SourceIdentity,
    source_revision: neco_brep::SourceRevision,
    validated_surface_mesh: ValidatedSurfaceMesh,
    precision_witness: AdaptivePrecisionWitness,
    brep_element_correspondence: BrepElementCorrespondence,
    face_triangle_correspondence: Vec<TessellatedFace>,
    provenance: AdaptiveTessellationProvenance,
    chord_bound_witnesses: Vec<FaceChordBoundWitness>,
    view_elements: TessellatedViewElements,
}

impl AdaptiveTessellation {
    pub fn identity(&self) -> &SourceIdentity {
        &self.identity
    }

    pub fn source_revision(&self) -> &neco_brep::SourceRevision {
        &self.source_revision
    }

    pub fn validated_surface_mesh(&self) -> &ValidatedSurfaceMesh {
        &self.validated_surface_mesh
    }

    pub fn into_validated_surface_mesh(self) -> ValidatedSurfaceMesh {
        self.validated_surface_mesh
    }

    pub fn into_closed_oriented_surface_mesh(
        self,
    ) -> Result<ClosedOrientedSurfaceMesh, MeshFailure> {
        let source = MeshSource::new(
            String::from(self.identity.as_str()),
            String::from(self.source_revision.as_str()),
        )?;
        validate_closed_oriented(self.validated_surface_mesh, source)
    }

    pub fn precision_witness(&self) -> &AdaptivePrecisionWitness {
        &self.precision_witness
    }

    pub fn brep_element_correspondence(&self) -> &BrepElementCorrespondence {
        &self.brep_element_correspondence
    }

    pub fn face_triangle_correspondence(&self) -> &[TessellatedFace] {
        &self.face_triangle_correspondence
    }

    pub fn edge_samples(&self) -> &[neco_tessellation::TessellatedCurve] {
        self.view_elements.edge_samples()
    }

    pub fn trim_samples(&self) -> &[neco_tessellation::TessellatedCurve] {
        self.view_elements.trim_samples()
    }

    pub fn vertex_positions(&self) -> &[neco_tessellation::TessellatedVertex] {
        self.view_elements.vertex_positions()
    }

    pub fn control_points(&self) -> &[neco_tessellation::TessellatedControlPoint] {
        self.view_elements.control_points()
    }
    pub fn provenance(&self) -> &AdaptiveTessellationProvenance {
        &self.provenance
    }

    pub fn chord_bound_witnesses(&self) -> &[FaceChordBoundWitness] {
        &self.chord_bound_witnesses
    }
}

pub struct ChordBoundEndpoint {
    u: ReducedRational,
    v: ReducedRational,
    point: [ReducedRational; 3],
}

impl ChordBoundEndpoint {
    pub fn u(&self) -> &ReducedRational {
        &self.u
    }

    pub fn v(&self) -> &ReducedRational {
        &self.v
    }

    pub fn point(&self) -> &[ReducedRational; 3] {
        &self.point
    }
}

pub struct ChordBoundArcRange {
    u_lower: ReducedRational,
    u_upper: ReducedRational,
    v_lower: ReducedRational,
    v_upper: ReducedRational,
}

impl ChordBoundArcRange {
    pub fn u_lower(&self) -> &ReducedRational {
        &self.u_lower
    }

    pub fn u_upper(&self) -> &ReducedRational {
        &self.u_upper
    }

    pub fn v_lower(&self) -> &ReducedRational {
        &self.v_lower
    }

    pub fn v_upper(&self) -> &ReducedRational {
        &self.v_upper
    }
}

pub enum ChordBoundEdgeShape {
    Line,
    Arc(Box<ChordBoundArcRange>),
}

impl ChordBoundEdgeShape {
    pub fn arc_box(
        &self,
    ) -> Option<(
        &ReducedRational,
        &ReducedRational,
        &ReducedRational,
        &ReducedRational,
    )> {
        match self {
            Self::Arc(range) => Some((
                range.u_lower(),
                range.u_upper(),
                range.v_lower(),
                range.v_upper(),
            )),
            Self::Line => None,
        }
    }
}

pub struct ChordBoundEdgeEvaluation {
    patch_index: usize,
    left: ChordBoundEndpoint,
    right: ChordBoundEndpoint,
    midpoint_u: ReducedRational,
    midpoint_v: ReducedRational,
    shape: ChordBoundEdgeShape,
    squared_bound: Option<Dyadic>,
}

impl ChordBoundEdgeEvaluation {
    pub fn patch_index(&self) -> usize {
        self.patch_index
    }

    pub fn left(&self) -> &ChordBoundEndpoint {
        &self.left
    }

    pub fn right(&self) -> &ChordBoundEndpoint {
        &self.right
    }

    pub fn midpoint_u(&self) -> &ReducedRational {
        &self.midpoint_u
    }

    pub fn midpoint_v(&self) -> &ReducedRational {
        &self.midpoint_v
    }

    pub fn shape(&self) -> &ChordBoundEdgeShape {
        &self.shape
    }

    pub fn squared_bound(&self) -> Option<&Dyadic> {
        self.squared_bound.as_ref()
    }
}

pub struct ChordBoundTriangleWitness {
    patch_index: usize,
    edge_indices: [usize; 3],
}

impl ChordBoundTriangleWitness {
    pub fn patch_index(&self) -> usize {
        self.patch_index
    }

    pub fn edge_indices(&self) -> &[usize; 3] {
        &self.edge_indices
    }
}

pub struct FaceChordBoundWitness {
    face_id: String,
    established_squared_bound: Option<Dyadic>,
    edges: Vec<ChordBoundEdgeEvaluation>,
    triangles: Vec<ChordBoundTriangleWitness>,
}

impl FaceChordBoundWitness {
    pub fn face_id(&self) -> &str {
        &self.face_id
    }

    pub fn established_squared_bound(&self) -> Option<&Dyadic> {
        self.established_squared_bound.as_ref()
    }

    pub fn edges(&self) -> &[ChordBoundEdgeEvaluation] {
        &self.edges
    }

    pub fn triangles(&self) -> &[ChordBoundTriangleWitness] {
        &self.triangles
    }
}

pub fn plane_surface_covering(
    domain: SurfaceParameterDomain,
    outer: Option<SurfaceTrimLoop2>,
    holes: Vec<SurfaceTrimLoop2>,
) -> Result<SurfaceCovering, CadFailure> {
    let chart = identity_chart("plane")?;
    let patch = match outer {
        Some(outer) => ChartPatch::Trimmed {
            declaration: TrimmedSurfaceDeclaration::try_new(
                chart,
                domain,
                outer,
                holes,
                Vec::new(),
            )
            .map_err(CadFailure::from)?,
        },
        None => ChartPatch::Full {
            chart,
            domain,
            disk: None,
        },
    };
    Ok(SurfaceCovering::new(vec![patch], Vec::new()))
}

fn rational_point(
    x: ReducedRational,
    y: ReducedRational,
    z: ReducedRational,
    identity: &str,
) -> Result<BezierPoint, CadFailure> {
    Ok(BezierPoint::Point3(Point3 {
        x: from_rational(&x, identity)?,
        y: from_rational(&y, identity)?,
        z: from_rational(&z, identity)?,
    }))
}

pub(crate) fn affine_plane_nurbs_surface(
    normal: &[RealAlgebraic; 3],
    constant: &RealAlgebraic,
    pivot_axis: usize,
) -> Result<NurbsTypes, CadFailure> {
    let identity = "affine-plane";
    let normal = [
        as_rational(&normal[0], identity)?,
        as_rational(&normal[1], identity)?,
        as_rational(&normal[2], identity)?,
    ];
    let constant = as_rational(constant, identity)?;
    let parameter_axes: Vec<usize> = (0..3).filter(|axis| *axis != pivot_axis).collect();
    if parameter_axes.len() != 2 || normal[pivot_axis].is_zero() {
        return Err(failure(
            AdaptiveTessellationCondition::SurfaceEvaluation,
            Some(String::from(identity)),
        ));
    }
    let point = |u: &ReducedRational, v: &ReducedRational| {
        let pivot = rational_div(
            &rational_negate(
                &rational_add(
                    &constant,
                    &rational_add(
                        &rational_mul(&normal[parameter_axes[0]], u, identity)?,
                        &rational_mul(&normal[parameter_axes[1]], v, identity)?,
                        identity,
                    )?,
                    identity,
                )?,
                identity,
            )?,
            &normal[pivot_axis],
            identity,
        )?;
        let mut coordinates = [
            integer_rational(0, identity)?,
            integer_rational(0, identity)?,
            integer_rational(0, identity)?,
        ];
        coordinates[pivot_axis] = pivot;
        coordinates[parameter_axes[0]] = clone_rational(u, identity)?;
        coordinates[parameter_axes[1]] = clone_rational(v, identity)?;
        rational_point(
            coordinates[0].try_clone().map_err(|_| {
                failure(
                    AdaptiveTessellationCondition::SurfaceEvaluation,
                    Some(String::from(identity)),
                )
            })?,
            coordinates[1].try_clone().map_err(|_| {
                failure(
                    AdaptiveTessellationCondition::SurfaceEvaluation,
                    Some(String::from(identity)),
                )
            })?,
            coordinates[2].try_clone().map_err(|_| {
                failure(
                    AdaptiveTessellationCondition::SurfaceEvaluation,
                    Some(String::from(identity)),
                )
            })?,
            identity,
        )
    };
    let minus_one = integer_rational(-1, identity)?;
    let plus_one = integer_rational(1, identity)?;
    define_nurbs(NurbsDefinitionRequest::PolynomialSurface {
        control_net: vec![
            vec![
                point(&minus_one, &minus_one)?,
                point(&minus_one, &plus_one)?,
            ],
            vec![point(&plus_one, &minus_one)?, point(&plus_one, &plus_one)?],
        ],
        u_knots: vec![algebraic(0)?, algebraic(0)?, algebraic(1)?, algebraic(1)?],
        v_knots: vec![algebraic(0)?, algebraic(0)?, algebraic(1)?, algebraic(1)?],
        u_degree: 1,
        v_degree: 1,
    })
    .map_err(|_| {
        failure(
            AdaptiveTessellationCondition::SurfaceEvaluation,
            Some(String::from(identity)),
        )
    })
}

pub(crate) fn sphere_nurbs_surface(
    center: &Point3,
    radius: &RealAlgebraic,
) -> Result<NurbsTypes, CadFailure> {
    let identity = "sphere";
    let center = [
        as_rational(&center.x, identity)?,
        as_rational(&center.y, identity)?,
        as_rational(&center.z, identity)?,
    ];
    let radius = as_rational(radius, identity)?;
    let weights = [[1, 1, 2], [1, 1, 2], [2, 2, 3]];
    let mut control_net = Vec::new();
    let mut weight_net = Vec::new();
    for (u, weights_row) in weights.iter().enumerate() {
        let mut control_row = Vec::new();
        let mut weight_row = Vec::new();
        for (v, weight_value) in weights_row.iter().enumerate() {
            let weight = integer_rational(*weight_value, identity)?;
            let x_offset = match u {
                0 => integer_rational(0, identity)?,
                1 => clone_rational(&radius, identity)?,
                _ => rational_mul(&integer_rational(2, identity)?, &radius, identity)?,
            };
            let y_offset = match v {
                0 => integer_rational(0, identity)?,
                1 => clone_rational(&radius, identity)?,
                _ => rational_mul(&integer_rational(2, identity)?, &radius, identity)?,
            };
            let z_offset = match (u, v) {
                (2, _) | (_, 2) if u == 2 && v == 2 => rational_negate(&radius, identity)?,
                (2, _) | (_, 2) => integer_rational(0, identity)?,
                _ => clone_rational(&radius, identity)?,
            };
            let x = rational_div(
                &rational_add(
                    &rational_mul(&center[0], &weight, identity)?,
                    &x_offset,
                    identity,
                )?,
                &weight,
                identity,
            )?;
            let y = rational_div(
                &rational_add(
                    &rational_mul(&center[1], &weight, identity)?,
                    &y_offset,
                    identity,
                )?,
                &weight,
                identity,
            )?;
            let z = rational_div(
                &rational_add(
                    &rational_mul(&center[2], &weight, identity)?,
                    &z_offset,
                    identity,
                )?,
                &weight,
                identity,
            )?;
            control_row.push(rational_point(x, y, z, identity)?);
            weight_row.push(from_rational(&weight, identity)?);
        }
        control_net.push(control_row);
        weight_net.push(weight_row);
    }
    define_nurbs(NurbsDefinitionRequest::RationalSurface {
        control_net,
        weights: weight_net,
        u_knots: vec![
            algebraic(0)?,
            algebraic(0)?,
            algebraic(0)?,
            algebraic(1)?,
            algebraic(1)?,
            algebraic(1)?,
        ],
        v_knots: vec![
            algebraic(0)?,
            algebraic(0)?,
            algebraic(0)?,
            algebraic(1)?,
            algebraic(1)?,
            algebraic(1)?,
        ],
        u_degree: 2,
        v_degree: 2,
    })
    .map_err(|_| {
        failure(
            AdaptiveTessellationCondition::SurfaceEvaluation,
            Some(String::from(identity)),
        )
    })
}

pub(crate) fn affine_plane_surface_covering(
    normal: &[RealAlgebraic; 3],
    constant: &RealAlgebraic,
    pivot_axis: usize,
    orientation: SurfaceOrientation,
    domain: SurfaceParameterDomain,
    outer: SurfaceTrimLoop2,
) -> Result<SurfaceCovering, CadFailure> {
    let identity = "affine-plane";
    let normal = [
        as_rational(&normal[0], identity)?,
        as_rational(&normal[1], identity)?,
        as_rational(&normal[2], identity)?,
    ];
    let constant = as_rational(constant, identity)?;
    let chart = affine_plane_chart(identity, &normal, &constant, pivot_axis, orientation)?;
    Ok(SurfaceCovering::new(
        vec![ChartPatch::Trimmed {
            declaration: TrimmedSurfaceDeclaration::try_new(
                chart,
                domain,
                outer,
                Vec::new(),
                Vec::new(),
            )
            .map_err(CadFailure::from)?,
        }],
        Vec::new(),
    ))
}

pub(crate) fn sphere_surface_covering(
    center: &Point3,
    radius: &RealAlgebraic,
) -> Result<SurfaceCovering, CadFailure> {
    let center = [
        as_rational(&center.x, "south-from")?,
        as_rational(&center.y, "south-from")?,
        as_rational(&center.z, "south-from")?,
    ];
    let radius = as_rational(radius, "south-from")?;
    let south = stereographic_chart_rational("south-from", true, &center, &radius)?;
    let north = stereographic_chart_rational("north-from", false, &center, &radius)?;
    let mut seams = Vec::new();
    for (u, v) in equator_points()? {
        seams.push(equator_seam(&south, &north, u, v)?);
    }
    Ok(SurfaceCovering::new(
        vec![
            ChartPatch::Full {
                chart: south,
                domain: unit_square_domain()?,
                disk: Some(DiskConstraint),
            },
            ChartPatch::Full {
                chart: north,
                domain: unit_square_domain()?,
                disk: Some(DiskConstraint),
            },
        ],
        seams,
    ))
}

pub fn unit_sphere_stereographic_covering() -> Result<SurfaceCovering, CadFailure> {
    sphere_surface_covering(
        &Point3 {
            x: algebraic(0)?,
            y: algebraic(0)?,
            z: algebraic(0)?,
        },
        &algebraic(1)?,
    )
}

enum SampleKind {
    Keep,
    Discard,
}

enum TriangleKind {
    Keep,
    Discard,
    Mixed,
}

fn parameters_equal(
    left_u: &RealAlgebraic,
    left_v: &RealAlgebraic,
    right_u: &RealAlgebraic,
    right_v: &RealAlgebraic,
    identity: &str,
) -> Result<bool, CadFailure> {
    Ok(
        as_rational(left_u, identity)?.cmp(&as_rational(right_u, identity)?) == Ordering::Equal
            && as_rational(left_v, identity)?.cmp(&as_rational(right_v, identity)?)
                == Ordering::Equal,
    )
}

fn interval_contains(
    interval: &ParameterInterval,
    value: &ReducedRational,
    identity: &str,
) -> Result<bool, CadFailure> {
    let lower = as_rational(interval.lower(), identity)?;
    let upper = as_rational(interval.upper(), identity)?;
    let after_lower = match value.cmp(&lower) {
        Ordering::Less => false,
        Ordering::Equal => interval.lower_closed(),
        Ordering::Greater => true,
    };
    let before_upper = match value.cmp(&upper) {
        Ordering::Greater => false,
        Ordering::Equal => interval.upper_closed(),
        Ordering::Less => true,
    };
    Ok(after_lower && before_upper)
}

fn domain_contains(
    domain: &SurfaceParameterDomain,
    u: &RealAlgebraic,
    v: &RealAlgebraic,
    identity: &str,
) -> Result<bool, CadFailure> {
    let u = as_rational(u, identity)?;
    let v = as_rational(v, identity)?;
    Ok(
        interval_contains(domain.u(), &u, identity)?
            && interval_contains(domain.v(), &v, identity)?,
    )
}

fn segment_orientation(
    u: &ReducedRational,
    v: &ReducedRational,
    start: &SurfaceParameterPoint,
    end: &SurfaceParameterPoint,
    identity: &str,
) -> Result<Ordering, CadFailure> {
    let start_u = as_rational(start.u(), identity)?;
    let start_v = as_rational(start.v(), identity)?;
    let end_u = as_rational(end.u(), identity)?;
    let end_v = as_rational(end.v(), identity)?;
    let dx = end_u.sub(&start_u).map_err(|_| {
        failure(
            AdaptiveTessellationCondition::TrimClassification,
            Some(String::from(identity)),
        )
    })?;
    let dy = end_v.sub(&start_v).map_err(|_| {
        failure(
            AdaptiveTessellationCondition::TrimClassification,
            Some(String::from(identity)),
        )
    })?;
    let px = u.sub(&start_u).map_err(|_| {
        failure(
            AdaptiveTessellationCondition::TrimClassification,
            Some(String::from(identity)),
        )
    })?;
    let py = v.sub(&start_v).map_err(|_| {
        failure(
            AdaptiveTessellationCondition::TrimClassification,
            Some(String::from(identity)),
        )
    })?;
    let cross = dx
        .mul(&py)
        .map_err(|_| {
            failure(
                AdaptiveTessellationCondition::TrimClassification,
                Some(String::from(identity)),
            )
        })?
        .sub(&dy.mul(&px).map_err(|_| {
            failure(
                AdaptiveTessellationCondition::TrimClassification,
                Some(String::from(identity)),
            )
        })?)
        .map_err(|_| {
            failure(
                AdaptiveTessellationCondition::TrimClassification,
                Some(String::from(identity)),
            )
        })?;
    Ok(cross.cmp(&integer_rational(0, identity)?))
}

fn point_on_segment(
    u: &ReducedRational,
    v: &ReducedRational,
    start: &SurfaceParameterPoint,
    end: &SurfaceParameterPoint,
    identity: &str,
) -> Result<bool, CadFailure> {
    if segment_orientation(u, v, start, end, identity)? != Ordering::Equal {
        return Ok(false);
    }
    let start_u = as_rational(start.u(), identity)?;
    let start_v = as_rational(start.v(), identity)?;
    let end_u = as_rational(end.u(), identity)?;
    let end_v = as_rational(end.v(), identity)?;
    let min_u = if start_u.cmp(&end_u) == Ordering::Less {
        &start_u
    } else {
        &end_u
    };
    let max_u = if start_u.cmp(&end_u) == Ordering::Greater {
        &start_u
    } else {
        &end_u
    };
    let min_v = if start_v.cmp(&end_v) == Ordering::Less {
        &start_v
    } else {
        &end_v
    };
    let max_v = if start_v.cmp(&end_v) == Ordering::Greater {
        &start_v
    } else {
        &end_v
    };
    Ok(u.cmp(min_u) != Ordering::Less
        && u.cmp(max_u) != Ordering::Greater
        && v.cmp(min_v) != Ordering::Less
        && v.cmp(max_v) != Ordering::Greater)
}

fn point_on_loop(
    u: &ReducedRational,
    v: &ReducedRational,
    vertices: &[SurfaceParameterPoint],
    identity: &str,
) -> Result<bool, CadFailure> {
    for index in 0..vertices.len() {
        let next = (index + 1) % vertices.len();
        if point_on_segment(u, v, &vertices[index], &vertices[next], identity)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn point_in_loop(
    u: &ReducedRational,
    v: &ReducedRational,
    vertices: &[SurfaceParameterPoint],
    identity: &str,
) -> Result<bool, CadFailure> {
    let mut winding = 0_i32;
    for index in 0..vertices.len() {
        let current = &vertices[index];
        let next = &vertices[(index + 1) % vertices.len()];
        let current_v = as_rational(current.v(), identity)?.cmp(v);
        let next_v = as_rational(next.v(), identity)?.cmp(v);
        if current_v == Ordering::Greater
            && next_v != Ordering::Greater
            && segment_orientation(u, v, current, next, identity)? == Ordering::Greater
        {
            winding += 1;
        } else if current_v != Ordering::Greater
            && next_v == Ordering::Greater
            && segment_orientation(u, v, current, next, identity)? == Ordering::Less
        {
            winding -= 1;
        }
    }
    Ok(winding != 0)
}

fn classify_trimmed(
    declaration: &TrimmedSurfaceDeclaration,
    u: &RealAlgebraic,
    v: &RealAlgebraic,
    identity: &str,
) -> Result<TrimClassification, CadFailure> {
    let u = as_rational(u, identity)?;
    let v = as_rational(v, identity)?;
    if point_on_loop(&u, &v, declaration.outer().vertices(), identity)? {
        return Ok(TrimClassification::TrimBoundary);
    }
    for hole in declaration.holes() {
        if point_on_loop(&u, &v, hole.vertices(), identity)? {
            return Ok(TrimClassification::TrimBoundary);
        }
    }
    if !point_in_loop(&u, &v, declaration.outer().vertices(), identity)? {
        return Ok(TrimClassification::TrimExterior);
    }
    for hole in declaration.holes() {
        if point_in_loop(&u, &v, hole.vertices(), identity)? {
            return Ok(TrimClassification::Hole);
        }
    }
    Ok(TrimClassification::Outer)
}

fn inside_disk(u: &RealAlgebraic, v: &RealAlgebraic, identity: &str) -> Result<bool, CadFailure> {
    let u = as_rational(u, identity)?;
    let v = as_rational(v, identity)?;
    let square = u
        .mul(&u)
        .map_err(|_| {
            failure(
                AdaptiveTessellationCondition::TrimClassification,
                Some(String::from(identity)),
            )
        })?
        .add(&v.mul(&v).map_err(|_| {
            failure(
                AdaptiveTessellationCondition::TrimClassification,
                Some(String::from(identity)),
            )
        })?)
        .map_err(|_| {
            failure(
                AdaptiveTessellationCondition::TrimClassification,
                Some(String::from(identity)),
            )
        })?;
    match square.cmp(&integer_rational(1, identity)?) {
        Ordering::Greater => Ok(false),
        Ordering::Equal | Ordering::Less => Ok(true),
    }
}

fn classify_sample(
    patch: &ChartPatch,
    u: &RealAlgebraic,
    v: &RealAlgebraic,
    identity: &str,
) -> Result<SampleKind, CadFailure> {
    if !domain_contains(patch.domain(), u, v, identity)? {
        return Ok(SampleKind::Discard);
    }
    if patch.disk() && !inside_disk(u, v, identity)? {
        return Ok(SampleKind::Discard);
    }
    if let Some(declaration) = patch.trimmed() {
        return match classify_trimmed(declaration, u, v, identity)? {
            TrimClassification::Outer | TrimClassification::TrimBoundary => Ok(SampleKind::Keep),
            TrimClassification::Hole | TrimClassification::TrimExterior => Ok(SampleKind::Discard),
        };
    }
    Ok(SampleKind::Keep)
}

fn mid_value(
    left: &RealAlgebraic,
    right: &RealAlgebraic,
    identity: &str,
) -> Result<RealAlgebraic, CadFailure> {
    let sum = as_rational(left, identity)?
        .add(&as_rational(right, identity)?)
        .map_err(|_| {
            failure(
                AdaptiveTessellationCondition::ChordUnboundable,
                Some(String::from(identity)),
            )
        })?;
    from_rational(
        &sum.div(&integer_rational(2, identity)?).map_err(|_| {
            failure(
                AdaptiveTessellationCondition::ChordUnboundable,
                Some(String::from(identity)),
            )
        })?,
        identity,
    )
}

struct SeamSlot {
    first_chart: String,
    first_u: RealAlgebraic,
    first_v: RealAlgebraic,
    second_chart: String,
    second_u: RealAlgebraic,
    second_v: RealAlgebraic,
    vertex: Option<usize>,
}

struct MeshVertex {
    u: RealAlgebraic,
    v: RealAlgebraic,
    point: Point3,
    weld_identity: Option<Vec<String>>,
}

struct MeshTriangle {
    face_id: String,
    patch: usize,
    vertices: [usize; 3],
    reverse: bool,
}

type ProjectedSurfaceMesh = (Vec<[f64; 3]>, Vec<[usize; 3]>);

fn evaluate_polynomial_rational(
    polynomial: &SurfacePolynomial,
    u: &ReducedRational,
    v: &ReducedRational,
    identity: &str,
) -> Result<ReducedRational, CadFailure> {
    let mut result = integer_rational(0, identity)?;
    for term in polynomial.terms() {
        let exponent_u = i32::try_from(term.exponent_u).map_err(|_| {
            failure(
                AdaptiveTessellationCondition::SurfaceEvaluation,
                Some(String::from(identity)),
            )
        })?;
        let exponent_v = i32::try_from(term.exponent_v).map_err(|_| {
            failure(
                AdaptiveTessellationCondition::SurfaceEvaluation,
                Some(String::from(identity)),
            )
        })?;
        let u_power = u.pow_i32(exponent_u).map_err(|_| {
            failure(
                AdaptiveTessellationCondition::SurfaceEvaluation,
                Some(String::from(identity)),
            )
        })?;
        let v_power = v.pow_i32(exponent_v).map_err(|_| {
            failure(
                AdaptiveTessellationCondition::SurfaceEvaluation,
                Some(String::from(identity)),
            )
        })?;
        let term_value = term
            .coefficient
            .mul(&u_power)
            .map_err(|_| {
                failure(
                    AdaptiveTessellationCondition::SurfaceEvaluation,
                    Some(String::from(identity)),
                )
            })?
            .mul(&v_power)
            .map_err(|_| {
                failure(
                    AdaptiveTessellationCondition::SurfaceEvaluation,
                    Some(String::from(identity)),
                )
            })?;
        result = result.add(&term_value).map_err(|_| {
            failure(
                AdaptiveTessellationCondition::SurfaceEvaluation,
                Some(String::from(identity)),
            )
        })?;
    }
    Ok(result)
}

fn evaluate_exact(
    patch: &ChartPatch,
    u: &RealAlgebraic,
    v: &RealAlgebraic,
    identity: &str,
) -> Result<Point3, CadFailure> {
    if !domain_contains(patch.domain(), u, v, identity)? {
        return Err(failure(
            AdaptiveTessellationCondition::SurfaceEvaluation,
            Some(String::from(identity)),
        ));
    }
    let u = as_rational(u, identity)?;
    let v = as_rational(v, identity)?;
    let surface = patch.chart().surface();
    let denominator = evaluate_polynomial_rational(&surface.denominator, &u, &v, identity)?;
    if denominator.is_zero() {
        return Err(failure(
            AdaptiveTessellationCondition::SurfaceEvaluation,
            Some(String::from(identity)),
        ));
    }
    let x = evaluate_polynomial_rational(&surface.x, &u, &v, identity)?
        .div(&denominator)
        .map_err(|_| {
            failure(
                AdaptiveTessellationCondition::SurfaceEvaluation,
                Some(String::from(identity)),
            )
        })?;
    let y = evaluate_polynomial_rational(&surface.y, &u, &v, identity)?
        .div(&denominator)
        .map_err(|_| {
            failure(
                AdaptiveTessellationCondition::SurfaceEvaluation,
                Some(String::from(identity)),
            )
        })?;
    let z = evaluate_polynomial_rational(&surface.z, &u, &v, identity)?
        .div(&denominator)
        .map_err(|_| {
            failure(
                AdaptiveTessellationCondition::SurfaceEvaluation,
                Some(String::from(identity)),
            )
        })?;
    Ok(Point3 {
        x: from_rational(&x, identity)?,
        y: from_rational(&y, identity)?,
        z: from_rational(&z, identity)?,
    })
}

fn project_point(
    identity: &str,
    revision: &neco_brep::SourceRevision,
    atom: &CanonicalAtom,
    point: &Point3,
    bits: u32,
) -> Result<([f64; 3], Dyadic), CadFailure> {
    let projection = project_numeric_coordinates(
        NumericProjectionRequest::new(
            identity,
            revision.clone(),
            atom.try_clone().map_err(|_| {
                failure(
                    AdaptiveTessellationCondition::Projection,
                    Some(String::from(identity)),
                )
            })?,
            bits,
            vec![NumericSample::AlgebraicPoint(clone_point(point, identity)?)],
        )
        .map_err(|error| match error {
            CadFailure::NumericProjection { .. } => failure(
                AdaptiveTessellationCondition::Projection,
                Some(String::from(identity)),
            ),
            other => other,
        })?,
    )
    .map_err(|error| match error {
        CadFailure::NumericProjection { .. } => failure(
            AdaptiveTessellationCondition::Projection,
            Some(String::from(identity)),
        ),
        other => other,
    })?;
    let projected = projection.coordinates().first().ok_or_else(|| {
        failure(
            AdaptiveTessellationCondition::Projection,
            Some(String::from(identity)),
        )
    })?;
    Ok((
        projected.rounded(),
        projected
            .surface_distance_bound()
            .try_clone()
            .map_err(|_| {
                failure(
                    AdaptiveTessellationCondition::Projection,
                    Some(String::from(identity)),
                )
            })?,
    ))
}

fn match_seam(
    slots: &mut [SeamSlot],
    chart: &str,
    u: &RealAlgebraic,
    v: &RealAlgebraic,
    identity: &str,
) -> Result<Option<usize>, CadFailure> {
    for slot in slots.iter_mut() {
        let first = chart == slot.first_chart
            && parameters_equal(u, v, &slot.first_u, &slot.first_v, identity)?;
        let second = chart == slot.second_chart
            && parameters_equal(u, v, &slot.second_u, &slot.second_v, identity)?;
        if first || second {
            return Ok(Some(slot.vertex.unwrap_or(usize::MAX)));
        }
    }
    Ok(None)
}

fn bind_seam(
    slots: &mut [SeamSlot],
    chart: &str,
    u: &RealAlgebraic,
    v: &RealAlgebraic,
    vertex: usize,
    identity: &str,
) -> Result<(), CadFailure> {
    for slot in slots.iter_mut() {
        let first = chart == slot.first_chart
            && parameters_equal(u, v, &slot.first_u, &slot.first_v, identity)?;
        let second = chart == slot.second_chart
            && parameters_equal(u, v, &slot.second_u, &slot.second_v, identity)?;
        if first || second {
            match slot.vertex {
                Some(existing) if existing != vertex => {
                    return Err(failure(
                        AdaptiveTessellationCondition::SeamUnpaired,
                        Some(String::from(identity)),
                    ));
                }
                Some(_) => {}
                None => slot.vertex = Some(vertex),
            }
        }
    }
    Ok(())
}

fn insert_vertex(
    vertices: &mut Vec<MeshVertex>,
    slots: &mut [SeamSlot],
    patch: &ChartPatch,
    identity: &str,
    u: RealAlgebraic,
    v: RealAlgebraic,
) -> Result<Option<usize>, CadFailure> {
    match classify_sample(patch, &u, &v, identity)? {
        SampleKind::Discard => return Ok(None),
        SampleKind::Keep => {}
    }
    let chart = patch.chart().identity().domain_id().as_str();
    if let Some(existing) = match_seam(slots, chart, &u, &v, identity)? {
        if existing != usize::MAX {
            return Ok(Some(existing));
        }
    }
    let point = evaluate_exact(patch, &u, &v, identity)?;
    let index = vertices.len();
    vertices.push(MeshVertex {
        u,
        v,
        point,
        weld_identity: None,
    });
    let stored = vertices.last().ok_or_else(|| {
        failure(
            AdaptiveTessellationCondition::SurfaceEvaluation,
            Some(String::from(identity)),
        )
    })?;
    bind_seam(
        slots,
        patch.chart().identity().domain_id().as_str(),
        &stored.u,
        &stored.v,
        index,
        identity,
    )?;
    Ok(Some(index))
}

fn triangle_centroid(
    vertices: &[MeshVertex],
    triangle: &MeshTriangle,
    identity: &str,
) -> Result<(RealAlgebraic, RealAlgebraic), CadFailure> {
    let a = &vertices[triangle.vertices[0]];
    let b = &vertices[triangle.vertices[1]];
    let c = &vertices[triangle.vertices[2]];
    let three = integer_rational(3, identity)?;
    let u = as_rational(&a.u, identity)?
        .add(&as_rational(&b.u, identity)?)
        .map_err(|_| {
            failure(
                AdaptiveTessellationCondition::TrimClassification,
                Some(String::from(identity)),
            )
        })?
        .add(&as_rational(&c.u, identity)?)
        .map_err(|_| {
            failure(
                AdaptiveTessellationCondition::TrimClassification,
                Some(String::from(identity)),
            )
        })?
        .div(&three)
        .map_err(|_| {
            failure(
                AdaptiveTessellationCondition::TrimClassification,
                Some(String::from(identity)),
            )
        })?;
    let v = as_rational(&a.v, identity)?
        .add(&as_rational(&b.v, identity)?)
        .map_err(|_| {
            failure(
                AdaptiveTessellationCondition::TrimClassification,
                Some(String::from(identity)),
            )
        })?
        .add(&as_rational(&c.v, identity)?)
        .map_err(|_| {
            failure(
                AdaptiveTessellationCondition::TrimClassification,
                Some(String::from(identity)),
            )
        })?
        .div(&three)
        .map_err(|_| {
            failure(
                AdaptiveTessellationCondition::TrimClassification,
                Some(String::from(identity)),
            )
        })?;
    Ok((from_rational(&u, identity)?, from_rational(&v, identity)?))
}

fn triangle_kind(
    patch: &ChartPatch,
    vertices: &[MeshVertex],
    triangle: &MeshTriangle,
    identity: &str,
) -> Result<TriangleKind, CadFailure> {
    let (u, v) = triangle_centroid(vertices, triangle, identity)?;
    let centroid = classify_sample(patch, &u, &v, identity)?;
    let mut keep = 0;
    let mut discard = 0;
    for index in triangle.vertices {
        match classify_sample(patch, &vertices[index].u, &vertices[index].v, identity)? {
            SampleKind::Keep => keep += 1,
            SampleKind::Discard => discard += 1,
        }
    }
    match (centroid, keep, discard) {
        (SampleKind::Discard, 0, 3) => Ok(TriangleKind::Discard),
        (SampleKind::Keep, _, _) => Ok(TriangleKind::Keep),
        _ => Ok(TriangleKind::Mixed),
    }
}

fn disk_boundary_midpoint(
    left: &MeshVertex,
    right: &MeshVertex,
    identity: &str,
) -> Result<Option<(RealAlgebraic, RealAlgebraic)>, CadFailure> {
    let left_u = as_rational(&left.u, identity)?;
    let left_v = as_rational(&left.v, identity)?;
    let right_u = as_rational(&right.u, identity)?;
    let right_v = as_rational(&right.v, identity)?;
    let one = integer_rational(1, identity)?;
    let two = integer_rational(2, identity)?;
    let zero = integer_rational(0, identity)?;
    let squared_radius = |u: &ReducedRational, v: &ReducedRational| {
        rational_add(
            &rational_mul(u, u, identity)?,
            &rational_mul(v, v, identity)?,
            identity,
        )
    };
    if squared_radius(&left_u, &left_v)?.cmp(&one) != Ordering::Equal
        || squared_radius(&right_u, &right_v)?.cmp(&one) != Ordering::Equal
    {
        return Ok(None);
    }
    let sum_u = rational_add(&left_u, &right_u, identity)?;
    let (u, v) = if sum_u.cmp(&zero) == Ordering::Less {
        let left_parameter =
            rational_div(&left_v, &rational_sub(&one, &left_u, identity)?, identity)?;
        let right_parameter =
            rational_div(&right_v, &rational_sub(&one, &right_u, identity)?, identity)?;
        let parameter = rational_div(
            &rational_add(&left_parameter, &right_parameter, identity)?,
            &two,
            identity,
        )?;
        let parameter_squared = rational_mul(&parameter, &parameter, identity)?;
        let denominator = rational_add(&one, &parameter_squared, identity)?;
        (
            rational_div(
                &rational_sub(&parameter_squared, &one, identity)?,
                &denominator,
                identity,
            )?,
            rational_div(
                &rational_mul(&two, &parameter, identity)?,
                &denominator,
                identity,
            )?,
        )
    } else {
        let left_parameter =
            rational_div(&left_v, &rational_add(&one, &left_u, identity)?, identity)?;
        let right_parameter =
            rational_div(&right_v, &rational_add(&one, &right_u, identity)?, identity)?;
        let parameter = rational_div(
            &rational_add(&left_parameter, &right_parameter, identity)?,
            &two,
            identity,
        )?;
        let parameter_squared = rational_mul(&parameter, &parameter, identity)?;
        let denominator = rational_add(&one, &parameter_squared, identity)?;
        (
            rational_div(
                &rational_sub(&one, &parameter_squared, identity)?,
                &denominator,
                identity,
            )?,
            rational_div(
                &rational_mul(&two, &parameter, identity)?,
                &denominator,
                identity,
            )?,
        )
    };
    Ok(Some((
        from_rational(&u, identity)?,
        from_rational(&v, identity)?,
    )))
}

fn edge_mid_parameters(
    vertices: &[MeshVertex],
    left: usize,
    right: usize,
    patch: &ChartPatch,
    identity: &str,
) -> Result<(RealAlgebraic, RealAlgebraic), CadFailure> {
    if patch.disk() {
        if let Some(parameters) =
            disk_boundary_midpoint(&vertices[left], &vertices[right], identity)?
        {
            return Ok(parameters);
        }
    }
    Ok((
        mid_value(&vertices[left].u, &vertices[right].u, identity)?,
        mid_value(&vertices[left].v, &vertices[right].v, identity)?,
    ))
}

fn rational_min(
    left: &ReducedRational,
    right: &ReducedRational,
    identity: &str,
) -> Result<ReducedRational, CadFailure> {
    clone_rational(
        if left.cmp(right) == Ordering::Greater {
            right
        } else {
            left
        },
        identity,
    )
}

fn rational_max(
    left: &ReducedRational,
    right: &ReducedRational,
    identity: &str,
) -> Result<ReducedRational, CadFailure> {
    clone_rational(
        if left.cmp(right) == Ordering::Less {
            right
        } else {
            left
        },
        identity,
    )
}

fn on_unit_circle(
    u: &ReducedRational,
    v: &ReducedRational,
    identity: &str,
) -> Result<bool, CadFailure> {
    let squared = rational_add(
        &rational_mul(u, u, identity)?,
        &rational_mul(v, v, identity)?,
        identity,
    )?;
    Ok(squared.cmp(&integer_rational(1, identity)?) == Ordering::Equal)
}

fn cross_parameters(
    first_u: &ReducedRational,
    first_v: &ReducedRational,
    second_u: &ReducedRational,
    second_v: &ReducedRational,
    identity: &str,
) -> Result<ReducedRational, CadFailure> {
    rational_sub(
        &rational_mul(first_u, second_v, identity)?,
        &rational_mul(first_v, second_u, identity)?,
        identity,
    )
}

#[derive(Clone, Copy)]
struct ParameterPoint<'a> {
    u: &'a ReducedRational,
    v: &'a ReducedRational,
}

struct DirectedArc<'a> {
    start: ParameterPoint<'a>,
    end: ParameterPoint<'a>,
    direction: Sign,
}

impl DirectedArc<'_> {
    fn contains(&self, point: ParameterPoint<'_>, identity: &str) -> Result<bool, CadFailure> {
        let before = cross_parameters(self.start.u, self.start.v, point.u, point.v, identity)?;
        let after = cross_parameters(point.u, point.v, self.end.u, self.end.v, identity)?;
        let consistent = |value: &ReducedRational| {
            let sign = value.numerator().sign();
            sign == Sign::Zero || sign == self.direction
        };
        Ok(consistent(&before) && consistent(&after))
    }
}

fn disk_arc_parameter_box(
    left: &MeshVertex,
    right: &MeshVertex,
    identity: &str,
) -> Result<
    Option<(
        ReducedRational,
        ReducedRational,
        ReducedRational,
        ReducedRational,
    )>,
    CadFailure,
> {
    let left_u = as_rational(&left.u, identity)?;
    let left_v = as_rational(&left.v, identity)?;
    let right_u = as_rational(&right.u, identity)?;
    let right_v = as_rational(&right.v, identity)?;
    if !on_unit_circle(&left_u, &left_v, identity)?
        || !on_unit_circle(&right_u, &right_v, identity)?
    {
        return Ok(None);
    }
    let one = integer_rational(1, identity)?;
    let zero = integer_rational(0, identity)?;
    let negative_one = rational_negate(&one, identity)?;
    let mut u_lo = rational_min(&left_u, &right_u, identity)?;
    let mut u_hi = rational_max(&left_u, &right_u, identity)?;
    let mut v_lo = rational_min(&left_v, &right_v, identity)?;
    let mut v_hi = rational_max(&left_v, &right_v, identity)?;
    let sweep = cross_parameters(&left_u, &left_v, &right_u, &right_v, identity)?;
    if sweep.numerator().sign() == Sign::Zero {
        return Ok(Some((
            clone_rational(&negative_one, identity)?,
            clone_rational(&one, identity)?,
            negative_one,
            one,
        )));
    }
    let minor_direction = sweep.numerator().sign();
    let sum_u = rational_add(&left_u, &right_u, identity)?;
    let (pole_u, pole_v) = if sum_u.cmp(&zero) == Ordering::Less {
        (
            clone_rational(&one, identity)?,
            clone_rational(&zero, identity)?,
        )
    } else {
        (
            clone_rational(&negative_one, identity)?,
            clone_rational(&zero, identity)?,
        )
    };
    let start = ParameterPoint {
        u: &left_u,
        v: &left_v,
    };
    let end = ParameterPoint {
        u: &right_u,
        v: &right_v,
    };
    let pole = ParameterPoint {
        u: &pole_u,
        v: &pole_v,
    };
    let minor_arc = DirectedArc {
        start,
        end,
        direction: minor_direction,
    };
    let direction = if minor_arc.contains(pole, identity)? {
        match minor_direction {
            Sign::Positive => Sign::Negative,
            Sign::Negative => Sign::Positive,
            Sign::Zero => Sign::Zero,
        }
    } else {
        minor_direction
    };
    let arc = DirectedArc {
        start,
        end,
        direction,
    };
    let u_hi_point = ParameterPoint { u: &one, v: &zero };
    let u_lo_point = ParameterPoint {
        u: &negative_one,
        v: &zero,
    };
    let v_hi_point = ParameterPoint { u: &zero, v: &one };
    let v_lo_point = ParameterPoint {
        u: &zero,
        v: &negative_one,
    };
    if arc.contains(u_hi_point, identity)? {
        u_hi = clone_rational(&one, identity)?;
    }
    if arc.contains(u_lo_point, identity)? {
        u_lo = clone_rational(&negative_one, identity)?;
    }
    if arc.contains(v_hi_point, identity)? {
        v_hi = clone_rational(&one, identity)?;
    }
    if arc.contains(v_lo_point, identity)? {
        v_lo = clone_rational(&negative_one, identity)?;
    }
    Ok(Some((u_lo, u_hi, v_lo, v_hi)))
}

fn rational_polynomial_multiply(
    left: &[ReducedRational],
    right: &[ReducedRational],
    identity: &str,
) -> Result<Vec<ReducedRational>, CadFailure> {
    if left.is_empty() || right.is_empty() {
        return Ok(Vec::new());
    }
    let mut result = Vec::with_capacity(left.len() + right.len() - 1);
    for _ in 0..left.len() + right.len() - 1 {
        result.push(integer_rational(0, identity)?);
    }
    for (i, left_coefficient) in left.iter().enumerate() {
        for (j, right_coefficient) in right.iter().enumerate() {
            result[i + j] = rational_add(
                &result[i + j],
                &rational_mul(left_coefficient, right_coefficient, identity)?,
                identity,
            )?;
        }
    }
    Ok(result)
}

fn rational_polynomial_subtract(
    left: &[ReducedRational],
    right: &[ReducedRational],
    identity: &str,
) -> Result<Vec<ReducedRational>, CadFailure> {
    let length = left.len().max(right.len());
    let mut result = Vec::with_capacity(length);
    for index in 0..length {
        result.push(match (left.get(index), right.get(index)) {
            (Some(l), Some(r)) => rational_sub(l, r, identity)?,
            (Some(l), None) => clone_rational(l, identity)?,
            (None, Some(r)) => rational_negate(r, identity)?,
            (None, None) => integer_rational(0, identity)?,
        });
    }
    Ok(result)
}

fn rational_linear_power(
    base: &ReducedRational,
    slope: &ReducedRational,
    degree: u32,
    identity: &str,
) -> Result<Vec<ReducedRational>, CadFailure> {
    let mut result = vec![integer_rational(1, identity)?];
    for _ in 0..degree {
        let mut next = Vec::with_capacity(result.len() + 1);
        next.push(rational_mul(&result[0], base, identity)?);
        for index in 1..result.len() {
            next.push(rational_add(
                &rational_mul(&result[index], base, identity)?,
                &rational_mul(&result[index - 1], slope, identity)?,
                identity,
            )?);
        }
        next.push(rational_mul(&result[result.len() - 1], slope, identity)?);
        result = next;
    }
    Ok(result)
}

fn substitute_edge_polynomial(
    polynomial: &SurfacePolynomial,
    base_u: &ReducedRational,
    slope_u: &ReducedRational,
    base_v: &ReducedRational,
    slope_v: &ReducedRational,
    identity: &str,
) -> Result<Vec<ReducedRational>, CadFailure> {
    let mut result: Vec<ReducedRational> = Vec::new();
    for term in polynomial.terms() {
        let u_power = rational_linear_power(base_u, slope_u, term.exponent_u, identity)?;
        let v_power = rational_linear_power(base_v, slope_v, term.exponent_v, identity)?;
        let monomial = rational_polynomial_multiply(&u_power, &v_power, identity)?;
        for (index, coefficient) in monomial.iter().enumerate() {
            let scaled = rational_mul(coefficient, &term.coefficient, identity)?;
            while result.len() <= index {
                result.push(integer_rational(0, identity)?);
            }
            result[index] = rational_add(&result[index], &scaled, identity)?;
        }
    }
    Ok(result)
}

fn univariate_interval(
    coefficients: &[ReducedRational],
    identity: &str,
) -> Result<(ReducedRational, ReducedRational), CadFailure> {
    let zero = integer_rational(0, identity)?;
    let mut lower = integer_rational(0, identity)?;
    let mut upper = integer_rational(0, identity)?;
    for (index, coefficient) in coefficients.iter().enumerate() {
        if index == 0 {
            lower = rational_add(&lower, coefficient, identity)?;
            upper = rational_add(&upper, coefficient, identity)?;
            continue;
        }
        match coefficient.cmp(&zero) {
            Ordering::Less => lower = rational_add(&lower, coefficient, identity)?,
            Ordering::Greater => upper = rational_add(&upper, coefficient, identity)?,
            Ordering::Equal => {}
        }
    }
    Ok((lower, upper))
}

fn affine_shift_polynomial(
    coefficients: &[ReducedRational],
    base: &ReducedRational,
    slope: &ReducedRational,
    identity: &str,
) -> Result<Vec<ReducedRational>, CadFailure> {
    let mut shifted: Vec<ReducedRational> = Vec::new();
    for (degree, coefficient) in coefficients.iter().enumerate() {
        let power = rational_linear_power(base, slope, degree as u32, identity)?;
        for (index, term) in power.iter().enumerate() {
            while shifted.len() <= index {
                shifted.push(integer_rational(0, identity)?);
            }
            shifted[index] = rational_add(
                &shifted[index],
                &rational_mul(coefficient, term, identity)?,
                identity,
            )?;
        }
    }
    Ok(shifted)
}

const EDGE_INTERVAL_SUBDIVISIONS: u32 = 4;

fn subdivided_univariate_interval(
    coefficients: &[ReducedRational],
    identity: &str,
) -> Result<(ReducedRational, ReducedRational), CadFailure> {
    let unboundable = || {
        failure(
            AdaptiveTessellationCondition::ChordUnboundable,
            Some(String::from(identity)),
        )
    };
    let mut lower: Option<ReducedRational> = None;
    let mut upper: Option<ReducedRational> = None;
    for part in 0..EDGE_INTERVAL_SUBDIVISIONS {
        let base = RawRational::new(
            BigInt::try_from(part).map_err(|_| unboundable())?,
            BigUint::try_from(EDGE_INTERVAL_SUBDIVISIONS).map_err(|_| unboundable())?,
        )
        .reduce()
        .map_err(|_| unboundable())?
        .into_reduced();
        let extent = RawRational::new(
            BigInt::try_from(1_u8).map_err(|_| unboundable())?,
            BigUint::try_from(EDGE_INTERVAL_SUBDIVISIONS).map_err(|_| unboundable())?,
        )
        .reduce()
        .map_err(|_| unboundable())?
        .into_reduced();
        let shifted = affine_shift_polynomial(coefficients, &base, &extent, identity)?;
        let (part_lower, part_upper) = univariate_interval(&shifted, identity)?;
        lower = Some(match lower {
            Some(current) => rational_min(&current, &part_lower, identity)?,
            None => part_lower,
        });
        upper = Some(match upper {
            Some(current) => rational_max(&current, &part_upper, identity)?,
            None => part_upper,
        });
    }
    Ok((
        lower.ok_or_else(unboundable)?,
        upper.ok_or_else(unboundable)?,
    ))
}

fn interval_add(
    left: &(ReducedRational, ReducedRational),
    right: &(ReducedRational, ReducedRational),
    identity: &str,
) -> Result<(ReducedRational, ReducedRational), CadFailure> {
    Ok((
        rational_add(&left.0, &right.0, identity)?,
        rational_add(&left.1, &right.1, identity)?,
    ))
}

fn interval_mul(
    left: &(ReducedRational, ReducedRational),
    right: &(ReducedRational, ReducedRational),
    identity: &str,
) -> Result<(ReducedRational, ReducedRational), CadFailure> {
    let corners = [
        rational_mul(&left.0, &right.0, identity)?,
        rational_mul(&left.0, &right.1, identity)?,
        rational_mul(&left.1, &right.0, identity)?,
        rational_mul(&left.1, &right.1, identity)?,
    ];
    let mut lower = clone_rational(&corners[0], identity)?;
    let mut upper = clone_rational(&corners[0], identity)?;
    for corner in corners.iter().skip(1) {
        if corner.cmp(&lower) == Ordering::Less {
            lower = clone_rational(corner, identity)?;
        }
        if corner.cmp(&upper) == Ordering::Greater {
            upper = clone_rational(corner, identity)?;
        }
    }
    Ok((lower, upper))
}

fn interval_power(
    base: &(ReducedRational, ReducedRational),
    exponent: u32,
    identity: &str,
) -> Result<(ReducedRational, ReducedRational), CadFailure> {
    let unboundable = || {
        failure(
            AdaptiveTessellationCondition::ChordUnboundable,
            Some(String::from(identity)),
        )
    };
    if exponent == 0 {
        let one = integer_rational(1, identity)?;
        return Ok((clone_rational(&one, identity)?, one));
    }
    let exponent = i32::try_from(exponent).map_err(|_| unboundable())?;
    let lower_power = base.0.pow_i32(exponent).map_err(|_| unboundable())?;
    let upper_power = base.1.pow_i32(exponent).map_err(|_| unboundable())?;
    let zero = integer_rational(0, identity)?;
    if exponent % 2 == 0
        && base.0.cmp(&zero) == Ordering::Less
        && base.1.cmp(&zero) == Ordering::Greater
    {
        Ok((zero, rational_max(&lower_power, &upper_power, identity)?))
    } else {
        Ok((
            rational_min(&lower_power, &upper_power, identity)?,
            rational_max(&lower_power, &upper_power, identity)?,
        ))
    }
}

fn interval_div(
    numerator: &(ReducedRational, ReducedRational),
    denominator: &(ReducedRational, ReducedRational),
    identity: &str,
) -> Result<(ReducedRational, ReducedRational), CadFailure> {
    let unboundable = || {
        failure(
            AdaptiveTessellationCondition::ChordUnboundable,
            Some(String::from(identity)),
        )
    };
    let zero = integer_rational(0, identity)?;
    if denominator.0.cmp(&zero) != Ordering::Greater && denominator.1.cmp(&zero) != Ordering::Less {
        return Err(unboundable());
    }
    let corners = [
        numerator.0.div(&denominator.0).map_err(|_| unboundable())?,
        numerator.0.div(&denominator.1).map_err(|_| unboundable())?,
        numerator.1.div(&denominator.0).map_err(|_| unboundable())?,
        numerator.1.div(&denominator.1).map_err(|_| unboundable())?,
    ];
    let mut lower = clone_rational(&corners[0], identity)?;
    let mut upper = clone_rational(&corners[0], identity)?;
    for corner in corners.iter().skip(1) {
        if corner.cmp(&lower) == Ordering::Less {
            lower = clone_rational(corner, identity)?;
        }
        if corner.cmp(&upper) == Ordering::Greater {
            upper = clone_rational(corner, identity)?;
        }
    }
    Ok((lower, upper))
}

fn evaluate_polynomial_interval(
    polynomial: &SurfacePolynomial,
    u: &(ReducedRational, ReducedRational),
    v: &(ReducedRational, ReducedRational),
    identity: &str,
) -> Result<(ReducedRational, ReducedRational), CadFailure> {
    let zero = integer_rational(0, identity)?;
    let mut result = (clone_rational(&zero, identity)?, zero);
    for term in polynomial.terms() {
        let u_power = interval_power(u, term.exponent_u, identity)?;
        let v_power = interval_power(v, term.exponent_v, identity)?;
        let monomial = interval_mul(&u_power, &v_power, identity)?;
        let coefficient = (
            clone_rational(&term.coefficient, identity)?,
            clone_rational(&term.coefficient, identity)?,
        );
        result = interval_add(
            &result,
            &interval_mul(&coefficient, &monomial, identity)?,
            identity,
        )?;
    }
    Ok(result)
}

fn surface_coordinate_bounds(
    patch: &ChartPatch,
    u: &(ReducedRational, ReducedRational),
    v: &(ReducedRational, ReducedRational),
    identity: &str,
) -> Result<[(ReducedRational, ReducedRational); 3], CadFailure> {
    let surface = patch.chart().surface();
    let denominator = evaluate_polynomial_interval(&surface.denominator, u, v, identity)?;
    let zero = integer_rational(0, identity)?;
    if denominator.0.cmp(&zero) != Ordering::Greater && denominator.1.cmp(&zero) != Ordering::Less {
        return Err(failure(
            AdaptiveTessellationCondition::ChordUnboundable,
            Some(String::from(identity)),
        ));
    }
    Ok([
        interval_div(
            &evaluate_polynomial_interval(&surface.x, u, v, identity)?,
            &denominator,
            identity,
        )?,
        interval_div(
            &evaluate_polynomial_interval(&surface.y, u, v, identity)?,
            &denominator,
            identity,
        )?,
        interval_div(
            &evaluate_polynomial_interval(&surface.z, u, v, identity)?,
            &denominator,
            identity,
        )?,
    ])
}

fn arc_surface_bounds(
    patch: &ChartPatch,
    u: &(ReducedRational, ReducedRational),
    v: &(ReducedRational, ReducedRational),
    identity: &str,
) -> Result<[(ReducedRational, ReducedRational); 3], CadFailure> {
    let unboundable = || {
        failure(
            AdaptiveTessellationCondition::ChordUnboundable,
            Some(String::from(identity)),
        )
    };
    let one = integer_rational(1, identity)?;
    let parts = integer_rational(EDGE_INTERVAL_SUBDIVISIONS as i32, identity)?;
    let step_u = rational_sub(&u.1, &u.0, identity)?
        .div(&parts)
        .map_err(|_| unboundable())?;
    let step_v = rational_sub(&v.1, &v.0, identity)?
        .div(&parts)
        .map_err(|_| unboundable())?;
    let mut merged: Option<[(ReducedRational, ReducedRational); 3]> = None;
    let mut u_lower = clone_rational(&u.0, identity)?;
    for _ in 0..EDGE_INTERVAL_SUBDIVISIONS {
        let u_upper = rational_add(&u_lower, &step_u, identity)?;
        let mut v_lower = clone_rational(&v.0, identity)?;
        for _ in 0..EDGE_INTERVAL_SUBDIVISIONS {
            let v_upper = rational_add(&v_lower, &step_v, identity)?;
            let sub_u = (
                clone_rational(&u_lower, identity)?,
                clone_rational(&u_upper, identity)?,
            );
            let sub_v = (
                clone_rational(&v_lower, identity)?,
                clone_rational(&v_upper, identity)?,
            );
            let radius = interval_add(
                &interval_power(&sub_u, 2, identity)?,
                &interval_power(&sub_v, 2, identity)?,
                identity,
            )?;
            if radius.0.cmp(&one) != Ordering::Greater && radius.1.cmp(&one) != Ordering::Less {
                let bounds = surface_coordinate_bounds(patch, &sub_u, &sub_v, identity)?;
                merged = Some(match merged {
                    Some(current) => [
                        (
                            rational_min(&current[0].0, &bounds[0].0, identity)?,
                            rational_max(&current[0].1, &bounds[0].1, identity)?,
                        ),
                        (
                            rational_min(&current[1].0, &bounds[1].0, identity)?,
                            rational_max(&current[1].1, &bounds[1].1, identity)?,
                        ),
                        (
                            rational_min(&current[2].0, &bounds[2].0, identity)?,
                            rational_max(&current[2].1, &bounds[2].1, identity)?,
                        ),
                    ],
                    None => bounds,
                });
            }
            v_lower = v_upper;
        }
        u_lower = u_upper;
    }
    merged.ok_or_else(unboundable)
}

fn point_rational(point: &Point3, identity: &str) -> Result<[ReducedRational; 3], CadFailure> {
    Ok([
        as_rational(&point.x, identity)?,
        as_rational(&point.y, identity)?,
        as_rational(&point.z, identity)?,
    ])
}

fn box_chord_deviations(
    bounds: &[(ReducedRational, ReducedRational); 3],
    start: &Point3,
    end: &Point3,
    identity: &str,
) -> Result<[(ReducedRational, ReducedRational); 3], CadFailure> {
    let start = point_rational(start, identity)?;
    let end = point_rational(end, identity)?;
    let mut deviations = Vec::with_capacity(3);
    for axis in 0..3 {
        let chord_lower = rational_min(&start[axis], &end[axis], identity)?;
        let chord_upper = rational_max(&start[axis], &end[axis], identity)?;
        deviations.push((
            rational_sub(&bounds[axis].0, &chord_upper, identity)?,
            rational_sub(&bounds[axis].1, &chord_lower, identity)?,
        ));
    }
    deviations.try_into().map_err(|_| {
        failure(
            AdaptiveTessellationCondition::ChordUnboundable,
            Some(String::from(identity)),
        )
    })
}

fn edge_deviation_bounds(
    vertices: &[MeshVertex],
    left: usize,
    right: usize,
    patch: &ChartPatch,
    identity: &str,
) -> Result<[(ReducedRational, ReducedRational); 3], CadFailure> {
    let unboundable = || {
        failure(
            AdaptiveTessellationCondition::ChordUnboundable,
            Some(String::from(identity)),
        )
    };
    let base_u = as_rational(&vertices[left].u, identity)?;
    let base_v = as_rational(&vertices[left].v, identity)?;
    let slope_u = rational_sub(
        &as_rational(&vertices[right].u, identity)?,
        &base_u,
        identity,
    )?;
    let slope_v = rational_sub(
        &as_rational(&vertices[right].v, identity)?,
        &base_v,
        identity,
    )?;
    let surface = patch.chart().surface();
    let denominator = substitute_edge_polynomial(
        &surface.denominator,
        &base_u,
        &slope_u,
        &base_v,
        &slope_v,
        identity,
    )?;
    let denominator_range = subdivided_univariate_interval(&denominator, identity)?;
    let zero = integer_rational(0, identity)?;
    if denominator_range.0.cmp(&zero) != Ordering::Greater
        && denominator_range.1.cmp(&zero) != Ordering::Less
    {
        return Err(unboundable());
    }
    let start = point_rational(&vertices[left].point, identity)?;
    let end = point_rational(&vertices[right].point, identity)?;
    let coordinate_polynomials = [&surface.x, &surface.y, &surface.z];
    let mut deviations = Vec::with_capacity(3);
    for axis in 0..3 {
        let numerator = substitute_edge_polynomial(
            coordinate_polynomials[axis],
            &base_u,
            &slope_u,
            &base_v,
            &slope_v,
            identity,
        )?;
        let chord = vec![
            clone_rational(&start[axis], identity)?,
            rational_sub(&end[axis], &start[axis], identity)?,
        ];
        let deviation_numerator = rational_polynomial_subtract(
            &numerator,
            &rational_polynomial_multiply(&denominator, &chord, identity)?,
            identity,
        )?;
        deviations.push(interval_div(
            &subdivided_univariate_interval(&deviation_numerator, identity)?,
            &denominator_range,
            identity,
        )?);
    }
    deviations.try_into().map_err(|_| unboundable())
}

fn deviation_squared_upper(
    deviations: &[(ReducedRational, ReducedRational); 3],
    bits: u32,
    identity: &str,
) -> Result<Dyadic, CadFailure> {
    let mut squared = integer_rational(0, identity)?;
    for (lower, upper) in deviations {
        let lower_squared = rational_mul(lower, lower, identity)?;
        let upper_squared = rational_mul(upper, upper, identity)?;
        squared = rational_add(
            &squared,
            &rational_max(&lower_squared, &upper_squared, identity)?,
            identity,
        )?;
    }
    squared.dyadic_ceil(bits).map_err(|_| {
        failure(
            AdaptiveTessellationCondition::ChordUnboundable,
            Some(String::from(identity)),
        )
    })
}

struct EdgeChordParts {
    midpoint_u: ReducedRational,
    midpoint_v: ReducedRational,
    shape: ChordBoundEdgeShape,
    squared_bound: Option<Dyadic>,
}

fn edge_chord_parts(
    vertices: &[MeshVertex],
    left: usize,
    right: usize,
    patch: &ChartPatch,
    bits: u32,
    identity: &str,
) -> Result<EdgeChordParts, CadFailure> {
    let (u, v) = edge_mid_parameters(vertices, left, right, patch, identity)?;
    let midpoint_u = as_rational(&u, identity)?;
    let midpoint_v = as_rational(&v, identity)?;
    let arc = if patch.disk() {
        disk_arc_parameter_box(&vertices[left], &vertices[right], identity)?
    } else {
        None
    };
    let discarded = matches!(
        classify_sample(patch, &u, &v, identity)?,
        SampleKind::Discard
    );
    let deviations = if discarded {
        None
    } else {
        Some(match &arc {
            Some((u_lo, u_hi, v_lo, v_hi)) => box_chord_deviations(
                &arc_surface_bounds(
                    patch,
                    &(
                        u_lo.try_clone().map_err(|_| {
                            failure(
                                AdaptiveTessellationCondition::ChordUnboundable,
                                Some(String::from(identity)),
                            )
                        })?,
                        u_hi.try_clone().map_err(|_| {
                            failure(
                                AdaptiveTessellationCondition::ChordUnboundable,
                                Some(String::from(identity)),
                            )
                        })?,
                    ),
                    &(
                        v_lo.try_clone().map_err(|_| {
                            failure(
                                AdaptiveTessellationCondition::ChordUnboundable,
                                Some(String::from(identity)),
                            )
                        })?,
                        v_hi.try_clone().map_err(|_| {
                            failure(
                                AdaptiveTessellationCondition::ChordUnboundable,
                                Some(String::from(identity)),
                            )
                        })?,
                    ),
                    identity,
                )?,
                &vertices[left].point,
                &vertices[right].point,
                identity,
            )?,
            None => edge_deviation_bounds(vertices, left, right, patch, identity)?,
        })
    };
    let shape = match arc {
        Some((u_lower, u_upper, v_lower, v_upper)) => {
            ChordBoundEdgeShape::Arc(Box::new(ChordBoundArcRange {
                u_lower,
                u_upper,
                v_lower,
                v_upper,
            }))
        }
        None => ChordBoundEdgeShape::Line,
    };
    Ok(EdgeChordParts {
        midpoint_u,
        midpoint_v,
        shape,
        squared_bound: deviations
            .map(|deviations| deviation_squared_upper(&deviations, bits, identity))
            .transpose()?,
    })
}

fn requested_squared(value: f64, identity: &str) -> Result<Dyadic, CadFailure> {
    let dyadic = Dyadic::from_f64_exact(value).map_err(|_| {
        failure(
            AdaptiveTessellationCondition::InvalidPrecision,
            Some(String::from(identity)),
        )
    })?;
    dyadic.mul(&dyadic).map_err(|_| {
        failure(
            AdaptiveTessellationCondition::InvalidPrecision,
            Some(String::from(identity)),
        )
    })
}

fn requested_dyadic(value: f64, identity: &str) -> Result<Dyadic, CadFailure> {
    Dyadic::from_f64_exact(value).map_err(|_| {
        failure(
            AdaptiveTessellationCondition::InvalidPrecision,
            Some(String::from(identity)),
        )
    })
}

fn established_chord_f64(squared_upper: &Dyadic, identity: &str) -> Result<f64, CadFailure> {
    let rounded = squared_upper.round_to_f64_ties_even().map_err(|_| {
        failure(
            AdaptiveTessellationCondition::ChordUnboundable,
            Some(String::from(identity)),
        )
    })?;
    if !rounded.is_finite() || rounded < 0.0 {
        return Err(failure(
            AdaptiveTessellationCondition::ChordUnboundable,
            Some(String::from(identity)),
        ));
    }
    Ok(nonnegative_sqrt(rounded))
}

fn nonnegative_sqrt(value: f64) -> f64 {
    if value == 0.0 {
        return 0.0;
    }
    let mut estimate = value;
    for _ in 0..16 {
        estimate = 0.5 * (estimate + value / estimate);
    }
    estimate
}

fn midpoint_vertex(
    vertices: &mut Vec<MeshVertex>,
    slots: &mut [SeamSlot],
    cache: &mut BTreeMap<(usize, usize), usize>,
    patch: &ChartPatch,
    identity: &str,
    left: usize,
    right: usize,
) -> Result<Option<usize>, CadFailure> {
    let key = if left < right {
        (left, right)
    } else {
        (right, left)
    };
    if let Some(existing) = cache.get(&key) {
        return Ok(Some(*existing));
    }
    let (u, v) = edge_mid_parameters(vertices, left, right, patch, identity)?;
    let inserted = insert_vertex(vertices, slots, patch, identity, u, v)?;
    if let Some(index) = inserted {
        cache.insert(key, index);
    }
    Ok(inserted)
}

fn split_failing_triangles(
    vertices: &mut Vec<MeshVertex>,
    triangles: Vec<MeshTriangle>,
    slots: &mut [SeamSlot],
    cache: &mut BTreeMap<(usize, usize), usize>,
    patches: &[ChartPatch],
    identity: &str,
) -> Result<Vec<MeshTriangle>, CadFailure> {
    let mut next = Vec::new();
    for triangle in triangles {
        let patch = &patches[triangle.patch];
        let ab = midpoint_vertex(
            vertices,
            slots,
            cache,
            patch,
            identity,
            triangle.vertices[0],
            triangle.vertices[1],
        )?;
        let bc = midpoint_vertex(
            vertices,
            slots,
            cache,
            patch,
            identity,
            triangle.vertices[1],
            triangle.vertices[2],
        )?;
        let ca = midpoint_vertex(
            vertices,
            slots,
            cache,
            patch,
            identity,
            triangle.vertices[2],
            triangle.vertices[0],
        )?;
        let (a, b, c) = (
            triangle.vertices[0],
            triangle.vertices[1],
            triangle.vertices[2],
        );
        match (ab, bc, ca) {
            (Some(ab), Some(bc), Some(ca)) => {
                next.push(MeshTriangle {
                    face_id: triangle.face_id.clone(),
                    patch: triangle.patch,
                    vertices: [a, ab, ca],
                    reverse: triangle.reverse,
                });
                next.push(MeshTriangle {
                    face_id: triangle.face_id.clone(),
                    patch: triangle.patch,
                    vertices: [ab, b, bc],
                    reverse: triangle.reverse,
                });
                next.push(MeshTriangle {
                    face_id: triangle.face_id.clone(),
                    patch: triangle.patch,
                    vertices: [ca, bc, c],
                    reverse: triangle.reverse,
                });
                next.push(MeshTriangle {
                    face_id: triangle.face_id,
                    patch: triangle.patch,
                    vertices: [ab, bc, ca],
                    reverse: triangle.reverse,
                });
            }
            _ => {
                return Err(failure(
                    AdaptiveTessellationCondition::SubdivisionLimit,
                    Some(String::from(identity)),
                ));
            }
        }
    }
    Ok(next)
}

fn grid_parameter(
    lower: &RealAlgebraic,
    upper: &RealAlgebraic,
    step: i32,
    steps: i32,
    identity: &str,
) -> Result<RealAlgebraic, CadFailure> {
    let span = as_rational(upper, identity)?
        .sub(&as_rational(lower, identity)?)
        .map_err(|_| {
            failure(
                AdaptiveTessellationCondition::TrimClassification,
                Some(String::from(identity)),
            )
        })?;
    let offset = span
        .mul(&integer_rational(step, identity)?)
        .map_err(|_| {
            failure(
                AdaptiveTessellationCondition::TrimClassification,
                Some(String::from(identity)),
            )
        })?
        .div(&integer_rational(steps, identity)?)
        .map_err(|_| {
            failure(
                AdaptiveTessellationCondition::TrimClassification,
                Some(String::from(identity)),
            )
        })?;
    from_rational(
        &as_rational(lower, identity)?.add(&offset).map_err(|_| {
            failure(
                AdaptiveTessellationCondition::TrimClassification,
                Some(String::from(identity)),
            )
        })?,
        identity,
    )
}

fn disk_lattice() -> Result<Vec<(RealAlgebraic, RealAlgebraic)>, CadFailure> {
    let mut points = vec![(algebraic(0)?, algebraic(0)?)];
    points.extend(equator_points()?);
    Ok(points)
}

fn push_triangle(
    triangles: &mut Vec<MeshTriangle>,
    face_id: &str,
    patch: usize,
    vertices: [Option<usize>; 3],
    reverse: bool,
) {
    if let [Some(a), Some(b), Some(c)] = vertices {
        if a != b && b != c && c != a {
            triangles.push(MeshTriangle {
                face_id: String::from(face_id),
                patch,
                vertices: if reverse { [a, c, b] } else { [a, b, c] },
                reverse,
            });
        }
    }
}

fn convex_unholed_trim(
    declaration: &TrimmedSurfaceDeclaration,
    identity: &str,
) -> Result<bool, CadFailure> {
    if !declaration.holes().is_empty() || declaration.outer().vertices().len() < 3 {
        return Ok(false);
    }
    let vertices = declaration.outer().vertices();
    let mut direction = None;
    for index in 0..vertices.len() {
        let first = &vertices[index];
        let second = &vertices[(index + 1) % vertices.len()];
        let third = &vertices[(index + 2) % vertices.len()];
        let orientation = segment_orientation(
            &as_rational(third.u(), identity)?,
            &as_rational(third.v(), identity)?,
            first,
            second,
            identity,
        )?;
        if orientation == Ordering::Equal {
            continue;
        }
        match direction {
            Some(expected) if expected != orientation => return Ok(false),
            Some(_) => {}
            None => direction = Some(orientation),
        }
    }
    Ok(direction.is_some())
}

fn attach_weld_identities(
    face: &FaceExactSurface,
    vertices: &mut [MeshVertex],
    identity: &str,
) -> Result<(), CadFailure> {
    for vertex in vertices {
        for weld_vertex in &face.weld_vertices {
            if points_equal(&vertex.point, &weld_vertex.point, identity)? {
                vertex.weld_identity = Some(weld_vertex.identity.clone());
                break;
            }
        }
    }
    Ok(())
}

fn chord_endpoint(vertex: &MeshVertex, identity: &str) -> Result<ChordBoundEndpoint, CadFailure> {
    Ok(ChordBoundEndpoint {
        u: as_rational(&vertex.u, identity)?,
        v: as_rational(&vertex.v, identity)?,
        point: point_rational(&vertex.point, identity)?,
    })
}

fn tessellate_face(
    face: &FaceExactSurface,
    identity: &str,
    precision: AdaptivePrecision,
    bits: u32,
) -> Result<(Vec<MeshVertex>, Vec<MeshTriangle>, FaceChordBoundWitness), CadFailure> {
    let mut slots = Vec::new();
    for seam in face.covering.seams() {
        slots.push(SeamSlot {
            first_chart: String::from(seam.first().identity().domain_id().as_str()),
            first_u: clone_algebraic(seam.first().parameter().u(), identity)?,
            first_v: clone_algebraic(seam.first().parameter().v(), identity)?,
            second_chart: String::from(seam.second().identity().domain_id().as_str()),
            second_u: clone_algebraic(seam.second().parameter().u(), identity)?,
            second_v: clone_algebraic(seam.second().parameter().v(), identity)?,
            vertex: None,
        });
    }
    let mut vertices = Vec::new();
    let mut triangles = Vec::new();
    for (patch_index, patch) in face.covering.patches().iter().enumerate() {
        let reverse = patch.chart().orientation() == SurfaceOrientation::Reverse;
        if let Some(declaration) = patch.trimmed() {
            if convex_unholed_trim(declaration, identity)? {
                let mut indices = Vec::new();
                for point in declaration.outer().vertices() {
                    indices.push(insert_vertex(
                        &mut vertices,
                        &mut slots,
                        patch,
                        identity,
                        clone_algebraic(point.u(), identity)?,
                        clone_algebraic(point.v(), identity)?,
                    )?);
                }
                for index in 1..indices.len() - 1 {
                    push_triangle(
                        &mut triangles,
                        face.face_id(),
                        patch_index,
                        [indices[0], indices[index], indices[index + 1]],
                        reverse,
                    );
                }
                continue;
            }
        }
        if patch.disk() {
            let lattice = disk_lattice()?;
            let mut indices = Vec::new();
            for (u, v) in lattice {
                indices.push(insert_vertex(
                    &mut vertices,
                    &mut slots,
                    patch,
                    identity,
                    u,
                    v,
                )?);
            }
            let pole = indices[0];
            for window in 0..4 {
                let first = indices[1 + window];
                let second = indices[1 + ((window + 1) % 4)];
                push_triangle(
                    &mut triangles,
                    face.face_id(),
                    patch_index,
                    [pole, first, second],
                    reverse,
                );
            }
        } else {
            let steps = 4;
            let mut indices = Vec::new();
            for row in 0..=steps {
                for column in 0..=steps {
                    let u = grid_parameter(
                        patch.domain().u().lower(),
                        patch.domain().u().upper(),
                        column,
                        steps,
                        identity,
                    )?;
                    let v = grid_parameter(
                        patch.domain().v().lower(),
                        patch.domain().v().upper(),
                        row,
                        steps,
                        identity,
                    )?;
                    indices.push(insert_vertex(
                        &mut vertices,
                        &mut slots,
                        patch,
                        identity,
                        u,
                        v,
                    )?);
                }
            }
            let stride = (steps + 1) as usize;
            for row in 0..steps as usize {
                for column in 0..steps as usize {
                    let a = indices[row * stride + column];
                    let b = indices[row * stride + column + 1];
                    let c = indices[(row + 1) * stride + column + 1];
                    let d = indices[(row + 1) * stride + column];
                    push_triangle(
                        &mut triangles,
                        face.face_id(),
                        patch_index,
                        [a, b, c],
                        reverse,
                    );
                    push_triangle(
                        &mut triangles,
                        face.face_id(),
                        patch_index,
                        [a, c, d],
                        reverse,
                    );
                }
            }
        }
    }
    let requested_sq = requested_squared(precision.maximum_chord_error(), identity)?;
    let _ = requested_dyadic(precision.maximum_surface_error(), identity)?;
    let mut midpoints = BTreeMap::new();
    for depth in 0..=DEFAULT_MAX_SUBDIVISION_DEPTH {
        let mut kept = Vec::new();
        let mut kept_edge_keys = Vec::new();
        let mut evaluations: BTreeMap<(usize, usize, usize), EdgeChordParts> = BTreeMap::new();
        let mut refine = false;
        let mut level_max: Option<Dyadic> = None;
        for triangle in triangles {
            match triangle_kind(
                &face.covering.patches()[triangle.patch],
                &vertices,
                &triangle,
                identity,
            )? {
                TriangleKind::Discard | TriangleKind::Mixed => {}
                TriangleKind::Keep => {
                    let patch = &face.covering.patches()[triangle.patch];
                    let edges = [
                        (triangle.vertices[0], triangle.vertices[1]),
                        (triangle.vertices[1], triangle.vertices[2]),
                        (triangle.vertices[2], triangle.vertices[0]),
                    ];
                    let mut keys = Vec::with_capacity(3);
                    for (left, right) in edges {
                        let key = (triangle.patch, left.min(right), left.max(right));
                        let parts = match evaluations.entry(key) {
                            Entry::Occupied(occupied) => occupied.into_mut(),
                            Entry::Vacant(vacant) => vacant.insert(edge_chord_parts(
                                &vertices, left, right, patch, bits, identity,
                            )?),
                        };
                        if let Some(upper) = &parts.squared_bound {
                            if upper > &requested_sq {
                                refine = true;
                            }
                            let upper = upper.try_clone().map_err(|_| {
                                failure(
                                    AdaptiveTessellationCondition::ChordUnboundable,
                                    Some(String::from(identity)),
                                )
                            })?;
                            level_max = Some(match level_max {
                                Some(current) if current > upper => current,
                                _ => upper,
                            });
                        }
                        keys.push(key);
                    }
                    kept_edge_keys.push(keys);
                    kept.push(triangle);
                }
            }
        }
        if !refine {
            attach_weld_identities(face, &mut vertices, identity)?;
            let mut edge_order: Vec<(usize, usize, usize)> = evaluations.keys().copied().collect();
            edge_order.sort_unstable();
            let mut edge_index: BTreeMap<(usize, usize, usize), usize> = BTreeMap::new();
            let mut witness_edges = Vec::with_capacity(edge_order.len());
            for key in edge_order {
                let (_, left, right) = key;
                let parts = evaluations.remove(&key).ok_or_else(|| {
                    failure(
                        AdaptiveTessellationCondition::ChordUnboundable,
                        Some(String::from(identity)),
                    )
                })?;
                edge_index.insert(key, witness_edges.len());
                witness_edges.push(ChordBoundEdgeEvaluation {
                    patch_index: key.0,
                    left: chord_endpoint(&vertices[left], identity)?,
                    right: chord_endpoint(&vertices[right], identity)?,
                    midpoint_u: parts.midpoint_u,
                    midpoint_v: parts.midpoint_v,
                    shape: parts.shape,
                    squared_bound: parts.squared_bound,
                });
            }
            let mut witness_triangles = Vec::with_capacity(kept.len());
            for (triangle, keys) in kept.iter().zip(kept_edge_keys.iter()) {
                let mut edge_indices = [0_usize; 3];
                for (slot, key) in keys.iter().enumerate() {
                    edge_indices[slot] = *edge_index.get(key).ok_or_else(|| {
                        failure(
                            AdaptiveTessellationCondition::ChordUnboundable,
                            Some(String::from(identity)),
                        )
                    })?;
                }
                witness_triangles.push(ChordBoundTriangleWitness {
                    patch_index: triangle.patch,
                    edge_indices,
                });
            }
            let witness = FaceChordBoundWitness {
                face_id: String::from(face.face_id()),
                established_squared_bound: level_max,
                edges: witness_edges,
                triangles: witness_triangles,
            };
            return Ok((vertices, kept, witness));
        }
        if depth == DEFAULT_MAX_SUBDIVISION_DEPTH {
            return Err(failure(
                AdaptiveTessellationCondition::SubdivisionLimit,
                Some(String::from(identity)),
            ));
        }
        triangles = split_failing_triangles(
            &mut vertices,
            kept,
            &mut slots,
            &mut midpoints,
            face.covering.patches(),
            identity,
        )?;
    }
    Err(failure(
        AdaptiveTessellationCondition::SubdivisionLimit,
        Some(String::from(identity)),
    ))
}

fn points_equal(left: &Point3, right: &Point3, identity: &str) -> Result<bool, CadFailure> {
    for (left_coordinate, right_coordinate) in [
        (&left.x, &right.x),
        (&left.y, &right.y),
        (&left.z, &right.z),
    ] {
        if left_coordinate.compare(right_coordinate).map_err(|_| {
            failure(
                AdaptiveTessellationCondition::InvalidMesh,
                Some(String::from(identity)),
            )
        })? != Ordering::Equal
        {
            return Ok(false);
        }
    }
    Ok(true)
}

fn weld_exact_vertices(
    vertices: &[MeshVertex],
    triangles: &[MeshTriangle],
    projected: &BTreeMap<usize, ([f64; 3], Dyadic)>,
    identity: &str,
) -> Result<ProjectedSurfaceMesh, CadFailure> {
    let mut weld_groups: BTreeMap<Vec<String>, (usize, usize)> = BTreeMap::new();
    let mut mesh_vertices = Vec::new();
    let mut remap = vec![None; vertices.len()];
    for triangle in triangles {
        for source_index in triangle.vertices {
            if remap[source_index].is_some() {
                continue;
            }
            let mesh_index =
                if let Some(weld_identity) = vertices[source_index].weld_identity.as_ref() {
                    match weld_groups.get(weld_identity) {
                        Some((representative, mesh_index)) => {
                            if !points_equal(
                                &vertices[source_index].point,
                                &vertices[*representative].point,
                                identity,
                            )? {
                                return Err(failure(
                                    AdaptiveTessellationCondition::InvalidMesh,
                                    Some(String::from(identity)),
                                ));
                            }
                            *mesh_index
                        }
                        None => {
                            let coordinate = projected.get(&source_index).ok_or_else(|| {
                                failure(
                                    AdaptiveTessellationCondition::Projection,
                                    Some(String::from(identity)),
                                )
                            })?;
                            mesh_vertices.push(coordinate.0);
                            let mesh_index = mesh_vertices.len() - 1;
                            weld_groups.insert(weld_identity.clone(), (source_index, mesh_index));
                            mesh_index
                        }
                    }
                } else {
                    let coordinate = projected.get(&source_index).ok_or_else(|| {
                        failure(
                            AdaptiveTessellationCondition::Projection,
                            Some(String::from(identity)),
                        )
                    })?;
                    mesh_vertices.push(coordinate.0);
                    mesh_vertices.len() - 1
                };
            remap[source_index] = Some(mesh_index);
        }
    }
    let mut mesh_triangles = Vec::new();
    for triangle in triangles {
        let mut mapped = [0; 3];
        for (slot, source_index) in triangle.vertices.iter().enumerate() {
            mapped[slot] = remap[*source_index].ok_or_else(|| {
                failure(
                    AdaptiveTessellationCondition::InvalidMesh,
                    Some(String::from(identity)),
                )
            })?;
        }
        mesh_triangles.push(mapped);
    }
    Ok((mesh_vertices, mesh_triangles))
}

pub fn tessellate_adaptive(
    request: AdaptiveTessellationRequest,
) -> Result<AdaptiveTessellation, CadFailure> {
    let identity = String::from(request.identity.as_str());
    if request.brep.faces().is_empty() {
        return Err(failure(
            AdaptiveTessellationCondition::EmptyFaces,
            Some(identity),
        ));
    }
    let mut known = BTreeMap::new();
    for surface in &request.surfaces {
        known.insert(surface.face_id.as_str(), surface);
    }
    for face in request.brep.faces() {
        if !known.contains_key(face.id()) {
            return Err(failure(
                AdaptiveTessellationCondition::MissingCovering,
                Some(identity),
            ));
        }
    }
    for surface in &request.surfaces {
        if request
            .brep
            .faces()
            .iter()
            .all(|face| face.id() != surface.face_id)
        {
            return Err(failure(
                AdaptiveTessellationCondition::UnknownFace,
                Some(identity),
            ));
        }
    }
    let view_elements = tessellate_view_elements(&request.brep)?;
    let revision = request.brep.witnesses().source_revision().clone();
    let bits = if request.enclosure_bits == 0 {
        DEFAULT_NUMERIC_PROJECTION_BITS
    } else {
        request.enclosure_bits
    };
    let mut vertices = Vec::new();
    let mut triangles = Vec::new();
    let mut face_atom_ids = Vec::new();
    let mut max_squared = Dyadic::from_f64_exact(0.0).map_err(|_| {
        failure(
            AdaptiveTessellationCondition::ChordUnboundable,
            Some(identity.clone()),
        )
    })?;
    let mut chord_bound_witnesses = Vec::new();
    let mut offset = 0;
    for surface in &request.surfaces {
        let (local_vertices, local_triangles, face_witness) =
            tessellate_face(surface, &identity, request.precision, bits)?;
        if let Some(face_maximum) = face_witness.established_squared_bound() {
            if face_maximum > &max_squared {
                max_squared = face_maximum.try_clone().map_err(|_| {
                    failure(
                        AdaptiveTessellationCondition::ChordUnboundable,
                        Some(identity.clone()),
                    )
                })?;
            }
        }
        chord_bound_witnesses.push(face_witness);
        face_atom_ids.push((
            surface.face_id.clone(),
            String::from(surface.exact_surface.atom_id()),
        ));
        for mut triangle in local_triangles {
            triangle.vertices = [
                triangle.vertices[0] + offset,
                triangle.vertices[1] + offset,
                triangle.vertices[2] + offset,
            ];
            triangles.push(triangle);
        }
        offset += local_vertices.len();
        vertices.extend(local_vertices);
    }
    if triangles.is_empty() {
        return Err(failure(
            AdaptiveTessellationCondition::EmptyFaces,
            Some(identity),
        ));
    }
    let mut projected: BTreeMap<usize, ([f64; 3], Dyadic)> = BTreeMap::new();
    for triangle in &triangles {
        let surface = known.get(triangle.face_id.as_str()).ok_or_else(|| {
            failure(
                AdaptiveTessellationCondition::UnknownFace,
                Some(identity.clone()),
            )
        })?;
        for index in triangle.vertices {
            if projected.contains_key(&index) {
                continue;
            }
            projected.insert(
                index,
                project_point(
                    &identity,
                    &revision,
                    surface.exact_surface(),
                    &vertices[index].point,
                    bits,
                )?,
            );
        }
    }
    let mut max_residual = projected
        .values()
        .next()
        .ok_or_else(|| {
            failure(
                AdaptiveTessellationCondition::Projection,
                Some(identity.clone()),
            )
        })?
        .1
        .try_clone()
        .map_err(|_| {
            failure(
                AdaptiveTessellationCondition::Projection,
                Some(identity.clone()),
            )
        })?;
    for (_, residual) in projected.values() {
        if residual > &max_residual {
            max_residual = residual.try_clone().map_err(|_| {
                failure(
                    AdaptiveTessellationCondition::Projection,
                    Some(identity.clone()),
                )
            })?;
        }
    }
    let requested_surface_error =
        requested_dyadic(request.precision.maximum_surface_error(), &identity)?;
    if max_residual > requested_surface_error {
        return Err(failure(
            AdaptiveTessellationCondition::SurfaceErrorExceeded,
            Some(identity),
        ));
    }
    let (mesh_vertices, mesh_triangles) =
        weld_exact_vertices(&vertices, &triangles, &projected, &identity)?;
    let mesh = SurfaceMesh::new(mesh_vertices, mesh_triangles, Vec::new()).map_err(|_| {
        failure(
            AdaptiveTessellationCondition::InvalidMesh,
            Some(identity.clone()),
        )
    })?;
    let validated = validate_surface_mesh(mesh).map_err(|_| {
        failure(
            AdaptiveTessellationCondition::InvalidMesh,
            Some(identity.clone()),
        )
    })?;
    let mut face_ids = Vec::new();
    let mut correspondence = Vec::new();
    for face in request.brep.faces() {
        face_ids.push(String::from(face.id()));
        let triangle_indices = triangles
            .iter()
            .enumerate()
            .filter(|(_, triangle)| triangle.face_id == face.id())
            .map(|(index, _)| index)
            .collect();
        correspondence.push(TessellatedFace::new(
            String::from(face.id()),
            triangle_indices,
        ));
    }
    Ok(AdaptiveTessellation {
        identity: request.identity,
        source_revision: revision,
        validated_surface_mesh: validated,
        precision_witness: AdaptivePrecisionWitness::new(
            request.precision.consumer(),
            request.precision.maximum_chord_error(),
            established_chord_f64(&max_squared, &identity)?,
            request.precision.maximum_surface_error(),
            max_residual,
        ),
        brep_element_correspondence: BrepElementCorrespondence::new(face_ids),
        face_triangle_correspondence: correspondence,
        provenance: AdaptiveTessellationProvenance {
            enclosure_bits: bits,
            face_atom_ids,
        },
        chord_bound_witnesses,
        view_elements,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn origin() -> Point3 {
        Point3 {
            x: algebraic(0).unwrap(),
            y: algebraic(0).unwrap(),
            z: algebraic(0).unwrap(),
        }
    }

    #[test]
    fn welding_requires_a_shared_topology_identity() {
        let vertices = vec![
            MeshVertex {
                u: algebraic(0).unwrap(),
                v: algebraic(0).unwrap(),
                point: origin(),
                weld_identity: None,
            },
            MeshVertex {
                u: algebraic(0).unwrap(),
                v: algebraic(0).unwrap(),
                point: origin(),
                weld_identity: None,
            },
        ];
        let triangles = vec![
            MeshTriangle {
                face_id: String::from("first"),
                patch: 0,
                vertices: [0, 0, 0],
                reverse: false,
            },
            MeshTriangle {
                face_id: String::from("second"),
                patch: 0,
                vertices: [1, 1, 1],
                reverse: false,
            },
        ];
        let projected = BTreeMap::from([
            (0, ([0.0, 0.0, 0.0], Dyadic::from_f64_exact(0.0).unwrap())),
            (1, ([0.0, 0.0, 0.0], Dyadic::from_f64_exact(0.0).unwrap())),
        ]);
        let (mesh_vertices, mesh_triangles) =
            weld_exact_vertices(&vertices, &triangles, &projected, "contact").unwrap();
        assert_eq!(mesh_vertices.len(), 2);
        assert_ne!(mesh_triangles[0][0], mesh_triangles[1][0]);
    }

    #[test]
    fn topology_identity_preserves_atom_id_structure() {
        let vertices = vec![
            MeshVertex {
                u: algebraic(0).unwrap(),
                v: algebraic(0).unwrap(),
                point: origin(),
                weld_identity: Some(vec![
                    String::from("a"),
                    String::from("b:c"),
                    String::from("z"),
                ]),
            },
            MeshVertex {
                u: algebraic(0).unwrap(),
                v: algebraic(0).unwrap(),
                point: origin(),
                weld_identity: Some(vec![
                    String::from("a:b"),
                    String::from("c"),
                    String::from("z"),
                ]),
            },
        ];
        let triangles = vec![
            MeshTriangle {
                face_id: String::from("first"),
                patch: 0,
                vertices: [0, 0, 0],
                reverse: false,
            },
            MeshTriangle {
                face_id: String::from("second"),
                patch: 0,
                vertices: [1, 1, 1],
                reverse: false,
            },
        ];
        let projected = BTreeMap::from([
            (0, ([0.0, 0.0, 0.0], Dyadic::from_f64_exact(0.0).unwrap())),
            (1, ([0.0, 0.0, 0.0], Dyadic::from_f64_exact(0.0).unwrap())),
        ]);
        let (mesh_vertices, mesh_triangles) =
            weld_exact_vertices(&vertices, &triangles, &projected, "structured").unwrap();
        assert_eq!(mesh_vertices.len(), 2);
        assert_ne!(mesh_triangles[0][0], mesh_triangles[1][0]);
    }

    #[test]
    fn certified_edge_bound_covers_sampled_chord_deviation() {
        let center = [
            integer_rational(0, "bound").unwrap(),
            integer_rational(0, "bound").unwrap(),
            integer_rational(0, "bound").unwrap(),
        ];
        let radius = integer_rational(1, "bound").unwrap();
        let chart = stereographic_chart_rational("bound-south", true, &center, &radius).unwrap();
        let patch = ChartPatch::Full {
            chart,
            domain: unit_square_domain().unwrap(),
            disk: Some(DiskConstraint),
        };
        let make_vertex = |u: RealAlgebraic, v: RealAlgebraic| {
            let point = evaluate_exact(&patch, &u, &v, "bound").unwrap();
            MeshVertex {
                u,
                v,
                point,
                weld_identity: None,
            }
        };
        let vertices = vec![
            make_vertex(algebraic(0).unwrap(), algebraic(0).unwrap()),
            make_vertex(ratio(3, 5).unwrap(), ratio(4, 5).unwrap()),
            make_vertex(ratio(-4, 5).unwrap(), ratio(3, 5).unwrap()),
        ];
        let cap = Dyadic::from_f64_exact(64.0).unwrap();
        for (left, right) in [(0_usize, 1_usize), (1, 2)] {
            let bound = edge_chord_parts(&vertices, left, right, &patch, 48, "bound")
                .unwrap()
                .squared_bound
                .expect("bound established");
            assert!(bound > Dyadic::from_f64_exact(0.0).unwrap());
            assert!(bound < cap);
            let half_u = mid_value(&vertices[left].u, &vertices[right].u, "bound").unwrap();
            let half_v = mid_value(&vertices[left].v, &vertices[right].v, "bound").unwrap();
            let samples = [
                (
                    half_u.try_clone().unwrap(),
                    half_v.try_clone().unwrap(),
                    ratio(1, 2).unwrap(),
                ),
                (
                    mid_value(&vertices[left].u, &half_u, "bound").unwrap(),
                    mid_value(&vertices[left].v, &half_v, "bound").unwrap(),
                    ratio(1, 4).unwrap(),
                ),
                (
                    mid_value(&half_u, &vertices[right].u, "bound").unwrap(),
                    mid_value(&half_v, &vertices[right].v, "bound").unwrap(),
                    ratio(3, 4).unwrap(),
                ),
            ];
            for (u, v, parameter) in samples {
                let surface = evaluate_exact(&patch, &u, &v, "bound").unwrap();
                let mut squared = integer_rational(0, "bound").unwrap();
                for (surface_axis, left_axis, right_axis) in [
                    (
                        &surface.x,
                        &vertices[left].point.x,
                        &vertices[right].point.x,
                    ),
                    (
                        &surface.y,
                        &vertices[left].point.y,
                        &vertices[right].point.y,
                    ),
                    (
                        &surface.z,
                        &vertices[left].point.z,
                        &vertices[right].point.z,
                    ),
                ] {
                    let left_axis = as_rational(left_axis, "bound").unwrap();
                    let span = rational_sub(
                        &as_rational(right_axis, "bound").unwrap(),
                        &left_axis,
                        "bound",
                    )
                    .unwrap();
                    let chord_axis = rational_add(
                        &left_axis,
                        &rational_mul(&as_rational(&parameter, "bound").unwrap(), &span, "bound")
                            .unwrap(),
                        "bound",
                    )
                    .unwrap();
                    let deviation = rational_sub(
                        &as_rational(surface_axis, "bound").unwrap(),
                        &chord_axis,
                        "bound",
                    )
                    .unwrap();
                    squared = rational_add(
                        &squared,
                        &rational_mul(&deviation, &deviation, "bound").unwrap(),
                        "bound",
                    )
                    .unwrap();
                }
                assert!(squared.dyadic_floor(48).unwrap() <= bound);
            }
        }
    }

    #[test]
    fn rational_nurbs_patch_lies_on_the_general_sphere() {
        let center = Point3 {
            x: algebraic(2).unwrap(),
            y: algebraic(-3).unwrap(),
            z: algebraic(4).unwrap(),
        };
        let radius = algebraic(2).unwrap();
        for ((u_numerator, u_denominator), (v_numerator, v_denominator)) in [
            ((1, 2), (0, 1)),
            ((0, 1), (1, 2)),
            ((1, 2), (1, 2)),
            ((1, 1), (1, 1)),
        ] {
            let representation = sphere_nurbs_surface(&center, &radius).unwrap();
            let point =
                match neco_nurbs::evaluate_nurbs(neco_nurbs::NurbsEvaluationRequest::Surface {
                    representation,
                    u: ratio(u_numerator, u_denominator).unwrap(),
                    v: ratio(v_numerator, v_denominator).unwrap(),
                })
                .unwrap()
                .into_point()
                {
                    BezierPoint::Point3(point) => point,
                    BezierPoint::Point2(_) => {
                        panic!("sphere surface returned a two-dimensional point")
                    }
                };
            let squared_distance = point
                .x
                .sub(&center.x)
                .and_then(|value| value.mul(&value))
                .and_then(|x| {
                    point
                        .y
                        .sub(&center.y)
                        .and_then(|value| value.mul(&value))
                        .and_then(|y| x.add(&y))
                })
                .and_then(|xy| {
                    point
                        .z
                        .sub(&center.z)
                        .and_then(|value| value.mul(&value))
                        .and_then(|z| xy.add(&z))
                })
                .unwrap();
            let radius_squared = radius.mul(&radius).unwrap();
            assert_eq!(
                squared_distance.compare(&radius_squared).unwrap(),
                Ordering::Equal
            );
        }
    }
}
