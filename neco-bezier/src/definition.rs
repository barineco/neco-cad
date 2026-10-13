extern crate alloc;

use alloc::vec::Vec;

use neco_algnum::{AlgnumError, RealAlgebraic};
use neco_bigint::Sign;
use neco_geometry::{Point2, Point3};

#[derive(Debug, Eq, PartialEq)]
pub enum CadFailure {
    EmptyControlPoints,
    ControlPointDimensionMismatch,
    DegreeMismatch,
    WeightCardinalityMismatch,
    NonPositiveWeight,
    AlgebraicFailure(AlgnumError),
    RequestShapeMismatch,
    ParameterOutOfRange,
    EmptyParameterDomain,
    EmptySurfaceRows,
    SurfaceRowCardinalityMismatch,
    SurfaceDegreeMismatch,
    KnotCardinalityMismatch,
    NonMonotonicKnots,
}

impl From<AlgnumError> for CadFailure {
    fn from(error: AlgnumError) -> Self {
        Self::AlgebraicFailure(error)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum BezierPoint {
    Point2(Point2),
    Point3(Point3),
}

impl BezierPoint {
    pub const fn dimension(&self) -> usize {
        match self {
            Self::Point2(_) => 2,
            Self::Point3(_) => 3,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct PolynomialBezierCurve {
    control_points: Vec<BezierPoint>,
    degree: usize,
    dimension: usize,
}

impl PolynomialBezierCurve {
    pub fn control_points(&self) -> &[BezierPoint] {
        &self.control_points
    }

    pub const fn degree(&self) -> usize {
        self.degree
    }

    pub const fn dimension(&self) -> usize {
        self.dimension
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct RationalBezierCurve {
    control_points: Vec<BezierPoint>,
    weights: Vec<RealAlgebraic>,
    degree: usize,
    dimension: usize,
}

impl RationalBezierCurve {
    pub fn control_points(&self) -> &[BezierPoint] {
        &self.control_points
    }

    pub fn weights(&self) -> &[RealAlgebraic] {
        &self.weights
    }

    pub const fn degree(&self) -> usize {
        self.degree
    }

    pub const fn dimension(&self) -> usize {
        self.dimension
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct PolynomialBezierSurface {
    control_net: Vec<Vec<BezierPoint>>,
    u_degree: usize,
    v_degree: usize,
    dimension: usize,
}

impl PolynomialBezierSurface {
    pub fn control_net(&self) -> &[Vec<BezierPoint>] {
        &self.control_net
    }

    pub const fn u_degree(&self) -> usize {
        self.u_degree
    }

    pub const fn v_degree(&self) -> usize {
        self.v_degree
    }

    pub const fn dimension(&self) -> usize {
        self.dimension
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct RationalBezierSurface {
    control_net: Vec<Vec<BezierPoint>>,
    weights: Vec<Vec<RealAlgebraic>>,
    u_degree: usize,
    v_degree: usize,
    dimension: usize,
}

impl RationalBezierSurface {
    pub fn control_net(&self) -> &[Vec<BezierPoint>] {
        &self.control_net
    }

    pub fn weights(&self) -> &[Vec<RealAlgebraic>] {
        &self.weights
    }

    pub const fn u_degree(&self) -> usize {
        self.u_degree
    }

    pub const fn v_degree(&self) -> usize {
        self.v_degree
    }

    pub const fn dimension(&self) -> usize {
        self.dimension
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum BezierDefinitionRequest {
    PolynomialCurve {
        control_points: Vec<BezierPoint>,
        degree: usize,
    },
    RationalCurve {
        control_points: Vec<BezierPoint>,
        weights: Vec<RealAlgebraic>,
        degree: usize,
    },
    PolynomialSurface {
        control_net: Vec<Vec<BezierPoint>>,
        u_degree: usize,
        v_degree: usize,
    },
    RationalSurface {
        control_net: Vec<Vec<BezierPoint>>,
        weights: Vec<Vec<RealAlgebraic>>,
        u_degree: usize,
        v_degree: usize,
    },
}

#[derive(Debug, Eq, PartialEq)]
pub enum BezierTypes {
    PolynomialCurve(PolynomialBezierCurve),
    RationalCurve(RationalBezierCurve),
    PolynomialSurface(PolynomialBezierSurface),
    RationalSurface(RationalBezierSurface),
}

pub type BezierRepresentation = BezierTypes;

pub fn define_bezier(request: BezierDefinitionRequest) -> Result<BezierTypes, CadFailure> {
    match request {
        BezierDefinitionRequest::PolynomialCurve {
            control_points,
            degree,
        } => define_polynomial_curve(control_points, degree),
        BezierDefinitionRequest::RationalCurve {
            control_points,
            weights,
            degree,
        } => define_rational_curve(control_points, weights, degree),
        BezierDefinitionRequest::PolynomialSurface {
            control_net,
            u_degree,
            v_degree,
        } => define_polynomial_surface(control_net, u_degree, v_degree),
        BezierDefinitionRequest::RationalSurface {
            control_net,
            weights,
            u_degree,
            v_degree,
        } => define_rational_surface(control_net, weights, u_degree, v_degree),
    }
}

fn define_polynomial_curve(
    control_points: Vec<BezierPoint>,
    degree: usize,
) -> Result<BezierTypes, CadFailure> {
    let dimension = validate_curve(&control_points, degree)?;
    Ok(BezierTypes::PolynomialCurve(PolynomialBezierCurve {
        control_points,
        degree,
        dimension,
    }))
}

fn define_rational_curve(
    control_points: Vec<BezierPoint>,
    weights: Vec<RealAlgebraic>,
    degree: usize,
) -> Result<BezierTypes, CadFailure> {
    let dimension = validate_curve(&control_points, degree)?;
    validate_curve_weights(&weights, control_points.len())?;
    Ok(BezierTypes::RationalCurve(RationalBezierCurve {
        control_points,
        weights,
        degree,
        dimension,
    }))
}

fn define_polynomial_surface(
    control_net: Vec<Vec<BezierPoint>>,
    u_degree: usize,
    v_degree: usize,
) -> Result<BezierTypes, CadFailure> {
    let dimension = validate_surface(&control_net, u_degree, v_degree)?;
    Ok(BezierTypes::PolynomialSurface(PolynomialBezierSurface {
        control_net,
        u_degree,
        v_degree,
        dimension,
    }))
}

fn define_rational_surface(
    control_net: Vec<Vec<BezierPoint>>,
    weights: Vec<Vec<RealAlgebraic>>,
    u_degree: usize,
    v_degree: usize,
) -> Result<BezierTypes, CadFailure> {
    let dimension = validate_surface(&control_net, u_degree, v_degree)?;
    validate_surface_weights(&weights, control_net.len(), control_net[0].len())?;
    Ok(BezierTypes::RationalSurface(RationalBezierSurface {
        control_net,
        weights,
        u_degree,
        v_degree,
        dimension,
    }))
}

fn validate_curve(control_points: &[BezierPoint], degree: usize) -> Result<usize, CadFailure> {
    let Some(first) = control_points.first() else {
        return Err(CadFailure::EmptyControlPoints);
    };
    if degree != control_points.len() - 1 {
        return Err(CadFailure::DegreeMismatch);
    }
    let dimension = first.dimension();
    if control_points
        .iter()
        .any(|point| point.dimension() != dimension)
    {
        return Err(CadFailure::ControlPointDimensionMismatch);
    }
    Ok(dimension)
}

fn validate_surface(
    control_net: &[Vec<BezierPoint>],
    u_degree: usize,
    v_degree: usize,
) -> Result<usize, CadFailure> {
    let Some(first_row) = control_net.first() else {
        return Err(CadFailure::EmptySurfaceRows);
    };
    let Some(first_point) = first_row.first() else {
        return Err(CadFailure::EmptyControlPoints);
    };
    if u_degree != control_net.len() - 1 || v_degree != first_row.len() - 1 {
        return Err(CadFailure::SurfaceDegreeMismatch);
    }
    let column_count = first_row.len();
    if control_net.iter().any(|row| row.len() != column_count) {
        return Err(CadFailure::SurfaceRowCardinalityMismatch);
    }
    let dimension = first_point.dimension();
    if control_net
        .iter()
        .flatten()
        .any(|point| point.dimension() != dimension)
    {
        return Err(CadFailure::ControlPointDimensionMismatch);
    }
    Ok(dimension)
}

fn validate_curve_weights(
    weights: &[RealAlgebraic],
    control_point_count: usize,
) -> Result<(), CadFailure> {
    if weights.len() != control_point_count {
        return Err(CadFailure::WeightCardinalityMismatch);
    }
    validate_weights(weights.iter())
}

fn validate_surface_weights(
    weights: &[Vec<RealAlgebraic>],
    row_count: usize,
    column_count: usize,
) -> Result<(), CadFailure> {
    if weights.len() != row_count || weights.iter().any(|row| row.len() != column_count) {
        return Err(CadFailure::WeightCardinalityMismatch);
    }
    validate_weights(weights.iter().flatten())
}

fn validate_weights<'a>(
    weights: impl Iterator<Item = &'a RealAlgebraic>,
) -> Result<(), CadFailure> {
    for weight in weights {
        match weight.sign().map_err(CadFailure::AlgebraicFailure)? {
            Sign::Positive => {}
            Sign::Zero | Sign::Negative => return Err(CadFailure::NonPositiveWeight),
        }
    }
    Ok(())
}
