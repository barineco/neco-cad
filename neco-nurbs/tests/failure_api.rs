use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use neco_algnum::RealAlgebraic;
use neco_bigint::BigInt;
use neco_geometry::Point2;
use neco_nurbs::{
    define_nurbs, evaluate_nurbs, BezierPoint, CadFailure, NurbsDefinitionRequest,
    NurbsEvaluationRequest, NurbsTypes,
};

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
fn reexports_bezier_cad_failure() {
    let failure: CadFailure = neco_bezier::CadFailure::EmptyControlPoints;
    assert_eq!(failure, CadFailure::EmptyControlPoints);

    let knot_cardinality: CadFailure = neco_nurbs::CadFailure::KnotCardinalityMismatch;
    let shared_knot_cardinality: neco_bezier::CadFailure =
        neco_nurbs::CadFailure::KnotCardinalityMismatch;
    assert_eq!(knot_cardinality, shared_knot_cardinality);
    assert_eq!(
        knot_cardinality,
        neco_bezier::CadFailure::KnotCardinalityMismatch
    );

    let non_monotonic: CadFailure = neco_nurbs::CadFailure::NonMonotonicKnots;
    let shared_non_monotonic: neco_bezier::CadFailure = neco_nurbs::CadFailure::NonMonotonicKnots;
    assert_eq!(non_monotonic, shared_non_monotonic);
    assert_eq!(non_monotonic, neco_bezier::CadFailure::NonMonotonicKnots);

    let empty_parameter_domain: CadFailure = neco_nurbs::CadFailure::EmptyParameterDomain;
    let shared_empty_parameter_domain: neco_bezier::CadFailure =
        neco_nurbs::CadFailure::EmptyParameterDomain;
    assert_eq!(empty_parameter_domain, shared_empty_parameter_domain);
    assert_eq!(
        empty_parameter_domain,
        neco_bezier::CadFailure::EmptyParameterDomain
    );
}

#[test]
fn public_evaluation_operation_preserves_empty_domain_and_algebraic_failures() {
    let empty_domain_representation = define_nurbs(NurbsDefinitionRequest::PolynomialCurve {
        control_points: vec![point2(0, 0), point2(1, 1)],
        knots: vec![integer(0), integer(0), integer(0), integer(0)],
        degree: 1,
    })
    .expect("valid curve with an empty parameter domain");

    match evaluate_nurbs(NurbsEvaluationRequest::Curve {
        representation: empty_domain_representation,
        parameter: integer(0),
    }) {
        Err(CadFailure::EmptyParameterDomain) => {}
        _ => panic!("evaluation propagates the empty parameter domain failure"),
    }

    let representation = define_nurbs(NurbsDefinitionRequest::PolynomialCurve {
        control_points: vec![point2(0, 0), point2(1, 1)],
        knots: vec![integer(0), integer(0), integer(1), integer(1)],
        degree: 1,
    })
    .expect("valid polynomial curve");

    let parameter = integer(0);

    fail_next_allocation();
    let direct_error = match &representation {
        NurbsTypes::PolynomialCurve(curve) => curve.knots()[1]
            .compare(&curve.knots()[2])
            .expect_err("span selection observes allocation failure"),
        _ => panic!("representation has the expected polynomial curve variant"),
    };
    assert!(allocation_failure_is_consumed());

    fail_next_allocation();
    let evaluation_error = match evaluate_nurbs(NurbsEvaluationRequest::Curve {
        representation,
        parameter,
    }) {
        Err(CadFailure::AlgebraicFailure(error)) => error,
        _ => panic!("evaluation propagates the span selection failure"),
    };
    assert!(allocation_failure_is_consumed());
    assert_eq!(evaluation_error, direct_error);
}
