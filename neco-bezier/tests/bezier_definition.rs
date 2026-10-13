use neco_algnum::RealAlgebraic;
use neco_bezier::{define_bezier, BezierDefinitionRequest, BezierPoint, BezierTypes, CadFailure};
use neco_bigint::{BigInt, Sign};
use neco_geometry::{Point2, Point3};

fn algebraic(value: i32) -> RealAlgebraic {
    RealAlgebraic::from_integer(BigInt::try_from(value).unwrap()).unwrap()
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

#[test]
fn valid_polynomial_and_rational_curves() {
    let polynomial = define_bezier(BezierDefinitionRequest::PolynomialCurve {
        control_points: vec![point2(0, 0), point2(1, 2), point2(2, 0)],
        degree: 2,
    })
    .unwrap();
    let BezierTypes::PolynomialCurve(polynomial) = polynomial else {
        panic!("expected a polynomial curve");
    };
    assert_eq!(polynomial.control_points().len(), 3);
    assert_eq!(polynomial.degree(), 2);
    assert_eq!(polynomial.dimension(), 2);

    let rational = define_bezier(BezierDefinitionRequest::RationalCurve {
        control_points: vec![point3(0, 0, 0), point3(1, 2, 3), point3(2, 0, 1)],
        weights: vec![algebraic(1), algebraic(2), algebraic(3)],
        degree: 2,
    })
    .unwrap();
    let BezierTypes::RationalCurve(rational) = rational else {
        panic!("expected a rational curve");
    };
    assert_eq!(rational.control_points().len(), 3);
    assert_eq!(rational.weights().len(), 3);
    assert_eq!(rational.degree(), 2);
    assert_eq!(rational.dimension(), 3);
}

#[test]
fn valid_polynomial_and_rational_surfaces() {
    let polynomial = define_bezier(BezierDefinitionRequest::PolynomialSurface {
        control_net: vec![
            vec![point2(0, 0), point2(0, 1), point2(0, 2)],
            vec![point2(1, 0), point2(1, 1), point2(1, 2)],
        ],
        u_degree: 1,
        v_degree: 2,
    })
    .unwrap();
    let BezierTypes::PolynomialSurface(polynomial) = polynomial else {
        panic!("expected a polynomial surface");
    };
    assert_eq!(polynomial.control_net().len(), 2);
    assert_eq!(polynomial.control_net()[0].len(), 3);
    assert_eq!(polynomial.u_degree(), 1);
    assert_eq!(polynomial.v_degree(), 2);
    assert_eq!(polynomial.dimension(), 2);

    let rational = define_bezier(BezierDefinitionRequest::RationalSurface {
        control_net: vec![
            vec![point3(0, 0, 0), point3(0, 1, 0)],
            vec![point3(1, 0, 1), point3(1, 1, 1)],
        ],
        weights: vec![
            vec![algebraic(1), algebraic(2)],
            vec![algebraic(3), algebraic(4)],
        ],
        u_degree: 1,
        v_degree: 1,
    })
    .unwrap();
    let BezierTypes::RationalSurface(rational) = rational else {
        panic!("expected a rational surface");
    };
    assert_eq!(rational.control_net().len(), 2);
    assert_eq!(rational.control_net()[0].len(), 2);
    assert_eq!(rational.weights().len(), 2);
    assert_eq!(rational.weights()[0].len(), 2);
    assert_eq!(rational.u_degree(), 1);
    assert_eq!(rational.v_degree(), 1);
    assert_eq!(rational.dimension(), 3);
}

#[test]
fn invalid_definition_input_returns_explicit_failures() {
    assert_eq!(
        define_bezier(BezierDefinitionRequest::PolynomialCurve {
            control_points: vec![],
            degree: 0,
        }),
        Err(CadFailure::EmptyControlPoints)
    );
    assert_eq!(
        define_bezier(BezierDefinitionRequest::PolynomialCurve {
            control_points: vec![point2(0, 0), point3(1, 1, 1)],
            degree: 1,
        }),
        Err(CadFailure::ControlPointDimensionMismatch)
    );
    assert_eq!(
        define_bezier(BezierDefinitionRequest::PolynomialCurve {
            control_points: vec![point2(0, 0), point2(1, 1)],
            degree: 0,
        }),
        Err(CadFailure::DegreeMismatch)
    );
    assert_eq!(
        define_bezier(BezierDefinitionRequest::RationalCurve {
            control_points: vec![point2(0, 0), point2(1, 1)],
            weights: vec![algebraic(1)],
            degree: 1,
        }),
        Err(CadFailure::WeightCardinalityMismatch)
    );
    assert_eq!(
        define_bezier(BezierDefinitionRequest::PolynomialSurface {
            control_net: vec![],
            u_degree: 0,
            v_degree: 0,
        }),
        Err(CadFailure::EmptySurfaceRows)
    );
    assert_eq!(
        define_bezier(BezierDefinitionRequest::PolynomialSurface {
            control_net: vec![vec![point2(0, 0)], vec![point2(1, 0), point2(1, 1)]],
            u_degree: 1,
            v_degree: 0,
        }),
        Err(CadFailure::SurfaceRowCardinalityMismatch)
    );
    assert_eq!(
        define_bezier(BezierDefinitionRequest::PolynomialSurface {
            control_net: vec![vec![point2(0, 0), point2(0, 1)]],
            u_degree: 1,
            v_degree: 1,
        }),
        Err(CadFailure::SurfaceDegreeMismatch)
    );
}

#[test]
fn rational_weight_construction_observes_sign_success_and_nonpositive_weight() {
    assert_eq!(algebraic(1).sign(), Ok(Sign::Positive));
    assert_eq!(algebraic(0).sign(), Ok(Sign::Zero));
    assert_eq!(algebraic(-1).sign(), Ok(Sign::Negative));

    assert_eq!(
        define_bezier(BezierDefinitionRequest::RationalCurve {
            control_points: vec![point2(0, 0), point2(1, 1)],
            weights: vec![algebraic(1), algebraic(0)],
            degree: 1,
        }),
        Err(CadFailure::NonPositiveWeight)
    );
    assert_eq!(
        define_bezier(BezierDefinitionRequest::RationalCurve {
            control_points: vec![point2(0, 0), point2(1, 1)],
            weights: vec![algebraic(1), algebraic(-1)],
            degree: 1,
        }),
        Err(CadFailure::NonPositiveWeight)
    );
}
