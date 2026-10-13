use neco_algnum::RealAlgebraic;
use neco_bigint::BigInt;
use neco_cad::{
    construct_region, derive_analytic_brep, derive_boundary, derive_complete_boundary,
    materialize_brep, project_numeric_coordinates, region_from_mesh, set_boolean, solid_boolean,
    tessellate_adaptive, AdaptiveTessellation, AdaptiveTessellationRequest, AlgebraicRegion,
    BoundaryDerivationRequest, BoundaryStratification, BrepMaterializationRequest, CadFailure,
    CanonicalAtom, CertifiedRegion, ClosedOrientedSurfaceMesh, ConstructionOutput,
    ConstructionResult, GeometryFailure, MeshFailure, NormalizedRegionFormula, NumericProjection,
    NumericProjectionRequest, RegionConstructionRequest, RegionFunction, RegionImportFailure,
    RegionImportRequest, RegionKind, RegionPolynomial3, RegionPolynomialTerm, SetBooleanRequest,
    SignRelation, SolidBooleanRequest, TessellationResult, ValidatedSurfaceMesh,
};

fn algebraic(value: i32) -> RealAlgebraic {
    RealAlgebraic::from_integer(BigInt::try_from(value).unwrap()).unwrap()
}

fn sphere(id: &str, center_x: i32, radius: i32, kind: RegionKind) -> AlgebraicRegion {
    let polynomial = RegionPolynomial3::try_from_terms(vec![
        RegionPolynomialTerm::try_new(algebraic(1), 2, 0, 0).unwrap(),
        RegionPolynomialTerm::try_new(algebraic(1), 0, 2, 0).unwrap(),
        RegionPolynomialTerm::try_new(algebraic(1), 0, 0, 2).unwrap(),
        RegionPolynomialTerm::try_new(algebraic(-2 * center_x), 1, 0, 0).unwrap(),
        RegionPolynomialTerm::try_new(algebraic(center_x * center_x - radius * radius), 0, 0, 0)
            .unwrap(),
    ])
    .unwrap();
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
        region_kind: kind,
    })
    .unwrap()
}

#[test]
fn crate_root_exposes_the_region_first_engine_api() {
    let construct: fn(RegionConstructionRequest) -> Result<AlgebraicRegion, CadFailure> =
        construct_region;
    let set: fn(SetBooleanRequest) -> Result<AlgebraicRegion, CadFailure> = set_boolean;
    let solid: fn(SolidBooleanRequest) -> Result<AlgebraicRegion, CadFailure> = solid_boolean;
    let boundary: fn(BoundaryDerivationRequest) -> Result<BoundaryStratification, CadFailure> =
        derive_boundary;
    let complete: fn(AlgebraicRegion) -> Result<BoundaryStratification, CadFailure> =
        derive_complete_boundary;
    let derived: fn(
        BoundaryStratification,
        neco_cad::SourceRevision,
    ) -> Result<neco_cad::DerivedAnalyticBrep, CadFailure> = derive_analytic_brep;
    let brep: fn(BrepMaterializationRequest) -> Result<neco_cad::AnalyticBrep, CadFailure> =
        materialize_brep;
    let project: fn(NumericProjectionRequest) -> Result<NumericProjection, CadFailure> =
        project_numeric_coordinates;
    let adaptive: fn(AdaptiveTessellationRequest) -> Result<AdaptiveTessellation, CadFailure> =
        tessellate_adaptive;
    let construction_outputs: fn(
        ConstructionResult,
    )
        -> Vec<(neco_cad::OperationOutputIdentity, ConstructionOutput)> =
        ConstructionResult::into_outputs;
    let tessellation_mesh: fn(TessellationResult) -> ValidatedSurfaceMesh =
        TessellationResult::into_validated_surface_mesh;
    let adaptive_mesh: fn(AdaptiveTessellation) -> ValidatedSurfaceMesh =
        AdaptiveTessellation::into_validated_surface_mesh;
    let closed_adaptive_mesh: fn(
        AdaptiveTessellation,
    ) -> Result<ClosedOrientedSurfaceMesh, MeshFailure> =
        AdaptiveTessellation::into_closed_oriented_surface_mesh;
    let imported_region: fn(
        ClosedOrientedSurfaceMesh,
        RegionImportRequest,
    ) -> Result<CertifiedRegion, RegionImportFailure> = region_from_mesh;
    let _ = (
        construct,
        set,
        solid,
        boundary,
        complete,
        derived,
        brep,
        project,
        adaptive,
        construction_outputs,
        tessellation_mesh,
        adaptive_mesh,
        closed_adaptive_mesh,
        imported_region,
    );
}

#[test]
fn set_and_solid_boolean_preserve_their_distinct_region_kinds() {
    let set = set_boolean(SetBooleanRequest {
        left: sphere("left", 0, 2, RegionKind::RegularClosedVolume),
        right: sphere("right", 1, 2, RegionKind::RegularClosedVolume),
        formula: NormalizedRegionFormula::Or(vec![
            NormalizedRegionFormula::Atom("left".into()),
            NormalizedRegionFormula::Atom("right".into()),
        ]),
    })
    .unwrap();
    assert_eq!(set.region_kind(), RegionKind::GeneralSemiAlgebraicSet);

    let solid = solid_boolean(SolidBooleanRequest {
        left: sphere("left", 0, 2, RegionKind::RegularClosedVolume),
        right: sphere("right", 1, 2, RegionKind::RegularClosedVolume),
        formula: NormalizedRegionFormula::Or(vec![
            NormalizedRegionFormula::Atom("left".into()),
            NormalizedRegionFormula::Atom("right".into()),
        ]),
    })
    .unwrap();
    assert_eq!(solid.region_kind(), RegionKind::RegularClosedVolume);
}

#[test]
fn cad_failure_preserves_geometry_and_brep_failures() {
    let result = solid_boolean(SolidBooleanRequest {
        left: sphere("volume", 0, 2, RegionKind::RegularClosedVolume),
        right: sphere("set", 1, 2, RegionKind::GeneralSemiAlgebraicSet),
        formula: NormalizedRegionFormula::Or(vec![
            NormalizedRegionFormula::Atom("volume".into()),
            NormalizedRegionFormula::Atom("set".into()),
        ]),
    });
    assert_eq!(
        result,
        Err(CadFailure::Geometry(GeometryFailure::RegionKindMismatch))
    );
    assert_eq!(
        CadFailure::from(neco_cad::BrepFailure::EmptyIdentifier),
        CadFailure::Brep(neco_cad::BrepFailure::EmptyIdentifier)
    );
}
