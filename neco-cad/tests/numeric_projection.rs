use core::cmp::Ordering;

use neco_algnum::RealAlgebraic;
use neco_bigint::{BigInt, BigUint, Dyadic, RawRational};
use neco_cad::{
    project_numeric_coordinates, CadFailure, CanonicalAtom, ChartDisplayName, ChartDomainId,
    NumericProjectionCondition, NumericProjectionRequest, NumericSample, ParameterInterval,
    ParametricSurfaceChart2, Point3, ProjectedCoordinate, RationalSurfacePolynomials,
    RegionFunction, RegionPolynomial3, RegionPolynomialTerm, SignRelation, SourceRevision,
    SurfaceOrientation, SurfaceParameterDomain, SurfacePolynomial, SurfaceTerm,
    DEFAULT_NUMERIC_PROJECTION_BITS,
};

fn algebraic(value: i32) -> RealAlgebraic {
    RealAlgebraic::from_integer(BigInt::try_from(value).unwrap()).unwrap()
}

fn rational(value: i32) -> neco_bigint::ReducedRational {
    RawRational::new(BigInt::try_from(value).unwrap(), BigUint::one().unwrap())
        .reduce()
        .unwrap()
        .into_reduced()
}

fn unit_sphere() -> CanonicalAtom {
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

fn revision() -> SourceRevision {
    SourceRevision::new(String::from("sphere-r1")).unwrap()
}

fn unit_point() -> Point3 {
    Point3 {
        x: algebraic(1),
        y: algebraic(0),
        z: algebraic(0),
    }
}

fn surface_term(coefficient: i32, exponent_u: u32, exponent_v: u32) -> SurfaceTerm {
    SurfaceTerm {
        coefficient: rational(coefficient),
        exponent_u,
        exponent_v,
    }
}

fn surface_polynomial(coefficient: i32, exponent_u: u32, exponent_v: u32) -> SurfacePolynomial {
    SurfacePolynomial::try_from_terms(vec![surface_term(coefficient, exponent_u, exponent_v)])
        .unwrap()
}

fn identity_plane() -> ParametricSurfaceChart2 {
    ParametricSurfaceChart2::try_new(
        ChartDomainId::new(String::from("plane")).unwrap(),
        ChartDisplayName::new(String::from("plane")).unwrap(),
        1,
        SurfaceOrientation::Preserve,
        RationalSurfacePolynomials {
            x: surface_polynomial(1, 1, 0),
            y: surface_polynomial(1, 0, 1),
            z: surface_polynomial(0, 0, 0),
            denominator: surface_polynomial(1, 0, 0),
        },
    )
    .unwrap()
}

fn parameter_domain() -> SurfaceParameterDomain {
    SurfaceParameterDomain::new(
        ParameterInterval::new(algebraic(0), algebraic(8), true, true).unwrap(),
        ParameterInterval::new(algebraic(0), algebraic(8), true, true).unwrap(),
    )
}

fn enclosure_contains(value: &RealAlgebraic, projection: &ProjectedCoordinate) {
    let enclosure = projection.enclosure();
    assert_ne!(
        value.compare_dyadic(enclosure.lower()).unwrap(),
        Ordering::Less
    );
    assert_ne!(
        value.compare_dyadic(enclosure.upper()).unwrap(),
        Ordering::Greater
    );
}

#[test]
fn unit_sphere_point_preserves_identity_revision_and_bounds() {
    let request = NumericProjectionRequest::new(
        "vertex:sphere",
        revision(),
        unit_sphere(),
        DEFAULT_NUMERIC_PROJECTION_BITS,
        vec![NumericSample::AlgebraicPoint(unit_point())],
    )
    .unwrap();
    let projection = project_numeric_coordinates(request).unwrap();
    assert_eq!(projection.identity().as_str(), "vertex:sphere");
    assert_eq!(projection.source_revision().as_str(), "sphere-r1");
    assert_eq!(projection.provenance().exact_surface_atom_id(), "sphere");
    assert_eq!(projection.provenance().enclosure_bits(), 48);
    let point = &projection.coordinates()[0];
    assert_eq!(point.rounded(), [1.0, 0.0, 0.0]);
    let unit = Dyadic::new(BigInt::try_from(1).unwrap(), 0);
    assert!(point.surface_distance_bound() < &unit);
}

#[test]
fn analytic_surface_sample_preserves_projection_witness() {
    let request = NumericProjectionRequest::new(
        "face:plane",
        revision(),
        unit_sphere(),
        DEFAULT_NUMERIC_PROJECTION_BITS,
        vec![NumericSample::AnalyticSurface {
            chart: Box::new(identity_plane()),
            domain: parameter_domain(),
            u: algebraic(1),
            v: algebraic(1),
        }],
    )
    .unwrap();
    let projection = project_numeric_coordinates(request).unwrap();
    assert_eq!(projection.identity().as_str(), "face:plane");
    assert_eq!(projection.source_revision().as_str(), "sphere-r1");
    assert_eq!(projection.provenance().exact_surface_atom_id(), "sphere");
    let point = &projection.coordinates()[0];
    assert_eq!(point.rounded(), [1.0, 1.0, 0.0]);
    let _ = point.surface_distance_bound();
}

#[test]
fn projected_enclosure_contains_exact_value() {
    let request = NumericProjectionRequest::new(
        "vertex:sphere",
        revision(),
        unit_sphere(),
        DEFAULT_NUMERIC_PROJECTION_BITS,
        vec![NumericSample::AlgebraicPoint(unit_point())],
    )
    .unwrap();
    let projection = project_numeric_coordinates(request).unwrap();
    let point = &projection.coordinates()[0];
    enclosure_contains(&algebraic(1), point.x());
    enclosure_contains(&algebraic(0), point.y());
    enclosure_contains(&algebraic(0), point.z());
}

#[test]
fn empty_identity_and_surface_evaluation_typed_failure() {
    assert_eq!(
        NumericProjectionRequest::new(
            "",
            revision(),
            unit_sphere(),
            DEFAULT_NUMERIC_PROJECTION_BITS,
            vec![NumericSample::AlgebraicPoint(unit_point())],
        )
        .unwrap_err(),
        CadFailure::NumericProjection {
            condition: NumericProjectionCondition::EmptyIdentity,
            identity: None,
        }
    );
    let request = NumericProjectionRequest::new(
        "face:plane",
        revision(),
        unit_sphere(),
        DEFAULT_NUMERIC_PROJECTION_BITS,
        vec![NumericSample::AnalyticSurface {
            chart: Box::new(identity_plane()),
            domain: parameter_domain(),
            u: algebraic(9),
            v: algebraic(0),
        }],
    )
    .unwrap();
    assert_eq!(
        project_numeric_coordinates(request).unwrap_err(),
        CadFailure::NumericProjection {
            condition: NumericProjectionCondition::SurfaceEvaluation,
            identity: Some(String::from("face:plane")),
        }
    );
}
