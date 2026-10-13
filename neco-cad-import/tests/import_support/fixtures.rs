use neco_algnum::{RationalPolynomial, RealAlgebraic};
use neco_arrangement2d::{
    construct_arrangement, ArrangementConstructionRequest, ArrangementCurve,
    ArrangementIntersectionBinding, CurveEventLocator, CurveIdentity, CurveParameterAssociation,
    Endpoint, EndpointRole, EventRotation, PlanarTopologyDeclaration, PlanarVertexDeclaration,
    PlanarWalkRole, PlanarWinding, RepresentationCategory, WalkClassification,
};
use neco_bigint::{BigInt, BigUint, Dyadic, RawRational};
use neco_brep::{
    materialize_analytic_brep, AnalyticBrep, ArrangementId, BrepAssignments,
    BrepMaterializationRequest, ChartArrangement, CurveRepresentation, CurveRepresentationId,
    EdgeAssignment, FaceAssignment, SourceRevision, SurfaceRepresentation, SurfaceRepresentationId,
    VertexAssignment,
};
use neco_calculus::{DerivativeOrder, Enclosure, Polynomial};
use neco_geometry::{
    construct_intersection, construct_region_types, derive_boundary, BoundaryCandidateSeed,
    BoundaryDerivationRequest, BoundaryIntersectionSelector, BoundaryRestrictionChart,
    BoundarySideSample, BoundaryStratumDimension, CanonicalAtom, ChartDefinition2,
    ChartDefinition3, ChartDisplayName, ChartDomainId, CurveChart, ExactShapeEvaluation,
    IntersectionComponentIdentity, IntersectionComponentView, IntersectionConstructionRequest,
    IntersectionConstructionRequestBody, IntersectionOperand, NormalizedRegionFormula,
    ParametricCurveDeclaration, Point2, Point3, RegionConstructionRequest, RegionFunction,
    RegionKind, RegionPolynomial3, RegionPolynomialTerm, RootIsolationPolicy, RootRefinementMethod,
    ShapeDeclaration, ShapeEvaluation, SignRelation,
};
use neco_nurbs::{define_nurbs, BezierPoint, NurbsDefinitionRequest};

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

fn parametric_curve(
    name: &str,
    numerator_x: &[i32],
    numerator_y: &[i32],
    numerator_z: &[i32],
) -> IntersectionOperand {
    IntersectionOperand::Declaration(ShapeDeclaration::ParametricCurve(
        ParametricCurveDeclaration::new(
            CurveChart::Chart3(
                ChartDefinition3::new(
                    ChartDomainId::new(name.to_string()).unwrap(),
                    ChartDisplayName::new(name.to_string()).unwrap(),
                    rational_polynomial(&[1]),
                    rational_polynomial(numerator_x),
                    rational_polynomial(numerator_y),
                    rational_polynomial(numerator_z),
                    rational_polynomial(&[1]),
                )
                .unwrap(),
            ),
            neco_geometry::ParameterInterval::new(algebraic(0), algebraic(1), true, true).unwrap(),
        ),
    ))
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
    let curve = parametric_curve("point-curve", &[-1, 3], &[1], &[1]);
    construct_intersection(IntersectionConstructionRequest::new(
        IntersectionConstructionRequestBody::new(point, curve, root_policy()),
    ))
    .unwrap()
}

fn curve_intersection() -> neco_geometry::IntersectionSet {
    construct_intersection(IntersectionConstructionRequest::new(
        IntersectionConstructionRequestBody::new(
            parametric_curve("left", &[-1, 3], &[1], &[1]),
            parametric_curve("right", &[0], &[1], &[1]),
            root_policy(),
        ),
    ))
    .unwrap()
}

fn seed(selector: Option<usize>) -> BoundaryCandidateSeed {
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
}

fn region() -> neco_geometry::AlgebraicRegion {
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
    construct_region_types(RegionConstructionRequest {
        ambient_dimension: 3,
        canonical_atoms: vec![atom],
        normalized_region_formula: NormalizedRegionFormula::Atom(String::from("x")),
        region_kind: RegionKind::GeneralSemiAlgebraicSet,
    })
    .unwrap()
    .into_region()
}

fn boundary_stratification(copies: usize) -> neco_geometry::BoundaryStratification {
    let mut loci = Vec::new();
    for _ in 0..copies {
        loci.extend([
            seed(Some(0)),
            seed(Some(0)),
            seed(Some(0)),
            seed(Some(1)),
            seed(Some(1)),
            seed(Some(1)),
            seed(None),
        ]);
    }
    let (_, stratification) = derive_boundary(BoundaryDerivationRequest {
        region: region(),
        intersections: vec![point_intersection(), curve_intersection()],
        candidate_loci: loci,
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

fn triangle_arrangement() -> neco_arrangement2d::Arrangement {
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

fn curve_representation(id: &str, start: (i32, i32), end: (i32, i32)) -> CurveRepresentation {
    let value = define_nurbs(NurbsDefinitionRequest::PolynomialCurve {
        control_points: vec![
            BezierPoint::Point2(Point2 {
                x: algebraic(start.0),
                y: algebraic(start.1),
            }),
            BezierPoint::Point2(Point2 {
                x: algebraic(end.0),
                y: algebraic(end.1),
            }),
        ],
        knots: vec![algebraic(0), algebraic(0), algebraic(1), algebraic(1)],
        degree: 1,
    })
    .unwrap();
    CurveRepresentation::new(CurveRepresentationId::new(String::from(id)).unwrap(), value).unwrap()
}

fn bilinear_surface(id: &str, net: [[(i32, i32, i32); 2]; 2]) -> SurfaceRepresentation {
    let point = |value: (i32, i32, i32)| {
        BezierPoint::Point3(Point3 {
            x: algebraic(value.0),
            y: algebraic(value.1),
            z: algebraic(value.2),
        })
    };
    let value = define_nurbs(NurbsDefinitionRequest::PolynomialSurface {
        control_net: vec![
            vec![point(net[0][0]), point(net[0][1])],
            vec![point(net[1][0]), point(net[1][1])],
        ],
        u_knots: vec![algebraic(0), algebraic(0), algebraic(1), algebraic(1)],
        v_knots: vec![algebraic(0), algebraic(0), algebraic(1), algebraic(1)],
        u_degree: 1,
        v_degree: 1,
    })
    .unwrap();
    SurfaceRepresentation::new(
        SurfaceRepresentationId::new(String::from(id)).unwrap(),
        value,
    )
    .unwrap()
}

fn degree_two_surface() -> SurfaceRepresentation {
    let point = |x: i32, y: i32, z: i32| {
        BezierPoint::Point3(Point3 {
            x: algebraic(x),
            y: algebraic(y),
            z: algebraic(z),
        })
    };
    let value = define_nurbs(NurbsDefinitionRequest::PolynomialSurface {
        control_net: vec![
            vec![point(0, 0, 0), point(0, 1, 0), point(0, 2, 0)],
            vec![point(1, 0, 0), point(1, 1, 0), point(1, 2, 0)],
            vec![point(2, 0, 0), point(2, 1, 0), point(2, 2, 0)],
        ],
        u_knots: vec![
            algebraic(0),
            algebraic(0),
            algebraic(0),
            algebraic(1),
            algebraic(1),
            algebraic(1),
        ],
        v_knots: vec![
            algebraic(0),
            algebraic(0),
            algebraic(0),
            algebraic(1),
            algebraic(1),
            algebraic(1),
        ],
        u_degree: 2,
        v_degree: 2,
    })
    .unwrap();
    SurfaceRepresentation::new(
        SurfaceRepresentationId::new(String::from("surface-rep")).unwrap(),
        value,
    )
    .unwrap()
}

fn strata_by_dimension(
    stratification: &neco_geometry::BoundaryStratification,
    dimension: i64,
) -> Vec<String> {
    stratification
        .strata()
        .iter()
        .filter(|stratum| {
            matches!(
                stratum.dimension(),
                BoundaryStratumDimension::Dimension(value) if *value == dimension
            )
        })
        .map(|stratum| String::from(stratum.stratum_id()))
        .collect()
}

fn first_shell_id(brep: &AnalyticBrep) -> String {
    String::from(brep.shells()[0].id())
}

fn volume_or_first(brep: &AnalyticBrep) -> String {
    brep.shells()
        .iter()
        .find(|shell| shell.id() == "shell:volume")
        .map(|shell| String::from(shell.id()))
        .unwrap_or_else(|| first_shell_id(brep))
}

fn triangle_request(
    chart_id: &str,
    stratification: neco_geometry::BoundaryStratification,
    surface: SurfaceRepresentation,
    revision: &str,
) -> BrepMaterializationRequest {
    let dim0 = strata_by_dimension(&stratification, 0);
    let dim1 = strata_by_dimension(&stratification, 1);
    let dim2 = strata_by_dimension(&stratification, 2);
    let chart = ArrangementId::new(String::from(chart_id)).unwrap();
    let surface_identity = surface_id(&surface);
    BrepMaterializationRequest::new(
        stratification,
        vec![ChartArrangement::new(chart.clone(), triangle_arrangement())],
        vec![
            curve_representation("curve-rep-a", (0, 0), (0, 1)),
            curve_representation("curve-rep-b", (0, 1), (1, 0)),
            curve_representation("curve-rep-c", (1, 0), (0, 0)),
        ],
        vec![surface],
        BrepAssignments::new(
            vec![
                VertexAssignment::new(dim0[0].clone(), chart.clone(), String::from("vertex-0"))
                    .unwrap(),
                VertexAssignment::new(dim0[1].clone(), chart.clone(), String::from("vertex-1"))
                    .unwrap(),
                VertexAssignment::new(dim0[2].clone(), chart.clone(), String::from("vertex-2"))
                    .unwrap(),
            ],
            vec![
                EdgeAssignment::new(
                    dim1[0].clone(),
                    chart.clone(),
                    String::from("stable-a:0"),
                    CurveRepresentationId::new(String::from("curve-rep-a")).unwrap(),
                )
                .unwrap(),
                EdgeAssignment::new(
                    dim1[1].clone(),
                    chart.clone(),
                    String::from("stable-b:1"),
                    CurveRepresentationId::new(String::from("curve-rep-b")).unwrap(),
                )
                .unwrap(),
                EdgeAssignment::new(
                    dim1[2].clone(),
                    chart.clone(),
                    String::from("stable-c:2"),
                    CurveRepresentationId::new(String::from("curve-rep-c")).unwrap(),
                )
                .unwrap(),
            ],
            vec![FaceAssignment::new(
                dim2[0].clone(),
                chart,
                String::from("face:finite"),
                SurfaceRepresentationId::new(surface_identity).unwrap(),
            )
            .unwrap()],
        )
        .unwrap(),
        SourceRevision::new(String::from(revision)).unwrap(),
    )
    .unwrap()
}

fn surface_id(surface: &SurfaceRepresentation) -> String {
    String::from(surface.id().as_str())
}

fn hole_surface() -> SurfaceRepresentation {
    bilinear_surface(
        "surface-rep",
        [[(1, 0, 0), (1, 0, 1)], [(1, 1, 0), (1, 1, 1)]],
    )
}

pub fn triangle_shell_brep() -> (AnalyticBrep, String) {
    let brep = materialize_analytic_brep(triangle_request(
        "chart",
        boundary_stratification(1),
        bilinear_surface(
            "surface-rep",
            [[(0, 0, 0), (0, 1, 0)], [(1, 0, 0), (1, 1, 0)]],
        ),
        "source-r1",
    ))
    .unwrap();
    let shell_id = first_shell_id(&brep);
    (brep, shell_id)
}

pub fn insufficient_shell_brep() -> (AnalyticBrep, String) {
    let brep = materialize_analytic_brep(triangle_request(
        "chart",
        boundary_stratification(1),
        degree_two_surface(),
        "source-r1",
    ))
    .unwrap();
    let shell_id = first_shell_id(&brep);
    (brep, shell_id)
}

pub fn hole_shell_brep() -> (AnalyticBrep, String) {
    let brep = materialize_analytic_brep(triangle_request(
        "hole",
        boundary_stratification(1),
        hole_surface(),
        "source-r1",
    ))
    .unwrap();
    let shell_id = first_shell_id(&brep);
    (brep, shell_id)
}

pub fn seam_shell_brep() -> (AnalyticBrep, String) {
    let brep = materialize_analytic_brep(triangle_request(
        "seam",
        boundary_stratification(1),
        hole_surface(),
        "source-r1",
    ))
    .unwrap();
    let shell_id = first_shell_id(&brep);
    (brep, shell_id)
}

pub fn unresolved_seam_brep() -> (AnalyticBrep, String) {
    let brep = materialize_analytic_brep(triangle_request(
        "unresolved",
        boundary_stratification(1),
        hole_surface(),
        "source-r1",
    ))
    .unwrap();
    let shell_id = first_shell_id(&brep);
    (brep, shell_id)
}

pub fn tetrahedral_shell_brep() -> (AnalyticBrep, String) {
    let stratification = boundary_stratification(4);
    let dim0 = strata_by_dimension(&stratification, 0);
    let dim1 = strata_by_dimension(&stratification, 1);
    let dim2 = strata_by_dimension(&stratification, 2);
    assert!(
        dim0.len() >= 12 && dim1.len() >= 12 && dim2.len() >= 4,
        "expanded stratification must supply tet assignments: dim0={} dim1={} dim2={}",
        dim0.len(),
        dim1.len(),
        dim2.len()
    );
    let surfaces = [
        bilinear_surface(
            "surface-z0",
            [[(0, 0, 0), (0, 4, 0)], [(4, 0, 0), (4, 4, 0)]],
        ),
        bilinear_surface(
            "surface-y0",
            [[(0, 0, 0), (0, 0, 4)], [(4, 0, 0), (4, 0, 4)]],
        ),
        bilinear_surface(
            "surface-x0",
            [[(0, 0, 0), (0, 0, 4)], [(0, 4, 0), (0, 4, 4)]],
        ),
        bilinear_surface(
            "surface-hyp",
            [[(4, 0, 0), (0, 0, 4)], [(0, 4, 0), (-4, 4, 4)]],
        ),
    ];
    let mut vertices = Vec::new();
    let mut edges = Vec::new();
    let mut faces = Vec::new();
    let mut charts = Vec::new();
    for index in 0..4 {
        let chart_id = format!("tet-{index}");
        let chart = ArrangementId::new(chart_id.clone()).unwrap();
        charts.push(ChartArrangement::new(chart.clone(), triangle_arrangement()));
        let base0 = index * 3;
        vertices.push(
            VertexAssignment::new(dim0[base0].clone(), chart.clone(), String::from("vertex-0"))
                .unwrap(),
        );
        vertices.push(
            VertexAssignment::new(
                dim0[base0 + 1].clone(),
                chart.clone(),
                String::from("vertex-1"),
            )
            .unwrap(),
        );
        vertices.push(
            VertexAssignment::new(
                dim0[base0 + 2].clone(),
                chart.clone(),
                String::from("vertex-2"),
            )
            .unwrap(),
        );
        edges.push(
            EdgeAssignment::new(
                dim1[base0].clone(),
                chart.clone(),
                String::from("stable-a:0"),
                CurveRepresentationId::new(String::from("curve-rep-a")).unwrap(),
            )
            .unwrap(),
        );
        edges.push(
            EdgeAssignment::new(
                dim1[base0 + 1].clone(),
                chart.clone(),
                String::from("stable-b:1"),
                CurveRepresentationId::new(String::from("curve-rep-b")).unwrap(),
            )
            .unwrap(),
        );
        edges.push(
            EdgeAssignment::new(
                dim1[base0 + 2].clone(),
                chart.clone(),
                String::from("stable-c:2"),
                CurveRepresentationId::new(String::from("curve-rep-c")).unwrap(),
            )
            .unwrap(),
        );
        faces.push(
            FaceAssignment::new(
                dim2[index].clone(),
                chart,
                String::from("face:finite"),
                SurfaceRepresentationId::new(String::from(surfaces[index].id().as_str())).unwrap(),
            )
            .unwrap(),
        );
    }
    let brep = materialize_analytic_brep(
        BrepMaterializationRequest::new(
            stratification,
            charts,
            vec![
                curve_representation("curve-rep-a", (0, 0), (0, 1)),
                curve_representation("curve-rep-b", (0, 1), (1, 0)),
                curve_representation("curve-rep-c", (1, 0), (0, 0)),
            ],
            surfaces.into(),
            BrepAssignments::new(vertices, edges, faces).unwrap(),
            SourceRevision::new(String::from("source-r1")).unwrap(),
        )
        .unwrap(),
    )
    .unwrap();
    let shell_id = volume_or_first(&brep);
    (brep, shell_id)
}
