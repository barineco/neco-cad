use neco_algnum::RealAlgebraic;
use neco_bigint::BigInt;
use neco_geometry::Point2;
use neco_nurbs::{
    define_nurbs, evaluate_nurbs, fit_nurbs_curve, BezierPoint, CadFailure, EvaluationResult,
    FittingRequest, FittingResult, NurbsDefinitionRequest, NurbsEvaluationRequest,
    NurbsFittingFailure, NurbsTypes, PolynomialNurbsCurve, PolynomialNurbsSurface,
    RationalNurbsCurve, RationalNurbsSurface,
};

fn algebraic(value: i32) -> RealAlgebraic {
    RealAlgebraic::from_integer(BigInt::try_from(value).unwrap()).unwrap()
}

fn point2(x: i32, y: i32) -> BezierPoint {
    BezierPoint::Point2(Point2 {
        x: algebraic(x),
        y: algebraic(y),
    })
}

#[test]
fn imports_and_defines_a_polynomial_curve_as_an_external_crate() {
    let define: fn(NurbsDefinitionRequest) -> Result<NurbsTypes, CadFailure> = define_nurbs;
    let request = NurbsDefinitionRequest::PolynomialCurve {
        control_points: vec![point2(0, 0), point2(1, 1)],
        knots: vec![algebraic(0), algebraic(0), algebraic(1), algebraic(1)],
        degree: 1,
    };

    let result = define(request).unwrap();
    let NurbsTypes::PolynomialCurve(_) = result else {
        panic!("expected a polynomial curve");
    };
}

#[test]
fn imports_and_evaluates_a_polynomial_curve_as_an_external_crate() {
    let evaluate: fn(NurbsEvaluationRequest) -> Result<EvaluationResult, CadFailure> =
        evaluate_nurbs;
    let representation = define_nurbs(NurbsDefinitionRequest::PolynomialCurve {
        control_points: vec![point2(0, 0), point2(2, 2)],
        knots: vec![algebraic(0), algebraic(0), algebraic(1), algebraic(1)],
        degree: 1,
    })
    .expect("valid polynomial curve");

    let result = evaluate(NurbsEvaluationRequest::Curve {
        representation,
        parameter: algebraic(0),
    })
    .expect("crate-root evaluation returns a result");

    assert_eq!(result.into_point(), point2(0, 0));
}

#[test]
fn imports_and_fits_a_polynomial_curve_as_an_external_crate() {
    let fit: fn(FittingRequest) -> Result<FittingResult, NurbsFittingFailure> = fit_nurbs_curve;
    let result = fit(FittingRequest {
        parameters: vec![algebraic(0), algebraic(1)],
        samples: vec![point2(0, 1), point2(2, 3)],
        knots: vec![algebraic(0), algebraic(0), algebraic(1), algebraic(1)],
        degree: 1,
        control_point_count: 2,
    })
    .expect("crate-root fitting returns a curve");

    assert_eq!(
        result.curve().control_points(),
        [point2(0, 1), point2(2, 3)]
    );
}

const _: fn() = || {
    let _ = core::marker::PhantomData::<PolynomialNurbsCurve>;
    let _ = core::marker::PhantomData::<RationalNurbsCurve>;
    let _ = core::marker::PhantomData::<PolynomialNurbsSurface>;
    let _ = core::marker::PhantomData::<RationalNurbsSurface>;
};
