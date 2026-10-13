use neco_algnum::RealAlgebraic;
use neco_bigint::BigInt;
use neco_geometry::{Point2, Point3};
use neco_nurbs::{define_nurbs, BezierPoint, CadFailure, NurbsDefinitionRequest, NurbsTypes};

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

fn curve_knots() -> Vec<RealAlgebraic> {
    vec![algebraic(0), algebraic(0), algebraic(1), algebraic(1)]
}

fn surface_net() -> Vec<Vec<BezierPoint>> {
    vec![
        vec![point3(0, 0, 0), point3(0, 1, 0)],
        vec![point3(1, 0, 1), point3(1, 1, 1)],
    ]
}

fn surface_knots() -> Vec<RealAlgebraic> {
    curve_knots()
}

#[test]
fn exact_curve_and_surface_values_are_preserved() {
    let curve = define_nurbs(NurbsDefinitionRequest::PolynomialCurve {
        control_points: vec![point2(0, 0), point2(1, 2)],
        knots: curve_knots(),
        degree: 1,
    })
    .unwrap();
    let curve_points = vec![point2(0, 0), point2(1, 2)];
    let knots = curve_knots();
    let NurbsTypes::PolynomialCurve(curve) = curve else {
        panic!("expected a polynomial curve");
    };
    assert_eq!(curve.control_points(), curve_points.as_slice());
    assert_eq!(curve.knots(), knots.as_slice());
    assert_eq!(curve.degree(), 1);
    assert_eq!(curve.dimension(), 2);

    let surface = define_nurbs(NurbsDefinitionRequest::RationalSurface {
        control_net: surface_net(),
        weights: vec![
            vec![algebraic(1), algebraic(2)],
            vec![algebraic(3), algebraic(4)],
        ],
        u_knots: surface_knots(),
        v_knots: surface_knots(),
        u_degree: 1,
        v_degree: 1,
    })
    .unwrap();
    let control_net = surface_net();
    let weights = vec![
        vec![algebraic(1), algebraic(2)],
        vec![algebraic(3), algebraic(4)],
    ];
    let u_knots = surface_knots();
    let v_knots = surface_knots();
    let NurbsTypes::RationalSurface(surface) = surface else {
        panic!("expected a rational surface");
    };
    assert_eq!(surface.control_net(), control_net.as_slice());
    assert_eq!(surface.u_knots(), u_knots.as_slice());
    assert_eq!(surface.v_knots(), v_knots.as_slice());
    assert_eq!(surface.weights(), weights.as_slice());
    assert_eq!(surface.u_degree(), 1);
    assert_eq!(surface.v_degree(), 1);
    assert_eq!(surface.dimension(), 3);
}

#[test]
fn clamped_knot_validation() {
    assert!(define_nurbs(NurbsDefinitionRequest::PolynomialCurve {
        control_points: vec![point2(0, 0), point2(1, 2)],
        knots: curve_knots(),
        degree: 1,
    })
    .is_ok());
    assert!(define_nurbs(NurbsDefinitionRequest::PolynomialSurface {
        control_net: vec![
            vec![point2(0, 0), point2(0, 1)],
            vec![point2(1, 0), point2(1, 1)]
        ],
        u_knots: surface_knots(),
        v_knots: surface_knots(),
        u_degree: 1,
        v_degree: 1,
    })
    .is_ok());

    assert_eq!(
        define_nurbs(NurbsDefinitionRequest::PolynomialCurve {
            control_points: vec![point2(0, 0), point2(1, 2)],
            knots: vec![
                algebraic(0),
                algebraic(0),
                algebraic(0),
                algebraic(1),
                algebraic(1)
            ],
            degree: 2,
        }),
        Err(CadFailure::DegreeMismatch)
    );
    assert_eq!(
        define_nurbs(NurbsDefinitionRequest::PolynomialCurve {
            control_points: vec![point2(0, 0), point2(1, 2)],
            knots: vec![algebraic(0), algebraic(0), algebraic(1)],
            degree: 1,
        }),
        Err(CadFailure::KnotCardinalityMismatch)
    );
    assert_eq!(
        define_nurbs(NurbsDefinitionRequest::PolynomialCurve {
            control_points: vec![point2(0, 0), point2(1, 2)],
            knots: vec![algebraic(0), algebraic(1), algebraic(0), algebraic(1)],
            degree: 1,
        }),
        Err(CadFailure::NonMonotonicKnots)
    );

    assert_eq!(
        define_nurbs(NurbsDefinitionRequest::PolynomialSurface {
            control_net: vec![
                vec![point2(0, 0), point2(0, 1)],
                vec![point2(1, 0), point2(1, 1)]
            ],
            u_knots: vec![
                algebraic(0),
                algebraic(0),
                algebraic(0),
                algebraic(1),
                algebraic(1)
            ],
            v_knots: surface_knots(),
            u_degree: 2,
            v_degree: 1,
        }),
        Err(CadFailure::SurfaceDegreeMismatch)
    );
    assert_eq!(
        define_nurbs(NurbsDefinitionRequest::PolynomialSurface {
            control_net: vec![
                vec![point2(0, 0), point2(0, 1)],
                vec![point2(1, 0), point2(1, 1)]
            ],
            u_knots: surface_knots(),
            v_knots: vec![algebraic(0), algebraic(0), algebraic(1)],
            u_degree: 1,
            v_degree: 1,
        }),
        Err(CadFailure::KnotCardinalityMismatch)
    );
    assert_eq!(
        define_nurbs(NurbsDefinitionRequest::PolynomialSurface {
            control_net: vec![
                vec![point2(0, 0), point2(0, 1)],
                vec![point2(1, 0), point2(1, 1)]
            ],
            u_knots: vec![algebraic(0), algebraic(1), algebraic(0), algebraic(1)],
            v_knots: surface_knots(),
            u_degree: 1,
            v_degree: 1,
        }),
        Err(CadFailure::NonMonotonicKnots)
    );
    assert_eq!(
        define_nurbs(NurbsDefinitionRequest::PolynomialSurface {
            control_net: vec![
                vec![point2(0, 0), point2(0, 1)],
                vec![point2(1, 0), point2(1, 1)]
            ],
            u_knots: surface_knots(),
            v_knots: vec![algebraic(0), algebraic(1), algebraic(0), algebraic(1)],
            u_degree: 1,
            v_degree: 1,
        }),
        Err(CadFailure::NonMonotonicKnots)
    );
}

#[test]
fn rational_weights_are_positive_and_shape_preserving() {
    assert!(define_nurbs(NurbsDefinitionRequest::RationalCurve {
        control_points: vec![point2(0, 0), point2(1, 2)],
        weights: vec![algebraic(1), algebraic(2)],
        knots: curve_knots(),
        degree: 1,
    })
    .is_ok());
    assert_eq!(
        define_nurbs(NurbsDefinitionRequest::RationalCurve {
            control_points: vec![point2(0, 0), point2(1, 2)],
            weights: vec![algebraic(1)],
            knots: curve_knots(),
            degree: 1,
        }),
        Err(CadFailure::WeightCardinalityMismatch)
    );
    for weight in [0, -1] {
        assert_eq!(
            define_nurbs(NurbsDefinitionRequest::RationalCurve {
                control_points: vec![point2(0, 0), point2(1, 2)],
                weights: vec![algebraic(1), algebraic(weight)],
                knots: curve_knots(),
                degree: 1,
            }),
            Err(CadFailure::NonPositiveWeight)
        );
    }

    assert!(define_nurbs(NurbsDefinitionRequest::RationalSurface {
        control_net: surface_net(),
        weights: vec![
            vec![algebraic(1), algebraic(2)],
            vec![algebraic(3), algebraic(4)]
        ],
        u_knots: surface_knots(),
        v_knots: surface_knots(),
        u_degree: 1,
        v_degree: 1,
    })
    .is_ok());
    assert_eq!(
        define_nurbs(NurbsDefinitionRequest::RationalSurface {
            control_net: surface_net(),
            weights: vec![vec![algebraic(1)], vec![algebraic(2), algebraic(3)]],
            u_knots: surface_knots(),
            v_knots: surface_knots(),
            u_degree: 1,
            v_degree: 1,
        }),
        Err(CadFailure::WeightCardinalityMismatch)
    );
    for weight in [0, -1] {
        assert_eq!(
            define_nurbs(NurbsDefinitionRequest::RationalSurface {
                control_net: surface_net(),
                weights: vec![
                    vec![algebraic(1), algebraic(2)],
                    vec![algebraic(3), algebraic(weight)]
                ],
                u_knots: surface_knots(),
                v_knots: surface_knots(),
                u_degree: 1,
                v_degree: 1,
            }),
            Err(CadFailure::NonPositiveWeight)
        );
    }
    assert_eq!(
        define_nurbs(NurbsDefinitionRequest::RationalSurface {
            control_net: vec![
                vec![point3(0, 0, 0), point3(0, 1, 0)],
                vec![point3(1, 0, 1)]
            ],
            weights: vec![vec![algebraic(1), algebraic(2)], vec![algebraic(3)]],
            u_knots: surface_knots(),
            v_knots: surface_knots(),
            u_degree: 1,
            v_degree: 1,
        }),
        Err(CadFailure::SurfaceRowCardinalityMismatch)
    );
    assert_eq!(
        define_nurbs(NurbsDefinitionRequest::RationalSurface {
            control_net: vec![
                vec![point2(0, 0), point3(0, 1, 0)],
                vec![point2(1, 0), point2(1, 1)]
            ],
            weights: vec![
                vec![algebraic(1), algebraic(2)],
                vec![algebraic(3), algebraic(4)]
            ],
            u_knots: surface_knots(),
            v_knots: surface_knots(),
            u_degree: 1,
            v_degree: 1,
        }),
        Err(CadFailure::ControlPointDimensionMismatch)
    );
}
