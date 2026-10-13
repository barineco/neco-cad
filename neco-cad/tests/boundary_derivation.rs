use neco_cad::*;

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

fn revolve_rectangle_profile(name: &str) -> ConstructionOperation {
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
    }
}

fn assert_proved_atom_coverage(region: AlgebraicRegion) {
    let atom_ids: Vec<String> = region
        .canonical_atoms()
        .iter()
        .map(|atom| atom.atom_id().into())
        .collect();
    let boundary = derive_complete_boundary(region).unwrap();
    for atom_id in atom_ids {
        assert!(
            boundary.strata().iter().any(|stratum| {
                matches!(
                    stratum.local_evidence(),
                    BoundaryLocalEvidence::Proved { .. }
                ) && stratum
                    .candidates()
                    .iter()
                    .any(|candidate| candidate.locus().atom_id() == atom_id)
            }),
            "{atom_id} must keep a proved stratum"
        );
    }
}

fn algebraic(value: i32) -> neco_algnum::RealAlgebraic {
    neco_algnum::RealAlgebraic::from_integer(neco_bigint::BigInt::try_from(value).unwrap()).unwrap()
}

fn term(coefficient: i32, x: u32, y: u32, z: u32) -> RegionPolynomialTerm {
    RegionPolynomialTerm::try_new(algebraic(coefficient), x, y, z).unwrap()
}

#[test]
fn sphere_boundary_has_proved_strata() {
    let constructed = construct(request(vec![primitive(
        "sphere",
        PrimitiveDeclaration::Sphere { radius: scalar(1.) },
    )]))
    .unwrap();
    let region = take_region(constructed, "sphere");
    let boundary = derive_complete_boundary(region).unwrap();
    assert!(!boundary.strata().is_empty());
    assert!(boundary.strata().iter().any(|stratum| {
        matches!(
            stratum.local_evidence(),
            BoundaryLocalEvidence::Proved { .. }
        )
    }));
}

#[test]
fn constructed_region_atom_coverage() {
    let box_solid = construct(request(vec![primitive(
        "box",
        PrimitiveDeclaration::Box {
            lengths: ExactVector3::new([2., 2., 2.]).unwrap(),
        },
    )]))
    .unwrap();
    assert_proved_atom_coverage(take_region(box_solid, "box"));

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
    assert_proved_atom_coverage(take_region(extrude, "extrude"));

    let revolve = construct(request(vec![
        revolve_rectangle_profile("profile"),
        ConstructionOperation {
            operation: id("revolve"),
            source_object: source("revolve"),
            expected_output: expected(GeometryOutputCategory::Solid),
            request: OperationRequest::Revolve {
                profile: output("profile"),
                axis_origin: ExactPoint3::new([0., 0., 0.]).unwrap(),
                axis_direction: ExactVector3::new([0., 1., 0.]).unwrap(),
                extent: RevolveExtent::Full,
            },
        },
    ]))
    .unwrap();
    assert_proved_atom_coverage(take_region(revolve, "revolve"));

    let moved = construct(request(vec![
        primitive(
            "sphere",
            PrimitiveDeclaration::Sphere { radius: scalar(1.) },
        ),
        ConstructionOperation {
            operation: id("moved"),
            source_object: source("moved"),
            expected_output: expected(GeometryOutputCategory::Solid),
            request: OperationRequest::Transform {
                source: output("sphere"),
                placement: ExactRigidPlacement::new(
                    ExactPoint3::new([2., 0., 0.]).unwrap(),
                    [1., 0., 0., 0.],
                )
                .unwrap(),
                scale: scalar(1.),
            },
        },
    ]))
    .unwrap();
    assert_proved_atom_coverage(take_region(moved, "moved"));
}

#[test]
fn contacting_spheres_preserve_intersection_branch() {
    let left = primitive("left", PrimitiveDeclaration::Sphere { radius: scalar(1.) });
    let right = primitive("right", PrimitiveDeclaration::Sphere { radius: scalar(1.) });
    let moved = ConstructionOperation {
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
    };
    let set = ConstructionOperation {
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
    };
    let constructed = construct(request(vec![left, right, moved, set])).unwrap();
    let region = take_region(constructed, "set");
    let boundary = derive_complete_boundary(region).unwrap();
    assert!(boundary.strata().iter().any(|stratum| {
        stratum.active_atom_ids().len() >= 2
            || matches!(stratum.dimension(), BoundaryStratumDimension::Dimension(0))
    }));
}

#[test]
fn singularity_or_seam_typed_failure() {
    let polynomial = RegionPolynomial3::try_from_terms(vec![
        term(1, 4, 0, 0),
        term(1, 0, 4, 0),
        term(1, 0, 0, 4),
        term(2, 2, 2, 0),
        term(2, 2, 0, 2),
        term(2, 0, 2, 2),
    ])
    .unwrap();
    let atom = CanonicalAtom::try_new(
        "isolated",
        RegionFunction::Polynomial(polynomial),
        SignRelation::Negative,
    )
    .unwrap();
    let region = construct_region(RegionConstructionRequest {
        ambient_dimension: 3,
        canonical_atoms: vec![atom],
        normalized_region_formula: NormalizedRegionFormula::Atom("isolated".into()),
        region_kind: RegionKind::RegularClosedVolume,
    })
    .unwrap();
    assert!(matches!(
        derive_complete_boundary(region),
        Err(CadFailure::Boundary {
            condition: BoundaryDerivationCondition::SingularityUnsupported
                | BoundaryDerivationCondition::IncompleteCoverage,
            ..
        })
    ));
}

#[test]
fn empty_region_and_incomplete_coverage_failure() {
    let empty = construct_region(RegionConstructionRequest {
        ambient_dimension: 3,
        canonical_atoms: Vec::new(),
        normalized_region_formula: NormalizedRegionFormula::False,
        region_kind: RegionKind::GeneralSemiAlgebraicSet,
    });
    if let Ok(region) = empty {
        assert!(matches!(
            derive_complete_boundary(region),
            Err(CadFailure::Boundary {
                condition: BoundaryDerivationCondition::EmptyRegion,
                ..
            })
        ));
    }

    let polynomial = RegionPolynomial3::try_from_terms(vec![
        term(1, 2, 0, 0),
        term(1, 0, 2, 0),
        term(1, 0, 0, 2),
        term(1, 0, 0, 0),
    ])
    .unwrap();
    let atom = CanonicalAtom::try_new(
        "empty-variety",
        RegionFunction::Polynomial(polynomial),
        SignRelation::Negative,
    )
    .unwrap();
    let region = construct_region(RegionConstructionRequest {
        ambient_dimension: 3,
        canonical_atoms: vec![atom],
        normalized_region_formula: NormalizedRegionFormula::Atom("empty-variety".into()),
        region_kind: RegionKind::RegularClosedVolume,
    })
    .unwrap();
    assert!(matches!(
        derive_complete_boundary(region),
        Err(CadFailure::Boundary {
            condition: BoundaryDerivationCondition::IncompleteCoverage,
            ..
        })
    ));
}
