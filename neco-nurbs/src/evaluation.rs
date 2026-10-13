use alloc::vec::Vec;
use core::cmp::Ordering;

use neco_algnum::RealAlgebraic;
use neco_bezier::{BezierPoint, CadFailure, EvaluationResult};
use neco_geometry::{Point2, Point3};

use crate::{
    NurbsTypes, PolynomialNurbsCurve, PolynomialNurbsSurface, RationalNurbsCurve,
    RationalNurbsSurface,
};

pub enum NurbsEvaluationRequest {
    Curve {
        representation: NurbsTypes,
        parameter: RealAlgebraic,
    },
    Surface {
        representation: NurbsTypes,
        u: RealAlgebraic,
        v: RealAlgebraic,
    },
}

pub fn evaluate_nurbs(request: NurbsEvaluationRequest) -> Result<EvaluationResult, CadFailure> {
    let point = match request {
        NurbsEvaluationRequest::Curve {
            representation,
            parameter,
        } => evaluate_curve_nurbs(&representation, &parameter)?.into_point(),
        NurbsEvaluationRequest::Surface {
            representation,
            u,
            v,
        } => match representation {
            NurbsTypes::PolynomialSurface(surface) => {
                evaluate_polynomial_surface(&surface, &u, &v)?
            }
            NurbsTypes::RationalSurface(surface) => evaluate_rational_surface(&surface, &u, &v)?,
            NurbsTypes::PolynomialCurve(_) | NurbsTypes::RationalCurve(_) => {
                return Err(CadFailure::RequestShapeMismatch);
            }
        },
    };
    Ok(EvaluationResult::from_point(point))
}

pub fn evaluate_curve_nurbs(
    representation: &NurbsTypes,
    parameter: &RealAlgebraic,
) -> Result<EvaluationResult, CadFailure> {
    let point = match representation {
        NurbsTypes::PolynomialCurve(curve) => evaluate_polynomial_curve(curve, parameter)?,
        NurbsTypes::RationalCurve(curve) => evaluate_rational_curve(curve, parameter)?,
        NurbsTypes::PolynomialSurface(_) | NurbsTypes::RationalSurface(_) => {
            return Err(CadFailure::RequestShapeMismatch);
        }
    };
    Ok(EvaluationResult::from_point(point))
}

fn evaluate_polynomial_curve(
    curve: &PolynomialNurbsCurve,
    parameter: &RealAlgebraic,
) -> Result<BezierPoint, CadFailure> {
    de_boor(
        curve.control_points(),
        curve.knots(),
        curve.degree(),
        parameter,
        clone_point,
        blend_point,
    )
}

fn evaluate_rational_curve(
    curve: &RationalNurbsCurve,
    parameter: &RealAlgebraic,
) -> Result<BezierPoint, CadFailure> {
    let values = homogeneous_values(curve.control_points(), curve.weights())?;
    let value = de_boor(
        &values,
        curve.knots(),
        curve.degree(),
        parameter,
        clone_homogeneous,
        blend_homogeneous,
    )?;
    resolve_homogeneous(value, &curve.control_points()[0])
}

fn evaluate_polynomial_surface(
    surface: &PolynomialNurbsSurface,
    u: &RealAlgebraic,
    v: &RealAlgebraic,
) -> Result<BezierPoint, CadFailure> {
    let mut row_points = Vec::with_capacity(surface.control_net().len());
    for row in surface.control_net() {
        row_points.push(de_boor(
            row,
            surface.v_knots(),
            surface.v_degree(),
            v,
            clone_point,
            blend_point,
        )?);
    }
    de_boor(
        &row_points,
        surface.u_knots(),
        surface.u_degree(),
        u,
        clone_point,
        blend_point,
    )
}

fn evaluate_rational_surface(
    surface: &RationalNurbsSurface,
    u: &RealAlgebraic,
    v: &RealAlgebraic,
) -> Result<BezierPoint, CadFailure> {
    let mut row_values = Vec::with_capacity(surface.control_net().len());
    for (row, weights) in surface.control_net().iter().zip(surface.weights()) {
        let values = homogeneous_values(row, weights)?;
        row_values.push(de_boor(
            &values,
            surface.v_knots(),
            surface.v_degree(),
            v,
            clone_homogeneous,
            blend_homogeneous,
        )?);
    }
    let value = de_boor(
        &row_values,
        surface.u_knots(),
        surface.u_degree(),
        u,
        clone_homogeneous,
        blend_homogeneous,
    )?;
    resolve_homogeneous(value, &surface.control_net()[0][0])
}

fn de_boor<T, CloneValue, Blend>(
    control_values: &[T],
    knots: &[RealAlgebraic],
    degree: usize,
    parameter: &RealAlgebraic,
    mut clone_value: CloneValue,
    mut blend: Blend,
) -> Result<T, CadFailure>
where
    CloneValue: FnMut(&T) -> Result<T, CadFailure>,
    Blend: FnMut(&T, &T, &RealAlgebraic) -> Result<T, CadFailure>,
{
    let span = select_span(control_values.len(), knots, degree, parameter)?;
    let mut values = Vec::with_capacity(degree + 1);
    for control_value in &control_values[span - degree..=span] {
        values.push(clone_value(control_value)?);
    }
    for round in 1..=degree {
        for local_index in (round..=degree).rev() {
            let knot_index = span - degree + local_index;
            let numerator = parameter.sub(&knots[knot_index])?;
            let denominator = knots[knot_index + degree - round + 1].sub(&knots[knot_index])?;
            let alpha = numerator.div(&denominator)?;
            let blended = blend(&values[local_index - 1], &values[local_index], &alpha)?;
            values[local_index] = blended;
        }
    }
    Ok(values.remove(degree))
}

fn select_span(
    control_count: usize,
    knots: &[RealAlgebraic],
    degree: usize,
    parameter: &RealAlgebraic,
) -> Result<usize, CadFailure> {
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
        _ => return Err(CadFailure::EmptyParameterDomain),
    };
    if parameter.compare(&knots[first])? == Ordering::Less
        || parameter.compare(&knots[last + 1])? == Ordering::Greater
    {
        return Err(CadFailure::ParameterOutOfRange);
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
    Err(CadFailure::ParameterOutOfRange)
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

fn blend_point(
    left: &BezierPoint,
    right: &BezierPoint,
    alpha: &RealAlgebraic,
) -> Result<BezierPoint, CadFailure> {
    let mut result = clone_point(left)?;
    match (&mut result, left, right) {
        (BezierPoint::Point2(result), BezierPoint::Point2(left), BezierPoint::Point2(right)) => {
            result.x = blend_value(&left.x, &right.x, alpha)?;
            result.y = blend_value(&left.y, &right.y, alpha)?;
        }
        (BezierPoint::Point3(result), BezierPoint::Point3(left), BezierPoint::Point3(right)) => {
            result.x = blend_value(&left.x, &right.x, alpha)?;
            result.y = blend_value(&left.y, &right.y, alpha)?;
            result.z = blend_value(&left.z, &right.z, alpha)?;
        }
        _ => return Err(CadFailure::ControlPointDimensionMismatch),
    }
    Ok(result)
}

fn blend_value(
    left: &RealAlgebraic,
    right: &RealAlgebraic,
    alpha: &RealAlgebraic,
) -> Result<RealAlgebraic, CadFailure> {
    Ok(left.add(&alpha.mul(&right.sub(left)?)?)?)
}

enum HomogeneousValue {
    Point2 {
        xw: RealAlgebraic,
        yw: RealAlgebraic,
        weight: RealAlgebraic,
    },
    Point3 {
        xw: RealAlgebraic,
        yw: RealAlgebraic,
        zw: RealAlgebraic,
        weight: RealAlgebraic,
    },
}

fn homogeneous_values(
    control_points: &[BezierPoint],
    weights: &[RealAlgebraic],
) -> Result<Vec<HomogeneousValue>, CadFailure> {
    let mut values = Vec::with_capacity(control_points.len());
    for (point, weight) in control_points.iter().zip(weights) {
        values.push(to_homogeneous(point, weight)?);
    }
    Ok(values)
}

fn to_homogeneous(
    point: &BezierPoint,
    weight: &RealAlgebraic,
) -> Result<HomogeneousValue, CadFailure> {
    match point {
        BezierPoint::Point2(point) => Ok(HomogeneousValue::Point2 {
            xw: point.x.mul(weight)?,
            yw: point.y.mul(weight)?,
            weight: weight.try_clone()?,
        }),
        BezierPoint::Point3(point) => Ok(HomogeneousValue::Point3 {
            xw: point.x.mul(weight)?,
            yw: point.y.mul(weight)?,
            zw: point.z.mul(weight)?,
            weight: weight.try_clone()?,
        }),
    }
}

fn clone_homogeneous(value: &HomogeneousValue) -> Result<HomogeneousValue, CadFailure> {
    match value {
        HomogeneousValue::Point2 { xw, yw, weight } => Ok(HomogeneousValue::Point2 {
            xw: xw.try_clone()?,
            yw: yw.try_clone()?,
            weight: weight.try_clone()?,
        }),
        HomogeneousValue::Point3 { xw, yw, zw, weight } => Ok(HomogeneousValue::Point3 {
            xw: xw.try_clone()?,
            yw: yw.try_clone()?,
            zw: zw.try_clone()?,
            weight: weight.try_clone()?,
        }),
    }
}

fn blend_homogeneous(
    left: &HomogeneousValue,
    right: &HomogeneousValue,
    alpha: &RealAlgebraic,
) -> Result<HomogeneousValue, CadFailure> {
    match (left, right) {
        (
            HomogeneousValue::Point2 {
                xw: left_xw,
                yw: left_yw,
                weight: left_weight,
            },
            HomogeneousValue::Point2 {
                xw: right_xw,
                yw: right_yw,
                weight: right_weight,
            },
        ) => Ok(HomogeneousValue::Point2 {
            xw: blend_value(left_xw, right_xw, alpha)?,
            yw: blend_value(left_yw, right_yw, alpha)?,
            weight: blend_value(left_weight, right_weight, alpha)?,
        }),
        (
            HomogeneousValue::Point3 {
                xw: left_xw,
                yw: left_yw,
                zw: left_zw,
                weight: left_weight,
            },
            HomogeneousValue::Point3 {
                xw: right_xw,
                yw: right_yw,
                zw: right_zw,
                weight: right_weight,
            },
        ) => Ok(HomogeneousValue::Point3 {
            xw: blend_value(left_xw, right_xw, alpha)?,
            yw: blend_value(left_yw, right_yw, alpha)?,
            zw: blend_value(left_zw, right_zw, alpha)?,
            weight: blend_value(left_weight, right_weight, alpha)?,
        }),
        _ => Err(CadFailure::ControlPointDimensionMismatch),
    }
}

fn resolve_homogeneous(
    value: HomogeneousValue,
    template: &BezierPoint,
) -> Result<BezierPoint, CadFailure> {
    let mut point = clone_point(template)?;
    match (&mut point, value) {
        (BezierPoint::Point2(point), HomogeneousValue::Point2 { xw, yw, weight }) => {
            point.x = xw.div(&weight)?;
            point.y = yw.div(&weight)?;
        }
        (BezierPoint::Point3(point), HomogeneousValue::Point3 { xw, yw, zw, weight }) => {
            point.x = xw.div(&weight)?;
            point.y = yw.div(&weight)?;
            point.z = zw.div(&weight)?;
        }
        _ => return Err(CadFailure::ControlPointDimensionMismatch),
    }
    Ok(point)
}
