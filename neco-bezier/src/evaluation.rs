use alloc::vec::Vec;
use core::cmp::Ordering;

use neco_algnum::{AlgnumError, RealAlgebraic};
use neco_bigint::BigInt;
use neco_geometry::{Point2, Point3};

use crate::{BezierPoint, BezierTypes, CadFailure};

pub enum BezierEvaluationRequest {
    Curve {
        representation: BezierTypes,
        parameter: RealAlgebraic,
    },
    Surface {
        representation: BezierTypes,
        u: RealAlgebraic,
        v: RealAlgebraic,
    },
}

pub struct EvaluationResult {
    point: BezierPoint,
}

impl EvaluationResult {
    pub fn from_point(point: BezierPoint) -> Self {
        Self { point }
    }

    pub fn point(&self) -> &BezierPoint {
        &self.point
    }

    pub fn into_point(self) -> BezierPoint {
        self.point
    }
}

pub fn evaluate_bezier(request: BezierEvaluationRequest) -> Result<EvaluationResult, CadFailure> {
    let point = match request {
        BezierEvaluationRequest::Curve {
            representation,
            parameter,
        } => {
            validate_parameter(&parameter)?;
            match representation {
                BezierTypes::PolynomialCurve(curve) => {
                    evaluate_polynomial_curve(curve.control_points(), &parameter)?
                }
                BezierTypes::RationalCurve(curve) => {
                    evaluate_rational_curve(curve.control_points(), curve.weights(), &parameter)?
                }
                BezierTypes::PolynomialSurface(_) | BezierTypes::RationalSurface(_) => {
                    return Err(CadFailure::RequestShapeMismatch);
                }
            }
        }
        BezierEvaluationRequest::Surface {
            representation,
            u,
            v,
        } => {
            validate_parameter(&u)?;
            validate_parameter(&v)?;
            match representation {
                BezierTypes::PolynomialSurface(surface) => {
                    evaluate_polynomial_surface(surface.control_net(), &u, &v)?
                }
                BezierTypes::RationalSurface(surface) => {
                    evaluate_rational_surface(surface.control_net(), surface.weights(), &u, &v)?
                }
                BezierTypes::PolynomialCurve(_) | BezierTypes::RationalCurve(_) => {
                    return Err(CadFailure::RequestShapeMismatch);
                }
            }
        }
    };
    Ok(EvaluationResult::from_point(point))
}

pub(crate) fn validate_parameter(parameter: &RealAlgebraic) -> Result<(), CadFailure> {
    let zero = algebraic_integer(0)?;
    let one = algebraic_integer(1)?;
    if parameter.compare(&zero)? == Ordering::Less || parameter.compare(&one)? == Ordering::Greater
    {
        return Err(CadFailure::ParameterOutOfRange);
    }
    Ok(())
}

fn algebraic_integer(value: i32) -> Result<RealAlgebraic, CadFailure> {
    let integer = BigInt::try_from(value).map_err(AlgnumError::from)?;
    Ok(RealAlgebraic::from_integer(integer)?)
}

fn evaluate_polynomial_curve(
    control_points: &[BezierPoint],
    parameter: &RealAlgebraic,
) -> Result<BezierPoint, CadFailure> {
    let points = clone_points(control_points)?;
    interpolate_points(points, parameter)
}

fn evaluate_rational_curve(
    control_points: &[BezierPoint],
    weights: &[RealAlgebraic],
    parameter: &RealAlgebraic,
) -> Result<BezierPoint, CadFailure> {
    let points = homogeneous_points(control_points, weights)?;
    resolve_homogeneous(interpolate_homogeneous(points, parameter)?)
}

fn evaluate_polynomial_surface(
    control_net: &[Vec<BezierPoint>],
    u: &RealAlgebraic,
    v: &RealAlgebraic,
) -> Result<BezierPoint, CadFailure> {
    let mut rows = Vec::with_capacity(control_net.len());
    for row in control_net {
        rows.push(evaluate_polynomial_curve(row, v)?);
    }
    interpolate_points(rows, u)
}

fn evaluate_rational_surface(
    control_net: &[Vec<BezierPoint>],
    weights: &[Vec<RealAlgebraic>],
    u: &RealAlgebraic,
    v: &RealAlgebraic,
) -> Result<BezierPoint, CadFailure> {
    let mut rows = Vec::with_capacity(control_net.len());
    for (points, row_weights) in control_net.iter().zip(weights) {
        rows.push(interpolate_homogeneous(
            homogeneous_points(points, row_weights)?,
            v,
        )?);
    }
    resolve_homogeneous(interpolate_homogeneous(rows, u)?)
}

fn clone_points(control_points: &[BezierPoint]) -> Result<Vec<BezierPoint>, CadFailure> {
    let mut points = Vec::with_capacity(control_points.len());
    for point in control_points {
        points.push(clone_point(point)?);
    }
    Ok(points)
}

fn clone_point(point: &BezierPoint) -> Result<BezierPoint, CadFailure> {
    match point {
        BezierPoint::Point2(point) => Ok(BezierPoint::Point2(Point2 {
            x: point.x.try_clone()?,
            y: point.y.try_clone()?,
        })),
        BezierPoint::Point3(point) => Ok(BezierPoint::Point3(Point3 {
            x: point.x.try_clone()?,
            y: point.y.try_clone()?,
            z: point.z.try_clone()?,
        })),
    }
}

fn interpolate_points(
    mut points: Vec<BezierPoint>,
    parameter: &RealAlgebraic,
) -> Result<BezierPoint, CadFailure> {
    while points.len() > 1 {
        let mut next = Vec::with_capacity(points.len() - 1);
        for pair in points.windows(2) {
            next.push(interpolate_point(&pair[0], &pair[1], parameter)?);
        }
        points = next;
    }
    Ok(points.remove(0))
}

fn interpolate_point(
    left: &BezierPoint,
    right: &BezierPoint,
    parameter: &RealAlgebraic,
) -> Result<BezierPoint, CadFailure> {
    match (left, right) {
        (BezierPoint::Point2(left), BezierPoint::Point2(right)) => {
            Ok(BezierPoint::Point2(Point2 {
                x: interpolate_value(&left.x, &right.x, parameter)?,
                y: interpolate_value(&left.y, &right.y, parameter)?,
            }))
        }
        (BezierPoint::Point3(left), BezierPoint::Point3(right)) => {
            Ok(BezierPoint::Point3(Point3 {
                x: interpolate_value(&left.x, &right.x, parameter)?,
                y: interpolate_value(&left.y, &right.y, parameter)?,
                z: interpolate_value(&left.z, &right.z, parameter)?,
            }))
        }
        _ => Err(CadFailure::ControlPointDimensionMismatch),
    }
}

fn interpolate_value(
    left: &RealAlgebraic,
    right: &RealAlgebraic,
    parameter: &RealAlgebraic,
) -> Result<RealAlgebraic, CadFailure> {
    Ok(left.add(&parameter.mul(&right.sub(left)?)?)?)
}

enum HomogeneousPoint {
    Point2 {
        x: RealAlgebraic,
        y: RealAlgebraic,
        weight: RealAlgebraic,
    },
    Point3 {
        x: RealAlgebraic,
        y: RealAlgebraic,
        z: RealAlgebraic,
        weight: RealAlgebraic,
    },
}

fn homogeneous_points(
    control_points: &[BezierPoint],
    weights: &[RealAlgebraic],
) -> Result<Vec<HomogeneousPoint>, CadFailure> {
    let mut points = Vec::with_capacity(control_points.len());
    for (point, weight) in control_points.iter().zip(weights) {
        points.push(to_homogeneous(point, weight)?);
    }
    Ok(points)
}

fn to_homogeneous(
    point: &BezierPoint,
    weight: &RealAlgebraic,
) -> Result<HomogeneousPoint, CadFailure> {
    match point {
        BezierPoint::Point2(point) => Ok(HomogeneousPoint::Point2 {
            x: point.x.mul(weight)?,
            y: point.y.mul(weight)?,
            weight: weight.try_clone()?,
        }),
        BezierPoint::Point3(point) => Ok(HomogeneousPoint::Point3 {
            x: point.x.mul(weight)?,
            y: point.y.mul(weight)?,
            z: point.z.mul(weight)?,
            weight: weight.try_clone()?,
        }),
    }
}

fn interpolate_homogeneous(
    mut points: Vec<HomogeneousPoint>,
    parameter: &RealAlgebraic,
) -> Result<HomogeneousPoint, CadFailure> {
    while points.len() > 1 {
        let mut next = Vec::with_capacity(points.len() - 1);
        for pair in points.windows(2) {
            next.push(interpolate_homogeneous_pair(&pair[0], &pair[1], parameter)?);
        }
        points = next;
    }
    Ok(points.remove(0))
}

fn interpolate_homogeneous_pair(
    left: &HomogeneousPoint,
    right: &HomogeneousPoint,
    parameter: &RealAlgebraic,
) -> Result<HomogeneousPoint, CadFailure> {
    match (left, right) {
        (
            HomogeneousPoint::Point2 {
                x: left_x,
                y: left_y,
                weight: left_weight,
            },
            HomogeneousPoint::Point2 {
                x: right_x,
                y: right_y,
                weight: right_weight,
            },
        ) => Ok(HomogeneousPoint::Point2 {
            x: interpolate_value(left_x, right_x, parameter)?,
            y: interpolate_value(left_y, right_y, parameter)?,
            weight: interpolate_value(left_weight, right_weight, parameter)?,
        }),
        (
            HomogeneousPoint::Point3 {
                x: left_x,
                y: left_y,
                z: left_z,
                weight: left_weight,
            },
            HomogeneousPoint::Point3 {
                x: right_x,
                y: right_y,
                z: right_z,
                weight: right_weight,
            },
        ) => Ok(HomogeneousPoint::Point3 {
            x: interpolate_value(left_x, right_x, parameter)?,
            y: interpolate_value(left_y, right_y, parameter)?,
            z: interpolate_value(left_z, right_z, parameter)?,
            weight: interpolate_value(left_weight, right_weight, parameter)?,
        }),
        _ => Err(CadFailure::ControlPointDimensionMismatch),
    }
}

fn resolve_homogeneous(point: HomogeneousPoint) -> Result<BezierPoint, CadFailure> {
    match point {
        HomogeneousPoint::Point2 { x, y, weight } => Ok(BezierPoint::Point2(Point2 {
            x: x.div(&weight)?,
            y: y.div(&weight)?,
        })),
        HomogeneousPoint::Point3 { x, y, z, weight } => Ok(BezierPoint::Point3(Point3 {
            x: x.div(&weight)?,
            y: y.div(&weight)?,
            z: z.div(&weight)?,
        })),
    }
}
