use neco_algnum::{RationalPolynomial, RealAlgebraic};
use neco_arrangement2d::{
    construct_arrangement, construct_full_domain_arrangement, ArrangementConstructionRequest,
    ArrangementCurve, ArrangementFailure, ArrangementIntersectionBinding, CurveEventLocator,
    CurveIdentity, CurveParameterAssociation, Endpoint, EndpointRole, PairedSpanAssociation,
    PlanarTopologyDeclaration, PlanarVertexDeclaration, RepresentationCategory,
    SpanEndpointCorrespondence, SpanSide,
};
use neco_bigint::{BigInt, BigUint, Dyadic, RawRational, ReducedRational};
use neco_calculus::Enclosure;
use neco_geometry::{
    construct_intersection, ChartDefinition2, ChartDefinition3, ChartDisplayName, ChartDomainId,
    CurveChart, ExactShapeEvaluation, GeometryFailure, ImplicitSurfaceDeclaration,
    IntersectionComponentView, IntersectionConstructionRequest,
    IntersectionConstructionRequestBody, IntersectionOperand, MultivariatePolynomial3,
    ParametricCurveDeclaration, Point3, RootIsolationPolicy, RootRefinementMethod,
    ShapeDeclaration, ShapeEvaluation, ShapeFamily,
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

fn chart() -> ChartDefinition2 {
    let polynomial = RationalPolynomial::from_coefficients(vec![ReducedRational::from_bigint(
        BigInt::try_from(1).unwrap(),
    )
    .unwrap()]);
    ChartDefinition2::new(
        ChartDomainId::new("t".to_string()).unwrap(),
        ChartDisplayName::new("t".to_string()).unwrap(),
        polynomial.try_clone().unwrap(),
        polynomial.try_clone().unwrap(),
        polynomial.try_clone().unwrap(),
        polynomial,
    )
    .unwrap()
}

fn curve(id: &str) -> ArrangementCurve {
    ArrangementCurve::new(
        CurveIdentity::new(id.into(), format!("stable-{id}")).unwrap(),
        RepresentationCategory::PolynomialBezier2D,
        chart(),
        neco_geometry::ParameterInterval::new(algebraic(0), algebraic(1), true, true).unwrap(),
    )
    .unwrap()
}

fn policy() -> RootIsolationPolicy {
    RootIsolationPolicy::new(
        8,
        RootRefinementMethod::Bisection,
        Enclosure::new(
            Dyadic::new(BigInt::zero(), 0),
            Dyadic::new(BigInt::one().unwrap(), 0),
        )
        .unwrap(),
    )
    .unwrap()
}

fn point_intersection() -> neco_geometry::IntersectionSet {
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
    construct_intersection(IntersectionConstructionRequest::new(
        IntersectionConstructionRequestBody::new(point, curve, policy()),
    ))
    .unwrap()
}

fn finite_point_intersection() -> neco_geometry::IntersectionSet {
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
                    ChartDomainId::new("finite".to_string()).unwrap(),
                    ChartDisplayName::new("finite".to_string()).unwrap(),
                    polynomial(&[1]),
                    polynomial(&[2, -9, 9]),
                    polynomial(&[1]),
                    polynomial(&[1]),
                    polynomial(&[1]),
                )
                .unwrap(),
            ),
            neco_geometry::ParameterInterval::new(algebraic(0), algebraic(1), true, true).unwrap(),
        ),
    ));
    construct_intersection(IntersectionConstructionRequest::new(
        IntersectionConstructionRequestBody::new(point, curve, policy()),
    ))
    .unwrap()
}

fn curve_intersection() -> neco_geometry::IntersectionSet {
    let left = IntersectionOperand::Declaration(ShapeDeclaration::ParametricCurve(
        ParametricCurveDeclaration::new(
            CurveChart::Chart3(
                ChartDefinition3::new(
                    ChartDomainId::new("left".to_string()).unwrap(),
                    ChartDisplayName::new("left".to_string()).unwrap(),
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
    let right = IntersectionOperand::Declaration(ShapeDeclaration::ParametricCurve(
        ParametricCurveDeclaration::new(
            CurveChart::Chart3(
                ChartDefinition3::new(
                    ChartDomainId::new("right".to_string()).unwrap(),
                    ChartDisplayName::new("right".to_string()).unwrap(),
                    polynomial(&[1]),
                    polynomial(&[0]),
                    polynomial(&[1]),
                    polynomial(&[1]),
                    polynomial(&[1]),
                )
                .unwrap(),
            ),
            neco_geometry::ParameterInterval::new(algebraic(0), algebraic(1), true, true).unwrap(),
        ),
    ));
    construct_intersection(IntersectionConstructionRequest::new(
        IntersectionConstructionRequestBody::new(left, right, policy()),
    ))
    .unwrap()
}

fn undetermined_intersection() -> neco_geometry::IntersectionSet {
    let unresolved = IntersectionOperand::Evaluated(ShapeEvaluation::Undetermined {
        family: ShapeFamily::ImplicitSurface,
    });
    let zero = IntersectionOperand::Declaration(ShapeDeclaration::ImplicitSurface(
        ImplicitSurfaceDeclaration::new(MultivariatePolynomial3::zero()),
    ));
    construct_intersection(IntersectionConstructionRequest::new(
        IntersectionConstructionRequestBody::new(unresolved, zero, policy()),
    ))
    .unwrap()
}

#[test]
fn empty_curve_input_is_an_explicit_failure() {
    let request = ArrangementConstructionRequest::new(
        Vec::new(),
        Vec::new(),
        PlanarTopologyDeclaration::new(Vec::new(), Vec::new(), Vec::new()).unwrap(),
    )
    .unwrap();
    assert_eq!(
        construct_arrangement(request),
        Err(ArrangementFailure::EmptyCurveInput)
    );
}

#[test]
fn endpoint_locator_membership_is_required_when_segments_exist() {
    let curve = curve("a");
    let topology = PlanarTopologyDeclaration::new(
        vec![PlanarVertexDeclaration::new(
            String::from("lower"),
            vec![CurveEventLocator::endpoint(String::from("a"), Endpoint::Lower).unwrap()],
        )
        .unwrap()],
        Vec::new(),
        Vec::new(),
    )
    .unwrap();
    let request = ArrangementConstructionRequest::new(vec![curve], Vec::new(), topology).unwrap();
    assert!(matches!(
        construct_arrangement(request),
        Err(ArrangementFailure::UncoveredVertexEventLocator { .. })
    ));
}

#[test]
fn point_association_identity_is_validated_against_component_view() {
    let intersection = point_intersection();
    let identity = match intersection.component_view() {
        IntersectionComponentView::Components(mut entries) => entries.next().unwrap().identity(),
        IntersectionComponentView::NoComponents | IntersectionComponentView::Undetermined(_) => {
            panic!("point intersection has one component")
        }
    };
    let association = CurveParameterAssociation::new(
        String::from("a"),
        String::from("intersection-a"),
        identity,
        algebraic(0),
        Some(EndpointRole::Point),
    )
    .unwrap();
    let binding = ArrangementIntersectionBinding::new(
        String::from("a"),
        String::from("b"),
        intersection,
        vec![association],
        Vec::new(),
    )
    .unwrap();
    let request = ArrangementConstructionRequest::new(
        vec![curve("a"), curve("b")],
        vec![binding],
        PlanarTopologyDeclaration::new(Vec::new(), Vec::new(), Vec::new()).unwrap(),
    )
    .unwrap();
    assert!(matches!(
        construct_arrangement(request),
        Err(ArrangementFailure::IncompletePointAssociation { .. })
    ));
}

#[test]
fn point_association_identity_outside_component_view_is_rejected() {
    let intersection = point_intersection();
    let foreign_identity = match finite_point_intersection().component_view() {
        IntersectionComponentView::Components(mut entries) => entries.nth(1).unwrap().identity(),
        IntersectionComponentView::NoComponents | IntersectionComponentView::Undetermined(_) => {
            panic!("finite point intersection has two components")
        }
    };
    let association = CurveParameterAssociation::new(
        String::from("a"),
        String::from("intersection-a"),
        foreign_identity,
        algebraic(0),
        Some(EndpointRole::Point),
    )
    .unwrap();
    let binding = ArrangementIntersectionBinding::new(
        String::from("a"),
        String::from("b"),
        intersection,
        vec![association],
        Vec::new(),
    )
    .unwrap();
    let request = ArrangementConstructionRequest::new(
        vec![curve("a"), curve("b")],
        vec![binding],
        PlanarTopologyDeclaration::new(Vec::new(), Vec::new(), Vec::new()).unwrap(),
    )
    .unwrap();
    assert!(matches!(
        construct_arrangement(request),
        Err(ArrangementFailure::AssociationComponentIdentityMismatch { .. })
    ));
}

#[test]
fn span_identity_outside_component_view_is_rejected() {
    let intersection = curve_intersection();
    let foreign_identity = match finite_point_intersection().component_view() {
        IntersectionComponentView::Components(mut entries) => entries.nth(1).unwrap().identity(),
        IntersectionComponentView::NoComponents | IntersectionComponentView::Undetermined(_) => {
            panic!("finite point intersection has two components")
        }
    };
    let endpoint = |curve_id: &str, parameter, role| {
        CurveParameterAssociation::new(
            String::from(curve_id),
            String::from("intersection-a"),
            foreign_identity,
            parameter,
            Some(role),
        )
        .unwrap()
    };
    let span = PairedSpanAssociation::new(
        String::from("span-a"),
        String::from("intersection-a"),
        foreign_identity,
        SpanSide {
            curve_id: String::from("a"),
            lower: endpoint("a", algebraic(0), EndpointRole::OverlapLower),
            upper: endpoint("a", algebraic(1), EndpointRole::OverlapUpper),
        },
        SpanSide {
            curve_id: String::from("b"),
            lower: endpoint("b", algebraic(0), EndpointRole::OverlapLower),
            upper: endpoint("b", algebraic(1), EndpointRole::OverlapUpper),
        },
        SpanEndpointCorrespondence::LowerToLower,
    )
    .unwrap();
    let binding = ArrangementIntersectionBinding::new(
        String::from("a"),
        String::from("b"),
        intersection,
        Vec::new(),
        vec![span],
    )
    .unwrap();
    let request = ArrangementConstructionRequest::new(
        vec![curve("a"), curve("b")],
        vec![binding],
        PlanarTopologyDeclaration::new(Vec::new(), Vec::new(), Vec::new()).unwrap(),
    )
    .unwrap();
    assert!(matches!(
        construct_arrangement(request),
        Err(ArrangementFailure::AssociationComponentIdentityMismatch { .. })
    ));
}

#[test]
fn paired_span_constructor_requires_endpoint_identity_consistency() {
    let span_identity = match curve_intersection().component_view() {
        IntersectionComponentView::Components(mut entries) => entries.next().unwrap().identity(),
        IntersectionComponentView::NoComponents | IntersectionComponentView::Undetermined(_) => {
            panic!("curve intersection has one component")
        }
    };
    let endpoint_identity = match finite_point_intersection().component_view() {
        IntersectionComponentView::Components(mut entries) => entries.nth(1).unwrap().identity(),
        IntersectionComponentView::NoComponents | IntersectionComponentView::Undetermined(_) => {
            panic!("finite point intersection has two components")
        }
    };
    let endpoint = |curve_id: &str, parameter, role| {
        CurveParameterAssociation::new(
            String::from(curve_id),
            String::from("intersection-a"),
            endpoint_identity,
            parameter,
            Some(role),
        )
        .unwrap()
    };
    assert!(matches!(
        PairedSpanAssociation::new(
            String::from("span-a"),
            String::from("intersection-a"),
            span_identity,
            SpanSide {
                curve_id: String::from("a"),
                lower: endpoint("a", algebraic(0), EndpointRole::OverlapLower),
                upper: endpoint("a", algebraic(1), EndpointRole::OverlapUpper),
            },
            SpanSide {
                curve_id: String::from("b"),
                lower: endpoint("b", algebraic(0), EndpointRole::OverlapLower),
                upper: endpoint("b", algebraic(1), EndpointRole::OverlapUpper),
            },
            SpanEndpointCorrespondence::LowerToLower,
        ),
        Err(ArrangementFailure::AssociationComponentIdentityMismatch { .. })
    ));
}

#[test]
fn undetermined_reason_precedes_association_validation() {
    let expected_reason = match undetermined_intersection().component_view() {
        IntersectionComponentView::Undetermined(reason) => reason,
        IntersectionComponentView::Components(_) | IntersectionComponentView::NoComponents => {
            panic!("intersection is undetermined")
        }
    };
    let binding = ArrangementIntersectionBinding::new(
        String::from("a"),
        String::from("b"),
        undetermined_intersection(),
        Vec::new(),
        Vec::new(),
    )
    .unwrap();
    let request = ArrangementConstructionRequest::new(
        vec![curve("a"), curve("b")],
        vec![binding],
        PlanarTopologyDeclaration::new(Vec::new(), Vec::new(), Vec::new()).unwrap(),
    )
    .unwrap();
    assert_eq!(
        construct_arrangement(request),
        Err(ArrangementFailure::IntersectionUndetermined {
            reason: expected_reason,
        })
    );
}

#[test]
fn geometry_failure_conversion_preserves_payload() {
    let failure: ArrangementFailure = GeometryFailure::RootIsolationPolicyInvalid.into();
    assert_eq!(
        failure,
        ArrangementFailure::GeometryFailure(GeometryFailure::RootIsolationPolicyInvalid)
    );
}

#[test]
fn full_domain_arrangement_has_one_face_and_cell() {
    let arrangement = construct_full_domain_arrangement(Vec::new()).unwrap();
    assert_eq!(arrangement.faces().len(), 1);
    assert_eq!(arrangement.faces()[0].face_id(), "full-domain-face");
    assert!(arrangement.closed_walks()[0].incidence_ids().is_empty());
    assert_eq!(arrangement.finite_cells().len(), 1);
}
