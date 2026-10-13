use neco_algnum::RealAlgebraic;
use neco_bezier::{
    aggregate_bezier, contain_bezier, define_bezier, evaluate_bezier, split_bezier,
    BezierAggregationInput, BezierDefinitionRequest, BezierEvaluationRequest, BezierPoint,
    BezierSplitRequest, BezierSplitResult, BezierTypes, CadFailure, ContainmentResult,
    EvaluationResult,
};
use neco_bigint::BigInt;
use neco_geometry::{Point2, Point3};

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

fn definition(dimension: usize) -> BezierTypes {
    let control_points = match dimension {
        2 => vec![point2(0, 0), point2(2, 4)],
        3 => vec![point3(0, 0, 0), point3(2, 4, 6)],
        _ => panic!("fixture dimension is two or three"),
    };
    define_bezier(BezierDefinitionRequest::PolynomialCurve {
        control_points,
        degree: 1,
    })
    .expect("fixture definition succeeds")
}

fn evaluation(dimension: usize) -> EvaluationResult {
    evaluate_bezier(BezierEvaluationRequest::Curve {
        representation: definition(dimension),
        parameter: algebraic(0),
    })
    .expect("fixture evaluation succeeds")
}

fn split(dimension: usize) -> BezierSplitResult {
    split_bezier(BezierSplitRequest::Curve {
        representation: definition(dimension),
        parameter: algebraic(0),
    })
    .expect("fixture split succeeds")
}

fn containment(dimension: usize) -> ContainmentResult {
    contain_bezier(definition(dimension)).expect("fixture containment succeeds")
}

fn input(dimension: usize) -> BezierAggregationInput {
    BezierAggregationInput::new(
        definition(dimension),
        evaluation(dimension),
        split(dimension),
        containment(dimension),
    )
}

fn assert_polynomial_curve_dimension(representation: BezierTypes, dimension: usize) {
    let BezierTypes::PolynomialCurve(curve) = representation else {
        panic!("aggregate preserves the polynomial curve variant");
    };
    assert_eq!(curve.dimension(), dimension);
}

#[test]
fn aggregate_consumes_four_same_dimension_products_and_preserves_type() {
    let two_dimensional = input(2);
    let _: &BezierTypes = two_dimensional.definition();
    let _: &EvaluationResult = two_dimensional.evaluation();
    let _: &BezierSplitResult = two_dimensional.split();
    let _: &ContainmentResult = two_dimensional.containment();
    assert_polynomial_curve_dimension(
        aggregate_bezier(two_dimensional).expect("two-dimensional aggregate succeeds"),
        2,
    );

    assert_polynomial_curve_dimension(
        aggregate_bezier(input(3)).expect("three-dimensional aggregate succeeds"),
        3,
    );
}

#[test]
fn aggregate_returns_dimension_mismatch_for_each_inconsistent_field() {
    let definition_mismatch =
        BezierAggregationInput::new(definition(3), evaluation(2), split(2), containment(2));
    assert_eq!(
        aggregate_bezier(definition_mismatch),
        Err(CadFailure::ControlPointDimensionMismatch)
    );

    let evaluation_mismatch =
        BezierAggregationInput::new(definition(2), evaluation(3), split(2), containment(2));
    assert_eq!(
        aggregate_bezier(evaluation_mismatch),
        Err(CadFailure::ControlPointDimensionMismatch)
    );

    let split_mismatch =
        BezierAggregationInput::new(definition(2), evaluation(2), split(3), containment(2));
    assert_eq!(
        aggregate_bezier(split_mismatch),
        Err(CadFailure::ControlPointDimensionMismatch)
    );

    let containment_mismatch =
        BezierAggregationInput::new(definition(2), evaluation(2), split(2), containment(3));
    assert_eq!(
        aggregate_bezier(containment_mismatch),
        Err(CadFailure::ControlPointDimensionMismatch)
    );
}

enum Product {
    Definition(BezierTypes),
    Evaluation(EvaluationResult),
    Split(BezierSplitResult),
    Containment(ContainmentResult),
}

fn produce(product: usize, dimension: usize) -> Product {
    match product {
        0 => Product::Definition(definition(dimension)),
        1 => Product::Evaluation(evaluation(dimension)),
        2 => Product::Split(split(dimension)),
        3 => Product::Containment(containment(dimension)),
        _ => panic!("product index is within the four aggregate fields"),
    }
}

fn aggregate_in_production_order(order: [usize; 4], dimension: usize) -> BezierTypes {
    let mut definition_product = None;
    let mut evaluation_product = None;
    let mut split_product = None;
    let mut containment_product = None;

    for product in order {
        match produce(product, dimension) {
            Product::Definition(value) => definition_product = Some(value),
            Product::Evaluation(value) => evaluation_product = Some(value),
            Product::Split(value) => split_product = Some(value),
            Product::Containment(value) => containment_product = Some(value),
        }
    }

    aggregate_bezier(BezierAggregationInput::new(
        definition_product.expect("definition field was generated"),
        evaluation_product.expect("evaluation field was generated"),
        split_product.expect("split field was generated"),
        containment_product.expect("containment field was generated"),
    ))
    .expect("permuted aggregate succeeds")
}

#[test]
fn aggregate_preserves_field_labels_for_all_product_generation_permutations() {
    let permutations = [
        [0, 1, 2, 3],
        [0, 1, 3, 2],
        [0, 2, 1, 3],
        [0, 2, 3, 1],
        [0, 3, 1, 2],
        [0, 3, 2, 1],
        [1, 0, 2, 3],
        [1, 0, 3, 2],
        [1, 2, 0, 3],
        [1, 2, 3, 0],
        [1, 3, 0, 2],
        [1, 3, 2, 0],
        [2, 0, 1, 3],
        [2, 0, 3, 1],
        [2, 1, 0, 3],
        [2, 1, 3, 0],
        [2, 3, 0, 1],
        [2, 3, 1, 0],
        [3, 0, 1, 2],
        [3, 0, 2, 1],
        [3, 1, 0, 2],
        [3, 1, 2, 0],
        [3, 2, 0, 1],
        [3, 2, 1, 0],
    ];

    for order in permutations {
        let result = aggregate_in_production_order(order, 2);
        assert_eq!(result, definition(2));
        assert_polynomial_curve_dimension(result, 2);
    }
}

#[test]
fn aggregate_repeats_successfully_from_independent_products() {
    let first = aggregate_bezier(input(2)).expect("first aggregate succeeds");
    let second = aggregate_bezier(input(2)).expect("second aggregate succeeds");

    assert_eq!(first, second);
    assert_polynomial_curve_dimension(first, 2);
    assert_polynomial_curve_dimension(second, 2);
}
