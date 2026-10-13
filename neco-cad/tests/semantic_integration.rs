use neco_cad::*;

fn id(value: &str) -> OperationIdentity {
    OperationIdentity::new(value).unwrap()
}

fn source(value: &str) -> SourceIdentity {
    SourceIdentity::new(value).unwrap()
}

fn scalar(value: f64) -> ExactScalar {
    ExactScalar::new(value).unwrap()
}

fn point() -> ExactPoint3 {
    ExactPoint3::new([0., 0., 0.]).unwrap()
}

fn vector() -> ExactVector3 {
    ExactVector3::new([1., 0., 0.]).unwrap()
}

fn z_axis() -> ExactVector3 {
    ExactVector3::new([0., 0., 1.]).unwrap()
}

fn y_axis() -> ExactVector3 {
    ExactVector3::new([0., 1., 0.]).unwrap()
}

fn placement() -> ExactRigidPlacement {
    ExactRigidPlacement::new(point(), [1., 0., 0., 0.]).unwrap()
}

fn output(name: &str) -> OperationOutputRef {
    OperationOutputRef::primary(id(name))
}

fn expected(category: GeometryOutputCategory) -> GeometryOutputExpectation {
    GeometryOutputExpectation {
        category,
        ambient_dimension: 3,
        manifold_dimension: match category {
            GeometryOutputCategory::Curve => 1,
            GeometryOutputCategory::Profile | GeometryOutputCategory::Sheet => 2,
            GeometryOutputCategory::Solid => 3,
        },
        closure: match category {
            GeometryOutputCategory::Profile | GeometryOutputCategory::Solid => {
                GeometryClosure::Closed
            }
            _ => GeometryClosure::Open,
        },
    }
}

fn op(
    name: &str,
    request: OperationRequest,
    category: GeometryOutputCategory,
) -> ConstructionOperation {
    ConstructionOperation {
        operation: id(name),
        source_object: source(name),
        expected_output: expected(category),
        request,
    }
}

fn ledger(operations: &[ConstructionOperation]) -> IdentityLedger {
    IdentityLedger::new(
        operations
            .iter()
            .map(|operation| OperationIdentityBinding {
                source: operation.source_object.clone(),
                operation: operation.operation.clone(),
                output: operation.output_identity(),
            })
            .collect(),
        operations
            .iter()
            .map(|operation| EntityIdentityBinding {
                source: operation.source_object.clone(),
                operation: operation.operation.clone(),
                output: operation.output_identity(),
                produced: format!("{}:primary", operation.operation.as_str()),
            })
            .collect(),
    )
    .unwrap()
}

fn provenance(revision: SourceRevision) -> Provenance {
    Provenance::new("schema", "ns", revision, "adapter", "exact", vec![], vec![])
}

fn request_with_revision(
    operations: Vec<ConstructionOperation>,
    revision: &str,
) -> ConstructionRequest {
    let revision = SourceRevision::new(revision.into()).unwrap();
    ConstructionRequest {
        source_revision: revision.clone(),
        precision: GeometryAccuracyPolicy::ExactOnly,
        identity_ledger: ledger(&operations),
        operations,
        roots: vec![],
        provenance: provenance(revision),
    }
}

fn request(operations: Vec<ConstructionOperation>) -> ConstructionRequest {
    request_with_revision(operations, "r1")
}

fn identity_of(name: &str) -> OperationOutputIdentity {
    OperationOutputIdentity {
        operation: id(name),
        slot: OutputSlot::Primary,
    }
}

fn primitive(name: &str, declaration: PrimitiveDeclaration) -> ConstructionOperation {
    op(
        name,
        OperationRequest::Primitive(declaration),
        GeometryOutputCategory::Solid,
    )
}

fn sphere(name: &str) -> ConstructionOperation {
    primitive(name, PrimitiveDeclaration::Sphere { radius: scalar(1.) })
}

fn torus(name: &str) -> ConstructionOperation {
    primitive(
        name,
        PrimitiveDeclaration::Torus {
            major_radius: scalar(2.),
            minor_radius: scalar(1.),
        },
    )
}

fn closed_square_profile(name: &str) -> ConstructionOperation {
    op(
        name,
        OperationRequest::Profile(ProfileRequest {
            origin: point(),
            x_axis: vector(),
            normal: z_axis(),
            corners: vec![
                ProfileCorner {
                    id: "a".into(),
                    position: [scalar(-1.), scalar(-1.)],
                },
                ProfileCorner {
                    id: "b".into(),
                    position: [scalar(1.), scalar(-1.)],
                },
                ProfileCorner {
                    id: "c".into(),
                    position: [scalar(1.), scalar(1.)],
                },
                ProfileCorner {
                    id: "d".into(),
                    position: [scalar(-1.), scalar(1.)],
                },
            ],
            segments: vec![
                ProfileSegment {
                    id: "ab".into(),
                    start_corner: "a".into(),
                    end_corner: "b".into(),
                    curve: ProfileSegmentCurve::Line,
                },
                ProfileSegment {
                    id: "bc".into(),
                    start_corner: "b".into(),
                    end_corner: "c".into(),
                    curve: ProfileSegmentCurve::Line,
                },
                ProfileSegment {
                    id: "cd".into(),
                    start_corner: "c".into(),
                    end_corner: "d".into(),
                    curve: ProfileSegmentCurve::Line,
                },
                ProfileSegment {
                    id: "da".into(),
                    start_corner: "d".into(),
                    end_corner: "a".into(),
                    curve: ProfileSegmentCurve::Line,
                },
            ],
            corner_order: vec!["a".into(), "b".into(), "c".into(), "d".into()],
            segment_order: vec!["ab".into(), "bc".into(), "cd".into(), "da".into()],
            closure: GeometryClosure::Closed,
        }),
        GeometryOutputCategory::Profile,
    )
}

fn revolve_rectangle_profile(name: &str) -> ConstructionOperation {
    op(
        name,
        OperationRequest::Profile(ProfileRequest {
            origin: point(),
            x_axis: vector(),
            normal: z_axis(),
            corners: vec![
                ProfileCorner {
                    id: "a".into(),
                    position: [scalar(0.), scalar(0.)],
                },
                ProfileCorner {
                    id: "b".into(),
                    position: [scalar(1.), scalar(0.)],
                },
                ProfileCorner {
                    id: "c".into(),
                    position: [scalar(1.), scalar(2.)],
                },
                ProfileCorner {
                    id: "d".into(),
                    position: [scalar(0.), scalar(2.)],
                },
            ],
            segments: vec![
                ProfileSegment {
                    id: "ab".into(),
                    start_corner: "a".into(),
                    end_corner: "b".into(),
                    curve: ProfileSegmentCurve::Line,
                },
                ProfileSegment {
                    id: "bc".into(),
                    start_corner: "b".into(),
                    end_corner: "c".into(),
                    curve: ProfileSegmentCurve::Line,
                },
                ProfileSegment {
                    id: "cd".into(),
                    start_corner: "c".into(),
                    end_corner: "d".into(),
                    curve: ProfileSegmentCurve::Line,
                },
                ProfileSegment {
                    id: "da".into(),
                    start_corner: "d".into(),
                    end_corner: "a".into(),
                    curve: ProfileSegmentCurve::Line,
                },
            ],
            corner_order: vec!["a".into(), "b".into(), "c".into(), "d".into()],
            segment_order: vec!["ab".into(), "bc".into(), "cd".into(), "da".into()],
            closure: GeometryClosure::Closed,
        }),
        GeometryOutputCategory::Profile,
    )
}

fn line_curve(name: &str) -> ConstructionOperation {
    op(
        name,
        OperationRequest::Curve(CurveDeclaration::Line {
            origin: point(),
            direction: vector(),
            length: scalar(1.),
        }),
        GeometryOutputCategory::Curve,
    )
}

fn bilinear_surface(name: &str) -> ConstructionOperation {
    op(
        name,
        OperationRequest::NurbsSurfacePatch(NurbsSurfacePatchRequest {
            u_degree: 1,
            v_degree: 1,
            u_knots: vec![scalar(0.), scalar(0.), scalar(1.), scalar(1.)],
            v_knots: vec![scalar(0.), scalar(0.), scalar(1.), scalar(1.)],
            control_net: vec![
                vec![
                    ExactPoint3::new([0., 0., 0.]).unwrap(),
                    ExactPoint3::new([1., 0., 0.]).unwrap(),
                ],
                vec![
                    ExactPoint3::new([0., 1., 0.]).unwrap(),
                    ExactPoint3::new([1., 1., 0.]).unwrap(),
                ],
            ],
            weight_net: vec![vec![scalar(1.), scalar(1.)], vec![scalar(1.), scalar(1.)]],
            parameter_domain: [[scalar(0.), scalar(1.)], [scalar(0.), scalar(1.)]],
            orientation: NurbsSurfaceOrientation {
                u_reversed: false,
                v_reversed: false,
                normal_reversed: false,
            },
            trims: vec![],
        }),
        GeometryOutputCategory::Sheet,
    )
}

fn take_region(result: ConstructionResult, name: &str) -> AlgebraicRegion {
    result
        .into_outputs()
        .into_iter()
        .find_map(|(identity, output)| (identity == identity_of(name)).then_some(output))
        .and_then(|output| match output {
            ConstructionOutput::Region(region) => Some(region),
            ConstructionOutput::AnalyticRepresentation(_)
            | ConstructionOutput::ImportedRegion(_) => None,
        })
        .unwrap()
}

fn constructed_region(name: &str, operations: Vec<ConstructionOperation>) -> AlgebraicRegion {
    take_region(construct(request(operations)).unwrap(), name)
}

fn derived_from_region(region: AlgebraicRegion, revision: &str) -> DerivedAnalyticBrep {
    let boundary = derive_complete_boundary(region).unwrap();
    derive_analytic_brep(boundary, SourceRevision::new(revision.into()).unwrap()).unwrap()
}

fn brep_from_region(region: AlgebraicRegion, revision: &str) -> AnalyticBrep {
    derived_from_region(region, revision).into_parts().0
}

fn foundation_mesh(brep: &AnalyticBrep) -> TessellationResult {
    tessellate(brep, TessellationRequest::new(0.5).unwrap()).unwrap()
}

fn assert_foundation(brep: &AnalyticBrep, mesh: &TessellationResult, revision: &str) {
    assert!(!brep.faces().is_empty());
    assert_eq!(mesh.source_brep_revision().as_str(), revision);
    let triangles = mesh.validated_surface_mesh().mesh().triangles();
    assert!(!triangles.is_empty());
    let mut covered = vec![false; triangles.len()];
    for face in mesh.face_triangle_correspondence() {
        for index in face.triangle_indices() {
            covered[*index] = true;
        }
    }
    assert!(covered.iter().all(|seen| *seen));
}

fn assert_volume_foundation(brep: &AnalyticBrep, mesh: &TessellationResult, revision: &str) {
    assert_foundation(brep, mesh, revision);
    assert!(!brep.witnesses().closed_shell_ids().is_empty());
}

fn import_request() -> RegionImportRequest {
    let enclosure = Enclosure::new(
        neco_bigint::Dyadic::from_f64_exact(0.0).unwrap(),
        neco_bigint::Dyadic::from_f64_exact(1.0 / 256.0).unwrap(),
    )
    .unwrap();
    let isolation =
        RootIsolationPolicy::new(8, RootRefinementMethod::Bisection, enclosure).unwrap();
    let crossing = RayCrossingPolicy::new(isolation, 64, 64).unwrap();
    RegionImportRequest::new(0.01, crossing).unwrap()
}

fn contacting_sphere(name: &str, source_name: &str) -> ConstructionOperation {
    op(
        name,
        OperationRequest::Transform {
            source: output(source_name),
            placement: ExactRigidPlacement::new(
                ExactPoint3::new([2., 0., 0.]).unwrap(),
                [1., 0., 0., 0.],
            )
            .unwrap(),
            scale: scalar(1.),
        },
        GeometryOutputCategory::Solid,
    )
}

fn boolean_union(name: &str, kind: BooleanKind, left: &str, right: &str) -> ConstructionOperation {
    op(
        name,
        OperationRequest::Boolean {
            request: BooleanRequest {
                kind,
                operator: BooleanOperator::Union,
            },
            left: output(left),
            right: output(right),
        },
        GeometryOutputCategory::Solid,
    )
}

#[test]
fn sphere_adaptive_mesh_preserves_identity_revision_and_established_bounds() {
    let constructed = construct(request(vec![sphere("sphere")])).unwrap();
    let source_revision = String::from(constructed.source_revision().as_str());
    let region = take_region(constructed, "sphere");
    assert_eq!(region.region_kind(), RegionKind::RegularClosedVolume);
    let derived = derived_from_region(region, &source_revision);
    let result = tessellate_adaptive(
        derived
            .into_adaptive_tessellation_request(
                SourceIdentity::new("mesh:sphere").unwrap(),
                AdaptivePrecision::new(TessellationConsumer::Display, 0.5, 1.0).unwrap(),
                48,
            )
            .unwrap(),
    )
    .unwrap();
    assert_eq!(result.identity().as_str(), "mesh:sphere");
    assert_eq!(result.source_revision().as_str(), "r1");
    assert!(result.precision_witness().established_maximum_chord_error() <= 0.5);
    assert!(
        result
            .precision_witness()
            .established_maximum_surface_error()
            < &{ neco_bigint::Dyadic::from_f64_exact(1.0).unwrap() }
    );
    assert!(result.validated_surface_mesh().mesh().triangles().len() > 2);
    assert!(!result.brep_element_correspondence().face_ids().is_empty());
}

#[test]
fn public_owned_pipeline_reimports_certified_region() {
    let constructed = construct(request_with_revision(vec![sphere("sphere")], "owned-r1")).unwrap();
    let source_revision =
        SourceRevision::new(String::from(constructed.source_revision().as_str())).unwrap();
    let outputs = constructed.into_outputs();
    assert!(outputs
        .iter()
        .any(|(identity, _)| identity == &identity_of("sphere")));
    let region = outputs
        .into_iter()
        .find_map(|(identity, output)| (identity == identity_of("sphere")).then_some(output))
        .and_then(|output| match output {
            ConstructionOutput::Region(region) => Some(region),
            ConstructionOutput::AnalyticRepresentation(_)
            | ConstructionOutput::ImportedRegion(_) => None,
        })
        .unwrap();
    let boundary = derive_complete_boundary(region).unwrap();
    let derived = derive_analytic_brep(boundary, source_revision).unwrap();
    assert_eq!(derived.face_surfaces().len(), derived.brep().faces().len());
    assert!(derived
        .face_surfaces()
        .iter()
        .all(|surface| !surface.covering().patches().is_empty()));
    let tessellation = tessellate_adaptive(
        derived
            .into_adaptive_tessellation_request(
                SourceIdentity::new("mesh:owned-pipeline").unwrap(),
                AdaptivePrecision::new(TessellationConsumer::Analysis, 0.5, 1.0).unwrap(),
                48,
            )
            .unwrap(),
    )
    .unwrap();
    assert_eq!(tessellation.source_revision().as_str(), "owned-r1");
    let closed = tessellation.into_closed_oriented_surface_mesh().unwrap();
    assert_eq!(closed.source().identity(), "mesh:owned-pipeline");
    assert_eq!(closed.source().revision(), "owned-r1");
    assert!(closed.closure_witness().edge_count() > 0);
    assert_eq!(
        closed.closure_witness().edge_count(),
        closed.orientation_witness().edge_count()
    );
    assert_eq!(
        closed.closure_witness().edge_count(),
        closed.manifold_witness().edge_count()
    );
    let certified = region_from_mesh(closed, import_request()).unwrap();
    assert_eq!(certified.input_identity(), "mesh:owned-pipeline");
    assert_eq!(certified.source_revision(), "owned-r1");
    assert!(certified.self_intersection().is_none());
}

#[test]
fn exact_covering_supports_box_and_extrude_and_classifies_torus_and_revolve() {
    let supported = [
        (
            "box",
            constructed_region(
                "box",
                vec![primitive(
                    "box",
                    PrimitiveDeclaration::Box {
                        lengths: ExactVector3::new([2., 4., 6.]).unwrap(),
                    },
                )],
            ),
        ),
        (
            "extrude",
            constructed_region(
                "extrude",
                vec![
                    closed_square_profile("profile"),
                    op(
                        "extrude",
                        OperationRequest::Extrude {
                            profile: output("profile"),
                            direction: z_axis(),
                            distance: scalar(2.),
                            symmetric: false,
                        },
                        GeometryOutputCategory::Solid,
                    ),
                ],
            ),
        ),
    ];
    for (name, region) in supported {
        let derived = derived_from_region(region, "r1");
        assert!(derived.face_surfaces().iter().all(|surface| {
            surface.covering().patches().len() == 1
                && matches!(surface.covering().patches()[0], ChartPatch::Trimmed { .. })
        }));
        let tessellation = tessellate_adaptive(
            derived
                .into_adaptive_tessellation_request(
                    SourceIdentity::new(format!("mesh:{name}")).unwrap(),
                    AdaptivePrecision::new(TessellationConsumer::Analysis, 0.25, 1.0).unwrap(),
                    48,
                )
                .unwrap(),
        )
        .unwrap();
        let closed = tessellation.into_closed_oriented_surface_mesh().unwrap();
        assert_eq!(closed.source().identity(), format!("mesh:{name}"));
        assert!(closed.closure_witness().edge_count() > 0);
    }

    for region in [
        constructed_region("torus", vec![torus("torus")]),
        constructed_region(
            "revolve",
            vec![
                revolve_rectangle_profile("profile"),
                op(
                    "revolve",
                    OperationRequest::Revolve {
                        profile: output("profile"),
                        axis_origin: point(),
                        axis_direction: y_axis(),
                        extent: RevolveExtent::Full,
                    },
                    GeometryOutputCategory::Solid,
                ),
            ],
        ),
    ] {
        let boundary = derive_complete_boundary(region).unwrap();
        assert!(matches!(
            derive_analytic_brep(boundary, SourceRevision::new("r1".into()).unwrap()),
            Err(CadFailure::BrepDerivation {
                condition: BrepDerivationCondition::UnsupportedSurfaceCovering,
                ..
            })
        ));
    }
}

#[test]
fn contacting_boolean_kinds_reach_foundation_mesh() {
    let left = sphere("left");
    let right = sphere("right");
    let moved = contacting_sphere("moved", "right");
    let set = boolean_union("set", BooleanKind::Set, "left", "moved");
    let set_result = construct(request(vec![left, right, moved, set])).unwrap();
    let set_region = take_region(set_result, "set");
    assert_eq!(
        set_region.region_kind(),
        RegionKind::GeneralSemiAlgebraicSet
    );
    let set_derived = derived_from_region(set_region, "r1");
    assert_eq!(set_derived.face_surfaces().len(), 2);
    assert!(set_derived.face_surfaces().iter().all(|surface| {
        surface.exact_surface().atom_id() == "left:sphere"
            || surface.exact_surface().atom_id() == "right:sphere"
    }));
    let set_mesh = foundation_mesh(set_derived.brep());
    assert_foundation(set_derived.brep(), &set_mesh, "r1");

    let left = sphere("left");
    let right = sphere("right");
    let moved = contacting_sphere("moved", "right");
    let solid = boolean_union("solid", BooleanKind::Solid, "left", "moved");
    let solid_result = construct(request(vec![left, right, moved, solid])).unwrap();
    let solid_region = take_region(solid_result, "solid");
    assert_eq!(solid_region.region_kind(), RegionKind::RegularClosedVolume);
    let solid_derived = derived_from_region(solid_region, "r1");
    assert_eq!(solid_derived.face_surfaces().len(), 2);
    assert!(solid_derived.face_surfaces().iter().all(|surface| {
        surface.exact_surface().atom_id() == "left:sphere"
            || surface.exact_surface().atom_id() == "right:sphere"
    }));
    let solid_mesh = foundation_mesh(solid_derived.brep());
    assert_volume_foundation(solid_derived.brep(), &solid_mesh, "r1");
}

#[test]
fn sweep_loft_finite_revolve_and_degenerate_typed_failure() {
    let profile = closed_square_profile("profile");
    let curve = line_curve("curve");
    let sweep = op(
        "sweep",
        OperationRequest::Sweep {
            profile: output("profile"),
            spine: output("curve"),
            frame_rule: SweepFrameRule::Frenet,
            scale_samples: vec![],
            twist_samples: vec![],
        },
        GeometryOutputCategory::Solid,
    );
    assert!(matches!(
        construct(request(vec![profile, curve, sweep])),
        Err(CadConstructionFailure::Lowering {
            condition: LoweringCondition::SweepUnsupported,
            ..
        })
    ));

    let first = closed_square_profile("s1");
    let second = closed_square_profile("s2");
    let loft = op(
        "loft",
        OperationRequest::Loft {
            sections: vec![
                LoftSection {
                    slot: "a".into(),
                    profile: output("s1"),
                    placement: placement(),
                },
                LoftSection {
                    slot: "b".into(),
                    profile: output("s2"),
                    placement: placement(),
                },
            ],
            section_order: vec!["a".into(), "b".into()],
            mode: LoftMode::Straight,
        },
        GeometryOutputCategory::Solid,
    );
    assert!(matches!(
        construct(request(vec![first, second, loft])),
        Err(CadConstructionFailure::Lowering {
            condition: LoweringCondition::LoftUnsupported,
            ..
        })
    ));

    let profile = revolve_rectangle_profile("profile");
    let revolve = op(
        "revolve",
        OperationRequest::Revolve {
            profile: output("profile"),
            axis_origin: point(),
            axis_direction: y_axis(),
            extent: RevolveExtent::FiniteAngle(scalar(1.)),
        },
        GeometryOutputCategory::Solid,
    );
    assert!(matches!(
        construct(request(vec![profile, revolve])),
        Err(CadConstructionFailure::Lowering {
            condition: LoweringCondition::FiniteRevolveAngle,
            ..
        })
    ));

    let mut invalid = sphere("primitive");
    invalid.request =
        OperationRequest::Primitive(PrimitiveDeclaration::Sphere { radius: scalar(0.) });
    assert!(matches!(
        construct(request(vec![invalid])),
        Err(CadConstructionFailure::PayloadInvalid {
            field: "primitive.dimensions",
            ..
        })
    ));

    let empty_brep = brep_from_region(constructed_region("sphere", vec![sphere("sphere")]), "r1");
    let missing_brep = brep_from_region(constructed_region("sphere", vec![sphere("sphere")]), "r1");
    assert_eq!(
        AdaptiveTessellationRequest::new(
            "",
            empty_brep,
            Vec::new(),
            AdaptivePrecision::new(TessellationConsumer::Display, 0.5, 1.0).unwrap(),
            48,
        )
        .err(),
        Some(CadFailure::AdaptiveTessellation {
            condition: AdaptiveTessellationCondition::EmptyIdentity,
            identity: None,
        })
    );
    assert_eq!(
        tessellate_adaptive(
            AdaptiveTessellationRequest::new(
                "mesh:missing",
                missing_brep,
                Vec::new(),
                AdaptivePrecision::new(TessellationConsumer::Display, 0.5, 1.0).unwrap(),
                48,
            )
            .unwrap(),
        )
        .err(),
        Some(CadFailure::AdaptiveTessellation {
            condition: AdaptiveTessellationCondition::MissingCovering,
            identity: Some(String::from("mesh:missing")),
        })
    );
}

#[test]
fn nurbs_patch_keeps_analytic_representation() {
    let result = construct(request(vec![bilinear_surface("surface")])).unwrap();
    assert!(matches!(
        result.analytic(&identity_of("surface")),
        Some(CurveOrSurfaceRepresentation::Surface(_))
    ));
    assert!(result.region(&identity_of("surface")).is_none());
}

#[test]
fn permutation_preserves_kind_and_revision_mutation() {
    let first_a = constructed_region("a", vec![sphere("a"), sphere("b")]);
    let first_b = constructed_region("b", vec![sphere("a"), sphere("b")]);
    let second_a = constructed_region("a", vec![sphere("b"), sphere("a")]);
    let second_b = constructed_region("b", vec![sphere("b"), sphere("a")]);
    assert_eq!(first_a.region_kind(), second_a.region_kind());
    assert_eq!(first_b.region_kind(), second_b.region_kind());
    let first_derived = derived_from_region(first_a, "r1");
    let second_derived = derived_from_region(second_a, "r1");
    let signature = |derived: &DerivedAnalyticBrep| {
        derived
            .face_surfaces()
            .iter()
            .map(|surface| {
                (
                    String::from(surface.face_id()),
                    String::from(surface.exact_surface().atom_id()),
                    surface.covering().patches().len(),
                    surface.covering().seams().len(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(signature(&first_derived), signature(&second_derived));
    assert_eq!(
        first_derived.brep().faces().len(),
        second_derived.brep().faces().len()
    );

    let first_brep = brep_from_region(constructed_region("sphere", vec![sphere("sphere")]), "r1");
    let second_brep = brep_from_region(constructed_region("sphere", vec![sphere("sphere")]), "r2");
    assert_eq!(first_brep.witnesses().source_revision().as_str(), "r1");
    assert_eq!(second_brep.witnesses().source_revision().as_str(), "r2");
    assert_eq!(first_brep.faces().len(), second_brep.faces().len());
}
