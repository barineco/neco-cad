use neco_cad::*;
use neco_calculus::DerivativeOrder;

fn id(v: &str) -> OperationIdentity {
    OperationIdentity::new(v).unwrap()
}
fn source(v: &str) -> SourceIdentity {
    SourceIdentity::new(v).unwrap()
}
fn scalar(v: f64) -> ExactScalar {
    ExactScalar::new(v).unwrap()
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
fn request(operations: Vec<ConstructionOperation>) -> ConstructionRequest {
    let revision = SourceRevision::new("r1".into()).unwrap();
    ConstructionRequest {
        source_revision: revision.clone(),
        precision: GeometryAccuracyPolicy::ExactOnly,
        identity_ledger: ledger(&operations),
        operations,
        roots: vec![],
        provenance: Provenance::new("schema", "ns", revision, "adapter", "exact", vec![], vec![]),
    }
}
fn primitive(name: &str, declaration: PrimitiveDeclaration) -> ConstructionOperation {
    ConstructionOperation {
        operation: id(name),
        source_object: source(name),
        expected_output: expected(GeometryOutputCategory::Solid),
        request: OperationRequest::Primitive(declaration),
    }
}
fn identity_of(name: &str) -> OperationOutputIdentity {
    OperationOutputIdentity {
        operation: id(name),
        slot: OutputSlot::Primary,
    }
}
fn output(name: &str) -> OperationOutputRef {
    OperationOutputRef::primary(id(name))
}
fn revision() -> SourceRevision {
    SourceRevision::new("r1".into()).unwrap()
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
fn closed_square_profile(name: &str) -> ConstructionOperation {
    ConstructionOperation {
        operation: id(name),
        source_object: source(name),
        expected_output: expected(GeometryOutputCategory::Profile),
        request: OperationRequest::Profile(ProfileRequest {
            origin: ExactPoint3::new([0., 0., 0.]).unwrap(),
            x_axis: ExactVector3::new([1., 0., 0.]).unwrap(),
            normal: ExactVector3::new([0., 0., 1.]).unwrap(),
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
    }
}

fn assert_face_coverage(region: AlgebraicRegion) {
    let atom_ids: Vec<String> = region
        .canonical_atoms()
        .iter()
        .map(|atom| atom.atom_id().into())
        .collect();
    let boundary = derive_complete_boundary(region).unwrap();
    let derived = derive_analytic_brep(boundary, revision()).unwrap();
    assert!(!derived.brep().faces().is_empty());
    assert!(!derived.brep().witnesses().closed_shell_ids().is_empty());
    for atom_id in atom_ids {
        assert!(
            derived
                .face_surfaces()
                .iter()
                .any(|surface| surface.exact_surface().atom_id() == atom_id),
            "{atom_id} must keep an exact face surface"
        );
    }
}

#[test]
fn sphere_brep_has_closed_shell() {
    let constructed = construct(request(vec![primitive(
        "sphere",
        PrimitiveDeclaration::Sphere { radius: scalar(1.) },
    )]))
    .unwrap();
    let region = take_region(constructed, "sphere");
    let boundary = derive_complete_boundary(region).unwrap();
    let derived = derive_analytic_brep(boundary, revision()).unwrap();
    assert!(!derived.brep().faces().is_empty());
    assert!(!derived.brep().witnesses().closed_shell_ids().is_empty());
}

#[test]
fn constructed_region_face_coverage() {
    let box_solid = construct(request(vec![primitive(
        "box",
        PrimitiveDeclaration::Box {
            lengths: ExactVector3::new([2., 2., 2.]).unwrap(),
        },
    )]))
    .unwrap();
    assert_face_coverage(take_region(box_solid, "box"));

    let extrude = construct(request(vec![
        closed_square_profile("profile"),
        ConstructionOperation {
            operation: id("extrude"),
            source_object: source("extrude"),
            expected_output: expected(GeometryOutputCategory::Solid),
            request: OperationRequest::Extrude {
                profile: output("profile"),
                direction: ExactVector3::new([0., 0., 1.]).unwrap(),
                distance: scalar(2.),
                symmetric: false,
            },
        },
    ]))
    .unwrap();
    assert_face_coverage(take_region(extrude, "extrude"));
}

#[test]
fn contacting_spheres_assign_contact_vertex() {
    let set = construct(request(vec![
        primitive("left", PrimitiveDeclaration::Sphere { radius: scalar(1.) }),
        primitive("right", PrimitiveDeclaration::Sphere { radius: scalar(1.) }),
        ConstructionOperation {
            operation: id("moved"),
            source_object: source("moved"),
            expected_output: expected(GeometryOutputCategory::Solid),
            request: OperationRequest::Transform {
                source: OperationOutputRef::primary(id("right")),
                placement: ExactRigidPlacement::new(
                    ExactPoint3::new([2., 0., 0.]).unwrap(),
                    [1., 0., 0., 0.],
                )
                .unwrap(),
                scale: scalar(1.),
            },
        },
        ConstructionOperation {
            operation: id("set"),
            source_object: source("set"),
            expected_output: expected(GeometryOutputCategory::Solid),
            request: OperationRequest::Boolean {
                request: BooleanRequest {
                    kind: BooleanKind::Set,
                    operator: BooleanOperator::Union,
                },
                left: OperationOutputRef::primary(id("left")),
                right: OperationOutputRef::primary(id("moved")),
            },
        },
    ]))
    .unwrap();
    let region = take_region(set, "set");
    let source_atom_ids: Vec<String> = region
        .canonical_atoms()
        .iter()
        .map(|atom| String::from(atom.atom_id()))
        .collect();
    let boundary = derive_complete_boundary(region).unwrap();
    let has_contact = boundary.strata().iter().any(|stratum| {
        matches!(stratum.dimension(), BoundaryStratumDimension::Dimension(0))
            || stratum.active_atom_ids().len() >= 2
    });
    assert!(has_contact);
    let derived = derive_analytic_brep(boundary, revision()).unwrap();
    assert!(!derived.brep().faces().is_empty());
    assert!(!derived.brep().vertices().is_empty() || derived.brep().faces().len() >= 2);
    assert!(derived.face_surfaces().iter().all(|surface| {
        source_atom_ids
            .iter()
            .any(|atom_id| atom_id == surface.exact_surface().atom_id())
    }));
}

#[test]
fn overlapping_sphere_boolean_requires_a_curved_trim() {
    let set = construct(request(vec![
        primitive("left", PrimitiveDeclaration::Sphere { radius: scalar(1.) }),
        primitive("right", PrimitiveDeclaration::Sphere { radius: scalar(1.) }),
        ConstructionOperation {
            operation: id("moved"),
            source_object: source("moved"),
            expected_output: expected(GeometryOutputCategory::Solid),
            request: OperationRequest::Transform {
                source: OperationOutputRef::primary(id("right")),
                placement: ExactRigidPlacement::new(
                    ExactPoint3::new([1., 0., 0.]).unwrap(),
                    [1., 0., 0., 0.],
                )
                .unwrap(),
                scale: scalar(1.),
            },
        },
        ConstructionOperation {
            operation: id("set"),
            source_object: source("set"),
            expected_output: expected(GeometryOutputCategory::Solid),
            request: OperationRequest::Boolean {
                request: BooleanRequest {
                    kind: BooleanKind::Set,
                    operator: BooleanOperator::Union,
                },
                left: OperationOutputRef::primary(id("left")),
                right: OperationOutputRef::primary(id("moved")),
            },
        },
    ]))
    .unwrap();
    let region = take_region(set, "set");
    match derive_complete_boundary(region) {
        Err(CadFailure::Boundary {
            condition: BoundaryDerivationCondition::IncompleteCoverage,
            ..
        }) => {}
        Ok(boundary) => assert!(matches!(
            derive_analytic_brep(boundary, revision()),
            Err(CadFailure::BrepDerivation {
                condition: BrepDerivationCondition::UnsupportedSurfaceCovering,
                ..
            })
        )),
        other => panic!("unexpected overlapping sphere result: {other:?}"),
    }
}

#[test]
fn undetermined_or_open_set_typed_failure() {
    let polynomial = RegionPolynomial3::try_from_terms(vec![
        RegionPolynomialTerm::try_new(
            neco_algnum::RealAlgebraic::from_integer(neco_bigint::BigInt::try_from(1).unwrap())
                .unwrap(),
            2,
            0,
            0,
        )
        .unwrap(),
        RegionPolynomialTerm::try_new(
            neco_algnum::RealAlgebraic::from_integer(neco_bigint::BigInt::try_from(1).unwrap())
                .unwrap(),
            0,
            2,
            0,
        )
        .unwrap(),
        RegionPolynomialTerm::try_new(
            neco_algnum::RealAlgebraic::from_integer(neco_bigint::BigInt::try_from(1).unwrap())
                .unwrap(),
            0,
            0,
            2,
        )
        .unwrap(),
        RegionPolynomialTerm::try_new(
            neco_algnum::RealAlgebraic::from_integer(neco_bigint::BigInt::try_from(-1).unwrap())
                .unwrap(),
            0,
            0,
            0,
        )
        .unwrap(),
    ])
    .unwrap();
    let atom = CanonicalAtom::try_new(
        "sheet",
        RegionFunction::Polynomial(polynomial),
        SignRelation::Negative,
    )
    .unwrap();
    let region = construct_region(RegionConstructionRequest {
        ambient_dimension: 3,
        canonical_atoms: vec![atom],
        normalized_region_formula: NormalizedRegionFormula::Atom("sheet".into()),
        region_kind: RegionKind::LowerDimensionalSet,
    })
    .unwrap();
    let boundary = derive_complete_boundary(region).unwrap();
    assert!(matches!(
        derive_analytic_brep(boundary, revision()),
        Err(CadFailure::BrepDerivation {
            condition: BrepDerivationCondition::OpenBoundary,
            ..
        })
    ));
}

#[test]
fn incomplete_face_coverage_failure() {
    let constructed = construct(request(vec![primitive(
        "box",
        PrimitiveDeclaration::Box {
            lengths: ExactVector3::new([2., 2., 2.]).unwrap(),
        },
    )]))
    .unwrap();
    let region = take_region(constructed, "box");
    let boundary = derive_boundary(BoundaryDerivationRequest {
        region,
        intersections: vec![],
        candidate_loci: vec![],
        max_order: DerivativeOrder::new(3),
    })
    .unwrap();
    assert!(matches!(
        derive_analytic_brep(boundary, revision()),
        Err(CadFailure::BrepDerivation {
            condition: BrepDerivationCondition::IncompleteCoverage,
            ..
        })
    ));
}
