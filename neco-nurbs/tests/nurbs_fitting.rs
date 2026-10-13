use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use neco_algnum::RealAlgebraic;
use neco_bigint::BigInt;
use neco_geometry::{Point2, Point3};
use neco_nurbs::{
    fit_nurbs_curve, BezierPoint, FittingRequest, FittingResult, NurbsFittingFailure,
    PolynomialNurbsCurve,
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

fn algebraic(value: i32) -> RealAlgebraic {
    RealAlgebraic::from_integer(BigInt::try_from(value).expect("i32 converts to BigInt"))
        .expect("integer is algebraic")
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

fn linear_knots() -> Vec<RealAlgebraic> {
    vec![algebraic(0), algebraic(0), algebraic(1), algebraic(1)]
}

fn linear_request(parameters: Vec<i32>, samples: Vec<BezierPoint>) -> FittingRequest {
    FittingRequest {
        parameters: parameters.into_iter().map(algebraic).collect(),
        samples,
        knots: linear_knots(),
        degree: 1,
        control_point_count: 2,
    }
}

fn constant_request(parameters: Vec<i32>, samples: Vec<BezierPoint>) -> FittingRequest {
    FittingRequest {
        parameters: parameters.into_iter().map(algebraic).collect(),
        samples,
        knots: vec![algebraic(0), algebraic(1), algebraic(1)],
        degree: 0,
        control_point_count: 2,
    }
}

fn fitted_curve(result: FittingResult) -> PolynomialNurbsCurve {
    result.into_curve()
}

fn fitting_failure(request: FittingRequest) -> NurbsFittingFailure {
    match fit_nurbs_curve(request) {
        Ok(_) => panic!("request produces a fitting failure"),
        Err(failure) => failure,
    }
}

#[test]
fn fits_point2_samples_with_a_linear_clamped_polynomial_curve() {
    let curve = fitted_curve(
        fit_nurbs_curve(linear_request(vec![0, 1], vec![point2(0, 1), point2(2, 3)]))
            .expect("linear clamped samples fit"),
    );

    assert_eq!(curve.control_points(), [point2(0, 1), point2(2, 3)]);
    assert_eq!(curve.knots(), linear_knots().as_slice());
    assert_eq!(curve.degree(), 1);
    assert_eq!(curve.dimension(), 2);
}

#[test]
fn rejects_invalid_request_shapes_and_domains() {
    assert_eq!(
        fitting_failure(linear_request(vec![], vec![])),
        NurbsFittingFailure::EmptySamples
    );
    assert_eq!(
        fitting_failure(linear_request(vec![0], vec![point2(0, 0), point2(1, 1)])),
        NurbsFittingFailure::ParameterSampleCardinalityMismatch
    );
    assert_eq!(
        fitting_failure(linear_request(
            vec![0, 1],
            vec![point2(0, 0), point3(1, 1, 1)]
        )),
        NurbsFittingFailure::ControlPointDimensionMismatch
    );
    assert_eq!(
        fitting_failure(FittingRequest {
            parameters: vec![algebraic(0)],
            samples: vec![point2(0, 0)],
            knots: vec![algebraic(0)],
            degree: 0,
            control_point_count: 0,
        }),
        NurbsFittingFailure::DegreeMismatch
    );
    assert_eq!(
        fitting_failure(FittingRequest {
            parameters: vec![algebraic(0), algebraic(1)],
            samples: vec![point2(0, 0), point2(1, 1)],
            knots: vec![algebraic(0), algebraic(0), algebraic(1)],
            degree: 1,
            control_point_count: 2,
        }),
        NurbsFittingFailure::KnotCardinalityMismatch
    );
    assert_eq!(
        fitting_failure(FittingRequest {
            parameters: vec![algebraic(0), algebraic(1)],
            samples: vec![point2(0, 0), point2(1, 1)],
            knots: vec![algebraic(0), algebraic(1), algebraic(0), algebraic(1)],
            degree: 1,
            control_point_count: 2,
        }),
        NurbsFittingFailure::NonMonotonicKnots
    );
    assert_eq!(
        fitting_failure(FittingRequest {
            parameters: vec![algebraic(0), algebraic(0)],
            samples: vec![point2(0, 0), point2(1, 1)],
            knots: vec![algebraic(0), algebraic(0), algebraic(0), algebraic(0)],
            degree: 1,
            control_point_count: 2,
        }),
        NurbsFittingFailure::EmptyParameterDomain
    );
    assert_eq!(
        fitting_failure(linear_request(
            vec![-1, 1],
            vec![point2(0, 0), point2(1, 1)]
        )),
        NurbsFittingFailure::ParameterOutOfRange
    );
}

#[test]
fn reports_a_rank_deficient_system() {
    assert_eq!(
        fitting_failure(constant_request(vec![0], vec![point2(0, 0)])),
        NurbsFittingFailure::RankDeficient
    );
}

#[test]
fn reports_an_inconsistent_system() {
    assert_eq!(
        fitting_failure(constant_request(
            vec![0, 0],
            vec![point2(0, 0), point2(1, 0)]
        )),
        NurbsFittingFailure::InconsistentSystem
    );
}

#[test]
fn propagates_a_cad_algebraic_failure() {
    let request = constant_request(vec![0, 1], vec![point2(0, 0), point2(1, 1)]);

    fail_next_allocation();
    let failure = fitting_failure(request);
    assert!(allocation_failure_is_consumed());
    assert!(matches!(
        failure,
        NurbsFittingFailure::Cad(neco_nurbs::CadFailure::AlgebraicFailure(_))
    ));
}
