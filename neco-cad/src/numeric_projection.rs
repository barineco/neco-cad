use crate::provenance::SourceIdentity;
use crate::types::{CadFailure, NumericProjectionCondition};
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use neco_algnum::RealAlgebraic;
use neco_bigint::{BigInt, Dyadic, DyadicEnclosure, Sign};
use neco_brep::SourceRevision;
use neco_geometry::{
    evaluate_surface_witness_exact, CanonicalAtom, ParametricSurfaceChart2, Point3,
    RationalFunction3, RegionFunction, RegionPolynomial3, SurfaceEvaluationWitness,
    SurfaceParameterDomain,
};
use neco_tessellation::{project_real_algebraic, ProjectedCoordinate, TessellationFailure};

fn failure(condition: NumericProjectionCondition, identity: Option<String>) -> CadFailure {
    CadFailure::NumericProjection {
        condition,
        identity,
    }
}

pub const DEFAULT_NUMERIC_PROJECTION_BITS: u32 = 48;

#[derive(Debug, Eq, PartialEq)]
pub enum NumericSample {
    AlgebraicPoint(Point3),
    AnalyticSurface {
        chart: Box<ParametricSurfaceChart2>,
        domain: SurfaceParameterDomain,
        u: RealAlgebraic,
        v: RealAlgebraic,
    },
}

#[derive(Debug, Eq, PartialEq)]
pub struct NumericProjectionRequest {
    identity: SourceIdentity,
    source_revision: SourceRevision,
    exact_surface: CanonicalAtom,
    enclosure_bits: u32,
    samples: Vec<NumericSample>,
}

impl NumericProjectionRequest {
    pub fn new(
        identity: impl Into<String>,
        source_revision: SourceRevision,
        exact_surface: CanonicalAtom,
        enclosure_bits: u32,
        samples: Vec<NumericSample>,
    ) -> Result<Self, CadFailure> {
        let identity = SourceIdentity::new(identity)
            .map_err(|_| failure(NumericProjectionCondition::EmptyIdentity, None))?;
        Ok(Self {
            identity,
            source_revision,
            exact_surface,
            enclosure_bits,
            samples,
        })
    }

    pub fn identity(&self) -> &SourceIdentity {
        &self.identity
    }

    pub fn source_revision(&self) -> &SourceRevision {
        &self.source_revision
    }

    pub fn exact_surface(&self) -> &CanonicalAtom {
        &self.exact_surface
    }

    pub fn enclosure_bits(&self) -> u32 {
        self.enclosure_bits
    }

    pub fn samples(&self) -> &[NumericSample] {
        &self.samples
    }
}

#[derive(Debug)]
pub struct ProjectedPoint {
    rounded: [f64; 3],
    x: ProjectedCoordinate,
    y: ProjectedCoordinate,
    z: ProjectedCoordinate,
    surface_distance_bound: Dyadic,
}

impl ProjectedPoint {
    pub fn rounded(&self) -> [f64; 3] {
        self.rounded
    }

    pub fn x(&self) -> &ProjectedCoordinate {
        &self.x
    }

    pub fn y(&self) -> &ProjectedCoordinate {
        &self.y
    }

    pub fn z(&self) -> &ProjectedCoordinate {
        &self.z
    }

    pub fn surface_distance_bound(&self) -> &Dyadic {
        &self.surface_distance_bound
    }
}

impl PartialEq for ProjectedPoint {
    fn eq(&self, other: &Self) -> bool {
        self.rounded
            .iter()
            .zip(other.rounded)
            .all(|(left, right)| left.to_bits() == right.to_bits())
            && self.x == other.x
            && self.y == other.y
            && self.z == other.z
            && self.surface_distance_bound == other.surface_distance_bound
    }
}

impl Eq for ProjectedPoint {}

#[derive(Debug, Eq, PartialEq)]
pub struct NumericProjectionProvenance {
    enclosure_bits: u32,
    exact_surface_atom_id: String,
}

impl NumericProjectionProvenance {
    pub fn enclosure_bits(&self) -> u32 {
        self.enclosure_bits
    }

    pub fn exact_surface_atom_id(&self) -> &str {
        &self.exact_surface_atom_id
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct NumericProjection {
    identity: SourceIdentity,
    source_revision: SourceRevision,
    coordinates: Vec<ProjectedPoint>,
    provenance: NumericProjectionProvenance,
}

impl NumericProjection {
    pub fn identity(&self) -> &SourceIdentity {
        &self.identity
    }

    pub fn source_revision(&self) -> &SourceRevision {
        &self.source_revision
    }

    pub fn coordinates(&self) -> &[ProjectedPoint] {
        &self.coordinates
    }

    pub fn provenance(&self) -> &NumericProjectionProvenance {
        &self.provenance
    }
}

pub fn project_numeric_coordinates(
    request: NumericProjectionRequest,
) -> Result<NumericProjection, CadFailure> {
    let identity = String::from(request.identity.as_str());
    let mut coordinates = Vec::new();
    for sample in &request.samples {
        let point = exact_point(sample, &identity)?;
        let x = project_coordinate(&point.x, request.enclosure_bits, &identity)?;
        let y = project_coordinate(&point.y, request.enclosure_bits, &identity)?;
        let z = project_coordinate(&point.z, request.enclosure_bits, &identity)?;
        let surface_distance_bound = residual_bound(
            request.exact_surface.function(),
            x.enclosure(),
            y.enclosure(),
            z.enclosure(),
            request.enclosure_bits,
            &identity,
        )?;
        coordinates.push(ProjectedPoint {
            rounded: [x.value(), y.value(), z.value()],
            x,
            y,
            z,
            surface_distance_bound,
        });
    }
    Ok(NumericProjection {
        identity: request.identity,
        source_revision: request.source_revision,
        coordinates,
        provenance: NumericProjectionProvenance {
            enclosure_bits: request.enclosure_bits,
            exact_surface_atom_id: String::from(request.exact_surface.atom_id()),
        },
    })
}

fn exact_point(sample: &NumericSample, identity: &str) -> Result<Point3, CadFailure> {
    match sample {
        NumericSample::AlgebraicPoint(point) => Ok(Point3 {
            x: clone_algebraic(&point.x, identity)?,
            y: clone_algebraic(&point.y, identity)?,
            z: clone_algebraic(&point.z, identity)?,
        }),
        NumericSample::AnalyticSurface {
            chart,
            domain,
            u,
            v,
        } => match evaluate_surface_witness_exact(chart, domain, u, v) {
            Ok(SurfaceEvaluationWitness::Exact { point, .. }) => Ok(point),
            Ok(SurfaceEvaluationWitness::Certified { .. }) | Err(_) => Err(failure(
                NumericProjectionCondition::SurfaceEvaluation,
                Some(String::from(identity)),
            )),
        },
    }
}

fn clone_algebraic(value: &RealAlgebraic, identity: &str) -> Result<RealAlgebraic, CadFailure> {
    value.try_clone().map_err(|_| {
        failure(
            NumericProjectionCondition::AlgebraicEnclosure,
            Some(String::from(identity)),
        )
    })
}

fn project_coordinate(
    value: &RealAlgebraic,
    enclosure_bits: u32,
    identity: &str,
) -> Result<ProjectedCoordinate, CadFailure> {
    project_real_algebraic(value, enclosure_bits).map_err(|error| match error {
        TessellationFailure::NonFiniteCoordinate => failure(
            NumericProjectionCondition::NonFiniteCoordinate,
            Some(String::from(identity)),
        ),
        _ => failure(
            NumericProjectionCondition::AlgebraicEnclosure,
            Some(String::from(identity)),
        ),
    })
}

fn residual_bound(
    function: &RegionFunction,
    x: &DyadicEnclosure,
    y: &DyadicEnclosure,
    z: &DyadicEnclosure,
    bits: u32,
    identity: &str,
) -> Result<Dyadic, CadFailure> {
    match function {
        RegionFunction::Polynomial(polynomial) => max_abs(
            &evaluate_polynomial_enclosure(polynomial, x, y, z, bits, identity)?,
            identity,
        ),
        RegionFunction::RationalFunction(function) => {
            rational_residual(function, x, y, z, bits, identity)
        }
    }
}

fn rational_residual(
    function: &RationalFunction3,
    x: &DyadicEnclosure,
    y: &DyadicEnclosure,
    z: &DyadicEnclosure,
    bits: u32,
    identity: &str,
) -> Result<Dyadic, CadFailure> {
    let numerator = evaluate_polynomial_enclosure(function.numerator(), x, y, z, bits, identity)?;
    let denominator =
        evaluate_polynomial_enclosure(function.denominator(), x, y, z, bits, identity)?;
    if contains_zero(&denominator) {
        return Err(failure(
            NumericProjectionCondition::ResidualUnboundable,
            Some(String::from(identity)),
        ));
    }
    let numerator_bound = max_abs(&numerator, identity)?;
    let denominator_bound = min_abs(&denominator, identity)?;
    if denominator_bound.is_zero() {
        return Err(failure(
            NumericProjectionCondition::ResidualUnboundable,
            Some(String::from(identity)),
        ));
    }
    let numerator_algebraic = dyadic_to_algebraic(&numerator_bound, identity)?;
    let denominator_algebraic = dyadic_to_algebraic(&denominator_bound, identity)?;
    let ratio = numerator_algebraic
        .div(&denominator_algebraic)
        .map_err(|_| {
            failure(
                NumericProjectionCondition::ResidualUnboundable,
                Some(String::from(identity)),
            )
        })?;
    let enclosure = ratio.enclose(bits).map_err(|_| {
        failure(
            NumericProjectionCondition::ResidualUnboundable,
            Some(String::from(identity)),
        )
    })?;
    max_abs(enclosure.enclosure(), identity)
}

fn evaluate_polynomial_enclosure(
    polynomial: &RegionPolynomial3,
    x: &DyadicEnclosure,
    y: &DyadicEnclosure,
    z: &DyadicEnclosure,
    bits: u32,
    identity: &str,
) -> Result<DyadicEnclosure, CadFailure> {
    let mut result = DyadicEnclosure::new(dyadic_zero()?, dyadic_zero()?)
        .map_err(|_| residual_failure(identity))?;
    for term in polynomial.terms() {
        let coefficient = project_real_algebraic(term.coefficient(), bits)
            .map_err(|_| residual_failure(identity))?;
        let powered = enclosure_mul(
            &enclosure_mul(
                &enclosure_pow(x, term.exponent_x(), identity)?,
                &enclosure_pow(y, term.exponent_y(), identity)?,
                identity,
            )?,
            &enclosure_pow(z, term.exponent_z(), identity)?,
            identity,
        )?;
        let term_value = enclosure_mul(coefficient.enclosure(), &powered, identity)?;
        result = enclosure_add(&result, &term_value, identity)?;
    }
    Ok(result)
}

fn residual_failure(identity: &str) -> CadFailure {
    failure(
        NumericProjectionCondition::ResidualUnboundable,
        Some(String::from(identity)),
    )
}

fn enclosure_add(
    left: &DyadicEnclosure,
    right: &DyadicEnclosure,
    identity: &str,
) -> Result<DyadicEnclosure, CadFailure> {
    DyadicEnclosure::new(
        left.lower()
            .add(right.lower())
            .map_err(|_| residual_failure(identity))?,
        left.upper()
            .add(right.upper())
            .map_err(|_| residual_failure(identity))?,
    )
    .map_err(|_| residual_failure(identity))
}

fn enclosure_mul(
    left: &DyadicEnclosure,
    right: &DyadicEnclosure,
    identity: &str,
) -> Result<DyadicEnclosure, CadFailure> {
    let products = [
        left.lower()
            .mul(right.lower())
            .map_err(|_| residual_failure(identity))?,
        left.lower()
            .mul(right.upper())
            .map_err(|_| residual_failure(identity))?,
        left.upper()
            .mul(right.lower())
            .map_err(|_| residual_failure(identity))?,
        left.upper()
            .mul(right.upper())
            .map_err(|_| residual_failure(identity))?,
    ];
    let mut lower = products[0]
        .try_clone()
        .map_err(|_| residual_failure(identity))?;
    let mut upper = products[0]
        .try_clone()
        .map_err(|_| residual_failure(identity))?;
    for product in products.iter().skip(1) {
        if product < &lower {
            lower = product
                .try_clone()
                .map_err(|_| residual_failure(identity))?;
        }
        if product > &upper {
            upper = product
                .try_clone()
                .map_err(|_| residual_failure(identity))?;
        }
    }
    DyadicEnclosure::new(lower, upper).map_err(|_| residual_failure(identity))
}

fn enclosure_pow(
    value: &DyadicEnclosure,
    exponent: u32,
    identity: &str,
) -> Result<DyadicEnclosure, CadFailure> {
    if exponent == 0 {
        return DyadicEnclosure::new(dyadic_one()?, dyadic_one()?)
            .map_err(|_| residual_failure(identity));
    }
    let mut result = value.try_clone().map_err(|_| residual_failure(identity))?;
    for _ in 1..exponent {
        result = enclosure_mul(&result, value, identity)?;
    }
    Ok(result)
}

fn contains_zero(enclosure: &DyadicEnclosure) -> bool {
    match dyadic_zero() {
        Ok(zero) => enclosure.contains_dyadic(&zero),
        Err(_) => true,
    }
}

fn max_abs(enclosure: &DyadicEnclosure, identity: &str) -> Result<Dyadic, CadFailure> {
    let lower = dyadic_abs(enclosure.lower(), identity)?;
    let upper = dyadic_abs(enclosure.upper(), identity)?;
    if upper > lower {
        Ok(upper)
    } else {
        Ok(lower)
    }
}

fn min_abs(enclosure: &DyadicEnclosure, identity: &str) -> Result<Dyadic, CadFailure> {
    let lower = dyadic_abs(enclosure.lower(), identity)?;
    let upper = dyadic_abs(enclosure.upper(), identity)?;
    if lower < upper {
        Ok(lower)
    } else {
        Ok(upper)
    }
}

fn dyadic_abs(value: &Dyadic, identity: &str) -> Result<Dyadic, CadFailure> {
    if value.integer().sign() == Sign::Negative {
        dyadic_zero()?
            .sub(value)
            .map_err(|_| residual_failure(identity))
    } else {
        value.try_clone().map_err(|_| residual_failure(identity))
    }
}

fn dyadic_to_algebraic(value: &Dyadic, identity: &str) -> Result<RealAlgebraic, CadFailure> {
    let integer = RealAlgebraic::from_integer(
        value
            .integer()
            .try_clone()
            .map_err(|_| residual_failure(identity))?,
    )
    .map_err(|_| residual_failure(identity))?;
    if value.exponent() == 0 {
        return Ok(integer);
    }
    let two = RealAlgebraic::from_integer(integer_from_u32(2, identity)?)
        .map_err(|_| residual_failure(identity))?;
    let scale = two
        .pow_integer(&integer_from_u32(value.exponent(), identity)?)
        .map_err(|_| residual_failure(identity))?;
    integer.div(&scale).map_err(|_| residual_failure(identity))
}

fn integer_from_u32(value: u32, identity: &str) -> Result<BigInt, CadFailure> {
    BigInt::try_from(value).map_err(|_| residual_failure(identity))
}

fn dyadic_zero() -> Result<Dyadic, CadFailure> {
    Ok(Dyadic::new(
        BigInt::try_from(0_i32).map_err(|_| residual_failure(""))?,
        0,
    ))
}

fn dyadic_one() -> Result<Dyadic, CadFailure> {
    Ok(Dyadic::new(
        BigInt::try_from(1_i32).map_err(|_| residual_failure(""))?,
        0,
    ))
}
