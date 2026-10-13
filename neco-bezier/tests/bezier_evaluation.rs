use neco_algnum::RealAlgebraic;
use neco_bezier::{
    define_bezier, evaluate_bezier, BezierDefinitionRequest, BezierEvaluationRequest, BezierPoint,
    CadFailure,
};
use neco_bigint::BigInt;
use neco_geometry::{Point2, Point3};

fn algebraic(value: i32) -> RealAlgebraic {
    RealAlgebraic::from_integer(BigInt::try_from(value).expect("i32 converts to BigInt"))
        .expect("integer is algebraic")
}

fn half() -> RealAlgebraic {
    algebraic(1)
        .div(&algebraic(2))
        .expect("one half is algebraic")
}

fn point2(x: i32, y: i32) -> BezierPoint {
    BezierPoint::Point2(Point2 {
        x: algebraic(x),
        y: algebraic(y),
    })
}

fn point3(x: i32, y: i32, z: i32) -> BezierPoint {
    BezierPoint::Point3(Point3 {
        x: algebraic(x),
        y: algebraic(y),
        z: algebraic(z),
    })
}

fn polynomial_curve() -> neco_bezier::BezierTypes {
    define_bezier(BezierDefinitionRequest::PolynomialCurve {
        control_points: vec![point2(0, 0), point2(2, 4)],
        degree: 1,
    })
    .expect("polynomial curve is valid")
}

fn rational_curve() -> neco_bezier::BezierTypes {
    define_bezier(BezierDefinitionRequest::RationalCurve {
        control_points: vec![point2(0, 0), point2(2, 4)],
        weights: vec![algebraic(1), algebraic(3)],
        degree: 1,
    })
    .expect("rational curve is valid")
}

fn polynomial_surface() -> neco_bezier::BezierTypes {
    define_bezier(BezierDefinitionRequest::PolynomialSurface {
        control_net: vec![
            vec![point2(0, 0), point2(0, 2)],
            vec![point2(2, 0), point2(2, 2)],
        ],
        u_degree: 1,
        v_degree: 1,
    })
    .expect("polynomial surface is valid")
}

fn rational_surface() -> neco_bezier::BezierTypes {
    define_bezier(BezierDefinitionRequest::RationalSurface {
        control_net: vec![
            vec![point2(0, 0), point2(0, 2)],
            vec![point2(2, 0), point2(2, 2)],
        ],
        weights: vec![
            vec![algebraic(1), algebraic(2)],
            vec![algebraic(3), algebraic(4)],
        ],
        u_degree: 1,
        v_degree: 1,
    })
    .expect("rational surface is valid")
}

#[test]
fn curve_endpoints_return_control_points() {
    for (representation, parameter, expected) in [
        (polynomial_curve(), algebraic(0), point2(0, 0)),
        (polynomial_curve(), algebraic(1), point2(2, 4)),
        (rational_curve(), algebraic(0), point2(0, 0)),
        (rational_curve(), algebraic(1), point2(2, 4)),
    ] {
        let result = evaluate_bezier(BezierEvaluationRequest::Curve {
            representation,
            parameter,
        })
        .expect("endpoint evaluation succeeds");
        assert_eq!(result.into_point(), expected);
    }
}

#[test]
fn linear_polynomial_curve_interpolates_exactly() {
    let result = evaluate_bezier(BezierEvaluationRequest::Curve {
        representation: polynomial_curve(),
        parameter: half(),
    })
    .expect("interior evaluation succeeds");

    assert_eq!(result.into_point(), point2(1, 2));
}

#[test]
fn surface_corners_return_control_net_points() {
    for (representation, u, v, expected) in [
        (
            polynomial_surface(),
            algebraic(0),
            algebraic(0),
            point2(0, 0),
        ),
        (
            polynomial_surface(),
            algebraic(0),
            algebraic(1),
            point2(0, 2),
        ),
        (
            polynomial_surface(),
            algebraic(1),
            algebraic(0),
            point2(2, 0),
        ),
        (
            polynomial_surface(),
            algebraic(1),
            algebraic(1),
            point2(2, 2),
        ),
        (rational_surface(), algebraic(0), algebraic(0), point2(0, 0)),
        (rational_surface(), algebraic(0), algebraic(1), point2(0, 2)),
        (rational_surface(), algebraic(1), algebraic(0), point2(2, 0)),
        (rational_surface(), algebraic(1), algebraic(1), point2(2, 2)),
    ] {
        let result = evaluate_bezier(BezierEvaluationRequest::Surface {
            representation,
            u,
            v,
        })
        .expect("corner evaluation succeeds");
        assert_eq!(result.into_point(), expected);
    }
}

#[test]
fn rational_curve_and_surface_use_homogeneous_coordinates() {
    let curve_result = evaluate_bezier(BezierEvaluationRequest::Curve {
        representation: rational_curve(),
        parameter: half(),
    })
    .expect("rational curve evaluation succeeds");
    let curve_expected = BezierPoint::Point2(Point2 {
        x: algebraic(3)
            .div(&algebraic(2))
            .expect("weighted x coordinate is algebraic"),
        y: algebraic(3),
    });
    assert_eq!(curve_result.into_point(), curve_expected);

    let surface_result = evaluate_bezier(BezierEvaluationRequest::Surface {
        representation: rational_surface(),
        u: half(),
        v: half(),
    })
    .expect("rational surface evaluation succeeds");
    let surface_weight = algebraic(5)
        .div(&algebraic(2))
        .expect("weighted surface denominator is algebraic");
    let surface_expected = BezierPoint::Point2(Point2 {
        x: algebraic(7)
            .div(&algebraic(2))
            .expect("weighted x numerator is algebraic")
            .div(&surface_weight)
            .expect("weighted x coordinate is algebraic"),
        y: algebraic(3)
            .div(
                &algebraic(5)
                    .div(&algebraic(2))
                    .expect("weight is algebraic"),
            )
            .expect("weighted y coordinate is algebraic"),
    });
    assert_eq!(surface_result.into_point(), surface_expected);
}

#[test]
fn preserves_point2_and_point3() {
    let curve = define_bezier(BezierDefinitionRequest::PolynomialCurve {
        control_points: vec![point2(0, 0), point2(2, 2)],
        degree: 1,
    })
    .expect("Point2 curve is valid");
    let surface = define_bezier(BezierDefinitionRequest::PolynomialSurface {
        control_net: vec![
            vec![point3(0, 0, 0), point3(0, 2, 2)],
            vec![point3(2, 0, 2), point3(2, 2, 4)],
        ],
        u_degree: 1,
        v_degree: 1,
    })
    .expect("Point3 surface is valid");

    let curve_result = evaluate_bezier(BezierEvaluationRequest::Curve {
        representation: curve,
        parameter: half(),
    })
    .expect("Point2 curve evaluation succeeds");
    assert!(matches!(curve_result.into_point(), BezierPoint::Point2(_)));

    let surface_result = evaluate_bezier(BezierEvaluationRequest::Surface {
        representation: surface,
        u: half(),
        v: half(),
    })
    .expect("Point3 surface evaluation succeeds");
    assert!(matches!(
        surface_result.into_point(),
        BezierPoint::Point3(_)
    ));
}

#[test]
fn request_shape_mismatch_returns_explicit_failure() {
    let curve_result = evaluate_bezier(BezierEvaluationRequest::Curve {
        representation: polynomial_surface(),
        parameter: algebraic(0),
    });
    assert!(matches!(
        curve_result,
        Err(CadFailure::RequestShapeMismatch)
    ));

    let surface_result = evaluate_bezier(BezierEvaluationRequest::Surface {
        representation: polynomial_curve(),
        u: algebraic(0),
        v: algebraic(0),
    });
    assert!(matches!(
        surface_result,
        Err(CadFailure::RequestShapeMismatch)
    ));
}

#[test]
fn parameters_below_and_above_unit_interval_fail() {
    let below_curve_result = evaluate_bezier(BezierEvaluationRequest::Curve {
        representation: polynomial_curve(),
        parameter: algebraic(-1)
            .div(&algebraic(2))
            .expect("negative one half is algebraic"),
    });
    assert!(matches!(
        below_curve_result,
        Err(CadFailure::ParameterOutOfRange)
    ));

    let above_curve_result = evaluate_bezier(BezierEvaluationRequest::Curve {
        representation: polynomial_curve(),
        parameter: algebraic(2),
    });
    assert!(matches!(
        above_curve_result,
        Err(CadFailure::ParameterOutOfRange)
    ));

    let below_u_result = evaluate_bezier(BezierEvaluationRequest::Surface {
        representation: polynomial_surface(),
        u: algebraic(-1)
            .div(&algebraic(2))
            .expect("negative one half is algebraic"),
        v: algebraic(0),
    });
    assert!(matches!(
        below_u_result,
        Err(CadFailure::ParameterOutOfRange)
    ));

    let above_v_result = evaluate_bezier(BezierEvaluationRequest::Surface {
        representation: polynomial_surface(),
        u: algebraic(0),
        v: algebraic(2),
    });
    assert!(matches!(
        above_v_result,
        Err(CadFailure::ParameterOutOfRange)
    ));
}
