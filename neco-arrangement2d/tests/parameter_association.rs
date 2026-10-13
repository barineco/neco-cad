use neco_algnum::{RationalPolynomial, RealAlgebraic};
use neco_arrangement2d::{
    CurveEventLocator, CurveEventLocatorKey, CurveIdentity, CurveParameterAssociation, Endpoint,
    EndpointRole,
};
use neco_bigint::{BigInt, BigUint, Dyadic, RawRational};
use neco_calculus::Enclosure;
use neco_geometry::{
    construct_intersection, ChartDefinition3, ChartDisplayName, ChartDomainId, CurveChart,
    ExactShapeEvaluation, IntersectionComponentIdentity, IntersectionComponentView,
    IntersectionConstructionRequest, IntersectionConstructionRequestBody, IntersectionOperand,
    ParametricCurveDeclaration, Point3, RootIsolationPolicy, RootRefinementMethod,
    ShapeDeclaration, ShapeEvaluation,
};

fn algebraic(value: i32) -> RealAlgebraic {
    RealAlgebraic::from_integer(BigInt::try_from(value).unwrap()).unwrap()
}

fn polynomial(values: &[i32]) -> RationalPolynomial {
    RationalPolynomial::from_coefficients(
        values
            .iter()
            .map(|value| {
                RawRational::new(BigInt::try_from(*value).unwrap(), BigUint::one().unwrap())
                    .reduce()
                    .unwrap()
                    .into_reduced()
            })
            .collect(),
    )
}

fn component_identity() -> IntersectionComponentIdentity {
    let policy = RootIsolationPolicy::new(
        8,
        RootRefinementMethod::Bisection,
        Enclosure::new(
            Dyadic::new(BigInt::zero(), 0),
            Dyadic::new(BigInt::one().unwrap(), 0),
        )
        .unwrap(),
    )
    .unwrap();
    let point = IntersectionOperand::Evaluated(ShapeEvaluation::Exact(
        ExactShapeEvaluation::ParametricSurface {
            coordinate: Point3 {
                x: algebraic(0),
                y: algebraic(0),
                z: algebraic(0),
            },
        },
    ));
    let curve = IntersectionOperand::Declaration(ShapeDeclaration::ParametricCurve(
        ParametricCurveDeclaration::new(
            CurveChart::Chart3(
                ChartDefinition3::new(
                    ChartDomainId::new("t".to_string()).unwrap(),
                    ChartDisplayName::new("t".to_string()).unwrap(),
                    polynomial(&[1]),
                    polynomial(&[-1, 3]),
                    polynomial(&[1]),
                    polynomial(&[1]),
                    polynomial(&[1]),
                )
                .unwrap(),
            ),
            neco_geometry::ParameterInterval::new(algebraic(0), algebraic(1), true, true).unwrap(),
        ),
    ));
    let intersection = construct_intersection(IntersectionConstructionRequest::new(
        IntersectionConstructionRequestBody::new(point, curve, policy),
    ))
    .unwrap();
    match intersection.component_view() {
        IntersectionComponentView::Components(mut entries) => entries.next().unwrap().identity(),
        IntersectionComponentView::NoComponents | IntersectionComponentView::Undetermined(_) => {
            panic!("point intersection has one component")
        }
    }
}

#[test]
fn point_association_preserves_locator_authority() {
    let identity = component_identity();
    let association = CurveParameterAssociation::new(
        String::from("curve-a"),
        String::from("intersection-a"),
        identity,
        algebraic(2),
        Some(EndpointRole::Point),
    )
    .unwrap();
    assert_eq!(association.curve_id(), "curve-a");
    assert_eq!(association.intersection_id(), "intersection-a");
    assert_eq!(association.component_identity(), identity);
    assert_eq!(
        association.parameter().compare(&algebraic(2)).unwrap(),
        core::cmp::Ordering::Equal
    );
    assert_eq!(association.endpoint_role(), Some(EndpointRole::Point));
}

#[test]
fn curve_identity_rejects_an_empty_key() {
    assert!(CurveIdentity::new(String::new(), String::from("stable")).is_err());
}

#[test]
fn locator_operations_are_fallible_and_value_based() {
    let identity = component_identity();
    let lower = CurveEventLocator::endpoint(String::from("curve-a"), Endpoint::Lower).unwrap();
    let upper = CurveEventLocator::endpoint(String::from("curve-a"), Endpoint::Upper).unwrap();
    assert_eq!(lower.compare(&upper).unwrap(), core::cmp::Ordering::Less);
    assert_eq!(lower.try_clone().unwrap(), lower);
    assert!(matches!(
        lower.canonical_key().unwrap(),
        CurveEventLocatorKey::Endpoint { .. }
    ));

    let point = CurveEventLocator::point_intersection(
        String::from("curve-a"),
        String::from("intersection-a"),
        identity,
        algebraic(2),
    )
    .unwrap();
    let point_clone = point.try_clone().unwrap();
    assert_eq!(
        point.compare(&point_clone).unwrap(),
        core::cmp::Ordering::Equal
    );
    assert!(matches!(
        point.canonical_key().unwrap(),
        CurveEventLocatorKey::PointIntersection {
            component_identity,
            ..
        } if component_identity == identity
    ));
    assert_eq!(point.compare(&upper).unwrap(), core::cmp::Ordering::Greater);
}
