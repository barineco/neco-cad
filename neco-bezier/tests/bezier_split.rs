use neco_algnum::RealAlgebraic;
use neco_bezier::{
    define_bezier, evaluate_bezier, split_bezier, BezierDefinitionRequest, BezierEvaluationRequest,
    BezierPoint, BezierSplitAxis, BezierSplitRequest, BezierSplitResult, BezierTypes, CadFailure,
};
use neco_bigint::BigInt;
use neco_geometry::{Point2, Point3};

fn algebraic(value: i32) -> RealAlgebraic {
    RealAlgebraic::from_integer(BigInt::try_from(value).expect("i32 converts to BigInt"))
        .expect("integer is algebraic")
}

fn fraction(numerator: i32, denominator: i32) -> RealAlgebraic {
    algebraic(numerator)
        .div(&algebraic(denominator))
        .expect("rational value is algebraic")
}

fn point2(x: i32, y: i32) -> BezierPoint {
    BezierPoint::Point2(Point2 {
        x: algebraic(x),
        y: algebraic(y),
    })
}

fn point2_fraction(
    x_numerator: i32,
    x_denominator: i32,
    y_numerator: i32,
    y_denominator: i32,
) -> BezierPoint {
    BezierPoint::Point2(Point2 {
        x: fraction(x_numerator, x_denominator),
        y: fraction(y_numerator, y_denominator),
    })
}

fn point3(x: i32, y: i32, z: i32) -> BezierPoint {
    BezierPoint::Point3(Point3 {
        x: algebraic(x),
        y: algebraic(y),
        z: algebraic(z),
    })
}

fn point3_fraction(
    x_numerator: i32,
    x_denominator: i32,
    y_numerator: i32,
    y_denominator: i32,
    z_numerator: i32,
    z_denominator: i32,
) -> BezierPoint {
    BezierPoint::Point3(Point3 {
        x: fraction(x_numerator, x_denominator),
        y: fraction(y_numerator, y_denominator),
        z: fraction(z_numerator, z_denominator),
    })
}

fn polynomial_curve() -> BezierTypes {
    define_bezier(BezierDefinitionRequest::PolynomialCurve {
        control_points: vec![point2(0, 0), point2(2, 4), point2(4, 0)],
        degree: 2,
    })
    .expect("polynomial curve is valid")
}

fn rational_curve() -> BezierTypes {
    define_bezier(BezierDefinitionRequest::RationalCurve {
        control_points: vec![point2(0, 0), point2(2, 4)],
        weights: vec![algebraic(1), algebraic(3)],
        degree: 1,
    })
    .expect("rational curve is valid")
}

fn polynomial_surface() -> BezierTypes {
    define_bezier(BezierDefinitionRequest::PolynomialSurface {
        control_net: vec![
            vec![point2(0, 0), point2(0, 2), point2(0, 6)],
            vec![point2(4, 1), point2(4, 3), point2(4, 7)],
        ],
        u_degree: 1,
        v_degree: 2,
    })
    .expect("polynomial surface is valid")
}

fn rational_surface() -> BezierTypes {
    define_bezier(BezierDefinitionRequest::RationalSurface {
        control_net: vec![
            vec![point3(0, 0, 0), point3(0, 2, 4)],
            vec![point3(4, 1, 3), point3(4, 3, 7)],
        ],
        weights: vec![
            vec![algebraic(1), algebraic(2)],
            vec![algebraic(3), algebraic(5)],
        ],
        u_degree: 1,
        v_degree: 1,
    })
    .expect("rational surface is valid")
}

fn split_curve(
    representation: BezierTypes,
    parameter: RealAlgebraic,
) -> (BezierTypes, BezierTypes) {
    match split_bezier(BezierSplitRequest::Curve {
        representation,
        parameter,
    })
    .expect("curve split succeeds")
    {
        BezierSplitResult::Curve { left, right } => (left, right),
        BezierSplitResult::Surface { .. } => panic!("curve request returns curve result"),
    }
}

fn split_surface(
    representation: BezierTypes,
    axis: BezierSplitAxis,
    parameter: RealAlgebraic,
) -> (BezierTypes, BezierTypes) {
    match split_bezier(BezierSplitRequest::Surface {
        representation,
        axis,
        parameter,
    })
    .expect("surface split succeeds")
    {
        BezierSplitResult::Surface { lower, upper } => (lower, upper),
        BezierSplitResult::Curve { .. } => panic!("surface request returns surface result"),
    }
}

fn assert_polynomial_surface(
    representation: BezierTypes,
    expected_net: Vec<Vec<BezierPoint>>,
    expected_u_degree: usize,
    expected_v_degree: usize,
) {
    let BezierTypes::PolynomialSurface(surface) = representation else {
        panic!("expected polynomial surface");
    };
    assert_eq!(surface.control_net(), expected_net.as_slice());
    assert_eq!(surface.u_degree(), expected_u_degree);
    assert_eq!(surface.v_degree(), expected_v_degree);
    assert_eq!(surface.dimension(), 2);
}

fn assert_rational_surface(
    representation: BezierTypes,
    expected_net: Vec<Vec<BezierPoint>>,
    expected_weights: Vec<Vec<RealAlgebraic>>,
) {
    let BezierTypes::RationalSurface(surface) = representation else {
        panic!("expected rational surface");
    };
    assert_eq!(surface.control_net(), expected_net.as_slice());
    assert_eq!(surface.weights(), expected_weights.as_slice());
    assert_eq!(surface.u_degree(), 1);
    assert_eq!(surface.v_degree(), 1);
    assert_eq!(surface.dimension(), 3);
}

#[test]
fn polynomial_curve_split_has_exact_boundary_and_interior_control_points() {
    let degree_zero = define_bezier(BezierDefinitionRequest::PolynomialCurve {
        control_points: vec![point3(7, 8, 9)],
        degree: 0,
    })
    .expect("degree-zero curve is valid");
    for parameter in [algebraic(0), algebraic(1)] {
        let (left, right) = split_curve(
            define_bezier(BezierDefinitionRequest::PolynomialCurve {
                control_points: vec![point3(7, 8, 9)],
                degree: 0,
            })
            .expect("degree-zero curve is valid"),
            parameter,
        );
        assert_eq!(left, degree_zero);
        assert_eq!(right, degree_zero);
    }

    let (left_zero, right_zero) = split_curve(polynomial_curve(), algebraic(0));
    let BezierTypes::PolynomialCurve(left_zero) = left_zero else {
        panic!("expected polynomial curve");
    };
    let BezierTypes::PolynomialCurve(right_zero) = right_zero else {
        panic!("expected polynomial curve");
    };
    assert_eq!(
        left_zero.control_points(),
        [point2(0, 0), point2(0, 0), point2(0, 0)]
    );
    assert_eq!(
        right_zero.control_points(),
        [point2(0, 0), point2(2, 4), point2(4, 0)]
    );

    let (left_one, right_one) = split_curve(polynomial_curve(), algebraic(1));
    let BezierTypes::PolynomialCurve(left_one) = left_one else {
        panic!("expected polynomial curve");
    };
    let BezierTypes::PolynomialCurve(right_one) = right_one else {
        panic!("expected polynomial curve");
    };
    assert_eq!(
        left_one.control_points(),
        [point2(0, 0), point2(2, 4), point2(4, 0)]
    );
    assert_eq!(
        right_one.control_points(),
        [point2(4, 0), point2(4, 0), point2(4, 0)]
    );

    let (left, right) = split_curve(polynomial_curve(), fraction(1, 2));
    let BezierTypes::PolynomialCurve(left) = left else {
        panic!("expected polynomial curve");
    };
    let BezierTypes::PolynomialCurve(right) = right else {
        panic!("expected polynomial curve");
    };
    assert_eq!(
        left.control_points(),
        [point2(0, 0), point2(1, 2), point2(2, 2)]
    );
    assert_eq!(
        right.control_points(),
        [point2(2, 2), point2(3, 2), point2(4, 0)]
    );
    assert_eq!(left.control_points()[2], right.control_points()[0]);
    assert_eq!(left.degree(), 2);
    assert_eq!(right.degree(), 2);
    assert_eq!(left.dimension(), 2);
    assert_eq!(right.dimension(), 2);
}

#[test]
fn rational_curve_split_has_exact_homogeneous_control_points_and_weights() {
    let (left, right) = split_curve(rational_curve(), fraction(1, 2));
    let BezierTypes::RationalCurve(left) = left else {
        panic!("expected rational curve");
    };
    let BezierTypes::RationalCurve(right) = right else {
        panic!("expected rational curve");
    };

    assert_eq!(
        left.control_points(),
        [point2(0, 0), point2_fraction(3, 2, 3, 1)]
    );
    assert_eq!(left.weights(), [algebraic(1), algebraic(2)]);
    assert_eq!(
        right.control_points(),
        [point2_fraction(3, 2, 3, 1), point2(2, 4)]
    );
    assert_eq!(right.weights(), [algebraic(2), algebraic(3)]);
    assert_eq!(left.control_points()[1], right.control_points()[0]);
    assert_eq!(left.weights()[1], right.weights()[0]);
}

#[test]
fn surface_u_and_v_splits_preserve_shared_edge_and_row_first_ordering() {
    let (polynomial_u_lower, polynomial_u_upper) =
        split_surface(polynomial_surface(), BezierSplitAxis::U, fraction(1, 2));
    assert_polynomial_surface(
        polynomial_u_lower,
        vec![
            vec![point2(0, 0), point2(0, 2), point2(0, 6)],
            vec![
                point2_fraction(2, 1, 1, 2),
                point2_fraction(2, 1, 5, 2),
                point2_fraction(2, 1, 13, 2),
            ],
        ],
        1,
        2,
    );
    assert_polynomial_surface(
        polynomial_u_upper,
        vec![
            vec![
                point2_fraction(2, 1, 1, 2),
                point2_fraction(2, 1, 5, 2),
                point2_fraction(2, 1, 13, 2),
            ],
            vec![point2(4, 1), point2(4, 3), point2(4, 7)],
        ],
        1,
        2,
    );

    let (polynomial_v_lower, polynomial_v_upper) =
        split_surface(polynomial_surface(), BezierSplitAxis::V, fraction(1, 2));
    assert_polynomial_surface(
        polynomial_v_lower,
        vec![
            vec![point2(0, 0), point2(0, 1), point2_fraction(0, 1, 5, 2)],
            vec![point2(4, 1), point2(4, 2), point2_fraction(4, 1, 7, 2)],
        ],
        1,
        2,
    );
    assert_polynomial_surface(
        polynomial_v_upper,
        vec![
            vec![point2_fraction(0, 1, 5, 2), point2(0, 4), point2(0, 6)],
            vec![point2_fraction(4, 1, 7, 2), point2(4, 5), point2(4, 7)],
        ],
        1,
        2,
    );

    let (rational_u_lower, rational_u_upper) =
        split_surface(rational_surface(), BezierSplitAxis::U, fraction(1, 2));
    assert_rational_surface(
        rational_u_lower,
        vec![
            vec![point3(0, 0, 0), point3(0, 2, 4)],
            vec![
                point3_fraction(3, 1, 3, 4, 9, 4),
                point3_fraction(20, 7, 19, 7, 43, 7),
            ],
        ],
        vec![
            vec![algebraic(1), algebraic(2)],
            vec![algebraic(2), fraction(7, 2)],
        ],
    );
    assert_rational_surface(
        rational_u_upper,
        vec![
            vec![
                point3_fraction(3, 1, 3, 4, 9, 4),
                point3_fraction(20, 7, 19, 7, 43, 7),
            ],
            vec![point3(4, 1, 3), point3(4, 3, 7)],
        ],
        vec![
            vec![algebraic(2), fraction(7, 2)],
            vec![algebraic(3), algebraic(5)],
        ],
    );

    let (rational_v_lower, rational_v_upper) =
        split_surface(rational_surface(), BezierSplitAxis::V, fraction(1, 2));
    assert_rational_surface(
        rational_v_lower,
        vec![
            vec![point3(0, 0, 0), point3_fraction(0, 1, 4, 3, 8, 3)],
            vec![point3(4, 1, 3), point3_fraction(4, 1, 9, 4, 11, 2)],
        ],
        vec![
            vec![algebraic(1), fraction(3, 2)],
            vec![algebraic(3), algebraic(4)],
        ],
    );
    assert_rational_surface(
        rational_v_upper,
        vec![
            vec![point3_fraction(0, 1, 4, 3, 8, 3), point3(0, 2, 4)],
            vec![point3_fraction(4, 1, 9, 4, 11, 2), point3(4, 3, 7)],
        ],
        vec![
            vec![fraction(3, 2), algebraic(2)],
            vec![algebraic(4), algebraic(5)],
        ],
    );

    let (polynomial_lower, polynomial_upper) =
        split_surface(polynomial_surface(), BezierSplitAxis::U, fraction(1, 2));
    let (
        BezierTypes::PolynomialSurface(polynomial_lower),
        BezierTypes::PolynomialSurface(polynomial_upper),
    ) = (polynomial_lower, polynomial_upper)
    else {
        panic!("polynomial U split returns polynomial surfaces");
    };
    assert_eq!(
        polynomial_lower.control_net()[1],
        polynomial_upper.control_net()[0]
    );

    let (rational_lower, rational_upper) =
        split_surface(rational_surface(), BezierSplitAxis::V, fraction(1, 2));
    let (
        BezierTypes::RationalSurface(rational_lower),
        BezierTypes::RationalSurface(rational_upper),
    ) = (rational_lower, rational_upper)
    else {
        panic!("rational V split returns rational surfaces");
    };
    assert_eq!(
        rational_lower.control_net()[0][1],
        rational_upper.control_net()[0][0]
    );
    assert_eq!(
        rational_lower.weights()[0][1],
        rational_upper.weights()[0][0]
    );
}

#[test]
fn split_pieces_evaluate_under_exact_affine_reparameterization() {
    macro_rules! assert_curve_pieces {
        ($builder:path) => {{
            let (left, right) = split_curve($builder(), fraction(1, 2));
            let expected_left = evaluate_bezier(BezierEvaluationRequest::Curve {
                representation: $builder(),
                parameter: fraction(1, 6),
            })
            .expect("original curve lower affine evaluation succeeds");
            let expected_right = evaluate_bezier(BezierEvaluationRequest::Curve {
                representation: $builder(),
                parameter: fraction(2, 3),
            })
            .expect("original curve upper affine evaluation succeeds");
            assert_eq!(
                evaluate_bezier(BezierEvaluationRequest::Curve {
                    representation: left,
                    parameter: fraction(1, 3),
                })
                .expect("left curve evaluation succeeds")
                .into_point(),
                expected_left.into_point()
            );
            assert_eq!(
                evaluate_bezier(BezierEvaluationRequest::Curve {
                    representation: right,
                    parameter: fraction(1, 3),
                })
                .expect("right curve evaluation succeeds")
                .into_point(),
                expected_right.into_point()
            );
        }};
    }

    macro_rules! assert_surface_u_pieces {
        ($builder:path) => {{
            let (lower, upper) = split_surface($builder(), BezierSplitAxis::U, fraction(1, 2));
            let expected_lower = evaluate_bezier(BezierEvaluationRequest::Surface {
                representation: $builder(),
                u: fraction(1, 6),
                v: fraction(1, 3),
            })
            .expect("original surface lower U affine evaluation succeeds");
            let expected_upper = evaluate_bezier(BezierEvaluationRequest::Surface {
                representation: $builder(),
                u: fraction(2, 3),
                v: fraction(1, 3),
            })
            .expect("original surface upper U affine evaluation succeeds");
            assert_eq!(
                evaluate_bezier(BezierEvaluationRequest::Surface {
                    representation: lower,
                    u: fraction(1, 3),
                    v: fraction(1, 3),
                })
                .expect("lower U surface evaluation succeeds")
                .into_point(),
                expected_lower.into_point()
            );
            assert_eq!(
                evaluate_bezier(BezierEvaluationRequest::Surface {
                    representation: upper,
                    u: fraction(1, 3),
                    v: fraction(1, 3),
                })
                .expect("upper U surface evaluation succeeds")
                .into_point(),
                expected_upper.into_point()
            );
        }};
    }

    macro_rules! assert_surface_v_pieces {
        ($builder:path) => {{
            let (lower, upper) = split_surface($builder(), BezierSplitAxis::V, fraction(1, 2));
            let expected_lower = evaluate_bezier(BezierEvaluationRequest::Surface {
                representation: $builder(),
                u: fraction(1, 3),
                v: fraction(1, 6),
            })
            .expect("original surface lower V affine evaluation succeeds");
            let expected_upper = evaluate_bezier(BezierEvaluationRequest::Surface {
                representation: $builder(),
                u: fraction(1, 3),
                v: fraction(2, 3),
            })
            .expect("original surface upper V affine evaluation succeeds");
            assert_eq!(
                evaluate_bezier(BezierEvaluationRequest::Surface {
                    representation: lower,
                    u: fraction(1, 3),
                    v: fraction(1, 3),
                })
                .expect("lower V surface evaluation succeeds")
                .into_point(),
                expected_lower.into_point()
            );
            assert_eq!(
                evaluate_bezier(BezierEvaluationRequest::Surface {
                    representation: upper,
                    u: fraction(1, 3),
                    v: fraction(1, 3),
                })
                .expect("upper V surface evaluation succeeds")
                .into_point(),
                expected_upper.into_point()
            );
        }};
    }

    assert_curve_pieces!(polynomial_curve);
    assert_curve_pieces!(rational_curve);
    assert_surface_u_pieces!(polynomial_surface);
    assert_surface_v_pieces!(polynomial_surface);
    assert_surface_u_pieces!(rational_surface);
    assert_surface_v_pieces!(rational_surface);
}

#[test]
fn split_preserves_point_dimension_degree_and_category() {
    let point3_curve = define_bezier(BezierDefinitionRequest::PolynomialCurve {
        control_points: vec![point3(0, 0, 0), point3(2, 4, 6)],
        degree: 1,
    })
    .expect("Point3 polynomial curve is valid");
    let (curve_left, curve_right) = split_curve(point3_curve, fraction(1, 2));
    for curve in [curve_left, curve_right] {
        let BezierTypes::PolynomialCurve(curve) = curve else {
            panic!("polynomial curve category is preserved");
        };
        assert_eq!(curve.degree(), 1);
        assert_eq!(curve.dimension(), 3);
        assert!(matches!(curve.control_points()[0], BezierPoint::Point3(_)));
    }

    let (curve_left, curve_right) = split_curve(rational_curve(), fraction(1, 2));
    for curve in [curve_left, curve_right] {
        let BezierTypes::RationalCurve(curve) = curve else {
            panic!("rational curve category is preserved");
        };
        assert_eq!(curve.degree(), 1);
        assert_eq!(curve.dimension(), 2);
        assert!(matches!(curve.control_points()[0], BezierPoint::Point2(_)));
    }

    let (surface_lower, surface_upper) =
        split_surface(polynomial_surface(), BezierSplitAxis::V, fraction(1, 2));
    for surface in [surface_lower, surface_upper] {
        let BezierTypes::PolynomialSurface(surface) = surface else {
            panic!("polynomial surface category is preserved");
        };
        assert_eq!(surface.u_degree(), 1);
        assert_eq!(surface.v_degree(), 2);
        assert_eq!(surface.dimension(), 2);
    }

    let (surface_lower, surface_upper) =
        split_surface(rational_surface(), BezierSplitAxis::U, fraction(1, 2));
    for surface in [surface_lower, surface_upper] {
        let BezierTypes::RationalSurface(surface) = surface else {
            panic!("rational surface category is preserved");
        };
        assert_eq!(surface.u_degree(), 1);
        assert_eq!(surface.v_degree(), 1);
        assert_eq!(surface.dimension(), 3);
    }
}

#[test]
fn request_shape_mismatch_returns_explicit_failure() {
    assert!(matches!(
        split_bezier(BezierSplitRequest::Curve {
            representation: polynomial_surface(),
            parameter: algebraic(0),
        }),
        Err(CadFailure::RequestShapeMismatch)
    ));
    assert!(matches!(
        split_bezier(BezierSplitRequest::Surface {
            representation: polynomial_curve(),
            axis: BezierSplitAxis::U,
            parameter: algebraic(0),
        }),
        Err(CadFailure::RequestShapeMismatch)
    ));
}

#[test]
fn parameters_below_and_above_unit_interval_fail() {
    for request in [
        BezierSplitRequest::Curve {
            representation: polynomial_curve(),
            parameter: fraction(-1, 2),
        },
        BezierSplitRequest::Curve {
            representation: polynomial_curve(),
            parameter: algebraic(2),
        },
        BezierSplitRequest::Surface {
            representation: polynomial_surface(),
            axis: BezierSplitAxis::U,
            parameter: fraction(-1, 2),
        },
        BezierSplitRequest::Surface {
            representation: polynomial_surface(),
            axis: BezierSplitAxis::V,
            parameter: algebraic(2),
        },
    ] {
        assert!(matches!(
            split_bezier(request),
            Err(CadFailure::ParameterOutOfRange)
        ));
    }
}
