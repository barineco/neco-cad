use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use neco_algnum::{AlgnumError, RealAlgebraic};
use neco_bezier::{
    aggregate_bezier, cad_failure_api, contain_bezier, define_bezier, evaluate_bezier,
    split_bezier, BezierAggregationInput, BezierDefinitionRequest, BezierEvaluationRequest,
    BezierPoint, BezierSplitAxis, BezierSplitRequest, BezierSplitResult, BezierTypes, CadFailure,
    CadFailureApi, ContainmentGeometricShape, ContainmentParameterDomain,
    ContainmentPointDimension, ContainmentRepresentationCategory, ContainmentResult,
    ContainmentWeightEvidence, ContainmentWitness, ConvexHull, CoordinateInterval,
    EvaluationResult, FailureApiRequest, Point2ConvexHull, Point3ConvexHull,
};
use neco_bigint::BigInt;
use neco_geometry::Point2;

std::thread_local! {
    static FAIL_NEXT_ALLOCATION: Cell<bool> = const { Cell::new(false) };
}

struct FailNextAllocator;

unsafe impl GlobalAlloc for FailNextAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if FAIL_NEXT_ALLOCATION.with(|flag| flag.replace(false)) {
            core::ptr::null_mut()
        } else {
            unsafe { System.alloc(layout) }
        }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        if FAIL_NEXT_ALLOCATION.with(|flag| flag.replace(false)) {
            core::ptr::null_mut()
        } else {
            unsafe { System.alloc_zeroed(layout) }
        }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        if FAIL_NEXT_ALLOCATION.with(|flag| flag.replace(false)) {
            core::ptr::null_mut()
        } else {
            unsafe { System.realloc(pointer, layout, size) }
        }
    }
}

#[global_allocator]
static TEST_ALLOCATOR: FailNextAllocator = FailNextAllocator;

fn fail_next_allocation() {
    FAIL_NEXT_ALLOCATION.with(|flag| flag.set(true));
}

fn allocation_failure_is_consumed() -> bool {
    FAIL_NEXT_ALLOCATION.with(|flag| !flag.get())
}

fn integer(value: i32) -> RealAlgebraic {
    RealAlgebraic::from_integer(BigInt::try_from(value).expect("i32 converts to BigInt"))
        .expect("integer is algebraic")
}

fn point2(x: i32, y: i32) -> BezierPoint {
    BezierPoint::Point2(Point2 {
        x: integer(x),
        y: integer(y),
    })
}

#[test]
fn imports_definition_api_from_the_external_crate() {
    let _: Option<BezierDefinitionRequest> = None;
    let _: Option<BezierTypes> = None;
    let _: Option<CadFailure> = None;
    let _: Option<BezierPoint> = None;
    let _: fn(BezierDefinitionRequest) -> Result<BezierTypes, CadFailure> = define_bezier;
}

#[test]
fn imports_aggregate_api_from_the_external_crate() {
    let _: Option<BezierAggregationInput> = None;
    let _: fn(BezierAggregationInput) -> Result<BezierTypes, CadFailure> = aggregate_bezier;
}

#[test]
fn public_aggregate_operation_reaches_result_from_upstream_products() {
    let definition = define_bezier(BezierDefinitionRequest::PolynomialCurve {
        control_points: vec![point2(0, 0), point2(2, 2)],
        degree: 1,
    })
    .expect("valid aggregate definition");
    let evaluation_representation = define_bezier(BezierDefinitionRequest::PolynomialCurve {
        control_points: vec![point2(0, 0), point2(2, 2)],
        degree: 1,
    })
    .expect("valid aggregate evaluation representation");
    let split_representation = define_bezier(BezierDefinitionRequest::PolynomialCurve {
        control_points: vec![point2(0, 0), point2(2, 2)],
        degree: 1,
    })
    .expect("valid aggregate split representation");
    let containment_representation = define_bezier(BezierDefinitionRequest::PolynomialCurve {
        control_points: vec![point2(0, 0), point2(2, 2)],
        degree: 1,
    })
    .expect("valid aggregate containment representation");

    let evaluation = evaluate_bezier(BezierEvaluationRequest::Curve {
        representation: evaluation_representation,
        parameter: integer(0),
    })
    .expect("valid aggregate evaluation");
    let split = split_bezier(BezierSplitRequest::Curve {
        representation: split_representation,
        parameter: integer(0),
    })
    .expect("valid aggregate split");
    let containment =
        contain_bezier(containment_representation).expect("valid aggregate containment");

    let result = aggregate_bezier(BezierAggregationInput::new(
        definition,
        evaluation,
        split,
        containment,
    ))
    .expect("crate-root aggregate returns a result");

    assert!(matches!(result, BezierTypes::PolynomialCurve(_)));
}

#[test]
fn imports_containment_api_from_the_external_crate() {
    let _: Option<CoordinateInterval> = None;
    let _: Option<Point2ConvexHull> = None;
    let _: Option<Point3ConvexHull> = None;
    let _: Option<ConvexHull> = None;
    let _: Option<ContainmentRepresentationCategory> = None;
    let _: Option<ContainmentGeometricShape> = None;
    let _: Option<ContainmentPointDimension> = None;
    let _: Option<ContainmentWeightEvidence> = None;
    let _: Option<ContainmentParameterDomain> = None;
    let _: Option<ContainmentWitness> = None;
    let _: Option<ContainmentResult> = None;
    let _: fn(BezierTypes) -> Result<ContainmentResult, CadFailure> = contain_bezier;
}

#[test]
fn public_containment_operation_reaches_result() {
    let representation = define_bezier(BezierDefinitionRequest::PolynomialCurve {
        control_points: vec![point2(0, 0), point2(2, 3)],
        degree: 1,
    })
    .expect("valid polynomial curve");

    let result = contain_bezier(representation).expect("crate-root containment returns a result");
    assert!(matches!(result.hull(), ConvexHull::Point2(_)));
}

#[test]
fn public_containment_operation_repeats_successfully() {
    let first = define_bezier(BezierDefinitionRequest::PolynomialCurve {
        control_points: vec![point2(-1, 2), point2(4, 5)],
        degree: 1,
    })
    .expect("valid first polynomial curve");
    let second = define_bezier(BezierDefinitionRequest::PolynomialCurve {
        control_points: vec![point2(-1, 2), point2(4, 5)],
        degree: 1,
    })
    .expect("valid second polynomial curve");

    let first_result = contain_bezier(first).expect("first containment succeeds");
    let second_result = contain_bezier(second).expect("repeated containment succeeds");
    assert!(matches!(first_result.hull(), ConvexHull::Point2(_)));
    assert!(matches!(second_result.hull(), ConvexHull::Point2(_)));
}

#[test]
fn containment_algebraic_failure_preserves_first_clone_payload() {
    let representation = define_bezier(BezierDefinitionRequest::PolynomialCurve {
        control_points: vec![point2(0, 0), point2(1, 1)],
        degree: 1,
    })
    .expect("valid polynomial curve");

    fail_next_allocation();
    let direct_error = match &representation {
        BezierTypes::PolynomialCurve(curve) => match &curve.control_points()[0] {
            BezierPoint::Point2(point) => point
                .x
                .try_clone()
                .expect_err("direct first coordinate clone observes allocation failure"),
            _ => panic!("control point has the expected two-dimensional variant"),
        },
        _ => panic!("representation has the expected polynomial curve variant"),
    };
    assert!(allocation_failure_is_consumed());

    fail_next_allocation();
    let containment_error = match contain_bezier(representation) {
        Err(CadFailure::AlgebraicFailure(error)) => error,
        _ => panic!("containment propagates the first clone failure"),
    };
    assert!(allocation_failure_is_consumed());
    assert_eq!(containment_error, direct_error);
}

#[test]
fn cad_failure_api_returns_zero_sized_success_type() {
    let operation: fn(FailureApiRequest) -> Result<CadFailureApi, CadFailure> = cad_failure_api;
    assert_eq!(operation(FailureApiRequest), Ok(CadFailureApi));
}

#[test]
fn algebraic_failure_from_preserves_algnum_error() {
    let error = AlgnumError::DivisionByZero;

    assert_eq!(
        CadFailure::from(error),
        CadFailure::AlgebraicFailure(AlgnumError::DivisionByZero)
    );
}

#[test]
fn public_definition_operation_returns_a_polynomial_curve() {
    let request = BezierDefinitionRequest::PolynomialCurve {
        control_points: vec![point2(0, 0), point2(1, 1)],
        degree: 1,
    };

    assert!(matches!(
        define_bezier(request),
        Ok(BezierTypes::PolynomialCurve(_))
    ));
}

#[test]
fn rational_weight_sign_failure_preserves_algnum_error() {
    let direct_weight = integer(1);
    let request = BezierDefinitionRequest::RationalCurve {
        control_points: vec![point2(0, 0), point2(1, 1)],
        weights: vec![integer(1), integer(1)],
        degree: 1,
    };

    fail_next_allocation();
    let sign_error = direct_weight
        .sign()
        .expect_err("sign observes allocation failure");
    assert!(allocation_failure_is_consumed());

    fail_next_allocation();
    let definition_error = match define_bezier(request) {
        Err(CadFailure::AlgebraicFailure(error)) => error,
        _ => panic!("definition propagates the sign failure"),
    };
    assert!(allocation_failure_is_consumed());
    assert_eq!(definition_error, sign_error);
}

#[test]
fn imports_evaluation_api_from_the_external_crate() {
    let _: Option<BezierEvaluationRequest> = None;
    let _: Option<EvaluationResult> = None;
    let _: fn(BezierEvaluationRequest) -> Result<EvaluationResult, CadFailure> = evaluate_bezier;
}

#[test]
fn public_evaluation_operation_reaches_result() {
    let representation = define_bezier(BezierDefinitionRequest::PolynomialCurve {
        control_points: vec![point2(0, 0), point2(2, 2)],
        degree: 1,
    })
    .expect("valid polynomial curve");

    let result = evaluate_bezier(BezierEvaluationRequest::Curve {
        representation,
        parameter: integer(0),
    })
    .expect("crate-root evaluation returns a result");

    assert_eq!(result.into_point(), point2(0, 0));
}

#[test]
fn public_evaluation_result_from_point_roundtrips_point() {
    let point = point2(3, -4);

    assert_eq!(
        EvaluationResult::from_point(point).into_point(),
        point2(3, -4)
    );
}

#[test]
fn evaluation_algebraic_failure_preserves_payload() {
    let parameter = integer(0);
    let representation = define_bezier(BezierDefinitionRequest::PolynomialCurve {
        control_points: vec![point2(0, 0), point2(1, 1)],
        degree: 1,
    })
    .expect("valid polynomial curve");

    fail_next_allocation();
    let direct_error = (|| -> Result<(), AlgnumError> {
        let zero = RealAlgebraic::from_integer(BigInt::try_from(0).map_err(AlgnumError::from)?)?;
        let one = RealAlgebraic::from_integer(BigInt::try_from(1).map_err(AlgnumError::from)?)?;
        parameter.compare(&zero)?;
        parameter.compare(&one)?;
        Ok(())
    })()
    .expect_err("parameter validation observes allocation failure");
    assert!(allocation_failure_is_consumed());

    fail_next_allocation();
    let evaluation_error = match evaluate_bezier(BezierEvaluationRequest::Curve {
        representation,
        parameter,
    }) {
        Err(CadFailure::AlgebraicFailure(error)) => error,
        _ => panic!("evaluation propagates the parameter validation failure"),
    };
    assert!(allocation_failure_is_consumed());
    assert_eq!(evaluation_error, direct_error);
}

#[test]
fn imports_split_api_from_the_external_crate() {
    let _: Option<BezierSplitAxis> = None;
    let _: Option<BezierSplitRequest> = None;
    let _: Option<BezierSplitResult> = None;
    let _: fn(BezierSplitRequest) -> Result<BezierSplitResult, CadFailure> = split_bezier;
}

#[test]
fn public_split_operation_reaches_result() {
    let representation = define_bezier(BezierDefinitionRequest::PolynomialCurve {
        control_points: vec![point2(0, 0), point2(2, 2)],
        degree: 1,
    })
    .expect("valid polynomial curve");

    let result = split_bezier(BezierSplitRequest::Curve {
        representation,
        parameter: integer(0),
    })
    .expect("crate-root split returns a result");

    assert!(matches!(result, BezierSplitResult::Curve { .. }));
}

#[test]
fn split_algebraic_failure_preserves_payload() {
    let parameter = integer(0);
    let representation = define_bezier(BezierDefinitionRequest::PolynomialCurve {
        control_points: vec![point2(0, 0), point2(1, 1)],
        degree: 1,
    })
    .expect("valid polynomial curve");

    fail_next_allocation();
    let direct_error = (|| -> Result<(), AlgnumError> {
        let zero = RealAlgebraic::from_integer(BigInt::try_from(0).map_err(AlgnumError::from)?)?;
        let one = RealAlgebraic::from_integer(BigInt::try_from(1).map_err(AlgnumError::from)?)?;
        parameter.compare(&zero)?;
        parameter.compare(&one)?;
        Ok(())
    })()
    .expect_err("parameter validation observes allocation failure");
    assert!(allocation_failure_is_consumed());

    fail_next_allocation();
    let split_error = match split_bezier(BezierSplitRequest::Curve {
        representation,
        parameter,
    }) {
        Err(CadFailure::AlgebraicFailure(error)) => error,
        _ => panic!("split propagates the parameter validation failure"),
    };
    assert!(allocation_failure_is_consumed());
    assert_eq!(split_error, direct_error);
}
