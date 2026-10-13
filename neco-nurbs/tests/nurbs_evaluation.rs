use neco_algnum::RealAlgebraic;
use neco_bigint::BigInt;
use neco_geometry::{Point2, Point3};
use neco_nurbs::{
    define_nurbs, evaluate_nurbs, BezierPoint, CadFailure, NurbsDefinitionRequest,
    NurbsEvaluationRequest,
};

fn algebraic(value: i32) -> RealAlgebraic {
    RealAlgebraic::from_integer(BigInt::try_from(value).unwrap()).unwrap()
}

fn fraction(numerator: i32, denominator: i32) -> RealAlgebraic {
    algebraic(numerator).div(&algebraic(denominator)).unwrap()
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

fn surface_knots() -> Vec<RealAlgebraic> {
    curve_knots()
}

fn evaluate_curve(
    representation: neco_nurbs::NurbsTypes,
    parameter: RealAlgebraic,
) -> Result<BezierPoint, CadFailure> {
    evaluate_nurbs(NurbsEvaluationRequest::Curve {
        representation,
        parameter,
    })
    .map(|result| result.into_point())
}

fn evaluate_surface(
    representation: neco_nurbs::NurbsTypes,
    u: i32,
    v: i32,
) -> Result<BezierPoint, CadFailure> {
    evaluate_nurbs(NurbsEvaluationRequest::Surface {
        representation,
        u: algebraic(u),
        v: algebraic(v),
    })
    .map(|result| result.into_point())
}

#[test]
fn evaluates_polynomial_and_rational_curves_and_surfaces_exactly() {
    let polynomial_curve = define_nurbs(NurbsDefinitionRequest::PolynomialCurve {
        control_points: vec![point2(0, 0), point2(2, 4)],
        knots: curve_knots(),
        degree: 1,
    })
    .unwrap();
    assert_eq!(
        evaluate_curve(polynomial_curve, algebraic(1)),
        Ok(point2(2, 4))
    );

    let rational_curve = define_nurbs(NurbsDefinitionRequest::RationalCurve {
        control_points: vec![point2(0, 0), point2(2, 4)],
        weights: vec![algebraic(1), algebraic(3)],
        knots: curve_knots(),
        degree: 1,
    })
    .unwrap();
    assert_eq!(
        evaluate_curve(rational_curve, algebraic(1)),
        Ok(point2(2, 4))
    );

    let polynomial_surface = define_nurbs(NurbsDefinitionRequest::PolynomialSurface {
        control_net: vec![
            vec![point3(0, 0, 0), point3(0, 2, 0)],
            vec![point3(2, 0, 2), point3(2, 2, 2)],
        ],
        u_knots: surface_knots(),
        v_knots: surface_knots(),
        u_degree: 1,
        v_degree: 1,
    })
    .unwrap();
    assert_eq!(
        evaluate_surface(polynomial_surface, 1, 1),
        Ok(point3(2, 2, 2))
    );

    let rational_surface = define_nurbs(NurbsDefinitionRequest::RationalSurface {
        control_net: vec![
            vec![point3(0, 0, 0), point3(0, 2, 0)],
            vec![point3(2, 0, 2), point3(2, 2, 2)],
        ],
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
    assert_eq!(
        evaluate_surface(rational_surface, 1, 1),
        Ok(point3(2, 2, 2))
    );
}

#[test]
fn evaluates_a_degree_two_nonuniform_knot_curve_by_de_boor_recurrence() {
    let representation = define_nurbs(NurbsDefinitionRequest::PolynomialCurve {
        control_points: vec![point2(0, 0), point2(2, 0), point2(2, 2), point2(4, 2)],
        knots: vec![
            algebraic(0),
            algebraic(0),
            algebraic(0),
            algebraic(1),
            algebraic(3),
            algebraic(3),
            algebraic(3),
        ],
        degree: 2,
    })
    .unwrap();

    assert_eq!(
        evaluate_curve(representation, fraction(3, 2)),
        Ok(BezierPoint::Point2(Point2 {
            x: fraction(17, 8),
            y: fraction(5, 4),
        }))
    );
}

#[test]
fn rational_evaluation_dehomogenizes_weighted_coordinates_exactly() {
    let representation = define_nurbs(NurbsDefinitionRequest::RationalCurve {
        control_points: vec![point2(0, 0), point2(2, 4)],
        weights: vec![algebraic(1), algebraic(3)],
        knots: curve_knots(),
        degree: 1,
    })
    .unwrap();

    assert_eq!(
        evaluate_curve(representation, fraction(1, 2)),
        Ok(BezierPoint::Point2(Point2 {
            x: fraction(3, 2),
            y: algebraic(3),
        }))
    );
}

#[test]
fn evaluates_the_right_endpoint_of_the_final_nonempty_span() {
    let representation = define_nurbs(NurbsDefinitionRequest::PolynomialCurve {
        control_points: vec![point2(0, 0), point2(2, 0), point2(2, 2), point2(4, 2)],
        knots: vec![
            algebraic(0),
            algebraic(0),
            algebraic(0),
            algebraic(1),
            algebraic(3),
            algebraic(3),
            algebraic(3),
        ],
        degree: 2,
    })
    .unwrap();

    assert_eq!(
        evaluate_curve(representation, algebraic(3)),
        Ok(point2(4, 2))
    );
}

#[test]
fn rejects_curve_and_surface_request_shape_mismatches() {
    let surface = define_nurbs(NurbsDefinitionRequest::PolynomialSurface {
        control_net: vec![
            vec![point2(0, 0), point2(0, 1)],
            vec![point2(1, 0), point2(1, 1)],
        ],
        u_knots: surface_knots(),
        v_knots: surface_knots(),
        u_degree: 1,
        v_degree: 1,
    })
    .unwrap();
    assert_eq!(
        evaluate_curve(surface, algebraic(0)),
        Err(CadFailure::RequestShapeMismatch)
    );

    let curve = define_nurbs(NurbsDefinitionRequest::PolynomialCurve {
        control_points: vec![point2(0, 0), point2(1, 1)],
        knots: curve_knots(),
        degree: 1,
    })
    .unwrap();
    assert_eq!(
        evaluate_surface(curve, 0, 0),
        Err(CadFailure::RequestShapeMismatch)
    );
}

#[test]
fn rejects_parameters_outside_a_nonempty_domain() {
    let representation = define_nurbs(NurbsDefinitionRequest::PolynomialCurve {
        control_points: vec![point2(0, 0), point2(1, 1)],
        knots: curve_knots(),
        degree: 1,
    })
    .unwrap();

    assert_eq!(
        evaluate_curve(representation, algebraic(-1)),
        Err(CadFailure::ParameterOutOfRange)
    );
}

#[test]
fn rejects_empty_curve_and_surface_parameter_domains() {
    let curve = define_nurbs(NurbsDefinitionRequest::PolynomialCurve {
        control_points: vec![point2(0, 0), point2(1, 1)],
        knots: vec![algebraic(0), algebraic(0), algebraic(0), algebraic(0)],
        degree: 1,
    })
    .unwrap();
    assert_eq!(
        evaluate_curve(curve, algebraic(0)),
        Err(CadFailure::EmptyParameterDomain)
    );

    let empty_u_surface = define_nurbs(NurbsDefinitionRequest::PolynomialSurface {
        control_net: vec![
            vec![point2(0, 0), point2(0, 1)],
            vec![point2(1, 0), point2(1, 1)],
        ],
        u_knots: vec![algebraic(0), algebraic(0), algebraic(0), algebraic(0)],
        v_knots: surface_knots(),
        u_degree: 1,
        v_degree: 1,
    })
    .unwrap();
    assert_eq!(
        evaluate_surface(empty_u_surface, 0, 0),
        Err(CadFailure::EmptyParameterDomain)
    );

    let empty_v_surface = define_nurbs(NurbsDefinitionRequest::PolynomialSurface {
        control_net: vec![
            vec![point2(0, 0), point2(0, 1)],
            vec![point2(1, 0), point2(1, 1)],
        ],
        u_knots: surface_knots(),
        v_knots: vec![algebraic(0), algebraic(0), algebraic(0), algebraic(0)],
        u_degree: 1,
        v_degree: 1,
    })
    .unwrap();
    assert_eq!(
        evaluate_surface(empty_v_surface, 0, 0),
        Err(CadFailure::EmptyParameterDomain)
    );
}
