use core::cmp::Ordering;

use neco_algnum::RealAlgebraic;
use neco_bezier::{
    contain_bezier, define_bezier, evaluate_bezier, BezierDefinitionRequest,
    BezierEvaluationRequest, BezierPoint, BezierTypes, ContainmentGeometricShape,
    ContainmentParameterDomain, ContainmentPointDimension, ContainmentRepresentationCategory,
    ContainmentWeightEvidence, ConvexHull,
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

fn assert_equal(left: &RealAlgebraic, right: &RealAlgebraic) {
    assert_eq!(
        left.compare(right).expect("algebraic comparison succeeds"),
        Ordering::Equal
    );
}

fn assert_inside(value: &RealAlgebraic, lower: &RealAlgebraic, upper: &RealAlgebraic) {
    assert_ne!(
        value.compare(lower).expect("lower comparison succeeds"),
        Ordering::Less
    );
    assert_ne!(
        value.compare(upper).expect("upper comparison succeeds"),
        Ordering::Greater
    );
}

fn extrema2(points: &[BezierPoint]) -> ([RealAlgebraic; 2], [RealAlgebraic; 2]) {
    let BezierPoint::Point2(first) = &points[0] else {
        panic!("expected Point2 control points");
    };
    let mut lower = [
        first.x.try_clone().expect("x clone succeeds"),
        first.y.try_clone().expect("y clone succeeds"),
    ];
    let mut upper = [
        first.x.try_clone().expect("x clone succeeds"),
        first.y.try_clone().expect("y clone succeeds"),
    ];
    for point in &points[1..] {
        let BezierPoint::Point2(point) = point else {
            panic!("expected Point2 control points");
        };
        for (index, value) in [&point.x, &point.y].into_iter().enumerate() {
            if value
                .compare(&lower[index])
                .expect("lower comparison succeeds")
                == Ordering::Less
            {
                lower[index] = value.try_clone().expect("lower clone succeeds");
            }
            if value
                .compare(&upper[index])
                .expect("upper comparison succeeds")
                == Ordering::Greater
            {
                upper[index] = value.try_clone().expect("upper clone succeeds");
            }
        }
    }
    (lower, upper)
}

fn extrema3(points: &[BezierPoint]) -> ([RealAlgebraic; 3], [RealAlgebraic; 3]) {
    let BezierPoint::Point3(first) = &points[0] else {
        panic!("expected Point3 control points");
    };
    let mut lower = [
        first.x.try_clone().expect("x clone succeeds"),
        first.y.try_clone().expect("y clone succeeds"),
        first.z.try_clone().expect("z clone succeeds"),
    ];
    let mut upper = [
        first.x.try_clone().expect("x clone succeeds"),
        first.y.try_clone().expect("y clone succeeds"),
        first.z.try_clone().expect("z clone succeeds"),
    ];
    for point in &points[1..] {
        let BezierPoint::Point3(point) = point else {
            panic!("expected Point3 control points");
        };
        for (index, value) in [&point.x, &point.y, &point.z].into_iter().enumerate() {
            if value
                .compare(&lower[index])
                .expect("lower comparison succeeds")
                == Ordering::Less
            {
                lower[index] = value.try_clone().expect("lower clone succeeds");
            }
            if value
                .compare(&upper[index])
                .expect("upper comparison succeeds")
                == Ordering::Greater
            {
                upper[index] = value.try_clone().expect("upper clone succeeds");
            }
        }
    }
    (lower, upper)
}

fn assert_point2_inside(point: &BezierPoint, hull: &ConvexHull) {
    let BezierPoint::Point2(point) = point else {
        panic!("expected Point2 observed point");
    };
    let ConvexHull::Point2(hull) = hull else {
        panic!("expected Point2 hull");
    };
    assert_inside(&point.x, hull.x().lower(), hull.x().upper());
    assert_inside(&point.y, hull.y().lower(), hull.y().upper());
}

fn assert_point3_inside(point: &BezierPoint, hull: &ConvexHull) {
    let BezierPoint::Point3(point) = point else {
        panic!("expected Point3 observed point");
    };
    let ConvexHull::Point3(hull) = hull else {
        panic!("expected Point3 hull");
    };
    assert_inside(&point.x, hull.x().lower(), hull.x().upper());
    assert_inside(&point.y, hull.y().lower(), hull.y().upper());
    assert_inside(&point.z, hull.z().lower(), hull.z().upper());
}

fn polynomial_curve2() -> BezierTypes {
    define_bezier(BezierDefinitionRequest::PolynomialCurve {
        control_points: vec![point2(-2, 3), point2(4, -1), point2(4, 3)],
        degree: 2,
    })
    .expect("polynomial Point2 curve is valid")
}

fn rational_curve3(weights: [i32; 3]) -> BezierTypes {
    define_bezier(BezierDefinitionRequest::RationalCurve {
        control_points: vec![point3(-2, 4, 1), point3(3, -1, -2), point3(3, 2, 5)],
        weights: weights.into_iter().map(algebraic).collect(),
        degree: 2,
    })
    .expect("rational Point3 curve is valid")
}

fn polynomial_surface2() -> BezierTypes {
    define_bezier(BezierDefinitionRequest::PolynomialSurface {
        control_net: vec![
            vec![point2(-2, 3), point2(-2, 0), point2(1, 3)],
            vec![point2(4, 3), point2(4, -1), point2(2, 1)],
        ],
        u_degree: 1,
        v_degree: 2,
    })
    .expect("polynomial Point2 surface is valid")
}

fn rational_surface3(weights: [[i32; 2]; 2]) -> BezierTypes {
    define_bezier(BezierDefinitionRequest::RationalSurface {
        control_net: vec![
            vec![point3(-2, 3, 1), point3(-2, 0, 4)],
            vec![point3(4, 3, -2), point3(4, -1, 5)],
        ],
        weights: weights
            .into_iter()
            .map(|row| row.into_iter().map(algebraic).collect())
            .collect(),
        u_degree: 1,
        v_degree: 1,
    })
    .expect("rational Point3 surface is valid")
}

#[test]
fn polynomial_point2_curve_reports_exact_extrema_and_observed_points() {
    let controls = vec![point2(-2, 3), point2(4, -1), point2(4, 3)];
    let (lower, upper) = extrema2(&controls);
    let result = contain_bezier(polynomial_curve2()).expect("containment succeeds");
    let ConvexHull::Point2(hull) = result.hull() else {
        panic!("expected Point2 curve hull");
    };
    assert_equal(hull.x().lower(), &lower[0]);
    assert_equal(hull.x().upper(), &upper[0]);
    assert_equal(hull.y().lower(), &lower[1]);
    assert_equal(hull.y().upper(), &upper[1]);
    for control in &controls {
        assert_point2_inside(control, result.hull());
    }
    for parameter in [algebraic(0), half(), algebraic(1)] {
        let point = evaluate_bezier(BezierEvaluationRequest::Curve {
            representation: polynomial_curve2(),
            parameter,
        })
        .expect("curve evaluation succeeds")
        .into_point();
        assert_point2_inside(&point, result.hull());
    }
}

#[test]
fn rational_point3_curve_uses_euclidean_extrema_independent_of_weights() {
    let controls = vec![point3(-2, 4, 1), point3(3, -1, -2), point3(3, 2, 5)];
    let (lower, upper) = extrema3(&controls);
    let first = contain_bezier(rational_curve3([1, 3, 7])).expect("first containment succeeds");
    let second = contain_bezier(rational_curve3([2, 5, 11])).expect("second containment succeeds");
    let ConvexHull::Point3(first_hull) = first.hull() else {
        panic!("expected Point3 curve hull");
    };
    let ConvexHull::Point3(second_hull) = second.hull() else {
        panic!("expected Point3 curve hull");
    };
    for (interval, lower, upper) in [
        (first_hull.x(), &lower[0], &upper[0]),
        (first_hull.y(), &lower[1], &upper[1]),
        (first_hull.z(), &lower[2], &upper[2]),
    ] {
        assert_equal(interval.lower(), lower);
        assert_equal(interval.upper(), upper);
    }
    for (left, right) in [
        (first_hull.x(), second_hull.x()),
        (first_hull.y(), second_hull.y()),
        (first_hull.z(), second_hull.z()),
    ] {
        assert_equal(left.lower(), right.lower());
        assert_equal(left.upper(), right.upper());
    }
    for control in &controls {
        assert_point3_inside(control, first.hull());
    }
    let interior = evaluate_bezier(BezierEvaluationRequest::Curve {
        representation: rational_curve3([1, 3, 7]),
        parameter: half(),
    })
    .expect("rational curve evaluation succeeds")
    .into_point();
    assert_point3_inside(&interior, first.hull());
}

#[test]
fn polynomial_point2_surface_reports_extrema_and_observed_points() {
    let controls = vec![
        point2(-2, 3),
        point2(-2, 0),
        point2(1, 3),
        point2(4, 3),
        point2(4, -1),
        point2(2, 1),
    ];
    let (lower, upper) = extrema2(&controls);
    let result = contain_bezier(polynomial_surface2()).expect("containment succeeds");
    let ConvexHull::Point2(hull) = result.hull() else {
        panic!("expected Point2 surface hull");
    };
    assert_equal(hull.x().lower(), &lower[0]);
    assert_equal(hull.x().upper(), &upper[0]);
    assert_equal(hull.y().lower(), &lower[1]);
    assert_equal(hull.y().upper(), &upper[1]);
    for control in &controls {
        assert_point2_inside(control, result.hull());
    }
    for (u, v) in [
        (algebraic(0), algebraic(0)),
        (half(), half()),
        (algebraic(1), algebraic(1)),
    ] {
        let point = evaluate_bezier(BezierEvaluationRequest::Surface {
            representation: polynomial_surface2(),
            u,
            v,
        })
        .expect("surface evaluation succeeds")
        .into_point();
        assert_point2_inside(&point, result.hull());
    }
}

#[test]
fn rational_point3_surface_reports_extrema_and_observed_points() {
    let controls = vec![
        point3(-2, 3, 1),
        point3(-2, 0, 4),
        point3(4, 3, -2),
        point3(4, -1, 5),
    ];
    let (lower, upper) = extrema3(&controls);
    let result = contain_bezier(rational_surface3([[1, 2], [3, 5]])).expect("containment succeeds");
    let ConvexHull::Point3(hull) = result.hull() else {
        panic!("expected Point3 surface hull");
    };
    for (interval, lower, upper) in [
        (hull.x(), &lower[0], &upper[0]),
        (hull.y(), &lower[1], &upper[1]),
        (hull.z(), &lower[2], &upper[2]),
    ] {
        assert_equal(interval.lower(), lower);
        assert_equal(interval.upper(), upper);
    }
    for control in &controls {
        assert_point3_inside(control, result.hull());
    }
    let interior = evaluate_bezier(BezierEvaluationRequest::Surface {
        representation: rational_surface3([[1, 2], [3, 5]]),
        u: half(),
        v: half(),
    })
    .expect("rational surface evaluation succeeds")
    .into_point();
    assert_point3_inside(&interior, result.hull());
}

#[test]
fn degree_zero_constant_coordinate_and_repeated_extrema_are_exact() {
    let degree_zero = define_bezier(BezierDefinitionRequest::PolynomialCurve {
        control_points: vec![point3(7, 7, 7)],
        degree: 0,
    })
    .expect("degree-zero curve is valid");
    let result = contain_bezier(degree_zero).expect("degree-zero containment succeeds");
    let ConvexHull::Point3(hull) = result.hull() else {
        panic!("expected Point3 degree-zero hull");
    };
    for interval in [hull.x(), hull.y(), hull.z()] {
        assert_equal(interval.lower(), interval.upper());
    }

    let controls = vec![point2(2, -1), point2(2, 3), point2(2, 0), point2(2, 3)];
    let result = contain_bezier(
        define_bezier(BezierDefinitionRequest::PolynomialCurve {
            control_points: vec![point2(2, -1), point2(2, 3), point2(2, 0), point2(2, 3)],
            degree: 3,
        })
        .expect("repeated-extrema curve is valid"),
    )
    .expect("repeated-extrema containment succeeds");
    let (lower, upper) = extrema2(&controls);
    let ConvexHull::Point2(hull) = result.hull() else {
        panic!("expected Point2 repeated-extrema hull");
    };
    assert_equal(hull.x().lower(), &lower[0]);
    assert_equal(hull.x().upper(), &upper[0]);
    assert_equal(hull.y().lower(), &lower[1]);
    assert_equal(hull.y().upper(), &upper[1]);
}

#[test]
fn witness_accessors_report_all_seven_coordinates_for_each_representation() {
    let results = [
        contain_bezier(polynomial_curve2()).expect("polynomial curve succeeds"),
        contain_bezier(rational_curve3([1, 3, 7])).expect("rational curve succeeds"),
        contain_bezier(polynomial_surface2()).expect("polynomial surface succeeds"),
        contain_bezier(rational_surface3([[1, 2], [3, 5]])).expect("rational surface succeeds"),
    ];
    for result in results {
        assert!(*result.witness().row_first());
        assert!(*result.witness().finite_control_values());
        let _ = result.witness().representation_category();
        let _ = result.witness().geometric_shape();
        let _ = result.witness().point_dimension();
        let _ = result.witness().weight_evidence();
        let _ = result.witness().parameter_domain();
    }
}

#[test]
fn all_four_bezier_types_are_accepted_by_containment() {
    for representation in [
        polynomial_curve2(),
        rational_curve3([1, 3, 7]),
        polynomial_surface2(),
        rational_surface3([[1, 2], [3, 5]]),
    ] {
        assert!(contain_bezier(representation).is_ok());
    }
}

#[test]
fn witness_variants_match_representation_and_shape() {
    let polynomial_curve = contain_bezier(polynomial_curve2()).expect("polynomial curve succeeds");
    assert!(matches!(
        polynomial_curve.witness().representation_category(),
        ContainmentRepresentationCategory::Polynomial
    ));
    assert!(matches!(
        polynomial_curve.witness().geometric_shape(),
        ContainmentGeometricShape::Curve
    ));
    assert!(matches!(
        polynomial_curve.witness().point_dimension(),
        ContainmentPointDimension::Point2
    ));
    assert!(matches!(
        polynomial_curve.witness().weight_evidence(),
        ContainmentWeightEvidence::PolynomialNoWeights
    ));
    assert!(matches!(
        polynomial_curve.witness().parameter_domain(),
        ContainmentParameterDomain::ClosedCurveUnitInterval
    ));

    let rational_surface =
        contain_bezier(rational_surface3([[1, 2], [3, 5]])).expect("rational surface succeeds");
    assert!(matches!(
        rational_surface.witness().representation_category(),
        ContainmentRepresentationCategory::Rational
    ));
    assert!(matches!(
        rational_surface.witness().geometric_shape(),
        ContainmentGeometricShape::Surface
    ));
    assert!(matches!(
        rational_surface.witness().point_dimension(),
        ContainmentPointDimension::Point3
    ));
    assert!(matches!(
        rational_surface.witness().weight_evidence(),
        ContainmentWeightEvidence::ValidatedPositiveWeights
    ));
    assert!(matches!(
        rational_surface.witness().parameter_domain(),
        ContainmentParameterDomain::ClosedSurfaceUnitSquare
    ));
}

#[test]
fn all_witness_accessors_are_borrow_accessors() {
    let result = contain_bezier(polynomial_curve2()).expect("containment succeeds");
    let witness = result.witness();
    let _: &ContainmentRepresentationCategory = witness.representation_category();
    let _: &ContainmentGeometricShape = witness.geometric_shape();
    let _: &ContainmentPointDimension = witness.point_dimension();
    let _: &bool = witness.row_first();
    let _: &bool = witness.finite_control_values();
    let _: &ContainmentWeightEvidence = witness.weight_evidence();
    let _: &ContainmentParameterDomain = witness.parameter_domain();
}

#[test]
fn control_points_and_evaluated_points_are_contained() {
    let curve = polynomial_curve2();
    let result = contain_bezier(curve).expect("containment succeeds");
    let endpoint = evaluate_bezier(BezierEvaluationRequest::Curve {
        representation: polynomial_curve2(),
        parameter: algebraic(0),
    })
    .expect("endpoint evaluation succeeds")
    .into_point();
    let interior = evaluate_bezier(BezierEvaluationRequest::Curve {
        representation: polynomial_curve2(),
        parameter: half(),
    })
    .expect("interior evaluation succeeds")
    .into_point();
    assert_point2_inside(&endpoint, result.hull());
    assert_point2_inside(&interior, result.hull());
}

#[test]
fn surface_corners_and_interior_are_contained() {
    let result = contain_bezier(rational_surface3([[1, 2], [3, 5]])).expect("containment succeeds");
    for (u, v) in [
        (algebraic(0), algebraic(0)),
        (algebraic(1), algebraic(1)),
        (half(), half()),
    ] {
        let point = evaluate_bezier(BezierEvaluationRequest::Surface {
            representation: rational_surface3([[1, 2], [3, 5]]),
            u,
            v,
        })
        .expect("surface evaluation succeeds")
        .into_point();
        assert_point3_inside(&point, result.hull());
    }
}
