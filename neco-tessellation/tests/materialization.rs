use neco_algnum::{RationalPolynomial, RealAlgebraic};
use neco_arrangement2d::{
    construct_arrangement, ArrangementConstructionRequest, ArrangementCurve,
    ArrangementIntersectionBinding, CurveEventLocator, CurveIdentity, CurveParameterAssociation,
    Endpoint, EndpointRole, EventRotation, PlanarTopologyDeclaration, PlanarVertexDeclaration,
    PlanarWalkRole, PlanarWinding, RepresentationCategory, WalkClassification,
};
use neco_bigint::{BigInt, BigUint, Dyadic, RawRational};
use neco_brep::{
    materialize_analytic_brep, AnalyticBrep, ArrangementId, BrepAssignments, BrepFailure,
    BrepMaterializationRequest, ChartArrangement, CurveRepresentation, CurveRepresentationId,
    EdgeAssignment, FaceAssignment, SourceRevision, SurfaceRepresentation, SurfaceRepresentationId,
    VertexAssignment,
};
use neco_calculus::{DerivativeOrder, Enclosure, Polynomial};
use neco_geometry::{
    construct_intersection, construct_region_types, derive_boundary, BoundaryCandidateSeed,
    BoundaryDerivationRequest, BoundaryIntersectionSelector, BoundaryRestrictionChart,
    BoundarySideSample, CanonicalAtom, ChartDefinition2, ChartDefinition3, ChartDisplayName,
    ChartDomainId, CurveChart, ExactShapeEvaluation, IntersectionComponentIdentity,
    IntersectionComponentView, IntersectionConstructionRequest,
    IntersectionConstructionRequestBody, IntersectionOperand, NormalizedRegionFormula,
    ParametricCurveDeclaration, Point2, Point3, RegionConstructionRequest, RegionFunction,
    RegionKind, RegionPolynomial3, RegionPolynomialTerm, RootIsolationPolicy, RootRefinementMethod,
    ShapeDeclaration, ShapeEvaluation, SignRelation,
};
use neco_nurbs::{define_nurbs, BezierPoint, NurbsDefinitionRequest};
use neco_tessellation::{tessellate, TessellationRequest};

fn algebraic(value: i32) -> RealAlgebraic {
    RealAlgebraic::from_integer(BigInt::try_from(value).unwrap()).unwrap()
}

fn rational_polynomial(values: &[i32]) -> RationalPolynomial {
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

fn root_policy() -> RootIsolationPolicy {
    RootIsolationPolicy::new(
        8,
        RootRefinementMethod::Bisection,
        Enclosure::new(
            Dyadic::new(BigInt::try_from(0).unwrap(), 0),
            Dyadic::new(BigInt::try_from(1).unwrap(), 0),
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
    let curve = parametric_curve("point-curve", &[-1, 3]);
    construct_intersection(IntersectionConstructionRequest::new(
        IntersectionConstructionRequestBody::new(point, curve, root_policy()),
    ))
    .unwrap()
}

fn curve_intersection() -> neco_geometry::IntersectionSet {
    construct_intersection(IntersectionConstructionRequest::new(
        IntersectionConstructionRequestBody::new(
            parametric_curve("left", &[-1, 3]),
            parametric_curve("right", &[0]),
            root_policy(),
        ),
    ))
    .unwrap()
}

fn parametric_curve(name: &str, numerator_x: &[i32]) -> IntersectionOperand {
    IntersectionOperand::Declaration(ShapeDeclaration::ParametricCurve(
        ParametricCurveDeclaration::new(
            CurveChart::Chart3(
                ChartDefinition3::new(
                    ChartDomainId::new(name.to_string()).unwrap(),
                    ChartDisplayName::new(name.to_string()).unwrap(),
                    rational_polynomial(&[1]),
                    rational_polynomial(numerator_x),
                    rational_polynomial(&[1]),
                    rational_polynomial(&[1]),
                    rational_polynomial(&[1]),
                )
                .unwrap(),
            ),
            neco_geometry::ParameterInterval::new(algebraic(0), algebraic(1), true, true).unwrap(),
        ),
    ))
}

fn boundary_stratification() -> neco_geometry::BoundaryStratification {
    let polynomial = RegionPolynomial3::try_from_terms(vec![RegionPolynomialTerm::try_new(
        algebraic(1),
        1,
        0,
        0,
    )
    .unwrap()])
    .unwrap();
    let atom = CanonicalAtom::try_new(
        "x",
        RegionFunction::Polynomial(polynomial),
        SignRelation::Positive,
    )
    .unwrap();
    let region = construct_region_types(RegionConstructionRequest {
        ambient_dimension: 3,
        canonical_atoms: vec![atom],
        normalized_region_formula: NormalizedRegionFormula::Atom(String::from("x")),
        region_kind: RegionKind::GeneralSemiAlgebraicSet,
    })
    .unwrap()
    .into_region();
    let seed = |selector: Option<usize>| {
        BoundaryCandidateSeed::try_new(
            "x",
            BoundaryRestrictionChart::try_new(
                "chart",
                Enclosure::new(
                    Dyadic::new(BigInt::try_from(-1).unwrap(), 0),
                    Dyadic::new(BigInt::try_from(1).unwrap(), 0),
                )
                .unwrap(),
                Polynomial::from_coefficients(vec![algebraic(0), algebraic(1)]).unwrap(),
                Polynomial::from_coefficients(vec![algebraic(0)]).unwrap(),
                Polynomial::from_coefficients(vec![algebraic(0)]).unwrap(),
            )
            .unwrap(),
            vec![Dyadic::new(BigInt::try_from(0).unwrap(), 0)],
            vec![
                BoundarySideSample::new(Dyadic::new(BigInt::try_from(-1).unwrap(), 0)),
                BoundarySideSample::new(Dyadic::new(BigInt::try_from(1).unwrap(), 0)),
            ],
            selector.map(|index| BoundaryIntersectionSelector::try_new(index, None).unwrap()),
        )
        .unwrap()
    };
    let (_, stratification) = derive_boundary(BoundaryDerivationRequest {
        region,
        intersections: vec![point_intersection(), curve_intersection()],
        candidate_loci: vec![
            seed(Some(0)),
            seed(Some(0)),
            seed(Some(0)),
            seed(Some(1)),
            seed(Some(1)),
            seed(Some(1)),
            seed(None),
        ],
        max_order: DerivativeOrder::new(3),
    })
    .unwrap()
    .into_parts();
    stratification
}

fn arrangement_curve(id: &str) -> ArrangementCurve {
    let polynomial = rational_polynomial(&[1]);
    ArrangementCurve::new(
        CurveIdentity::new(String::from(id), format!("stable-{id}")).unwrap(),
        RepresentationCategory::PolynomialBezier2D,
        ChartDefinition2::new(
            ChartDomainId::new("t".to_string()).unwrap(),
            ChartDisplayName::new("t".to_string()).unwrap(),
            polynomial.try_clone().unwrap(),
            polynomial.try_clone().unwrap(),
            polynomial.try_clone().unwrap(),
            polynomial,
        )
        .unwrap(),
        neco_geometry::ParameterInterval::new(algebraic(0), algebraic(1), true, true).unwrap(),
    )
    .unwrap()
}

fn component_identity(
    intersection: &neco_geometry::IntersectionSet,
) -> IntersectionComponentIdentity {
    match intersection.component_view() {
        IntersectionComponentView::Components(mut entries) => entries.next().unwrap().identity(),
        IntersectionComponentView::NoComponents | IntersectionComponentView::Undetermined(_) => {
            panic!("point intersection has one component")
        }
    }
}

fn point_binding(
    left_curve_id: &str,
    right_curve_id: &str,
    intersection_id: &str,
    left_parameter: i32,
    right_parameter: i32,
) -> ArrangementIntersectionBinding {
    let intersection = point_intersection();
    let identity = component_identity(&intersection);
    ArrangementIntersectionBinding::new(
        String::from(left_curve_id),
        String::from(right_curve_id),
        intersection,
        vec![
            CurveParameterAssociation::new(
                String::from(left_curve_id),
                String::from(intersection_id),
                identity,
                algebraic(left_parameter),
                Some(EndpointRole::Point),
            )
            .unwrap(),
            CurveParameterAssociation::new(
                String::from(right_curve_id),
                String::from(intersection_id),
                identity,
                algebraic(right_parameter),
                Some(EndpointRole::Point),
            )
            .unwrap(),
        ],
        Vec::new(),
    )
    .unwrap()
}

fn endpoint_locator(curve_id: &str, endpoint: Endpoint) -> CurveEventLocator {
    CurveEventLocator::endpoint(String::from(curve_id), endpoint).unwrap()
}

fn point_locator(curve_id: &str, intersection_id: &str, parameter: i32) -> CurveEventLocator {
    let intersection = point_intersection();
    CurveEventLocator::point_intersection(
        String::from(curve_id),
        String::from(intersection_id),
        component_identity(&intersection),
        algebraic(parameter),
    )
    .unwrap()
}

fn arrangement() -> neco_arrangement2d::Arrangement {
    let topology = PlanarTopologyDeclaration::new(
        vec![
            PlanarVertexDeclaration::new(
                String::from("vertex-0"),
                vec![
                    endpoint_locator("a", Endpoint::Lower),
                    point_locator("a", "intersection-ac", 0),
                    endpoint_locator("c", Endpoint::Upper),
                    point_locator("c", "intersection-ac", 1),
                ],
            )
            .unwrap(),
            PlanarVertexDeclaration::new(
                String::from("vertex-1"),
                vec![
                    endpoint_locator("a", Endpoint::Upper),
                    point_locator("a", "intersection-ab", 1),
                    endpoint_locator("b", Endpoint::Lower),
                    point_locator("b", "intersection-ab", 0),
                ],
            )
            .unwrap(),
            PlanarVertexDeclaration::new(
                String::from("vertex-2"),
                vec![
                    endpoint_locator("b", Endpoint::Upper),
                    point_locator("b", "intersection-bc", 1),
                    endpoint_locator("c", Endpoint::Lower),
                    point_locator("c", "intersection-bc", 0),
                ],
            )
            .unwrap(),
        ],
        vec![
            EventRotation::new(
                String::from("vertex-0"),
                vec![
                    String::from("stable-c:2:reverse"),
                    String::from("stable-a:0:forward"),
                ],
            )
            .unwrap(),
            EventRotation::new(
                String::from("vertex-1"),
                vec![
                    String::from("stable-a:0:reverse"),
                    String::from("stable-b:1:forward"),
                ],
            )
            .unwrap(),
            EventRotation::new(
                String::from("vertex-2"),
                vec![
                    String::from("stable-b:1:reverse"),
                    String::from("stable-c:2:forward"),
                ],
            )
            .unwrap(),
        ],
        vec![
            WalkClassification::new(
                String::from("finite"),
                vec![
                    String::from("stable-a:0:forward"),
                    String::from("stable-b:1:forward"),
                    String::from("stable-c:2:forward"),
                ],
                PlanarWalkRole::FiniteOuter,
                Some(PlanarWinding::Counterclockwise),
                None,
            )
            .unwrap(),
            WalkClassification::new(
                String::from("exterior"),
                vec![
                    String::from("stable-a:0:reverse"),
                    String::from("stable-c:2:reverse"),
                    String::from("stable-b:1:reverse"),
                ],
                PlanarWalkRole::Exterior,
                None,
                None,
            )
            .unwrap(),
        ],
    )
    .unwrap();
    construct_arrangement(
        ArrangementConstructionRequest::new(
            vec![
                arrangement_curve("a"),
                arrangement_curve("b"),
                arrangement_curve("c"),
            ],
            vec![
                point_binding("a", "b", "intersection-ab", 1, 0),
                point_binding("a", "c", "intersection-ac", 0, 1),
                point_binding("b", "c", "intersection-bc", 1, 0),
            ],
            topology,
        )
        .unwrap(),
    )
    .unwrap()
}

fn curve_representation(id: &str) -> CurveRepresentation {
    let [(x0, y0), (x1, y1)] = match id {
        "curve-rep-a" => [(0, 0), (0, 1)],
        "curve-rep-b" => [(0, 1), (1, 0)],
        "curve-rep-c" => [(1, 0), (0, 0)],
        _ => panic!("fixture curve representation must be assigned"),
    };
    let value = define_nurbs(NurbsDefinitionRequest::PolynomialCurve {
        control_points: vec![
            BezierPoint::Point2(Point2 {
                x: algebraic(x0),
                y: algebraic(y0),
            }),
            BezierPoint::Point2(Point2 {
                x: algebraic(x1),
                y: algebraic(y1),
            }),
        ],
        knots: vec![algebraic(0), algebraic(0), algebraic(1), algebraic(1)],
        degree: 1,
    })
    .unwrap();
    CurveRepresentation::new(CurveRepresentationId::new(String::from(id)).unwrap(), value).unwrap()
}

fn surface_representation() -> SurfaceRepresentation {
    let value = define_nurbs(NurbsDefinitionRequest::PolynomialSurface {
        control_net: vec![
            vec![
                BezierPoint::Point3(Point3 {
                    x: algebraic(0),
                    y: algebraic(0),
                    z: algebraic(0),
                }),
                BezierPoint::Point3(Point3 {
                    x: algebraic(0),
                    y: algebraic(1),
                    z: algebraic(0),
                }),
            ],
            vec![
                BezierPoint::Point3(Point3 {
                    x: algebraic(1),
                    y: algebraic(0),
                    z: algebraic(0),
                }),
                BezierPoint::Point3(Point3 {
                    x: algebraic(1),
                    y: algebraic(1),
                    z: algebraic(0),
                }),
            ],
        ],
        u_knots: vec![algebraic(0), algebraic(0), algebraic(1), algebraic(1)],
        v_knots: vec![algebraic(0), algebraic(0), algebraic(1), algebraic(1)],
        u_degree: 1,
        v_degree: 1,
    })
    .unwrap();
    SurfaceRepresentation::new(
        SurfaceRepresentationId::new(String::from("surface-rep")).unwrap(),
        value,
    )
    .unwrap()
}

#[test]
fn materialization_accepts_an_owned_request() {
    fn observe(_: fn(BrepMaterializationRequest) -> Result<AnalyticBrep, BrepFailure>) {}
    observe(materialize_analytic_brep);
}

#[test]
fn assignments_construct_topology_geometry_and_witnesses() {
    let stratification = boundary_stratification();
    let dimensions: Vec<_> = stratification
        .strata()
        .iter()
        .map(|stratum| stratum.dimension())
        .collect();
    assert!(matches!(
        dimensions.as_slice(),
        [
            neco_geometry::BoundaryStratumDimension::Dimension(2),
            neco_geometry::BoundaryStratumDimension::Dimension(0),
            neco_geometry::BoundaryStratumDimension::Dimension(0),
            neco_geometry::BoundaryStratumDimension::Dimension(0),
            neco_geometry::BoundaryStratumDimension::Dimension(1),
            neco_geometry::BoundaryStratumDimension::Dimension(1),
            neco_geometry::BoundaryStratumDimension::Dimension(1),
        ]
    ));
    let chart_id = ArrangementId::new(String::from("chart")).unwrap();
    let request = BrepMaterializationRequest::new(
        stratification,
        vec![ChartArrangement::new(chart_id.clone(), arrangement())],
        vec![
            curve_representation("curve-rep-a"),
            curve_representation("curve-rep-b"),
            curve_representation("curve-rep-c"),
        ],
        vec![surface_representation()],
        BrepAssignments::new(
            vec![
                VertexAssignment::new(
                    String::from("boundary-stratum-1"),
                    chart_id.clone(),
                    String::from("vertex-0"),
                )
                .unwrap(),
                VertexAssignment::new(
                    String::from("boundary-stratum-2"),
                    chart_id.clone(),
                    String::from("vertex-1"),
                )
                .unwrap(),
                VertexAssignment::new(
                    String::from("boundary-stratum-3"),
                    chart_id.clone(),
                    String::from("vertex-2"),
                )
                .unwrap(),
            ],
            vec![
                EdgeAssignment::new(
                    String::from("boundary-stratum-4"),
                    chart_id.clone(),
                    String::from("stable-a:0"),
                    CurveRepresentationId::new(String::from("curve-rep-a")).unwrap(),
                )
                .unwrap(),
                EdgeAssignment::new(
                    String::from("boundary-stratum-5"),
                    chart_id.clone(),
                    String::from("stable-b:1"),
                    CurveRepresentationId::new(String::from("curve-rep-b")).unwrap(),
                )
                .unwrap(),
                EdgeAssignment::new(
                    String::from("boundary-stratum-6"),
                    chart_id.clone(),
                    String::from("stable-c:2"),
                    CurveRepresentationId::new(String::from("curve-rep-c")).unwrap(),
                )
                .unwrap(),
            ],
            vec![FaceAssignment::new(
                String::from("boundary-stratum-0"),
                chart_id,
                String::from("face:finite"),
                SurfaceRepresentationId::new(String::from("surface-rep")).unwrap(),
            )
            .unwrap()],
        )
        .unwrap(),
        SourceRevision::new(String::from("source-r1")).unwrap(),
    )
    .unwrap();
    let brep = materialize_analytic_brep(request).unwrap();
    assert_eq!(brep.vertices().len(), 3);
    assert_eq!(brep.edges().len(), 3);
    assert_eq!(brep.incidences().len(), 6);
    assert_eq!(brep.faces().len(), 1);
    assert_eq!(brep.trims().len(), 3);
    assert_eq!(brep.shells().len(), 1);
    assert_eq!(brep.curve_representations().len(), 3);
    assert_eq!(brep.surface_representations().len(), 1);
    assert_eq!(
        brep.faces()[0].surface_representation_id().as_str(),
        "surface-rep"
    );
    assert_eq!(brep.correspondence().assignments().faces().len(), 1);
    assert_eq!(brep.orientation().face_orientations().len(), 1);
    assert_eq!(brep.orientation().shell_orientations().len(), 1);
    assert_eq!(brep.witnesses().closed_shell_ids().len(), 1);
    assert_eq!(brep.witnesses().source_revision().as_str(), "source-r1");

    let result = tessellate(&brep, TessellationRequest::new(0.01).unwrap()).unwrap();
    assert_eq!(
        result
            .validated_surface_mesh()
            .validation_witness()
            .triangle_count(),
        2
    );
    assert_eq!(
        result.brep_element_correspondence().face_ids(),
        &[String::from("face:boundary-stratum-0")]
    );
    assert_eq!(result.source_brep_revision().as_str(), "source-r1");
    assert_eq!(
        result.precision_witness().requested_maximum_chord_error(),
        0.01
    );
    assert_eq!(
        result.precision_witness().established_maximum_chord_error(),
        None
    );
}
