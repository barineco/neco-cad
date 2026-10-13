#![cfg(feature = "test-fixtures")]

use neco_algnum::{RationalPolynomial, RealAlgebraic};
use neco_arrangement2d::{
    construct_arrangement, ArrangementConstructionRequest, ArrangementCurve,
    ArrangementIntersectionBinding, CurveEventLocator, CurveIdentity, CurveParameterAssociation,
    Endpoint, EndpointRole, EventRotation, PlanarTopologyDeclaration, PlanarVertexDeclaration,
    PlanarWalkRole, PlanarWinding, RepresentationCategory, WalkClassification,
};
use neco_bigint::{BigInt, BigUint, Dyadic, RawRational};
use neco_brep::{
    materialize_analytic_brep, ArrangementId, BrepAssignments, BrepMaterializationRequest,
    ChartArrangement, CurveRepresentation, CurveRepresentationId, EdgeAssignment, FaceAssignment,
    SourceRevision, SurfaceRepresentation, SurfaceRepresentationId, VertexAssignment,
};
use neco_cad::{
    construct, derive_analytic_brep, derive_complete_boundary, AdaptivePrecision,
    AdaptiveTessellation, ConstructionOperation, ConstructionOutput, ConstructionRequest,
    EntityIdentityBinding, ExactScalar, GeometryAccuracyPolicy, GeometryClosure,
    GeometryOutputCategory, GeometryOutputExpectation, IdentityLedger, OperationIdentity,
    OperationIdentityBinding, OperationOutputIdentity, OperationRequest, OutputSlot,
    PrimitiveDeclaration, Provenance, SourceIdentity, TessellationConsumer,
};
use neco_cad_view::{
    derive_cad_view_model, pick_cpu, project_cpu_reference, ray_from_viewport_point,
    AchievedPrecision, CadElementIdentity, CadViewFailure, CadViewModelRequest,
    CadViewTessellation, Camera, CorrespondenceClass, CpuPrimitive, CpuProjectionFixture,
    DisplaySettings, PickKind, PickTolerance, Point2, Point3, Selection, Vector3, Viewport,
    Visibility, VisibleRange,
};
use neco_calculus::{DerivativeOrder, Enclosure, Polynomial};
use neco_geometry::{
    construct_intersection, construct_region_types, derive_boundary, BoundaryCandidateSeed,
    BoundaryDerivationRequest, BoundaryIntersectionSelector, BoundaryRestrictionChart,
    BoundarySideSample, CanonicalAtom, ChartDefinition2, ChartDefinition3, ChartDisplayName,
    ChartDomainId, CurveChart, ExactShapeEvaluation, IntersectionComponentIdentity,
    IntersectionComponentView, IntersectionConstructionRequest,
    IntersectionConstructionRequestBody, IntersectionOperand, NormalizedRegionFormula,
    ParametricCurveDeclaration, Point2 as GeometryPoint2, Point3 as GeometryPoint3,
    RegionConstructionRequest, RegionFunction, RegionKind, RegionPolynomial3, RegionPolynomialTerm,
    RootIsolationPolicy, RootRefinementMethod, ShapeDeclaration, ShapeEvaluation, SignRelation,
};
use neco_nurbs::{define_nurbs, BezierPoint, NurbsDefinitionRequest};
use neco_tessellation::{tessellate, TessellationRequest, TessellationResultFixture};

type FixtureMutation = fn(TessellationResultFixture) -> TessellationResultFixture;
type FixtureCase = (CorrespondenceClass, FixtureMutation);

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
            coordinate: GeometryPoint3 {
                x: algebraic(0),
                y: algebraic(0),
                z: algebraic(0),
            },
        },
    ));
    construct_intersection(IntersectionConstructionRequest::new(
        IntersectionConstructionRequestBody::new(
            point,
            parametric_curve("point-curve", &[-1, 3]),
            root_policy(),
        ),
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

fn component_identity(
    intersection: &neco_geometry::IntersectionSet,
) -> IntersectionComponentIdentity {
    match intersection.component_view() {
        IntersectionComponentView::Components(mut entries) => entries.next().unwrap().identity(),
        IntersectionComponentView::NoComponents | IntersectionComponentView::Undetermined(_) => {
            panic!("fixture intersection has one component")
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
            selector
                .map(|index| BoundaryIntersectionSelector::try_new(index, None))
                .transpose()
                .unwrap(),
        )
        .unwrap()
    };
    derive_boundary(BoundaryDerivationRequest {
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
    .into_parts()
    .1
}

fn curve_representation(id: &str) -> CurveRepresentation {
    let [(x0, y0), (x1, y1)] = match id {
        "curve-rep-a" => [(0, 0), (0, 1)],
        "curve-rep-b" => [(0, 1), (1, 0)],
        "curve-rep-c" => [(1, 0), (0, 0)],
        _ => panic!("fixture curve representation must be assigned"),
    };
    CurveRepresentation::new(
        CurveRepresentationId::new(String::from(id)).unwrap(),
        define_nurbs(NurbsDefinitionRequest::PolynomialCurve {
            control_points: vec![
                BezierPoint::Point2(GeometryPoint2 {
                    x: algebraic(x0),
                    y: algebraic(y0),
                }),
                BezierPoint::Point2(GeometryPoint2 {
                    x: algebraic(x1),
                    y: algebraic(y1),
                }),
            ],
            knots: vec![algebraic(0), algebraic(0), algebraic(1), algebraic(1)],
            degree: 1,
        })
        .unwrap(),
    )
    .unwrap()
}

fn surface_representation() -> SurfaceRepresentation {
    SurfaceRepresentation::new(
        SurfaceRepresentationId::new(String::from("surface-rep")).unwrap(),
        define_nurbs(NurbsDefinitionRequest::PolynomialSurface {
            control_net: vec![
                vec![
                    BezierPoint::Point3(GeometryPoint3 {
                        x: algebraic(0),
                        y: algebraic(0),
                        z: algebraic(0),
                    }),
                    BezierPoint::Point3(GeometryPoint3 {
                        x: algebraic(0),
                        y: algebraic(1),
                        z: algebraic(0),
                    }),
                ],
                vec![
                    BezierPoint::Point3(GeometryPoint3 {
                        x: algebraic(1),
                        y: algebraic(0),
                        z: algebraic(0),
                    }),
                    BezierPoint::Point3(GeometryPoint3 {
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
        .unwrap(),
    )
    .unwrap()
}

fn brep(revision: &str) -> neco_brep::AnalyticBrep {
    let chart_id = ArrangementId::new(String::from("chart")).unwrap();
    materialize_analytic_brep(
        BrepMaterializationRequest::new(
            boundary_stratification(),
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
            SourceRevision::new(String::from(revision)).unwrap(),
        )
        .unwrap(),
    )
    .unwrap()
}

fn brep_with_one_assigned_edge(revision: &str) -> neco_brep::AnalyticBrep {
    let chart_id = ArrangementId::new(String::from("chart")).unwrap();
    materialize_analytic_brep(
        BrepMaterializationRequest::new(
            boundary_stratification(),
            vec![ChartArrangement::new(chart_id.clone(), arrangement())],
            vec![curve_representation("curve-rep-a")],
            Vec::new(),
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
                vec![EdgeAssignment::new(
                    String::from("boundary-stratum-4"),
                    chart_id.clone(),
                    String::from("stable-a:0"),
                    CurveRepresentationId::new(String::from("curve-rep-a")).unwrap(),
                )
                .unwrap()],
                Vec::new(),
            )
            .unwrap(),
            SourceRevision::new(String::from(revision)).unwrap(),
        )
        .unwrap(),
    )
    .unwrap()
}

fn camera() -> Camera {
    Camera::orthographic(
        Point3::new(0.5, 0.5, 2.0).unwrap(),
        Point3::new(0.5, 0.5, 0.0).unwrap(),
        Vector3::new(0.0, 1.0, 0.0).unwrap(),
        2.0,
    )
    .unwrap()
}

fn display() -> DisplaySettings {
    DisplaySettings::new(
        Visibility::Visible,
        Visibility::Visible,
        Visibility::Visible,
        Visibility::Visible,
        Visibility::Visible,
        false,
    )
}

fn faces_only() -> DisplaySettings {
    DisplaySettings::new(
        Visibility::Visible,
        Visibility::Hidden,
        Visibility::Hidden,
        Visibility::Hidden,
        Visibility::Hidden,
        false,
    )
}

fn display_with_visibility(
    edges: Visibility,
    trims: Visibility,
    vertices: Visibility,
    control_points: Visibility,
) -> DisplaySettings {
    DisplaySettings::new(
        Visibility::Visible,
        edges,
        trims,
        vertices,
        control_points,
        false,
    )
}

fn projected_identities(
    projection: &neco_cad_view::CpuProjection,
    class: CorrespondenceClass,
) -> std::collections::BTreeSet<CadElementIdentity> {
    projection
        .primitives()
        .iter()
        .filter_map(|primitive| match (class, primitive) {
            (CorrespondenceClass::Face, CpuPrimitive::Triangle { identity, .. })
                if matches!(identity, CadElementIdentity::Face(_)) =>
            {
                Some(identity.clone())
            }
            (
                CorrespondenceClass::Edge,
                CpuPrimitive::Segment {
                    identity: identity @ CadElementIdentity::Edge(_),
                    ..
                },
            )
            | (
                CorrespondenceClass::Trim,
                CpuPrimitive::Segment {
                    identity: identity @ CadElementIdentity::Trim(_),
                    ..
                },
            )
            | (
                CorrespondenceClass::Vertex,
                CpuPrimitive::Point {
                    identity: identity @ CadElementIdentity::Vertex(_),
                    ..
                },
            )
            | (
                CorrespondenceClass::ControlPoint,
                CpuPrimitive::Point {
                    identity: identity @ CadElementIdentity::ControlPoint { .. },
                    ..
                },
            ) => Some(identity.clone()),
            _ => None,
        })
        .collect()
}

fn derive(
    brep: &neco_brep::AnalyticBrep,
    result: &neco_tessellation::TessellationResult,
    request: f64,
) -> Result<neco_cad_view::CadViewModel, CadViewFailure> {
    derive_with_display(brep, result, request, display())
}

fn derive_with_display(
    brep: &neco_brep::AnalyticBrep,
    result: &neco_tessellation::TessellationResult,
    request: f64,
    display: DisplaySettings,
) -> Result<neco_cad_view::CadViewModel, CadViewFailure> {
    derive_with_camera(brep, result, request, camera(), display)
}

fn derive_with_camera(
    brep: &neco_brep::AnalyticBrep,
    result: &neco_tessellation::TessellationResult,
    request: f64,
    camera: Camera,
    display: DisplaySettings,
) -> Result<neco_cad_view::CadViewModel, CadViewFailure> {
    derive_cad_view_model(CadViewModelRequest {
        brep,
        tessellation: CadViewTessellation::Basic {
            result,
            request: TessellationRequest::new(request).unwrap(),
        },
        camera,
        viewport: Viewport::new(100, 100).unwrap(),
        display,
        selection: Selection::new(Vec::new()).unwrap(),
    })
}

fn fixture_result() -> (
    neco_brep::AnalyticBrep,
    neco_tessellation::TessellationResult,
) {
    let brep = brep("source-a");
    let result = tessellate(&brep, TessellationRequest::new(0.01).unwrap()).unwrap();
    (brep, result)
}

fn assert_class(error: CadViewFailure, class: CorrespondenceClass) {
    match error {
        CadViewFailure::MissingCorrespondence { class: actual, .. }
        | CadViewFailure::DuplicateCorrespondenceIdentity { class: actual, .. }
        | CadViewFailure::UnknownCorrespondenceIdentity { class: actual, .. }
        | CadViewFailure::InvalidCorrespondence { class: actual, .. } => assert_eq!(actual, class),
        other => panic!("expected correspondence failure, got {other:?}"),
    }
}

fn assert_class_or_nonfinite(error: CadViewFailure, class: CorrespondenceClass) {
    if matches!(error, CadViewFailure::NonFiniteCoordinate) {
        return;
    }
    assert_class(error, class);
}

#[test]
fn derive_reports_source_revision_and_precision_witness_mismatches() {
    let (brep, result) = fixture_result();
    let mismatch = TessellationResultFixture::from_result(result)
        .source_revision(SourceRevision::new(String::from("source-b")).unwrap())
        .into_result();
    assert!(matches!(
        derive(&brep, &mismatch, 0.01),
        Err(CadViewFailure::SourceRevisionMismatch { .. })
    ));

    let (_, result) = fixture_result();
    let mismatch = TessellationResultFixture::from_result(result)
        .precision(0.02)
        .into_result();
    assert_eq!(
        derive(&brep, &mismatch, 0.01),
        Err(CadViewFailure::RequestedPrecisionMismatch)
    );
}

#[test]
fn derivation_observes_all_correspondence_classes_through_real_tessellation() {
    let cases: Vec<FixtureCase> = vec![
        (
            CorrespondenceClass::FaceTriangle,
            TessellationResultFixture::uncovered_triangle,
        ),
        (
            CorrespondenceClass::Edge,
            TessellationResultFixture::clear_edges,
        ),
        (
            CorrespondenceClass::Trim,
            TessellationResultFixture::clear_trims,
        ),
        (
            CorrespondenceClass::Vertex,
            TessellationResultFixture::clear_vertices,
        ),
        (
            CorrespondenceClass::ControlPoint,
            TessellationResultFixture::clear_control_points,
        ),
    ];
    for (class, mutate) in cases {
        let (brep, result) = fixture_result();
        assert_class(
            derive(
                &brep,
                &mutate(TessellationResultFixture::from_result(result)).into_result(),
                0.01,
            )
            .unwrap_err(),
            class,
        );
    }
}

#[test]
fn derivation_separates_duplicate_unknown_and_invalid_correspondence_classes() {
    let cases: Vec<FixtureCase> = vec![
        (
            CorrespondenceClass::Face,
            TessellationResultFixture::duplicate_face_triangle_correspondence,
        ),
        (CorrespondenceClass::Face, |fixture| {
            fixture.unknown_face_triangle_correspondence("unknown-face")
        }),
        (
            CorrespondenceClass::Face,
            TessellationResultFixture::invalid_face_triangle_correspondence,
        ),
        (
            CorrespondenceClass::Edge,
            TessellationResultFixture::duplicate_edges,
        ),
        (CorrespondenceClass::Edge, |fixture| {
            fixture.unknown_edge("unknown-edge")
        }),
        (
            CorrespondenceClass::Edge,
            TessellationResultFixture::invalid_edges,
        ),
        (
            CorrespondenceClass::Trim,
            TessellationResultFixture::duplicate_trims,
        ),
        (CorrespondenceClass::Trim, |fixture| {
            fixture.unknown_trim("unknown-trim")
        }),
        (
            CorrespondenceClass::Trim,
            TessellationResultFixture::invalid_trims,
        ),
        (
            CorrespondenceClass::Vertex,
            TessellationResultFixture::duplicate_vertices,
        ),
        (CorrespondenceClass::Vertex, |fixture| {
            fixture.unknown_vertex("unknown-vertex")
        }),
        (
            CorrespondenceClass::Vertex,
            TessellationResultFixture::invalid_vertices,
        ),
        (
            CorrespondenceClass::ControlPoint,
            TessellationResultFixture::duplicate_control_points,
        ),
        (CorrespondenceClass::ControlPoint, |fixture| {
            fixture.unknown_control_point("unknown-surface")
        }),
        (
            CorrespondenceClass::ControlPoint,
            TessellationResultFixture::invalid_control_points,
        ),
    ];
    for (class, mutate) in cases {
        let (brep, result) = fixture_result();
        assert_class_or_nonfinite(
            derive(
                &brep,
                &mutate(TessellationResultFixture::from_result(result)).into_result(),
                0.01,
            )
            .unwrap_err(),
            class,
        );
    }
}

#[test]
fn derived_model_projects_and_picks_its_real_face() {
    let (brep, result) = fixture_result();
    let model = derive_with_display(&brep, &result, 0.01, faces_only()).unwrap();
    let projection = project_cpu_reference(&model, VisibleRange::new(0.0, 4.0).unwrap()).unwrap();
    let pick = projection
        .primitives()
        .iter()
        .find_map(|primitive| match primitive {
            CpuPrimitive::Triangle { vertices, .. } => Some(Point2::new(
                (vertices[0].x() + vertices[1].x() + vertices[2].x()) / 3.0,
                (vertices[0].y() + vertices[1].y() + vertices[2].y()) / 3.0,
            )),
            CpuPrimitive::Segment { .. } | CpuPrimitive::Point { .. } => None,
        })
        .unwrap()
        .unwrap();
    let pick = pick_cpu(&projection, pick, PickTolerance::new(0.0).unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(pick.kind(), PickKind::Face);
    assert_eq!(
        pick.identity(),
        &CadElementIdentity::Face(String::from("face:boundary-stratum-0"))
    );
}

#[test]
fn derived_model_rays_follow_pixel_and_projection_mode() {
    let (brep, result) = fixture_result();
    let model = derive(&brep, &result, 0.01).unwrap();
    assert!(matches!(
        model.achieved_precision(),
        AchievedPrecision::NotEstablished { requested }
            if requested.maximum_chord_error() == 0.01
                && requested.maximum_surface_error().is_none()
    ));
    let center = ray_from_viewport_point(&model, Point2::new(49.5, 49.5).unwrap()).unwrap();
    let corner = ray_from_viewport_point(&model, Point2::new(0.0, 0.0).unwrap()).unwrap();
    assert_eq!(center.origin(), &[0.5, 0.5, 2.0]);
    assert_eq!(center.direction(), &[0.0, 0.0, -1.0]);
    assert_ne!(corner.origin(), center.origin());
    assert_eq!(corner.direction(), center.direction());

    let perspective_camera = Camera::perspective(
        Point3::new(0.5, 0.5, 2.0).unwrap(),
        Point3::new(0.5, 0.5, 0.0).unwrap(),
        Vector3::new(0.0, 1.0, 0.0).unwrap(),
        core::f64::consts::FRAC_PI_3,
    )
    .unwrap();
    let perspective =
        derive_with_camera(&brep, &result, 0.01, perspective_camera, display()).unwrap();
    let center = ray_from_viewport_point(&perspective, Point2::new(49.5, 49.5).unwrap()).unwrap();
    let corner = ray_from_viewport_point(&perspective, Point2::new(0.0, 0.0).unwrap()).unwrap();
    assert_eq!(corner.origin(), center.origin());
    assert_ne!(corner.direction(), center.direction());
}

#[test]
fn cpu_projection_applies_each_display_class_without_changing_the_others() {
    let (brep, result) = fixture_result();
    let range = VisibleRange::new(0.0, 4.0).unwrap();
    let all_visible = derive(&brep, &result, 0.01).unwrap();
    let baseline = project_cpu_reference(&all_visible, range).unwrap();
    let classes = [
        CorrespondenceClass::Face,
        CorrespondenceClass::Edge,
        CorrespondenceClass::Trim,
        CorrespondenceClass::Vertex,
        CorrespondenceClass::ControlPoint,
    ];
    for class in classes {
        assert!(
            !projected_identities(&baseline, class).is_empty(),
            "the source-a plane projects {class:?} identities"
        );
    }

    let hidden_cases = [
        (
            CorrespondenceClass::Edge,
            display_with_visibility(
                Visibility::Hidden,
                Visibility::Visible,
                Visibility::Visible,
                Visibility::Visible,
            ),
        ),
        (
            CorrespondenceClass::Trim,
            display_with_visibility(
                Visibility::Visible,
                Visibility::Hidden,
                Visibility::Visible,
                Visibility::Visible,
            ),
        ),
        (
            CorrespondenceClass::Vertex,
            display_with_visibility(
                Visibility::Visible,
                Visibility::Visible,
                Visibility::Hidden,
                Visibility::Visible,
            ),
        ),
        (
            CorrespondenceClass::ControlPoint,
            display_with_visibility(
                Visibility::Visible,
                Visibility::Visible,
                Visibility::Visible,
                Visibility::Hidden,
            ),
        ),
    ];
    for (hidden_class, settings) in hidden_cases {
        let model = derive_with_display(&brep, &result, 0.01, settings).unwrap();
        let projection = project_cpu_reference(&model, range).unwrap();
        assert!(
            projected_identities(&projection, hidden_class).is_empty(),
            "{hidden_class:?} identities are hidden"
        );
        for class in classes {
            if class != hidden_class {
                assert_eq!(
                    projected_identities(&projection, class),
                    projected_identities(&baseline, class),
                    "hiding {hidden_class:?} preserves {class:?} identities"
                );
            }
        }
    }
}

#[test]
fn cpu_projection_visible_range_clips_the_depth_two_face_and_pick() {
    let (brep, result) = fixture_result();
    let model = derive_with_display(&brep, &result, 0.01, faces_only()).unwrap();
    let face_identity = CadElementIdentity::Face(String::from("face:boundary-stratum-0"));
    assert_eq!(model.identity().source_revision().as_str(), "source-a");

    let visible_projection =
        project_cpu_reference(&model, VisibleRange::new(0.0, 4.0).unwrap()).unwrap();
    let (vertices, depth) = visible_projection
        .primitives()
        .iter()
        .find_map(|primitive| match primitive {
            CpuPrimitive::Triangle {
                identity,
                vertices,
                depth,
            } if identity == &face_identity => Some((*vertices, *depth)),
            CpuPrimitive::Triangle { .. }
            | CpuPrimitive::Segment { .. }
            | CpuPrimitive::Point { .. } => None,
        })
        .expect("the full range projects the source-a face");
    assert_eq!(depth, [2.0; 3]);

    let face_point = Point2::new(
        (vertices[0].x() + vertices[1].x() + vertices[2].x()) / 3.0,
        (vertices[0].y() + vertices[1].y() + vertices[2].y()) / 3.0,
    )
    .unwrap();
    let visible_pick = pick_cpu(
        &visible_projection,
        face_point,
        PickTolerance::new(0.0).unwrap(),
    )
    .unwrap()
    .unwrap();
    assert_eq!(visible_pick.identity(), &face_identity);

    let clipped_projection =
        project_cpu_reference(&model, VisibleRange::new(0.0, 1.0).unwrap()).unwrap();
    assert!(
        clipped_projection.primitives().is_empty(),
        "the range ending before depth two emits no source-a primitives"
    );
    assert!(
        pick_cpu(
            &clipped_projection,
            face_point,
            PickTolerance::new(0.0).unwrap()
        )
        .unwrap()
        .is_none(),
        "the clipped face cannot be picked"
    );
}

#[test]
fn public_projection_fixture_orders_equal_candidates_by_kind_then_identity() {
    let (brep, result) = fixture_result();
    let model = derive(&brep, &result, 0.01).unwrap();
    let range = VisibleRange::new(0.0, 4.0).unwrap();
    let point = Point2::new(50.0, 50.0).unwrap();
    let projection = CpuProjectionFixture::from_model(&model, range)
        .with_primitive(CpuPrimitive::Triangle {
            identity: CadElementIdentity::Face(String::from("face-a")),
            vertices: [
                Point2::new(49.0, 49.0).unwrap(),
                Point2::new(51.0, 49.0).unwrap(),
                Point2::new(49.0, 51.0).unwrap(),
            ],
            depth: [1.0; 3],
        })
        .with_primitive(CpuPrimitive::Segment {
            identity: CadElementIdentity::Edge(String::from("edge-a")),
            start: Point2::new(49.0, 50.0).unwrap(),
            end: Point2::new(51.0, 50.0).unwrap(),
            depth: [1.0, 1.0],
        })
        .with_primitive(CpuPrimitive::Segment {
            identity: CadElementIdentity::Trim(String::from("trim-a")),
            start: Point2::new(50.0, 49.0).unwrap(),
            end: Point2::new(50.0, 51.0).unwrap(),
            depth: [1.0, 1.0],
        })
        .with_primitive(CpuPrimitive::Point {
            identity: CadElementIdentity::ControlPoint {
                surface_representation_id: String::from("surface"),
                row: 0,
                column: 0,
            },
            position: point,
            depth: 1.0,
        })
        .with_primitive(CpuPrimitive::Point {
            identity: CadElementIdentity::Vertex(String::from("vertex-z")),
            position: point,
            depth: 1.0,
        })
        .with_primitive(CpuPrimitive::Point {
            identity: CadElementIdentity::Vertex(String::from("vertex-a")),
            position: point,
            depth: 1.0,
        })
        .into_projection();
    let pick = pick_cpu(&projection, point, PickTolerance::new(0.0).unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(pick.kind(), PickKind::Vertex);
    assert_eq!(
        pick.identity(),
        &CadElementIdentity::Vertex(String::from("vertex-a"))
    );
}

fn sphere_region() -> neco_geometry::AlgebraicRegion {
    let revision = SourceRevision::new(String::from("owned-r1")).unwrap();
    let operation_identity = OperationIdentity::new("sphere").unwrap();
    let source_identity = SourceIdentity::new("sphere-source").unwrap();
    let output_identity = OperationOutputIdentity {
        operation: operation_identity.clone(),
        slot: OutputSlot::Primary,
    };
    let operation = ConstructionOperation {
        operation: operation_identity.clone(),
        source_object: source_identity.clone(),
        expected_output: GeometryOutputExpectation {
            category: GeometryOutputCategory::Solid,
            ambient_dimension: 3,
            manifold_dimension: 3,
            closure: GeometryClosure::Closed,
        },
        request: OperationRequest::Primitive(PrimitiveDeclaration::Sphere {
            radius: ExactScalar::new(1.0).unwrap(),
        }),
    };
    let identity_ledger = IdentityLedger::new(
        vec![OperationIdentityBinding {
            source: source_identity.clone(),
            operation: operation_identity.clone(),
            output: output_identity.clone(),
        }],
        vec![EntityIdentityBinding {
            source: source_identity,
            operation: operation_identity,
            output: output_identity.clone(),
            produced: String::from("sphere:primary"),
        }],
    )
    .unwrap();
    construct(ConstructionRequest {
        source_revision: revision.clone(),
        precision: GeometryAccuracyPolicy::ExactOnly,
        identity_ledger,
        operations: vec![operation],
        roots: Vec::new(),
        provenance: Provenance::new(
            "schema",
            "ns",
            revision,
            "adapter",
            "exact",
            Vec::new(),
            Vec::new(),
        ),
    })
    .unwrap()
    .into_outputs()
    .into_iter()
    .find_map(|(identity, output)| (identity == output_identity).then_some(output))
    .and_then(|output| match output {
        ConstructionOutput::Region(region) => Some(region),
        ConstructionOutput::AnalyticRepresentation(_) | ConstructionOutput::ImportedRegion(_) => {
            None
        }
    })
    .unwrap()
}

fn adaptive_sphere_fixture(
    consumer: TessellationConsumer,
) -> (
    neco_brep::AnalyticBrep,
    AdaptiveTessellation,
    SourceIdentity,
    AdaptivePrecision,
) {
    let region_for_mesh = sphere_region();
    let region_for_view = sphere_region();
    let view_derived = derive_analytic_brep(
        derive_complete_boundary(region_for_view).unwrap(),
        SourceRevision::new(String::from("owned-r1")).unwrap(),
    )
    .unwrap();
    let (brep, _) = view_derived.into_parts();
    let derived = derive_analytic_brep(
        derive_complete_boundary(region_for_mesh).unwrap(),
        SourceRevision::new(String::from("owned-r1")).unwrap(),
    )
    .unwrap();
    let source_identity = SourceIdentity::new("mesh:owned-pipeline").unwrap();
    let precision = AdaptivePrecision::new(consumer, 0.5, 1.0).unwrap();
    let request = derived
        .into_adaptive_tessellation_request(source_identity.clone(), precision, 48)
        .unwrap();
    let tessellation = neco_cad::tessellate_adaptive(request).unwrap();
    (brep, tessellation, source_identity, precision)
}

#[test]
fn adaptive_sphere_flows_through_view_projection_and_picking() {
    let (sphere_brep, tessellation, source_identity, precision) =
        adaptive_sphere_fixture(TessellationConsumer::Analysis);
    let request = |brep, expected_source_identity, request| CadViewModelRequest {
        brep,
        tessellation: CadViewTessellation::Adaptive {
            result: &tessellation,
            request,
            expected_source_identity,
        },
        camera: camera(),
        viewport: Viewport::new(100, 100).unwrap(),
        display: display(),
        selection: Selection::new(Vec::new()).unwrap(),
    };
    let model = derive_cad_view_model(request(&sphere_brep, &source_identity, precision)).unwrap();
    assert_eq!(model.identity().source_revision().as_str(), "owned-r1");
    assert_eq!(model.identity().mesh_identity(), Some(&source_identity));
    let witness = tessellation.precision_witness();
    let requested = model.requested_precision();
    assert_eq!(
        requested.maximum_chord_error(),
        precision.maximum_chord_error()
    );
    assert_eq!(
        requested.maximum_surface_error(),
        Some(precision.maximum_surface_error())
    );
    let certified = match model.achieved_precision() {
        AchievedPrecision::Certified(value) => value,
        AchievedPrecision::NotEstablished { .. } => {
            panic!("adaptive precision remains certified in the view")
        }
    };
    assert_eq!(
        witness.established_maximum_chord_error(),
        certified.established_maximum_chord_error()
    );
    assert_eq!(
        witness.established_maximum_surface_error(),
        certified.established_maximum_surface_error()
    );
    assert_ne!(
        witness.established_maximum_chord_error(),
        0.0,
        "a zero chord bound inside the request is not the producer witness"
    );
    assert_ne!(
        witness.established_maximum_surface_error(),
        &Dyadic::new(BigInt::try_from(0).unwrap(), 0),
        "a zero surface bound inside the request is not the producer witness"
    );
    assert!(witness.established_maximum_chord_error() <= precision.maximum_chord_error());
    assert!(
        witness.established_maximum_surface_error()
            <= &Dyadic::from_f64_exact(precision.maximum_surface_error()).unwrap()
    );
    assert_ne!(
        witness.established_maximum_chord_error(),
        witness.requested_maximum_chord_error(),
        "the established chord bound retains its measured value"
    );
    assert_ne!(
        witness.established_maximum_surface_error(),
        &Dyadic::from_f64_exact(witness.requested_maximum_surface_error()).unwrap(),
        "the established surface bound retains its exact dyadic value"
    );
    let ray = ray_from_viewport_point(&model, Point2::new(49.5, 49.5).unwrap()).unwrap();
    assert_eq!(ray.origin(), &[0.5, 0.5, 2.0]);
    assert_eq!(ray.direction(), &[0.0, 0.0, -1.0]);

    let projection = project_cpu_reference(&model, VisibleRange::new(0.0, 4.0).unwrap()).unwrap();
    let vertices = projection
        .primitives()
        .iter()
        .find_map(|primitive| match primitive {
            CpuPrimitive::Triangle { vertices, .. } => Some(*vertices),
            CpuPrimitive::Segment { .. } | CpuPrimitive::Point { .. } => None,
        })
        .unwrap();
    let point = Point2::new(
        (vertices[0].x() + vertices[1].x() + vertices[2].x()) / 3.0,
        (vertices[0].y() + vertices[1].y() + vertices[2].y()) / 3.0,
    )
    .unwrap();
    let pick = pick_cpu(&projection, point, PickTolerance::new(0.0).unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(pick.kind(), PickKind::Face);

    let wrong_identity = SourceIdentity::new("mesh:wrong").unwrap();
    assert!(matches!(
        derive_cad_view_model(request(&sphere_brep, &wrong_identity, precision)),
        Err(CadViewFailure::SourceIdentityMismatch { .. })
    ));
    let wrong_request = AdaptivePrecision::new(TessellationConsumer::Analysis, 0.25, 1.0).unwrap();
    assert_eq!(
        derive_cad_view_model(request(&sphere_brep, &source_identity, wrong_request)),
        Err(CadViewFailure::RequestedPrecisionMismatch)
    );
    let wrong_consumer = AdaptivePrecision::new(TessellationConsumer::Picking, 0.5, 1.0).unwrap();
    assert!(matches!(
        derive_cad_view_model(request(&sphere_brep, &source_identity, wrong_consumer)),
        Err(CadViewFailure::PrecisionConsumerMismatch { .. })
    ));
    let different_revision = brep("source-a");
    assert!(matches!(
        derive_cad_view_model(request(&different_revision, &source_identity, precision,)),
        Err(CadViewFailure::SourceRevisionMismatch { .. })
    ));
}

#[test]
fn certified_view_preserves_display_and_picking_consumers() {
    for consumer in [TessellationConsumer::Display, TessellationConsumer::Picking] {
        let (sphere_brep, tessellation, source_identity, precision) =
            adaptive_sphere_fixture(consumer);
        let model = derive_cad_view_model(CadViewModelRequest {
            brep: &sphere_brep,
            tessellation: CadViewTessellation::Adaptive {
                result: &tessellation,
                request: precision,
                expected_source_identity: &source_identity,
            },
            camera: camera(),
            viewport: Viewport::new(100, 100).unwrap(),
            display: display(),
            selection: Selection::new(Vec::new()).unwrap(),
        })
        .unwrap();
        assert!(matches!(
            model.achieved_precision(),
            AchievedPrecision::Certified(_)
        ));
        let projection =
            project_cpu_reference(&model, VisibleRange::new(0.0, 4.0).unwrap()).unwrap();
        assert!(projection
            .primitives()
            .iter()
            .any(|primitive| matches!(primitive, CpuPrimitive::Triangle { .. })));
    }
}

#[test]
fn view_preserves_vertex_helper_failure_identity() {
    let brep = brep_with_one_assigned_edge("vertex-helper-r1");
    let expected = neco_tessellation::TessellationFailure::VertexPositionMissing {
        vertex_id: String::from("vertex:boundary-stratum-3"),
    };
    let helper_failure = neco_cad::tessellate_view_elements(&brep).unwrap_err();
    assert_eq!(helper_failure, expected);

    let cad_failure: neco_cad::CadFailure = helper_failure.into();
    assert!(matches!(
        cad_failure,
        neco_cad::CadFailure::Tessellation(error) if *error == expected
    ));
}
