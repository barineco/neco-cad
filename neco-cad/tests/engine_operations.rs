use neco_algnum::RealAlgebraic;
use neco_bigint::BigInt;
use neco_cad::{
    construct_region, derive_analytic_brep, derive_complete_boundary, AlgebraicRegion,
    CanonicalAtom, NormalizedRegionFormula, RegionConstructionRequest, RegionFunction, RegionKind,
    RegionPolynomial3, RegionPolynomialTerm, SignRelation, SourceRevision,
};

fn algebraic(value: i32) -> RealAlgebraic {
    RealAlgebraic::from_integer(BigInt::try_from(value).unwrap()).unwrap()
}

fn region(id: &str, terms: Vec<RegionPolynomialTerm>) -> AlgebraicRegion {
    let polynomial = RegionPolynomial3::try_from_terms(terms).unwrap();
    let atom = CanonicalAtom::try_new(
        id,
        RegionFunction::Polynomial(polynomial),
        SignRelation::Negative,
    )
    .unwrap();
    construct_region(RegionConstructionRequest {
        ambient_dimension: 3,
        canonical_atoms: vec![atom],
        normalized_region_formula: NormalizedRegionFormula::Atom(id.into()),
        region_kind: RegionKind::RegularClosedVolume,
    })
    .unwrap()
}

#[test]
fn region_boundary_and_brep_operations_form_an_engine_path() {
    let sphere = region(
        "sphere",
        vec![
            RegionPolynomialTerm::try_new(algebraic(1), 2, 0, 0).unwrap(),
            RegionPolynomialTerm::try_new(algebraic(1), 0, 2, 0).unwrap(),
            RegionPolynomialTerm::try_new(algebraic(1), 0, 0, 2).unwrap(),
            RegionPolynomialTerm::try_new(algebraic(-1), 0, 0, 0).unwrap(),
        ],
    );
    let torus = region(
        "torus",
        vec![
            RegionPolynomialTerm::try_new(algebraic(9), 0, 0, 0).unwrap(),
            RegionPolynomialTerm::try_new(algebraic(-10), 2, 0, 0).unwrap(),
            RegionPolynomialTerm::try_new(algebraic(-10), 0, 2, 0).unwrap(),
            RegionPolynomialTerm::try_new(algebraic(6), 0, 0, 2).unwrap(),
            RegionPolynomialTerm::try_new(algebraic(1), 4, 0, 0).unwrap(),
            RegionPolynomialTerm::try_new(algebraic(1), 0, 4, 0).unwrap(),
            RegionPolynomialTerm::try_new(algebraic(1), 0, 0, 4).unwrap(),
            RegionPolynomialTerm::try_new(algebraic(2), 2, 2, 0).unwrap(),
            RegionPolynomialTerm::try_new(algebraic(2), 2, 0, 2).unwrap(),
            RegionPolynomialTerm::try_new(algebraic(2), 0, 2, 2).unwrap(),
        ],
    );
    assert_eq!(sphere.region_kind(), RegionKind::RegularClosedVolume);
    assert_eq!(torus.region_kind(), RegionKind::RegularClosedVolume);
    let boundary = derive_complete_boundary(sphere).unwrap();
    assert_eq!(boundary.ambient_dimension(), 3);
    assert!(!boundary.strata().is_empty());

    let derived =
        derive_analytic_brep(boundary, SourceRevision::new("sphere-r1".into()).unwrap()).unwrap();
    assert!(!derived.brep().faces().is_empty());
    assert!(!derived.brep().witnesses().closed_shell_ids().is_empty());
    assert_eq!(
        derived.brep().witnesses().source_revision().as_str(),
        "sphere-r1"
    );
    assert_eq!(
        derived.face_surfaces()[0].exact_surface().atom_id(),
        "sphere"
    );
}
