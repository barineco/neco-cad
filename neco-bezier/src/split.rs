use alloc::vec::Vec;

use neco_algnum::RealAlgebraic;
use neco_geometry::{Point2, Point3};

use crate::{
    define_bezier, BezierDefinitionRequest, BezierPoint, BezierTypes, CadFailure,
    PolynomialBezierSurface, RationalBezierSurface,
};

type SplitCurveHalves = (Vec<BezierPoint>, Vec<BezierPoint>);
type SplitRationalCurveHalves = (
    Vec<BezierPoint>,
    Vec<RealAlgebraic>,
    Vec<BezierPoint>,
    Vec<RealAlgebraic>,
);
type SplitSurfaceHalves = (Vec<Vec<BezierPoint>>, Vec<Vec<BezierPoint>>);
type SplitRationalSurfaceHalves = (
    Vec<Vec<BezierPoint>>,
    Vec<Vec<RealAlgebraic>>,
    Vec<Vec<BezierPoint>>,
    Vec<Vec<RealAlgebraic>>,
);

pub enum BezierSplitAxis {
    U,
    V,
}

pub enum BezierSplitRequest {
    Curve {
        representation: BezierTypes,
        parameter: RealAlgebraic,
    },
    Surface {
        representation: BezierTypes,
        axis: BezierSplitAxis,
        parameter: RealAlgebraic,
    },
}

pub enum BezierSplitResult {
    Curve {
        left: BezierTypes,
        right: BezierTypes,
    },
    Surface {
        lower: BezierTypes,
        upper: BezierTypes,
    },
}

pub fn split_bezier(request: BezierSplitRequest) -> Result<BezierSplitResult, CadFailure> {
    match request {
        BezierSplitRequest::Curve {
            representation,
            parameter,
        } => {
            crate::evaluation::validate_parameter(&parameter)?;
            match representation {
                BezierTypes::PolynomialCurve(curve) => {
                    let (left, right) = split_polynomial_curve(curve.control_points(), &parameter)?;
                    Ok(BezierSplitResult::Curve {
                        left: define_bezier(BezierDefinitionRequest::PolynomialCurve {
                            control_points: left,
                            degree: curve.degree(),
                        })?,
                        right: define_bezier(BezierDefinitionRequest::PolynomialCurve {
                            control_points: right,
                            degree: curve.degree(),
                        })?,
                    })
                }
                BezierTypes::RationalCurve(curve) => {
                    let (left_points, left_weights, right_points, right_weights) =
                        split_rational_curve(curve.control_points(), curve.weights(), &parameter)?;
                    Ok(BezierSplitResult::Curve {
                        left: define_bezier(BezierDefinitionRequest::RationalCurve {
                            control_points: left_points,
                            weights: left_weights,
                            degree: curve.degree(),
                        })?,
                        right: define_bezier(BezierDefinitionRequest::RationalCurve {
                            control_points: right_points,
                            weights: right_weights,
                            degree: curve.degree(),
                        })?,
                    })
                }
                BezierTypes::PolynomialSurface(_) | BezierTypes::RationalSurface(_) => {
                    Err(CadFailure::RequestShapeMismatch)
                }
            }
        }
        BezierSplitRequest::Surface {
            representation,
            axis,
            parameter,
        } => {
            crate::evaluation::validate_parameter(&parameter)?;
            match representation {
                BezierTypes::PolynomialSurface(surface) => {
                    split_polynomial_surface(surface, axis, &parameter)
                }
                BezierTypes::RationalSurface(surface) => {
                    split_rational_surface(surface, axis, &parameter)
                }
                BezierTypes::PolynomialCurve(_) | BezierTypes::RationalCurve(_) => {
                    Err(CadFailure::RequestShapeMismatch)
                }
            }
        }
    }
}

fn split_polynomial_curve(
    control_points: &[BezierPoint],
    parameter: &RealAlgebraic,
) -> Result<SplitCurveHalves, CadFailure> {
    let mut level = clone_points(control_points)?;
    let mut left = Vec::with_capacity(level.len());
    let mut right = Vec::with_capacity(level.len());

    while let Some(first) = level.first() {
        left.push(clone_point(first)?);
        if let Some(last) = level.last() {
            right.push(clone_point(last)?);
        }
        if level.len() == 1 {
            break;
        }
        level = interpolate_point_level(&level, parameter)?;
    }

    right.reverse();
    Ok((left, right))
}

fn split_rational_curve(
    control_points: &[BezierPoint],
    weights: &[RealAlgebraic],
    parameter: &RealAlgebraic,
) -> Result<SplitRationalCurveHalves, CadFailure> {
    let mut level = homogeneous_points(control_points, weights)?;
    let mut left = Vec::with_capacity(level.len());
    let mut right = Vec::with_capacity(level.len());

    while let Some(first) = level.first() {
        left.push(clone_homogeneous(first)?);
        if let Some(last) = level.last() {
            right.push(clone_homogeneous(last)?);
        }
        if level.len() == 1 {
            break;
        }
        level = interpolate_homogeneous_level(&level, parameter)?;
    }

    right.reverse();
    let (left_points, left_weights) = resolve_homogeneous_points(left)?;
    let (right_points, right_weights) = resolve_homogeneous_points(right)?;
    Ok((left_points, left_weights, right_points, right_weights))
}

fn split_polynomial_surface(
    surface: PolynomialBezierSurface,
    axis: BezierSplitAxis,
    parameter: &RealAlgebraic,
) -> Result<BezierSplitResult, CadFailure> {
    let (lower_net, upper_net) = match axis {
        BezierSplitAxis::U => split_polynomial_surface_u(surface.control_net(), parameter)?,
        BezierSplitAxis::V => split_polynomial_surface_v(surface.control_net(), parameter)?,
    };
    Ok(BezierSplitResult::Surface {
        lower: define_bezier(BezierDefinitionRequest::PolynomialSurface {
            control_net: lower_net,
            u_degree: surface.u_degree(),
            v_degree: surface.v_degree(),
        })?,
        upper: define_bezier(BezierDefinitionRequest::PolynomialSurface {
            control_net: upper_net,
            u_degree: surface.u_degree(),
            v_degree: surface.v_degree(),
        })?,
    })
}

fn split_rational_surface(
    surface: RationalBezierSurface,
    axis: BezierSplitAxis,
    parameter: &RealAlgebraic,
) -> Result<BezierSplitResult, CadFailure> {
    let (lower_net, lower_weights, upper_net, upper_weights) = match axis {
        BezierSplitAxis::U => {
            split_rational_surface_u(surface.control_net(), surface.weights(), parameter)?
        }
        BezierSplitAxis::V => {
            split_rational_surface_v(surface.control_net(), surface.weights(), parameter)?
        }
    };
    Ok(BezierSplitResult::Surface {
        lower: define_bezier(BezierDefinitionRequest::RationalSurface {
            control_net: lower_net,
            weights: lower_weights,
            u_degree: surface.u_degree(),
            v_degree: surface.v_degree(),
        })?,
        upper: define_bezier(BezierDefinitionRequest::RationalSurface {
            control_net: upper_net,
            weights: upper_weights,
            u_degree: surface.u_degree(),
            v_degree: surface.v_degree(),
        })?,
    })
}

fn split_polynomial_surface_u(
    control_net: &[Vec<BezierPoint>],
    parameter: &RealAlgebraic,
) -> Result<SplitSurfaceHalves, CadFailure> {
    let row_count = control_net.len();
    let column_count = control_net[0].len();
    let mut lower = (0..row_count)
        .map(|_| Vec::with_capacity(column_count))
        .collect::<Vec<_>>();
    let mut upper = (0..row_count)
        .map(|_| Vec::with_capacity(column_count))
        .collect::<Vec<_>>();

    for column_index in 0..column_count {
        let mut column = Vec::with_capacity(row_count);
        for row in control_net {
            column.push(clone_point(&row[column_index])?);
        }
        let (lower_column, upper_column) = split_polynomial_curve(&column, parameter)?;
        for (row, point) in lower.iter_mut().zip(lower_column) {
            row.push(point);
        }
        for (row, point) in upper.iter_mut().zip(upper_column) {
            row.push(point);
        }
    }

    Ok((lower, upper))
}

fn split_polynomial_surface_v(
    control_net: &[Vec<BezierPoint>],
    parameter: &RealAlgebraic,
) -> Result<SplitSurfaceHalves, CadFailure> {
    let mut lower = Vec::with_capacity(control_net.len());
    let mut upper = Vec::with_capacity(control_net.len());
    for row in control_net {
        let (lower_row, upper_row) = split_polynomial_curve(row, parameter)?;
        lower.push(lower_row);
        upper.push(upper_row);
    }
    Ok((lower, upper))
}

fn split_rational_surface_u(
    control_net: &[Vec<BezierPoint>],
    weights: &[Vec<RealAlgebraic>],
    parameter: &RealAlgebraic,
) -> Result<SplitRationalSurfaceHalves, CadFailure> {
    let row_count = control_net.len();
    let column_count = control_net[0].len();
    let mut lower_points = (0..row_count)
        .map(|_| Vec::with_capacity(column_count))
        .collect::<Vec<_>>();
    let mut lower_weights = (0..row_count)
        .map(|_| Vec::with_capacity(column_count))
        .collect::<Vec<_>>();
    let mut upper_points = (0..row_count)
        .map(|_| Vec::with_capacity(column_count))
        .collect::<Vec<_>>();
    let mut upper_weights = (0..row_count)
        .map(|_| Vec::with_capacity(column_count))
        .collect::<Vec<_>>();

    for column_index in 0..column_count {
        let mut column_points = Vec::with_capacity(row_count);
        let mut column_weights = Vec::with_capacity(row_count);
        for (row, weight_row) in control_net.iter().zip(weights) {
            column_points.push(clone_point(&row[column_index])?);
            column_weights.push(weight_row[column_index].try_clone()?);
        }
        let (lower_column_points, lower_column_weights, upper_column_points, upper_column_weights) =
            split_rational_curve(&column_points, &column_weights, parameter)?;
        for (row, point) in lower_points.iter_mut().zip(lower_column_points) {
            row.push(point);
        }
        for (row, weight) in lower_weights.iter_mut().zip(lower_column_weights) {
            row.push(weight);
        }
        for (row, point) in upper_points.iter_mut().zip(upper_column_points) {
            row.push(point);
        }
        for (row, weight) in upper_weights.iter_mut().zip(upper_column_weights) {
            row.push(weight);
        }
    }

    Ok((lower_points, lower_weights, upper_points, upper_weights))
}

fn split_rational_surface_v(
    control_net: &[Vec<BezierPoint>],
    weights: &[Vec<RealAlgebraic>],
    parameter: &RealAlgebraic,
) -> Result<SplitRationalSurfaceHalves, CadFailure> {
    let mut lower_points = Vec::with_capacity(control_net.len());
    let mut lower_weights = Vec::with_capacity(control_net.len());
    let mut upper_points = Vec::with_capacity(control_net.len());
    let mut upper_weights = Vec::with_capacity(control_net.len());

    for (row, weight_row) in control_net.iter().zip(weights) {
        let (lower_row_points, lower_row_weights, upper_row_points, upper_row_weights) =
            split_rational_curve(row, weight_row, parameter)?;
        lower_points.push(lower_row_points);
        lower_weights.push(lower_row_weights);
        upper_points.push(upper_row_points);
        upper_weights.push(upper_row_weights);
    }

    Ok((lower_points, lower_weights, upper_points, upper_weights))
}

fn clone_points(points: &[BezierPoint]) -> Result<Vec<BezierPoint>, CadFailure> {
    points.iter().map(clone_point).collect()
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

fn interpolate_point_level(
    points: &[BezierPoint],
    parameter: &RealAlgebraic,
) -> Result<Vec<BezierPoint>, CadFailure> {
    points
        .windows(2)
        .map(|pair| interpolate_point(&pair[0], &pair[1], parameter))
        .collect()
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
    control_points
        .iter()
        .zip(weights)
        .map(|(point, weight)| to_homogeneous(point, weight))
        .collect()
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

fn clone_homogeneous(point: &HomogeneousPoint) -> Result<HomogeneousPoint, CadFailure> {
    match point {
        HomogeneousPoint::Point2 { x, y, weight } => Ok(HomogeneousPoint::Point2 {
            x: x.try_clone()?,
            y: y.try_clone()?,
            weight: weight.try_clone()?,
        }),
        HomogeneousPoint::Point3 { x, y, z, weight } => Ok(HomogeneousPoint::Point3 {
            x: x.try_clone()?,
            y: y.try_clone()?,
            z: z.try_clone()?,
            weight: weight.try_clone()?,
        }),
    }
}

fn interpolate_homogeneous_level(
    points: &[HomogeneousPoint],
    parameter: &RealAlgebraic,
) -> Result<Vec<HomogeneousPoint>, CadFailure> {
    points
        .windows(2)
        .map(|pair| interpolate_homogeneous(&pair[0], &pair[1], parameter))
        .collect()
}

fn interpolate_homogeneous(
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

fn resolve_homogeneous_points(
    points: Vec<HomogeneousPoint>,
) -> Result<(Vec<BezierPoint>, Vec<RealAlgebraic>), CadFailure> {
    let mut resolved_points = Vec::with_capacity(points.len());
    let mut weights = Vec::with_capacity(points.len());
    for point in points {
        let (resolved, weight) = resolve_homogeneous(point)?;
        resolved_points.push(resolved);
        weights.push(weight);
    }
    Ok((resolved_points, weights))
}

fn resolve_homogeneous(
    point: HomogeneousPoint,
) -> Result<(BezierPoint, RealAlgebraic), CadFailure> {
    match point {
        HomogeneousPoint::Point2 { x, y, weight } => Ok((
            BezierPoint::Point2(Point2 {
                x: x.div(&weight)?,
                y: y.div(&weight)?,
            }),
            weight,
        )),
        HomogeneousPoint::Point3 { x, y, z, weight } => Ok((
            BezierPoint::Point3(Point3 {
                x: x.div(&weight)?,
                y: y.div(&weight)?,
                z: z.div(&weight)?,
            }),
            weight,
        )),
    }
}
