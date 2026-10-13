use alloc::vec::Vec;
use core::cmp::Ordering;

use neco_algnum::{AlgnumError, RealAlgebraic};
use neco_geometry::{Point2, Point3};
use neco_linear_exact::{ExactLinearError, ExactLinearSolution, ExactMatrix, ExactScalar};
use neco_linear_types::{LinearError, Shape, Vector};

use crate::{
    define_nurbs, BezierPoint, CadFailure, NurbsDefinitionRequest, NurbsTypes, PolynomialNurbsCurve,
};

pub struct FittingRequest {
    pub parameters: Vec<RealAlgebraic>,
    pub samples: Vec<BezierPoint>,
    pub knots: Vec<RealAlgebraic>,
    pub degree: usize,
    pub control_point_count: usize,
}

pub struct FittingResult {
    curve: PolynomialNurbsCurve,
}

impl FittingResult {
    pub fn curve(&self) -> &PolynomialNurbsCurve {
        &self.curve
    }
    pub fn into_curve(self) -> PolynomialNurbsCurve {
        self.curve
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum NurbsFittingFailure {
    EmptySamples,
    ParameterSampleCardinalityMismatch,
    ControlPointDimensionMismatch,
    DegreeMismatch,
    KnotCardinalityMismatch,
    NonMonotonicKnots,
    EmptyParameterDomain,
    ParameterOutOfRange,
    DegenerateBasis,
    RankDeficient,
    InconsistentSystem,
    Cad(CadFailure),
    ExactLinear(ExactLinearError),
}

impl From<CadFailure> for NurbsFittingFailure {
    fn from(error: CadFailure) -> Self {
        Self::Cad(error)
    }
}
impl From<ExactLinearError> for NurbsFittingFailure {
    fn from(error: ExactLinearError) -> Self {
        Self::ExactLinear(error)
    }
}
impl From<AlgnumError> for NurbsFittingFailure {
    fn from(error: AlgnumError) -> Self {
        Self::Cad(CadFailure::AlgebraicFailure(error))
    }
}
impl From<LinearError> for NurbsFittingFailure {
    fn from(error: LinearError) -> Self {
        Self::ExactLinear(ExactLinearError::Linear(error))
    }
}

pub fn fit_nurbs_curve(request: FittingRequest) -> Result<FittingResult, NurbsFittingFailure> {
    validate_request(&request)?;
    let basis = build_basis_matrix(&request)?;
    let dimension = request.samples[0].dimension();
    let coordinates = solve_coordinates(&basis, &request.samples, dimension)?;
    define_fitted_curve(coordinates, request.knots, request.degree)
}

fn validate_request(request: &FittingRequest) -> Result<(), NurbsFittingFailure> {
    if request.samples.is_empty() {
        return Err(NurbsFittingFailure::EmptySamples);
    }
    if request.parameters.len() != request.samples.len() {
        return Err(NurbsFittingFailure::ParameterSampleCardinalityMismatch);
    }
    let dimension = request.samples[0].dimension();
    if request
        .samples
        .iter()
        .any(|sample| sample.dimension() != dimension)
    {
        return Err(NurbsFittingFailure::ControlPointDimensionMismatch);
    }
    if request.degree >= request.control_point_count || request.control_point_count == 0 {
        return Err(NurbsFittingFailure::DegreeMismatch);
    }
    if request.knots.len() != request.control_point_count + request.degree + 1 {
        return Err(NurbsFittingFailure::KnotCardinalityMismatch);
    }
    for pair in request.knots.windows(2) {
        if pair[0].compare(&pair[1])? == Ordering::Greater {
            return Err(NurbsFittingFailure::NonMonotonicKnots);
        }
    }
    let mut first = None;
    let mut last = None;
    for index in request.degree..request.control_point_count {
        if request.knots[index].compare(&request.knots[index + 1])? == Ordering::Less {
            first.get_or_insert(index);
            last = Some(index);
        }
    }
    let (first, last) = match (first, last) {
        (Some(first), Some(last)) => (first, last),
        _ => return Err(NurbsFittingFailure::EmptyParameterDomain),
    };
    for parameter in &request.parameters {
        if parameter.compare(&request.knots[first])? == Ordering::Less
            || parameter.compare(&request.knots[last + 1])? == Ordering::Greater
        {
            return Err(NurbsFittingFailure::ParameterOutOfRange);
        }
    }
    Ok(())
}

fn build_basis_matrix(
    request: &FittingRequest,
) -> Result<ExactMatrix<RealAlgebraic>, NurbsFittingFailure> {
    let rows = request.parameters.len();
    let columns = request.control_point_count;
    let size = rows
        .checked_mul(columns)
        .ok_or_else(|| allocation_failure(usize::MAX))?;
    let mut values = Vec::new();
    values
        .try_reserve_exact(size)
        .map_err(|_| allocation_failure(size))?;
    for parameter in &request.parameters {
        let row = build_basis_row(parameter, &request.knots, request.degree, columns)?;
        values.extend(row);
    }
    ExactMatrix::from_row_major(Shape::new(rows, columns), values).map_err(Into::into)
}

fn build_basis_row(
    parameter: &RealAlgebraic,
    knots: &[RealAlgebraic],
    degree: usize,
    control_count: usize,
) -> Result<Vec<RealAlgebraic>, NurbsFittingFailure> {
    let span = select_span(control_count, knots, degree, parameter)?;
    let mut row = Vec::new();
    row.try_reserve_exact(control_count)
        .map_err(|_| allocation_failure(control_count))?;
    let zero = RealAlgebraic::zero()?;
    for _ in 0..control_count {
        row.push(zero.try_clone()?);
    }
    let mut values = Vec::new();
    values
        .try_reserve_exact(degree + 1)
        .map_err(|_| allocation_failure(degree + 1))?;
    for _ in 0..=degree {
        values.push(zero.try_clone()?);
    }
    values[0] = RealAlgebraic::one()?;
    let mut left = Vec::new();
    left.try_reserve_exact(degree + 1)
        .map_err(|_| allocation_failure(degree + 1))?;
    let mut right = Vec::new();
    right
        .try_reserve_exact(degree + 1)
        .map_err(|_| allocation_failure(degree + 1))?;
    for j in 1..=degree {
        left.push(parameter.sub(&knots[span + 1 - j])?);
        right.push(knots[span + j].sub(parameter)?);
        let mut saved = zero.try_clone()?;
        for r in 0..j {
            let left_index = j - r - 1;
            let denominator = right[r].add(&left[left_index])?;
            if denominator.is_zero() {
                return Err(NurbsFittingFailure::DegenerateBasis);
            }
            let ratio = values[r].div(&denominator)?;
            let next = ratio.mul(&right[r])?;
            values[r] = saved.add(&next)?;
            saved = ratio.mul(&left[left_index])?;
        }
        values[j] = saved;
    }
    for (local, value) in values.into_iter().enumerate() {
        row[span - degree + local] = value;
    }
    Ok(row)
}

fn select_span(
    control_count: usize,
    knots: &[RealAlgebraic],
    degree: usize,
    parameter: &RealAlgebraic,
) -> Result<usize, NurbsFittingFailure> {
    let mut first = None;
    let mut last = None;
    for index in degree..control_count {
        if knots[index].compare(&knots[index + 1])? == Ordering::Less {
            first.get_or_insert(index);
            last = Some(index);
        }
    }
    let (first, last) = match (first, last) {
        (Some(first), Some(last)) => (first, last),
        _ => return Err(NurbsFittingFailure::EmptyParameterDomain),
    };
    if parameter.compare(&knots[first])? == Ordering::Less
        || parameter.compare(&knots[last + 1])? == Ordering::Greater
    {
        return Err(NurbsFittingFailure::ParameterOutOfRange);
    }
    if parameter.compare(&knots[last + 1])? == Ordering::Equal {
        return Ok(last);
    }
    for index in first..=last {
        if knots[index].compare(&knots[index + 1])? == Ordering::Less
            && parameter.compare(&knots[index])? != Ordering::Less
            && parameter.compare(&knots[index + 1])? == Ordering::Less
        {
            return Ok(index);
        }
    }
    Err(NurbsFittingFailure::ParameterOutOfRange)
}

fn solve_coordinates(
    matrix: &ExactMatrix<RealAlgebraic>,
    samples: &[BezierPoint],
    dimension: usize,
) -> Result<Vec<Vec<RealAlgebraic>>, NurbsFittingFailure> {
    let mut result = Vec::new();
    result
        .try_reserve_exact(matrix.columns())
        .map_err(|_| allocation_failure(matrix.columns()))?;
    for coordinate in 0..dimension {
        let mut rhs_values = Vec::new();
        rhs_values
            .try_reserve_exact(samples.len())
            .map_err(|_| allocation_failure(samples.len()))?;
        for sample in samples {
            rhs_values.push(coordinate_value(sample, coordinate)?.try_clone()?);
        }
        let rhs = Vector::try_from_vec(rhs_values)?;
        let solution = match matrix.solve(&rhs)? {
            ExactLinearSolution::Unique(solution) => solution.into_values(),
            ExactLinearSolution::Affine { .. } => return Err(NurbsFittingFailure::RankDeficient),
            ExactLinearSolution::Inconsistent => {
                return Err(NurbsFittingFailure::InconsistentSystem)
            }
        };
        result.push(solution);
    }
    Ok(result)
}

fn coordinate_value(
    point: &BezierPoint,
    coordinate: usize,
) -> Result<&RealAlgebraic, NurbsFittingFailure> {
    match (point, coordinate) {
        (BezierPoint::Point2(point), 0) => Ok(&point.x),
        (BezierPoint::Point2(point), 1) => Ok(&point.y),
        (BezierPoint::Point3(point), 0) => Ok(&point.x),
        (BezierPoint::Point3(point), 1) => Ok(&point.y),
        (BezierPoint::Point3(point), 2) => Ok(&point.z),
        _ => Err(NurbsFittingFailure::ControlPointDimensionMismatch),
    }
}

fn define_fitted_curve(
    coordinates: Vec<Vec<RealAlgebraic>>,
    knots: Vec<RealAlgebraic>,
    degree: usize,
) -> Result<FittingResult, NurbsFittingFailure> {
    let dimension = coordinates.len();
    let first_column = coordinates.first().map_or(&[][..], Vec::as_slice);
    let count = first_column.len();
    let mut points = Vec::new();
    points
        .try_reserve_exact(count)
        .map_err(|_| allocation_failure(count))?;
    for (index, x) in first_column.iter().enumerate() {
        points.push(match dimension {
            2 => BezierPoint::Point2(Point2 {
                x: x.try_clone()?,
                y: coordinates[1][index].try_clone()?,
            }),
            3 => BezierPoint::Point3(Point3 {
                x: x.try_clone()?,
                y: coordinates[1][index].try_clone()?,
                z: coordinates[2][index].try_clone()?,
            }),
            _ => return Err(NurbsFittingFailure::ControlPointDimensionMismatch),
        });
    }
    let curve = match define_nurbs(NurbsDefinitionRequest::PolynomialCurve {
        control_points: points,
        knots,
        degree,
    })? {
        NurbsTypes::PolynomialCurve(curve) => curve,
        _ => return Err(NurbsFittingFailure::Cad(CadFailure::RequestShapeMismatch)),
    };
    Ok(FittingResult { curve })
}

fn allocation_failure(requested: usize) -> NurbsFittingFailure {
    NurbsFittingFailure::ExactLinear(ExactLinearError::Linear(LinearError::AllocationFailure {
        requested,
    }))
}
