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
            .map(|o| OperationIdentityBinding {
                source: o.source_object.clone(),
                operation: o.operation.clone(),
                output: o.output_identity(),
            })
            .collect(),
        operations
            .iter()
            .map(|o| EntityIdentityBinding {
                source: o.source_object.clone(),
                operation: o.operation.clone(),
                output: o.output_identity(),
                produced: format!("{}:primary", o.operation.as_str()),
            })
            .collect(),
    )
    .unwrap()
}
fn provenance(revision: SourceRevision) -> Provenance {
    Provenance::new("schema", "ns", revision, "adapter", "exact", vec![], vec![])
}
fn request(operations: Vec<ConstructionOperation>) -> ConstructionRequest {
    let r = SourceRevision::new("r1".into()).unwrap();
    ConstructionRequest {
        source_revision: r.clone(),
        precision: GeometryAccuracyPolicy::ExactOnly,
        identity_ledger: ledger(&operations),
        operations,
        roots: vec![],
        provenance: provenance(r),
    }
}
fn primitive(name: &str) -> ConstructionOperation {
    op(
        name,
        OperationRequest::Primitive(PrimitiveDeclaration::Sphere { radius: scalar(1.) }),
        GeometryOutputCategory::Solid,
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
fn classify(region: &AlgebraicRegion, coordinates: [f64; 3]) -> MembershipClassification {
    classify_region_membership(
        region,
        [
            scalar(coordinates[0]),
            scalar(coordinates[1]),
            scalar(coordinates[2]),
        ],
    )
    .unwrap()
}
fn identity_of(name: &str) -> OperationOutputIdentity {
    OperationOutputIdentity {
        operation: id(name),
        slot: OutputSlot::Primary,
    }
}
fn tetrahedron_solid(name: &str) -> ConstructionOperation {
    op(
        name,
        OperationRequest::ExplicitSolid {
            vertices: vec![
                ExplicitSolidVertex {
                    id: "a".into(),
                    position: ExactPoint3::new([0., 0., 0.]).unwrap(),
                },
                ExplicitSolidVertex {
                    id: "b".into(),
                    position: ExactPoint3::new([4., 0., 0.]).unwrap(),
                },
                ExplicitSolidVertex {
                    id: "c".into(),
                    position: ExactPoint3::new([0., 4., 0.]).unwrap(),
                },
                ExplicitSolidVertex {
                    id: "d".into(),
                    position: ExactPoint3::new([0., 0., 4.]).unwrap(),
                },
            ],
            faces: vec![
                ExplicitSolidFace {
                    id: "abc".into(),
                    vertex_order: vec!["a".into(), "c".into(), "b".into()],
                },
                ExplicitSolidFace {
                    id: "abd".into(),
                    vertex_order: vec!["a".into(), "b".into(), "d".into()],
                },
                ExplicitSolidFace {
                    id: "bcd".into(),
                    vertex_order: vec!["b".into(), "c".into(), "d".into()],
                },
                ExplicitSolidFace {
                    id: "cad".into(),
                    vertex_order: vec!["c".into(), "a".into(), "d".into()],
                },
            ],
        },
        GeometryOutputCategory::Solid,
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

#[test]
fn exact_region_input_api_connection() {
    let coefficient = scalar(1.).to_real_algebraic().unwrap();
    let term = RegionPolynomialTerm::try_new(coefficient, 2, 0, 0).unwrap();
    let polynomial = RegionPolynomial3::try_from_terms(vec![term]).unwrap();
    let scaled = polynomial
        .try_scale(&scalar(2.).to_real_algebraic().unwrap())
        .unwrap();
    let added = polynomial.try_add(&scaled).unwrap();
    let multiplied = polynomial.try_multiply(&scaled).unwrap();
    let map = RegionAffineMap3::try_new(
        RegionPolynomial3::try_from_terms(vec![RegionPolynomialTerm::try_new(
            scalar(1.).to_real_algebraic().unwrap(),
            1,
            0,
            0,
        )
        .unwrap()])
        .unwrap(),
        RegionPolynomial3::try_from_terms(vec![RegionPolynomialTerm::try_new(
            scalar(1.).to_real_algebraic().unwrap(),
            0,
            1,
            0,
        )
        .unwrap()])
        .unwrap(),
        RegionPolynomial3::zero(),
    )
    .unwrap();
    let substituted = added.try_affine_substitute(&map).unwrap();
    let function = RegionFunction::Polynomial(substituted);
    let atom = CanonicalAtom::try_new("x", function, SignRelation::Negative).unwrap();
    let region = construct_region(RegionConstructionRequest {
        ambient_dimension: 3,
        canonical_atoms: vec![atom],
        normalized_region_formula: NormalizedRegionFormula::Atom("x".into()),
        region_kind: RegionKind::RegularClosedVolume,
    })
    .unwrap();
    assert_eq!(region.canonical_atoms().len(), 1);
    let _ = multiplied;
}

#[test]
fn exact_region_unconvertible_coefficient_failure() {
    assert!(matches!(
        ExactScalar::new(f64::NAN),
        Err(CadConstructionFailure::ExactConversion(
            ExactConversionFailure {
                kind: ExactConversionFailureKind::NonFinite,
                ..
            }
        ))
    ));
}

#[test]
fn exact_region_zero_denominator_failure() {
    let zero = RegionPolynomial3::zero();
    let one = RegionPolynomial3::try_from_terms(vec![RegionPolynomialTerm::try_new(
        scalar(1.).to_real_algebraic().unwrap(),
        0,
        0,
        0,
    )
    .unwrap()])
    .unwrap();
    let domain = RationalFunctionDomain3::try_new(zero);
    assert!(matches!(
        domain,
        Err(GeometryFailure::MalformedRegionFormula)
    ));
    let _ = one;
}

#[test]
fn exact_region_duplicate_exponent_failure() {
    let failure =
        region_polynomial_from_exact_terms(&[(scalar(1.), 1, 0, 0), (scalar(-1.), 1, 0, 0)]);
    assert!(matches!(
        failure,
        Err(CadConstructionFailure::Lowering {
            condition: LoweringCondition::DuplicateExponent,
            ..
        })
    ));
}

#[test]
fn exact_region_duplicate_term_failure() {
    let failure =
        region_polynomial_from_exact_terms(&[(scalar(1.), 1, 0, 0), (scalar(1.), 1, 0, 0)]);
    assert!(matches!(
        failure,
        Err(CadConstructionFailure::Lowering {
            condition: LoweringCondition::DuplicateTerm,
            ..
        })
    ));
}

#[test]
fn exact_region_dimension_mismatch_failure() {
    let constructed = construct(request(vec![primitive("sphere")])).unwrap();
    let region = constructed.region(&identity_of("sphere")).unwrap();
    let dummy = construct_region(RegionConstructionRequest {
        ambient_dimension: 3,
        canonical_atoms: Vec::new(),
        normalized_region_formula: NormalizedRegionFormula::True,
        region_kind: RegionKind::GeneralSemiAlgebraicSet,
    })
    .unwrap();
    let result = region.membership(RegionMembershipRequest {
        region: dummy,
        point: RegionPoint::Planar(Point2 {
            x: scalar(0.).to_real_algebraic().unwrap(),
            y: scalar(0.).to_real_algebraic().unwrap(),
        }),
    });
    assert_eq!(result, Err(GeometryFailure::RegionPointDimensionMismatch));
}

#[test]
fn exact_region_unknown_atom_failure() {
    let polynomial = RegionPolynomial3::try_from_terms(vec![RegionPolynomialTerm::try_new(
        scalar(1.).to_real_algebraic().unwrap(),
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
    let result = construct_region(RegionConstructionRequest {
        ambient_dimension: 3,
        canonical_atoms: vec![atom],
        normalized_region_formula: NormalizedRegionFormula::Atom("missing".into()),
        region_kind: RegionKind::GeneralSemiAlgebraicSet,
    });
    assert_eq!(
        result,
        Err(CadFailure::Geometry(
            GeometryFailure::MalformedRegionFormula
        ))
    );
}

#[test]
fn primitive_region_membership() {
    let primitives = vec![
        (
            "sphere",
            PrimitiveDeclaration::Sphere { radius: scalar(1.) },
            [0., 0., 0.],
            [1., 0., 0.],
            [2., 0., 0.],
        ),
        (
            "ellipsoid",
            PrimitiveDeclaration::Ellipsoid {
                radii: ExactVector3::new([2., 1., 1.]).unwrap(),
            },
            [0., 0., 0.],
            [2., 0., 0.],
            [3., 0., 0.],
        ),
        (
            "box",
            PrimitiveDeclaration::Box {
                lengths: ExactVector3::new([2., 2., 2.]).unwrap(),
            },
            [0., 0., 0.],
            [1., 0., 0.],
            [2., 0., 0.],
        ),
        (
            "cylinder",
            PrimitiveDeclaration::Cylinder {
                outer_radius: scalar(1.),
                inner_radius: None,
                height: scalar(2.),
            },
            [0., 0., 1.],
            [1., 0., 1.],
            [2., 0., 1.],
        ),
        (
            "hollow",
            PrimitiveDeclaration::Cylinder {
                outer_radius: scalar(2.),
                inner_radius: Some(scalar(1.)),
                height: scalar(2.),
            },
            [1.5, 0., 1.],
            [2., 0., 1.],
            [3., 0., 1.],
        ),
        (
            "cone",
            PrimitiveDeclaration::Cone {
                base_radius: scalar(2.),
                top_radius: scalar(1.),
                height: scalar(2.),
            },
            [0., 0., 1.],
            [0., 0., 0.],
            [3., 0., 1.],
        ),
        (
            "torus",
            PrimitiveDeclaration::Torus {
                major_radius: scalar(2.),
                minor_radius: scalar(1.),
            },
            [2., 0., 0.],
            [3., 0., 0.],
            [4., 0., 0.],
        ),
    ];
    for (name, declaration, inside, boundary, outside) in primitives {
        let result = construct(request(vec![op(
            name,
            OperationRequest::Primitive(declaration),
            GeometryOutputCategory::Solid,
        )]))
        .unwrap();
        let region = result.region(&identity_of(name)).unwrap();
        assert_eq!(region.region_kind(), RegionKind::RegularClosedVolume);
        assert!(!region.canonical_atoms().is_empty());
        assert_eq!(classify(region, inside), MembershipClassification::Inside);
        assert_eq!(
            classify(region, boundary),
            MembershipClassification::Boundary
        );
        assert_eq!(classify(region, outside), MembershipClassification::Outside);
    }
}

#[test]
fn profile_lowering_classification_and_embedding() {
    let result = construct(request(vec![closed_square_profile("profile")])).unwrap();
    let region = result.region(&identity_of("profile")).unwrap();
    assert_eq!(region.region_kind(), RegionKind::GeneralSemiAlgebraicSet);
    assert_eq!(
        classify(region, [0., 0., 0.]),
        MembershipClassification::Inside
    );
    assert_eq!(
        classify(region, [1., 0., 0.]),
        MembershipClassification::Boundary
    );
    assert_eq!(
        classify(region, [2., 0., 0.]),
        MembershipClassification::Outside
    );

    let mut open = closed_square_profile("open");
    if let OperationRequest::Profile(profile) = &mut open.request {
        profile.closure = GeometryClosure::Open;
        open.expected_output.closure = GeometryClosure::Open;
    }
    assert!(matches!(
        construct(request(vec![open])),
        Err(CadConstructionFailure::Lowering {
            condition: LoweringCondition::OpenProfile,
            ..
        })
    ));

    let skewed = op(
        "skewed",
        OperationRequest::Profile(ProfileRequest {
            origin: point(),
            x_axis: vector(),
            normal: vector(),
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
                    position: [scalar(1.), scalar(1.)],
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
                    id: "ca".into(),
                    start_corner: "c".into(),
                    end_corner: "a".into(),
                    curve: ProfileSegmentCurve::Line,
                },
            ],
            corner_order: vec!["a".into(), "b".into(), "c".into()],
            segment_order: vec!["ab".into(), "bc".into(), "ca".into()],
            closure: GeometryClosure::Closed,
        }),
        GeometryOutputCategory::Profile,
    );
    assert!(matches!(
        construct(request(vec![skewed])),
        Err(CadConstructionFailure::Lowering {
            condition: LoweringCondition::NonOrthonormalFrame,
            ..
        })
    ));

    let crossing = op(
        "crossing",
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
                    position: [scalar(2.), scalar(2.)],
                },
                ProfileCorner {
                    id: "c".into(),
                    position: [scalar(0.), scalar(2.)],
                },
                ProfileCorner {
                    id: "d".into(),
                    position: [scalar(2.), scalar(0.)],
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
    );
    assert!(matches!(
        construct(request(vec![crossing])),
        Err(CadConstructionFailure::Lowering {
            condition: LoweringCondition::SelfIntersection,
            ..
        })
    ));

    let nurbs = op(
        "nurbs-profile",
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
                    position: [scalar(1.), scalar(1.)],
                },
            ],
            segments: vec![
                ProfileSegment {
                    id: "ab".into(),
                    start_corner: "a".into(),
                    end_corner: "b".into(),
                    curve: ProfileSegmentCurve::Nurbs {
                        degree: 1,
                        knots: vec![scalar(0.), scalar(0.), scalar(1.), scalar(1.)],
                        control_points: vec![[scalar(0.), scalar(0.)], [scalar(1.), scalar(0.)]],
                        weights: vec![scalar(1.), scalar(1.)],
                        parameter_domain: [scalar(0.), scalar(1.)],
                    },
                },
                ProfileSegment {
                    id: "bc".into(),
                    start_corner: "b".into(),
                    end_corner: "c".into(),
                    curve: ProfileSegmentCurve::Line,
                },
                ProfileSegment {
                    id: "ca".into(),
                    start_corner: "c".into(),
                    end_corner: "a".into(),
                    curve: ProfileSegmentCurve::Line,
                },
            ],
            corner_order: vec!["a".into(), "b".into(), "c".into()],
            segment_order: vec!["ab".into(), "bc".into(), "ca".into()],
            closure: GeometryClosure::Closed,
        }),
        GeometryOutputCategory::Profile,
    );
    assert!(matches!(
        construct(request(vec![nurbs])),
        Err(CadConstructionFailure::Lowering {
            condition: LoweringCondition::GeneralNurbsSegment,
            ..
        })
    ));

    let mut reordered = closed_square_profile("reordered");
    if let OperationRequest::Profile(profile) = &mut reordered.request {
        profile.segment_order = vec!["ab".into(), "cd".into(), "bc".into(), "da".into()];
    }
    assert!(matches!(
        construct(request(vec![reordered])),
        Err(CadConstructionFailure::Lowering {
            condition: LoweringCondition::OrderMismatch,
            ..
        })
    ));

    let disconnected = op(
        "disconnected",
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
                    position: [scalar(0.), scalar(1.)],
                },
                ProfileCorner {
                    id: "d".into(),
                    position: [scalar(3.), scalar(0.)],
                },
                ProfileCorner {
                    id: "e".into(),
                    position: [scalar(4.), scalar(0.)],
                },
                ProfileCorner {
                    id: "f".into(),
                    position: [scalar(3.), scalar(1.)],
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
                    id: "ca".into(),
                    start_corner: "c".into(),
                    end_corner: "a".into(),
                    curve: ProfileSegmentCurve::Line,
                },
                ProfileSegment {
                    id: "de".into(),
                    start_corner: "d".into(),
                    end_corner: "e".into(),
                    curve: ProfileSegmentCurve::Line,
                },
                ProfileSegment {
                    id: "ef".into(),
                    start_corner: "e".into(),
                    end_corner: "f".into(),
                    curve: ProfileSegmentCurve::Line,
                },
                ProfileSegment {
                    id: "fd".into(),
                    start_corner: "f".into(),
                    end_corner: "d".into(),
                    curve: ProfileSegmentCurve::Line,
                },
            ],
            corner_order: vec![
                "a".into(),
                "b".into(),
                "c".into(),
                "d".into(),
                "e".into(),
                "f".into(),
            ],
            segment_order: vec![
                "ab".into(),
                "bc".into(),
                "ca".into(),
                "de".into(),
                "ef".into(),
                "fd".into(),
            ],
            closure: GeometryClosure::Closed,
        }),
        GeometryOutputCategory::Profile,
    );
    assert!(matches!(
        construct(request(vec![disconnected])),
        Err(CadConstructionFailure::Lowering {
            condition: LoweringCondition::DisconnectedProfile,
            ..
        })
    ));

    let hole = op(
        "hole",
        OperationRequest::Profile(ProfileRequest {
            origin: point(),
            x_axis: vector(),
            normal: z_axis(),
            corners: vec![
                ProfileCorner {
                    id: "a".into(),
                    position: [scalar(-2.), scalar(-2.)],
                },
                ProfileCorner {
                    id: "b".into(),
                    position: [scalar(2.), scalar(-2.)],
                },
                ProfileCorner {
                    id: "c".into(),
                    position: [scalar(2.), scalar(2.)],
                },
                ProfileCorner {
                    id: "d".into(),
                    position: [scalar(-2.), scalar(2.)],
                },
                ProfileCorner {
                    id: "e".into(),
                    position: [scalar(-0.5), scalar(-0.5)],
                },
                ProfileCorner {
                    id: "f".into(),
                    position: [scalar(0.5), scalar(-0.5)],
                },
                ProfileCorner {
                    id: "g".into(),
                    position: [scalar(0.5), scalar(0.5)],
                },
                ProfileCorner {
                    id: "h".into(),
                    position: [scalar(-0.5), scalar(0.5)],
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
                ProfileSegment {
                    id: "ef".into(),
                    start_corner: "e".into(),
                    end_corner: "f".into(),
                    curve: ProfileSegmentCurve::Line,
                },
                ProfileSegment {
                    id: "fg".into(),
                    start_corner: "f".into(),
                    end_corner: "g".into(),
                    curve: ProfileSegmentCurve::Line,
                },
                ProfileSegment {
                    id: "gh".into(),
                    start_corner: "g".into(),
                    end_corner: "h".into(),
                    curve: ProfileSegmentCurve::Line,
                },
                ProfileSegment {
                    id: "he".into(),
                    start_corner: "h".into(),
                    end_corner: "e".into(),
                    curve: ProfileSegmentCurve::Line,
                },
            ],
            corner_order: vec![
                "a".into(),
                "b".into(),
                "c".into(),
                "d".into(),
                "e".into(),
                "f".into(),
                "g".into(),
                "h".into(),
            ],
            segment_order: vec![
                "ab".into(),
                "bc".into(),
                "cd".into(),
                "da".into(),
                "ef".into(),
                "fg".into(),
                "gh".into(),
                "he".into(),
            ],
            closure: GeometryClosure::Closed,
        }),
        GeometryOutputCategory::Profile,
    );
    assert!(matches!(
        construct(request(vec![hole])),
        Err(CadConstructionFailure::Lowering {
            condition: LoweringCondition::ProfileHole,
            ..
        })
    ));

    let concave = op(
        "concave",
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
                    position: [scalar(2.), scalar(0.)],
                },
                ProfileCorner {
                    id: "c".into(),
                    position: [scalar(1.), scalar(1.)],
                },
                ProfileCorner {
                    id: "d".into(),
                    position: [scalar(2.), scalar(2.)],
                },
                ProfileCorner {
                    id: "e".into(),
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
                    id: "de".into(),
                    start_corner: "d".into(),
                    end_corner: "e".into(),
                    curve: ProfileSegmentCurve::Line,
                },
                ProfileSegment {
                    id: "ea".into(),
                    start_corner: "e".into(),
                    end_corner: "a".into(),
                    curve: ProfileSegmentCurve::Line,
                },
            ],
            corner_order: vec!["a".into(), "b".into(), "c".into(), "d".into(), "e".into()],
            segment_order: vec![
                "ab".into(),
                "bc".into(),
                "cd".into(),
                "de".into(),
                "ea".into(),
            ],
            closure: GeometryClosure::Closed,
        }),
        GeometryOutputCategory::Profile,
    );
    assert!(matches!(
        construct(request(vec![concave])),
        Err(CadConstructionFailure::Lowering {
            condition: LoweringCondition::ImplicitizationFailed,
            ..
        })
    ));
}

#[test]
fn extrude_membership_and_substitution() {
    let profile = closed_square_profile("profile");
    let extrude = op(
        "extrude",
        OperationRequest::Extrude {
            profile: output("profile"),
            direction: z_axis(),
            distance: scalar(2.),
            symmetric: false,
        },
        GeometryOutputCategory::Solid,
    );
    let result = construct(request(vec![profile, extrude])).unwrap();
    let region = result.region(&identity_of("extrude")).unwrap();
    assert_eq!(region.region_kind(), RegionKind::RegularClosedVolume);
    assert_eq!(
        classify(region, [0., 0., 1.]),
        MembershipClassification::Inside
    );
    assert_eq!(
        classify(region, [1., 0., 1.]),
        MembershipClassification::Boundary
    );
    assert_eq!(
        classify(region, [0., 0., 3.]),
        MembershipClassification::Outside
    );

    let profile = closed_square_profile("profile");
    let symmetric = op(
        "extrude",
        OperationRequest::Extrude {
            profile: output("profile"),
            direction: z_axis(),
            distance: scalar(2.),
            symmetric: true,
        },
        GeometryOutputCategory::Solid,
    );
    let result = construct(request(vec![profile, symmetric])).unwrap();
    let region = result.region(&identity_of("extrude")).unwrap();
    assert_eq!(
        classify(region, [0., 0., 0.]),
        MembershipClassification::Inside
    );
    assert_eq!(
        classify(region, [0., 0., 1.]),
        MembershipClassification::Boundary
    );
    assert_eq!(
        classify(region, [0., 0., 2.]),
        MembershipClassification::Outside
    );
}

#[test]
fn revolve_full_membership() {
    let profile = revolve_rectangle_profile("profile");
    let revolve = op(
        "revolve",
        OperationRequest::Revolve {
            profile: output("profile"),
            axis_origin: point(),
            axis_direction: y_axis(),
            extent: RevolveExtent::Full,
        },
        GeometryOutputCategory::Solid,
    );
    let result = construct(request(vec![profile, revolve])).unwrap();
    let region = result.region(&identity_of("revolve")).unwrap();
    assert_eq!(region.region_kind(), RegionKind::RegularClosedVolume);
    assert_eq!(
        classify(region, [0.5, 1., 0.]),
        MembershipClassification::Inside
    );
    assert_eq!(
        classify(region, [0., 1., 0.]),
        MembershipClassification::Inside
    );
    assert_eq!(
        classify(region, [1., 1., 0.]),
        MembershipClassification::Boundary
    );
    assert_eq!(
        classify(region, [2., 1., 0.]),
        MembershipClassification::Outside
    );
}

#[test]
fn revolve_finite_angle_lowering_condition() {
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
}

#[test]
fn transform_membership_preservation() {
    let sphere = primitive("sphere");
    let translated = op(
        "moved",
        OperationRequest::Transform {
            source: output("sphere"),
            placement: ExactRigidPlacement::new(
                ExactPoint3::new([2., 0., 0.]).unwrap(),
                [1., 0., 0., 0.],
            )
            .unwrap(),
            scale: scalar(1.),
        },
        GeometryOutputCategory::Solid,
    );
    let result = construct(request(vec![sphere, translated])).unwrap();
    let source = result.region(&identity_of("sphere")).unwrap();
    let transformed = result.region(&identity_of("moved")).unwrap();
    assert_eq!(source.region_kind(), transformed.region_kind());
    assert_eq!(source.ambient_dimension(), transformed.ambient_dimension());
    assert_eq!(
        classify(transformed, [2., 0., 0.]),
        classify(source, [0., 0., 0.])
    );
    assert_eq!(
        classify(transformed, [3., 0., 0.]),
        classify(source, [1., 0., 0.])
    );
    assert_eq!(
        classify(transformed, [4., 0., 0.]),
        classify(source, [2., 0., 0.])
    );

    let sphere = primitive("sphere");
    let rotated = op(
        "rotated",
        OperationRequest::Transform {
            source: output("sphere"),
            placement: ExactRigidPlacement::new(point(), [0., 0., 0., 1.]).unwrap(),
            scale: scalar(1.),
        },
        GeometryOutputCategory::Solid,
    );
    let result = construct(request(vec![sphere, rotated])).unwrap();
    let source = result.region(&identity_of("sphere")).unwrap();
    let transformed = result.region(&identity_of("rotated")).unwrap();
    assert_eq!(
        classify(transformed, [0., 0., 0.]),
        classify(source, [0., 0., 0.])
    );

    let sphere = primitive("sphere");
    let scaled = op(
        "scaled",
        OperationRequest::Transform {
            source: output("sphere"),
            placement: placement(),
            scale: scalar(2.),
        },
        GeometryOutputCategory::Solid,
    );
    let result = construct(request(vec![sphere, scaled])).unwrap();
    let source = result.region(&identity_of("sphere")).unwrap();
    let transformed = result.region(&identity_of("scaled")).unwrap();
    assert_eq!(
        classify(transformed, [2., 0., 0.]),
        classify(source, [1., 0., 0.])
    );
    assert_eq!(result.source_revision().as_str(), "r1");
    assert!(result
        .identity_ledger()
        .operations
        .iter()
        .any(|binding| binding.operation.as_str() == "scaled"));
}

#[test]
fn owned_outputs_preserve_multiple_identities_and_variants() {
    let result = construct(request(vec![primitive("first"), primitive("second")])).unwrap();
    let outputs = result.into_outputs();
    assert_eq!(outputs.len(), 2);
    assert_eq!(outputs[0].0, identity_of("first"));
    assert_eq!(outputs[1].0, identity_of("second"));
    assert!(matches!(outputs[0].1, ConstructionOutput::Region(_)));
    assert!(matches!(outputs[1].1, ConstructionOutput::Region(_)));
}

#[test]
fn analytic_representation_map() {
    let result = construct(request(vec![
        line_curve("curve"),
        bilinear_surface("surface"),
    ]))
    .unwrap();
    assert!(matches!(
        result.analytic(&identity_of("curve")),
        Some(CurveOrSurfaceRepresentation::Curve(_))
    ));
    assert!(matches!(
        result.analytic(&identity_of("surface")),
        Some(CurveOrSurfaceRepresentation::Surface(_))
    ));
}

#[test]
fn unsupported_analytic_operation_lowering_condition() {
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
}

#[test]
fn typed_output_map_and_explicit_solid_adapter() {
    let sphere = primitive("sphere");
    let curve = line_curve("curve");
    let solid = tetrahedron_solid("solid");
    let result = construct(request(vec![sphere, curve, solid])).unwrap();
    assert!(result.region(&identity_of("sphere")).is_some());
    assert!(result.analytic(&identity_of("curve")).is_some());
    let imported = result.imported(&identity_of("solid")).unwrap();
    assert_eq!(imported.input_identity(), "solid");
}

fn contacting_sphere(name: &str, source: &str) -> ConstructionOperation {
    op(
        name,
        OperationRequest::Transform {
            source: output(source),
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

#[test]
fn set_and_solid_boolean_differ_on_contact_and_reject_imported() {
    let left = primitive("left");
    let right = primitive("right");
    let moved = contacting_sphere("moved", "right");
    let set = op(
        "set",
        OperationRequest::Boolean {
            request: BooleanRequest {
                kind: BooleanKind::Set,
                operator: BooleanOperator::Union,
            },
            left: output("left"),
            right: output("moved"),
        },
        GeometryOutputCategory::Solid,
    );
    let set_result = construct(request(vec![left, right, moved, set])).unwrap();
    assert_eq!(
        set_result
            .region(&identity_of("set"))
            .unwrap()
            .region_kind(),
        RegionKind::GeneralSemiAlgebraicSet
    );
    assert_eq!(
        classify(
            set_result.region(&identity_of("set")).unwrap(),
            [1., 0., 0.]
        ),
        MembershipClassification::Boundary
    );

    let left = primitive("left");
    let right = primitive("right");
    let moved = contacting_sphere("moved", "right");
    let solid = op(
        "solid",
        OperationRequest::Boolean {
            request: BooleanRequest {
                kind: BooleanKind::Solid,
                operator: BooleanOperator::Union,
            },
            left: output("left"),
            right: output("moved"),
        },
        GeometryOutputCategory::Solid,
    );
    let solid_result = construct(request(vec![left, right, moved, solid])).unwrap();
    assert_eq!(
        solid_result
            .region(&identity_of("solid"))
            .unwrap()
            .region_kind(),
        RegionKind::RegularClosedVolume
    );

    let imported = tetrahedron_solid("imported");
    let sphere = primitive("sphere");
    let boolean = op(
        "boolean",
        OperationRequest::Boolean {
            request: BooleanRequest {
                kind: BooleanKind::Set,
                operator: BooleanOperator::Union,
            },
            left: output("imported"),
            right: output("sphere"),
        },
        GeometryOutputCategory::Solid,
    );
    assert!(matches!(
        construct(request(vec![imported, sphere, boolean])),
        Err(CadConstructionFailure::Lowering {
            condition: LoweringCondition::ImportedBooleanChild,
            ..
        })
    ));
}

#[test]
fn external_adapter_failure_provenance() {
    let external = op(
        "external",
        OperationRequest::ExternalGeometry(ExternalGeometryRequest {
            asset_id: "asset".into(),
            source_relative_path: "asset.step".into(),
            unit_scale: scalar(1.),
            adapter: "step".into(),
            asset_schema: expected(GeometryOutputCategory::Solid),
        }),
        GeometryOutputCategory::Solid,
    );
    assert!(matches!(
        construct(request(vec![external])),
        Err(CadConstructionFailure::ExternalAdapter {
            asset_id,
            source_relative_path,
            adapter,
            ..
        }) if asset_id == "asset"
            && source_relative_path == "asset.step"
            && adapter == "step"
    ));

    let degenerate = op(
        "degenerate",
        OperationRequest::ExplicitSolid {
            vertices: vec![
                ExplicitSolidVertex {
                    id: "a".into(),
                    position: point(),
                },
                ExplicitSolidVertex {
                    id: "b".into(),
                    position: point(),
                },
                ExplicitSolidVertex {
                    id: "c".into(),
                    position: point(),
                },
            ],
            faces: vec![ExplicitSolidFace {
                id: "face".into(),
                vertex_order: vec!["a".into(), "b".into(), "c".into()],
            }],
        },
        GeometryOutputCategory::Solid,
    );
    assert!(matches!(
        construct(request(vec![degenerate])),
        Err(CadConstructionFailure::Lowering {
            lower: Some(boxed),
            ..
        }) if matches!(
            boxed.as_ref(),
            CadConstructionLowerFailure::Mesh(_) | CadConstructionLowerFailure::Import(_)
        )
    ));
}

#[test]
fn construction_permutation_and_first_failure_propagation() {
    let profile = closed_square_profile("profile");
    let sphere = primitive("sphere");
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
    let curve = line_curve("curve");
    let first = construct(request(vec![
        sphere.clone(),
        profile.clone(),
        curve.clone(),
        sweep.clone(),
    ]));
    let second = construct(request(vec![sweep, curve, sphere, profile]));
    assert_eq!(first, second);
    assert!(matches!(
        first,
        Err(CadConstructionFailure::Lowering {
            condition: LoweringCondition::SweepUnsupported,
            operation,
            ..
        }) if operation.as_str() == "sweep"
    ));
}

#[test]
fn exact_binary_and_payload_validation_precedes_capability() {
    assert_eq!(
        ExactScalar::new(0.).unwrap(),
        ExactScalar::new(-0.).unwrap()
    );
    assert!(!ExactScalar::new(f64::from_bits(1)).unwrap().is_zero());
    assert!(!ExactScalar::new(f64::MAX).unwrap().is_zero());
    assert!(matches!(
        ExactScalar::new(f64::NAN),
        Err(CadConstructionFailure::ExactConversion(
            ExactConversionFailure {
                kind: ExactConversionFailureKind::NonFinite,
                ..
            }
        ))
    ));
    assert!(matches!(
        ExactVector3::new([0.; 3]),
        Err(CadConstructionFailure::ExactConversion(
            ExactConversionFailure {
                kind: ExactConversionFailureKind::ZeroDirection,
                ..
            }
        ))
    ));
    assert!(matches!(
        ExactRigidPlacement::new(point(), [1., 1e-13, 0., 0.]),
        Err(CadConstructionFailure::ExactConversion(
            ExactConversionFailure {
                kind: ExactConversionFailureKind::NonUnitQuaternion,
                ..
            }
        ))
    ));
    let invalid = op(
        "n",
        OperationRequest::Curve(CurveDeclaration::Nurbs {
            degree: 2,
            knots: vec![scalar(0.)],
            control_points: vec![point()],
            weights: vec![scalar(1.)],
            parameter_domain: [scalar(0.), scalar(1.)],
            closed: false,
        }),
        GeometryOutputCategory::Curve,
    );
    assert!(matches!(
        construct(request(vec![invalid])),
        Err(CadConstructionFailure::PayloadInvalid { .. })
    ));
}

#[test]
fn graph_requires_canonical_solid_boolean_and_profile_inputs() {
    let curve = line_curve("curve");
    let boolean = op(
        "boolean",
        OperationRequest::Boolean {
            request: BooleanRequest {
                kind: BooleanKind::Set,
                operator: BooleanOperator::Union,
            },
            left: output("curve"),
            right: output("curve"),
        },
        GeometryOutputCategory::Solid,
    );
    assert!(matches!(
        ConstructionGraph::build(&[curve.clone(), boolean]),
        Err(ConstructionGraphFailure::InputKindMismatch {
            role: InputRole::Left,
            ..
        })
    ));
    let solid = primitive("solid");
    let extrude = op(
        "e",
        OperationRequest::Extrude {
            profile: output("solid"),
            direction: vector(),
            distance: scalar(1.),
            symmetric: false,
        },
        GeometryOutputCategory::Solid,
    );
    assert!(matches!(
        ConstructionGraph::build(&[solid, extrude]),
        Err(ConstructionGraphFailure::InputKindMismatch {
            role: InputRole::Profile,
            ..
        })
    ));
    let mut bad = output("curve");
    bad.output.slot = OutputSlot::Named("bad".into());
    let transform = op(
        "t",
        OperationRequest::Transform {
            source: bad,
            placement: placement(),
            scale: scalar(1.),
        },
        GeometryOutputCategory::Sheet,
    );
    assert!(matches!(
        ConstructionGraph::build(&[curve, transform]),
        Err(ConstructionGraphFailure::UnknownOutputSlot(_))
    ));
}

#[test]
fn output_schema_ledger_and_roots_are_checked() {
    let mut bad = primitive("p");
    bad.expected_output.ambient_dimension = 2;
    assert!(matches!(
        construct(request(vec![bad])),
        Err(CadConstructionFailure::OutputSchemaMismatch { .. })
    ));
    let p = primitive("p");
    assert!(matches!(
        IdentityLedger::new(vec![], vec![]),
        Err(IdentityFailure::EmptyLedger)
    ));
    let mut r = request(vec![p.clone()]);
    r.identity_ledger.operations.clear();
    r.identity_ledger.entities.clear();
    assert!(matches!(
        construct(r),
        Err(CadConstructionFailure::Identity(
            IdentityFailure::MissingOperationBinding(_)
        ))
    ));
    let mut r = request(vec![p]);
    r.roots = vec![
        ConstructionRoot::PhysicalBody {
            body: source("b"),
            output: output("p"),
            ordinal: 0,
        },
        ConstructionRoot::PhysicalBody {
            body: source("b2"),
            output: output("p"),
            ordinal: 0,
        },
    ];
    assert!(matches!(
        construct(r),
        Err(CadConstructionFailure::Graph(
            ConstructionGraphFailure::DuplicateBodyOrdinal(0)
        ))
    ));
}

#[test]
fn roots_are_order_independent_and_identity_entities_are_complete() {
    let p = primitive("p");
    let composition = BodyComposition {
        policy_id: "policy".into(),
        obstacle_operation: "obstacle".into(),
        material_operation: "material".into(),
        overlap_precedence: "first".into(),
        tolerance_policy: "exact".into(),
    };
    let body = ConstructionRoot::PhysicalBody {
        body: source("body"),
        output: output("p"),
        ordinal: 0,
    };
    let domain = ConstructionRoot::PhysicalDomain {
        domain: source("domain"),
        bodies: vec![source("body")],
        composition: composition.clone(),
    };
    let mut domain_first = request(vec![p.clone()]);
    domain_first.provenance.composition = vec![composition.clone()];
    domain_first.roots = vec![domain.clone(), body.clone()];
    let mut body_first = request(vec![p]);
    body_first.provenance.composition = vec![composition];
    body_first.roots = vec![body, domain];
    let first = construct(domain_first).unwrap();
    let second = construct(body_first).unwrap();
    assert_eq!(
        first.region(&identity_of("p")).unwrap().region_kind(),
        RegionKind::RegularClosedVolume
    );
    assert_eq!(
        first.region(&identity_of("p")).unwrap().region_kind(),
        second.region(&identity_of("p")).unwrap().region_kind()
    );

    let p = primitive("p");
    let mut r = request(vec![p.clone()]);
    r.identity_ledger.entities.clear();
    assert!(matches!(
        construct(r),
        Err(CadConstructionFailure::Identity(
            IdentityFailure::MissingEntityBinding(_)
        ))
    ));
    let mut r = request(vec![p.clone()]);
    r.identity_ledger.entities.push(EntityIdentityBinding {
        source: p.source_object.clone(),
        operation: p.operation.clone(),
        output: p.output_identity(),
        produced: "p:other".into(),
    });
    assert!(matches!(
        construct(r),
        Err(CadConstructionFailure::Identity(
            IdentityFailure::DuplicateEntityBinding(_)
        ))
    ));
    let mut r = request(vec![p]);
    r.identity_ledger.entities[0].operation = id("missing");
    assert!(matches!(
        construct(r),
        Err(CadConstructionFailure::Identity(
            IdentityFailure::ExtraEntityBinding(_)
        ))
    ));
}

#[test]
fn payload_schema_and_external_asset_counterexamples_are_rejected_before_dispatch() {
    let mut invalid_primitive = primitive("primitive");
    invalid_primitive.request =
        OperationRequest::Primitive(PrimitiveDeclaration::Sphere { radius: scalar(0.) });
    assert!(matches!(
        construct(request(vec![invalid_primitive])),
        Err(CadConstructionFailure::PayloadInvalid {
            field: "primitive.dimensions",
            ..
        })
    ));

    let profile = closed_square_profile("profile");
    let mut open_profile = profile.clone();
    if let OperationRequest::Profile(request) = &mut open_profile.request {
        request.closure = GeometryClosure::Open;
    }
    open_profile.expected_output.closure = GeometryClosure::Open;
    let extrusion = op(
        "extrude",
        OperationRequest::Extrude {
            profile: output("profile"),
            direction: vector(),
            distance: scalar(1.),
            symmetric: false,
        },
        GeometryOutputCategory::Solid,
    );
    assert!(matches!(
        construct(request(vec![open_profile, extrusion])),
        Err(CadConstructionFailure::OutputSchemaMismatch { operation, .. }) if operation.as_str() == "extrude"
    ));

    let external = op(
        "external",
        OperationRequest::ExternalGeometry(ExternalGeometryRequest {
            asset_id: "asset".into(),
            source_relative_path: "asset.step".into(),
            unit_scale: scalar(1.),
            adapter: "step".into(),
            asset_schema: expected(GeometryOutputCategory::Solid),
        }),
        GeometryOutputCategory::Sheet,
    );
    assert!(matches!(
        construct(request(vec![external])),
        Err(CadConstructionFailure::OutputSchemaMismatch { .. })
    ));
}

#[test]
fn ledger_and_graph_invariants_are_typed_and_canonical() {
    let operation = primitive("op");
    let binding = OperationIdentityBinding {
        source: operation.source_object.clone(),
        operation: operation.operation.clone(),
        output: operation.output_identity(),
    };
    let entity = EntityIdentityBinding {
        source: operation.source_object.clone(),
        operation: operation.operation.clone(),
        output: operation.output_identity(),
        produced: "op:primary".into(),
    };
    assert!(matches!(
        IdentityLedger::new(vec![binding.clone(), binding.clone()], vec![entity.clone()]),
        Err(IdentityFailure::DuplicateOperation(_))
    ));
    assert!(matches!(
        IdentityLedger::new(vec![binding.clone()], vec![]),
        Err(IdentityFailure::EmptyLedger)
    ));
    let mut placeholder = entity.clone();
    placeholder.produced = "placeholder".into();
    assert!(matches!(
        IdentityLedger::new(vec![binding.clone()], vec![placeholder]),
        Err(IdentityFailure::UnknownProducedEntity(_))
    ));
    let mut extra = entity.clone();
    extra.operation = id("other");
    assert!(matches!(
        IdentityLedger::new(vec![binding.clone()], vec![extra]),
        Err(IdentityFailure::ExtraEntityBinding(_))
    ));

    let a = primitive("a");
    let bad_slot = op(
        "z",
        OperationRequest::Transform {
            source: OperationOutputRef {
                output: OperationOutputIdentity {
                    operation: id("missing"),
                    slot: OutputSlot::Named("secondary".into()),
                },
            },
            placement: placement(),
            scale: scalar(1.),
        },
        GeometryOutputCategory::Solid,
    );
    let bad_dependency = op(
        "y",
        OperationRequest::Transform {
            source: output("also-missing"),
            placement: placement(),
            scale: scalar(1.),
        },
        GeometryOutputCategory::Solid,
    );
    let first = ConstructionGraph::build(&[a.clone(), bad_slot.clone(), bad_dependency.clone()]);
    let second = ConstructionGraph::build(&[bad_dependency, a, bad_slot]);
    assert_eq!(first, second);
    assert!(matches!(
        first,
        Err(ConstructionGraphFailure::UnknownDependency(_))
    ));
}
