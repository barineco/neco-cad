use neco_algnum::RealAlgebraic;
use neco_bigint::BigInt;
use neco_cad::{
    construct_region, derive_analytic_brep, derive_complete_boundary, plane_surface_covering,
    tessellate_adaptive, unit_sphere_stereographic_covering, AdaptivePrecision,
    AdaptiveTessellationCondition, AdaptiveTessellationRequest, AlgebraicRegion, CadFailure,
    CanonicalAtom, FaceExactSurface, NormalizedRegionFormula, ParameterInterval,
    RegionConstructionRequest, RegionFunction, RegionKind, RegionPolynomial3, RegionPolynomialTerm,
    SignRelation, SourceRevision, SurfaceParameterDomain, SurfaceParameterPoint, SurfaceTrimLoop2,
    TessellationConsumer, TrimLoopRole,
};

fn algebraic(value: i32) -> RealAlgebraic {
    RealAlgebraic::from_integer(BigInt::try_from(value).unwrap()).unwrap()
}

fn sphere_region() -> AlgebraicRegion {
    let polynomial = RegionPolynomial3::try_from_terms(vec![
        RegionPolynomialTerm::try_new(algebraic(1), 2, 0, 0).unwrap(),
        RegionPolynomialTerm::try_new(algebraic(1), 0, 2, 0).unwrap(),
        RegionPolynomialTerm::try_new(algebraic(1), 0, 0, 2).unwrap(),
        RegionPolynomialTerm::try_new(algebraic(-1), 0, 0, 0).unwrap(),
    ])
    .unwrap();
    let atom = CanonicalAtom::try_new(
        "sphere",
        RegionFunction::Polynomial(polynomial),
        SignRelation::Negative,
    )
    .unwrap();
    construct_region(RegionConstructionRequest {
        ambient_dimension: 3,
        canonical_atoms: vec![atom],
        normalized_region_formula: NormalizedRegionFormula::Atom(String::from("sphere")),
        region_kind: RegionKind::RegularClosedVolume,
    })
    .unwrap()
}

fn sphere_atom() -> CanonicalAtom {
    let polynomial = RegionPolynomial3::try_from_terms(vec![
        RegionPolynomialTerm::try_new(algebraic(1), 2, 0, 0).unwrap(),
        RegionPolynomialTerm::try_new(algebraic(1), 0, 2, 0).unwrap(),
        RegionPolynomialTerm::try_new(algebraic(1), 0, 0, 2).unwrap(),
        RegionPolynomialTerm::try_new(algebraic(-1), 0, 0, 0).unwrap(),
    ])
    .unwrap();
    CanonicalAtom::try_new(
        "sphere",
        RegionFunction::Polynomial(polynomial),
        SignRelation::Zero,
    )
    .unwrap()
}

fn plane_atom() -> CanonicalAtom {
    let polynomial = RegionPolynomial3::try_from_terms(vec![RegionPolynomialTerm::try_new(
        algebraic(1),
        0,
        0,
        1,
    )
    .unwrap()])
    .unwrap();
    CanonicalAtom::try_new(
        "plane",
        RegionFunction::Polynomial(polynomial),
        SignRelation::Zero,
    )
    .unwrap()
}

fn sphere_brep() -> neco_cad::AnalyticBrep {
    let boundary = derive_complete_boundary(sphere_region()).unwrap();
    derive_analytic_brep(
        boundary,
        SourceRevision::new(String::from("sphere-r1")).unwrap(),
    )
    .unwrap()
    .into_parts()
    .0
}

fn precision(consumer: TessellationConsumer, chord: f64, surface: f64) -> AdaptivePrecision {
    AdaptivePrecision::new(consumer, chord, surface).unwrap()
}

fn plane_domain() -> SurfaceParameterDomain {
    SurfaceParameterDomain::new(
        ParameterInterval::new(algebraic(1), algebraic(5), true, true).unwrap(),
        ParameterInterval::new(algebraic(1), algebraic(5), true, true).unwrap(),
    )
}

fn square_loop(role: TrimLoopRole, values: [(i32, i32); 4]) -> SurfaceTrimLoop2 {
    let domain = plane_domain();
    SurfaceTrimLoop2::try_new(
        role,
        values
            .into_iter()
            .map(|(u, v)| SurfaceParameterPoint::new(algebraic(u), algebraic(v)))
            .collect(),
        &domain,
    )
    .unwrap()
}

fn tessellate_sphere(consumer: TessellationConsumer, chord: f64) -> neco_cad::AdaptiveTessellation {
    let brep = sphere_brep();
    let face_id = String::from(brep.faces()[0].id());
    tessellate_adaptive(
        AdaptiveTessellationRequest::new(
            "mesh:sphere",
            brep,
            vec![FaceExactSurface::new(
                face_id,
                sphere_atom(),
                unit_sphere_stereographic_covering().unwrap(),
            )],
            precision(consumer, chord, 1.0),
            48,
        )
        .unwrap(),
    )
    .unwrap()
}

#[test]
fn unit_sphere_preserves_identity_revision_and_established_bounds() {
    let result = tessellate_sphere(TessellationConsumer::Display, 0.5);
    assert_eq!(result.identity().as_str(), "mesh:sphere");
    assert_eq!(result.source_revision().as_str(), "sphere-r1");
    assert_eq!(
        result.provenance().face_atom_ids()[0].1,
        String::from("sphere")
    );
    assert!(result.precision_witness().established_maximum_chord_error() <= 0.5);
    assert!(
        result
            .precision_witness()
            .established_maximum_surface_error()
            < &{ neco_bigint::Dyadic::from_f64_exact(1.0).unwrap() }
    );
    assert!(result.validated_surface_mesh().mesh().triangles().len() > 2);
}

#[test]
fn ownership_conversions_preserve_adaptive_mesh_and_source() {
    let mesh = tessellate_sphere(TessellationConsumer::Display, 0.5).into_validated_surface_mesh();
    assert!(mesh.mesh().triangles().len() > 2);

    let closed = tessellate_sphere(TessellationConsumer::Display, 0.5)
        .into_closed_oriented_surface_mesh()
        .unwrap();
    assert_eq!(closed.source().identity(), "mesh:sphere");
    assert_eq!(closed.source().revision(), "sphere-r1");
    assert!(closed.closure_witness().edge_count() > 0);
}

#[test]
fn plane_hole_excludes_hole_centroids() {
    let brep = sphere_brep();
    let face_id = String::from(brep.faces()[0].id());
    let domain = plane_domain();
    let covering = plane_surface_covering(
        domain,
        Some(square_loop(
            TrimLoopRole::Outer,
            [(1, 1), (5, 1), (5, 5), (1, 5)],
        )),
        vec![square_loop(
            TrimLoopRole::Hole,
            [(2, 2), (4, 2), (4, 4), (2, 4)],
        )],
    )
    .unwrap();
    let result = tessellate_adaptive(
        AdaptiveTessellationRequest::new(
            "mesh:plane-hole",
            brep,
            vec![FaceExactSurface::new(face_id, plane_atom(), covering)],
            precision(TessellationConsumer::Display, 2.0, 1.0),
            48,
        )
        .unwrap(),
    )
    .unwrap();
    let mesh = result.validated_surface_mesh().mesh();
    assert!(!mesh.triangles().is_empty());
    for triangle in mesh.triangles() {
        let [a, b, c] = *triangle;
        let centroid_u =
            (mesh.vertices()[a][0] + mesh.vertices()[b][0] + mesh.vertices()[c][0]) / 3.0;
        let centroid_v =
            (mesh.vertices()[a][1] + mesh.vertices()[b][1] + mesh.vertices()[c][1]) / 3.0;
        assert!(
            !(centroid_u > 2.0 && centroid_u < 4.0 && centroid_v > 2.0 && centroid_v < 4.0),
            "centroid ({centroid_u}, {centroid_v}) stayed inside the hole"
        );
    }
}

#[test]
fn full_domain_plane_returns_established_bounds() {
    let brep = sphere_brep();
    let face_id = String::from(brep.faces()[0].id());
    let result = tessellate_adaptive(
        AdaptiveTessellationRequest::new(
            "mesh:plane",
            brep,
            vec![FaceExactSurface::new(
                face_id,
                plane_atom(),
                plane_surface_covering(plane_domain(), None, Vec::new()).unwrap(),
            )],
            precision(TessellationConsumer::Picking, 2.0, 1.0),
            48,
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(result.identity().as_str(), "mesh:plane");
    assert!(result.precision_witness().established_maximum_chord_error() <= 2.0);
    assert!(!result
        .validated_surface_mesh()
        .mesh()
        .triangles()
        .is_empty());
}

#[test]
fn tighter_analysis_precision_refines_sphere() {
    let display = tessellate_sphere(TessellationConsumer::Display, 0.45);
    let analysis = tessellate_sphere(TessellationConsumer::Analysis, 0.35);
    assert!(
        analysis.validated_surface_mesh().mesh().triangles().len()
            > display.validated_surface_mesh().mesh().triangles().len()
    );
    assert!(
        analysis
            .precision_witness()
            .established_maximum_chord_error()
            <= 0.35
    );
}

#[test]
fn surface_error_requirement_is_enforced() {
    let brep = sphere_brep();
    let face_id = String::from(brep.faces()[0].id());
    assert_eq!(
        tessellate_adaptive(
            AdaptiveTessellationRequest::new(
                "mesh:surface-error",
                brep,
                vec![FaceExactSurface::new(
                    face_id,
                    sphere_atom(),
                    unit_sphere_stereographic_covering().unwrap(),
                )],
                precision(TessellationConsumer::Analysis, 0.5, 1.0e-30),
                48,
            )
            .unwrap(),
        )
        .err(),
        Some(CadFailure::AdaptiveTessellation {
            condition: AdaptiveTessellationCondition::SurfaceErrorExceeded,
            identity: Some(String::from("mesh:surface-error")),
        })
    );
}

#[test]
fn empty_identity_invalid_precision_and_missing_covering_typed_failure() {
    assert_eq!(
        AdaptiveTessellationRequest::new(
            "",
            sphere_brep(),
            Vec::new(),
            precision(TessellationConsumer::Display, 0.5, 1.0),
            48,
        )
        .err(),
        Some(CadFailure::AdaptiveTessellation {
            condition: AdaptiveTessellationCondition::EmptyIdentity,
            identity: None,
        })
    );
    assert_eq!(
        CadFailure::from(
            AdaptivePrecision::new(TessellationConsumer::Display, 0.0, 1.0).unwrap_err()
        ),
        CadFailure::AdaptiveTessellation {
            condition: AdaptiveTessellationCondition::InvalidPrecision,
            identity: None,
        }
    );
    let brep = sphere_brep();
    assert_eq!(
        tessellate_adaptive(
            AdaptiveTessellationRequest::new(
                "mesh:missing",
                brep,
                Vec::new(),
                precision(TessellationConsumer::Display, 0.5, 1.0),
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
