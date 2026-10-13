use alloc::vec::Vec;
use core::cmp::Ordering;

use neco_algnum::RealAlgebraic;
use neco_bigint::Sign;

pub use neco_bezier::{BezierPoint, CadFailure};

#[derive(Debug, Eq, PartialEq)]
pub struct PolynomialNurbsCurve {
    control_points: Vec<BezierPoint>,
    knots: Vec<RealAlgebraic>,
    degree: usize,
    dimension: usize,
}

impl PolynomialNurbsCurve {
    pub fn control_points(&self) -> &[BezierPoint] {
        &self.control_points
    }

    pub fn knots(&self) -> &[RealAlgebraic] {
        &self.knots
    }

    pub const fn degree(&self) -> usize {
        self.degree
    }

    pub const fn dimension(&self) -> usize {
        self.dimension
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct RationalNurbsCurve {
    control_points: Vec<BezierPoint>,
    weights: Vec<RealAlgebraic>,
    knots: Vec<RealAlgebraic>,
    degree: usize,
    dimension: usize,
}

impl RationalNurbsCurve {
    pub fn control_points(&self) -> &[BezierPoint] {
        &self.control_points
    }

    pub fn weights(&self) -> &[RealAlgebraic] {
        &self.weights
    }

    pub fn knots(&self) -> &[RealAlgebraic] {
        &self.knots
    }

    pub const fn degree(&self) -> usize {
        self.degree
    }

    pub const fn dimension(&self) -> usize {
        self.dimension
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct PolynomialNurbsSurface {
    control_net: Vec<Vec<BezierPoint>>,
    u_knots: Vec<RealAlgebraic>,
    v_knots: Vec<RealAlgebraic>,
    u_degree: usize,
    v_degree: usize,
    dimension: usize,
}

impl PolynomialNurbsSurface {
    pub fn control_net(&self) -> &[Vec<BezierPoint>] {
        &self.control_net
    }

    pub fn u_knots(&self) -> &[RealAlgebraic] {
        &self.u_knots
    }

    pub fn v_knots(&self) -> &[RealAlgebraic] {
        &self.v_knots
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
pub struct RationalNurbsSurface {
    control_net: Vec<Vec<BezierPoint>>,
    weights: Vec<Vec<RealAlgebraic>>,
    u_knots: Vec<RealAlgebraic>,
    v_knots: Vec<RealAlgebraic>,
    u_degree: usize,
    v_degree: usize,
    dimension: usize,
}

impl RationalNurbsSurface {
    pub fn control_net(&self) -> &[Vec<BezierPoint>] {
        &self.control_net
    }

    pub fn weights(&self) -> &[Vec<RealAlgebraic>] {
        &self.weights
    }

    pub fn u_knots(&self) -> &[RealAlgebraic] {
        &self.u_knots
    }

    pub fn v_knots(&self) -> &[RealAlgebraic] {
        &self.v_knots
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
pub enum NurbsDefinitionRequest {
    PolynomialCurve {
        control_points: Vec<BezierPoint>,
        knots: Vec<RealAlgebraic>,
        degree: usize,
    },
    RationalCurve {
        control_points: Vec<BezierPoint>,
        weights: Vec<RealAlgebraic>,
        knots: Vec<RealAlgebraic>,
        degree: usize,
    },
    PolynomialSurface {
        control_net: Vec<Vec<BezierPoint>>,
        u_knots: Vec<RealAlgebraic>,
        v_knots: Vec<RealAlgebraic>,
        u_degree: usize,
        v_degree: usize,
    },
    RationalSurface {
        control_net: Vec<Vec<BezierPoint>>,
        weights: Vec<Vec<RealAlgebraic>>,
        u_knots: Vec<RealAlgebraic>,
        v_knots: Vec<RealAlgebraic>,
        u_degree: usize,
        v_degree: usize,
    },
}

#[derive(Debug, Eq, PartialEq)]
pub enum NurbsTypes {
    PolynomialCurve(PolynomialNurbsCurve),
    RationalCurve(RationalNurbsCurve),
    PolynomialSurface(PolynomialNurbsSurface),
    RationalSurface(RationalNurbsSurface),
}

pub type NurbsRepresentation = NurbsTypes;

pub fn define_nurbs(request: NurbsDefinitionRequest) -> Result<NurbsTypes, CadFailure> {
    match request {
        NurbsDefinitionRequest::PolynomialCurve {
            control_points,
            knots,
            degree,
        } => define_polynomial_curve(control_points, knots, degree),
        NurbsDefinitionRequest::RationalCurve {
            control_points,
            weights,
            knots,
            degree,
        } => define_rational_curve(control_points, weights, knots, degree),
        NurbsDefinitionRequest::PolynomialSurface {
            control_net,
            u_knots,
            v_knots,
            u_degree,
            v_degree,
        } => define_polynomial_surface(control_net, u_knots, v_knots, u_degree, v_degree),
        NurbsDefinitionRequest::RationalSurface {
            control_net,
            weights,
            u_knots,
            v_knots,
            u_degree,
            v_degree,
        } => define_rational_surface(control_net, weights, u_knots, v_knots, u_degree, v_degree),
    }
}

fn define_polynomial_curve(
    control_points: Vec<BezierPoint>,
    knots: Vec<RealAlgebraic>,
    degree: usize,
) -> Result<NurbsTypes, CadFailure> {
    let dimension = validate_curve(&control_points, &knots, degree)?;
    Ok(NurbsTypes::PolynomialCurve(PolynomialNurbsCurve {
        control_points,
        knots,
        degree,
        dimension,
    }))
}

fn define_rational_curve(
    control_points: Vec<BezierPoint>,
    weights: Vec<RealAlgebraic>,
    knots: Vec<RealAlgebraic>,
    degree: usize,
) -> Result<NurbsTypes, CadFailure> {
    let dimension = validate_curve(&control_points, &knots, degree)?;
    validate_curve_weights(&weights, control_points.len())?;
    Ok(NurbsTypes::RationalCurve(RationalNurbsCurve {
        control_points,
        weights,
        knots,
        degree,
        dimension,
    }))
}

fn define_polynomial_surface(
    control_net: Vec<Vec<BezierPoint>>,
    u_knots: Vec<RealAlgebraic>,
    v_knots: Vec<RealAlgebraic>,
    u_degree: usize,
    v_degree: usize,
) -> Result<NurbsTypes, CadFailure> {
    let dimension = validate_surface(&control_net, &u_knots, &v_knots, u_degree, v_degree)?;
    Ok(NurbsTypes::PolynomialSurface(PolynomialNurbsSurface {
        control_net,
        u_knots,
        v_knots,
        u_degree,
        v_degree,
        dimension,
    }))
}

fn define_rational_surface(
    control_net: Vec<Vec<BezierPoint>>,
    weights: Vec<Vec<RealAlgebraic>>,
    u_knots: Vec<RealAlgebraic>,
    v_knots: Vec<RealAlgebraic>,
    u_degree: usize,
    v_degree: usize,
) -> Result<NurbsTypes, CadFailure> {
    let dimension = validate_surface(&control_net, &u_knots, &v_knots, u_degree, v_degree)?;
    validate_surface_weights(&weights, control_net.len(), control_net[0].len())?;
    Ok(NurbsTypes::RationalSurface(RationalNurbsSurface {
        control_net,
        weights,
        u_knots,
        v_knots,
        u_degree,
        v_degree,
        dimension,
    }))
}

fn validate_curve(
    control_points: &[BezierPoint],
    knots: &[RealAlgebraic],
    degree: usize,
) -> Result<usize, CadFailure> {
    let Some(first) = control_points.first() else {
        return Err(CadFailure::EmptyControlPoints);
    };
    validate_curve_direction(control_points.len(), knots, degree)?;
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
    u_knots: &[RealAlgebraic],
    v_knots: &[RealAlgebraic],
    u_degree: usize,
    v_degree: usize,
) -> Result<usize, CadFailure> {
    let Some(first_row) = control_net.first() else {
        return Err(CadFailure::EmptySurfaceRows);
    };
    let Some(first_point) = first_row.first() else {
        return Err(CadFailure::EmptyControlPoints);
    };
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
    validate_surface_direction(control_net.len(), u_knots, u_degree)?;
    validate_surface_direction(column_count, v_knots, v_degree)?;
    Ok(dimension)
}

fn validate_curve_direction(
    control_count: usize,
    knots: &[RealAlgebraic],
    degree: usize,
) -> Result<(), CadFailure> {
    if degree >= control_count {
        return Err(CadFailure::DegreeMismatch);
    }
    validate_knots(control_count, knots, degree)
}

fn validate_surface_direction(
    control_count: usize,
    knots: &[RealAlgebraic],
    degree: usize,
) -> Result<(), CadFailure> {
    if degree >= control_count {
        return Err(CadFailure::SurfaceDegreeMismatch);
    }
    validate_knots(control_count, knots, degree)
}

fn validate_knots(
    control_count: usize,
    knots: &[RealAlgebraic],
    degree: usize,
) -> Result<(), CadFailure> {
    if knots.len() != control_count + degree + 1 {
        return Err(CadFailure::KnotCardinalityMismatch);
    }
    for pair in knots.windows(2) {
        if pair[0]
            .compare(&pair[1])
            .map_err(CadFailure::AlgebraicFailure)?
            == Ordering::Greater
        {
            return Err(CadFailure::NonMonotonicKnots);
        }
    }
    Ok(())
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
