use crate::construction::classify_region_membership;
use crate::construction_types::ExactScalar;
use crate::types::{BoundaryDerivationCondition, CadFailure};
use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use neco_algnum::AlgnumError;
use neco_algnum::RealAlgebraic;
use neco_bigint::{BigInt, Dyadic};
use neco_calculus::{
    CalculusFailure, DerivativeOrder, Enclosure, Polynomial, RootRefinementMethod,
};
use neco_geometry::{
    construct_intersection, AlgebraicRegion, BoundaryCandidateSeed, BoundaryDerivationRequest,
    BoundaryIntersectionSelector, BoundaryLocalEvidence, BoundaryRestrictionChart,
    BoundarySideSample, BoundaryStratification, CanonicalAtom, ExactShapeEvaluation,
    GeometryFailure, ImplicitSurfaceDeclaration, IntersectionConstructionRequest,
    IntersectionConstructionRequestBody, IntersectionKind, IntersectionOperand, IntersectionSet,
    MembershipClassification, MultivariatePolynomial3, NormalizedRegionFormula, Point3,
    PolynomialTerm3, RegionFunction, RegionKind, RegionPolynomial3, RootIsolationPolicy,
    ShapeDeclaration, ShapeEvaluation,
};

fn algebra(error: AlgnumError) -> CadFailure {
    CadFailure::Geometry(GeometryFailure::AlgebraFailure(error))
}

const LATTICE: i32 = 3;

fn failure(condition: BoundaryDerivationCondition, atom_id: Option<String>) -> CadFailure {
    CadFailure::Boundary { condition, atom_id }
}

fn algebraic(value: i32) -> Result<RealAlgebraic, CadFailure> {
    RealAlgebraic::from_integer(
        BigInt::try_from(value)
            .map_err(|_| failure(BoundaryDerivationCondition::EmptyRegion, None))?,
    )
    .map_err(algebra)
}

fn dyadic(value: i32) -> Result<Dyadic, CadFailure> {
    Ok(Dyadic::new(
        BigInt::try_from(value)
            .map_err(|_| failure(BoundaryDerivationCondition::EmptyRegion, None))?,
        0,
    ))
}

fn univariate(coefficients: &[i32]) -> Result<Polynomial, CadFailure> {
    let mut values = Vec::new();
    for coefficient in coefficients {
        values.push(algebraic(*coefficient)?);
    }
    Polynomial::from_coefficients(values).map_err(|error| {
        CadFailure::Geometry(GeometryFailure::CalculusFailure {
            error: CalculusFailure::Polynomial(error),
        })
    })
}

fn power(base: &RealAlgebraic, exponent: u32) -> Result<RealAlgebraic, CadFailure> {
    let mut value = algebraic(1)?;
    for _ in 0..exponent {
        value = value.mul(base).map_err(algebra)?;
    }
    Ok(value)
}

fn evaluate(
    polynomial: &RegionPolynomial3,
    x: i32,
    y: i32,
    z: i32,
) -> Result<RealAlgebraic, CadFailure> {
    let ax = algebraic(x)?;
    let ay = algebraic(y)?;
    let az = algebraic(z)?;
    let mut acc = algebraic(0)?;
    for term in polynomial.terms() {
        let mut piece = term.coefficient().try_clone().map_err(algebra)?;
        piece = piece
            .mul(&power(&ax, term.exponent_x())?)
            .map_err(algebra)?;
        piece = piece
            .mul(&power(&ay, term.exponent_y())?)
            .map_err(algebra)?;
        piece = piece
            .mul(&power(&az, term.exponent_z())?)
            .map_err(algebra)?;
        acc = acc.add(&piece).map_err(algebra)?;
    }
    Ok(acc)
}

fn gradient_vanishes(
    polynomial: &RegionPolynomial3,
    x: i32,
    y: i32,
    z: i32,
) -> Result<bool, CadFailure> {
    let ax = algebraic(x)?;
    let ay = algebraic(y)?;
    let az = algebraic(z)?;
    let mut gx = algebraic(0)?;
    let mut gy = algebraic(0)?;
    let mut gz = algebraic(0)?;
    for term in polynomial.terms() {
        if term.exponent_x() > 0 {
            let mut piece = term.coefficient().try_clone().map_err(algebra)?;
            piece = piece
                .mul(&algebraic(term.exponent_x() as i32)?)
                .map_err(algebra)?;
            piece = piece
                .mul(&power(&ax, term.exponent_x() - 1)?)
                .map_err(algebra)?;
            piece = piece
                .mul(&power(&ay, term.exponent_y())?)
                .map_err(algebra)?;
            piece = piece
                .mul(&power(&az, term.exponent_z())?)
                .map_err(algebra)?;
            gx = gx.add(&piece).map_err(algebra)?;
        }
        if term.exponent_y() > 0 {
            let mut piece = term.coefficient().try_clone().map_err(algebra)?;
            piece = piece
                .mul(&algebraic(term.exponent_y() as i32)?)
                .map_err(algebra)?;
            piece = piece
                .mul(&power(&ax, term.exponent_x())?)
                .map_err(algebra)?;
            piece = piece
                .mul(&power(&ay, term.exponent_y() - 1)?)
                .map_err(algebra)?;
            piece = piece
                .mul(&power(&az, term.exponent_z())?)
                .map_err(algebra)?;
            gy = gy.add(&piece).map_err(algebra)?;
        }
        if term.exponent_z() > 0 {
            let mut piece = term.coefficient().try_clone().map_err(algebra)?;
            piece = piece
                .mul(&algebraic(term.exponent_z() as i32)?)
                .map_err(algebra)?;
            piece = piece
                .mul(&power(&ax, term.exponent_x())?)
                .map_err(algebra)?;
            piece = piece
                .mul(&power(&ay, term.exponent_y())?)
                .map_err(algebra)?;
            piece = piece
                .mul(&power(&az, term.exponent_z() - 1)?)
                .map_err(algebra)?;
            gz = gz.add(&piece).map_err(algebra)?;
        }
    }
    Ok(gx.is_zero() && gy.is_zero() && gz.is_zero())
}

fn point_on_offset_axis(axis: usize, parameter: i32, offset_a: i32, offset_b: i32) -> [i32; 3] {
    match axis {
        0 => [parameter, offset_a, offset_b],
        1 => [offset_a, parameter, offset_b],
        _ => [offset_a, offset_b, parameter],
    }
}

fn shift(point: [i32; 3], axis: usize, delta: i32) -> [i32; 3] {
    let mut coordinates = point;
    coordinates[axis] += delta;
    coordinates
}

fn classify_lattice(
    region: &AlgebraicRegion,
    coordinates: [i32; 3],
) -> Result<MembershipClassification, CadFailure> {
    classify_region_membership(
        region,
        [
            ExactScalar::new(f64::from(coordinates[0]))
                .map_err(|_| failure(BoundaryDerivationCondition::IncompleteCoverage, None))?,
            ExactScalar::new(f64::from(coordinates[1]))
                .map_err(|_| failure(BoundaryDerivationCondition::IncompleteCoverage, None))?,
            ExactScalar::new(f64::from(coordinates[2]))
                .map_err(|_| failure(BoundaryDerivationCondition::IncompleteCoverage, None))?,
        ],
    )
    .map_err(|_| failure(BoundaryDerivationCondition::IncompleteCoverage, None))
}

fn chart_vanishes_identically(
    polynomial: &RegionPolynomial3,
    point: [i32; 3],
    axis: usize,
) -> Result<bool, CadFailure> {
    for delta in -1..=1 {
        let coordinates = shift(point, axis, delta);
        if !evaluate(polynomial, coordinates[0], coordinates[1], coordinates[2])?.is_zero() {
            return Ok(false);
        }
    }
    Ok(true)
}

fn opposite_region_sides(
    region: &AlgebraicRegion,
    point: [i32; 3],
    axis: usize,
) -> Result<bool, CadFailure> {
    let left = classify_lattice(region, shift(point, axis, -1))?;
    let right = classify_lattice(region, shift(point, axis, 1))?;
    Ok(matches!(
        (left, right),
        (
            MembershipClassification::Inside,
            MembershipClassification::Outside
        ) | (
            MembershipClassification::Outside,
            MembershipClassification::Inside
        )
    ))
}

fn centered_chart(point: [i32; 3], axis: usize) -> Result<BoundaryRestrictionChart, CadFailure> {
    let mut coordinates = [
        univariate(&[point[0]])?,
        univariate(&[point[1]])?,
        univariate(&[point[2]])?,
    ];
    coordinates[axis] = univariate(&[point[axis], 1])?;
    let enclosure = Enclosure::new(dyadic(-1)?, dyadic(1)?).map_err(|error| {
        CadFailure::Geometry(GeometryFailure::CalculusFailure {
            error: CalculusFailure::Interval(error),
        })
    })?;
    let [x, y, z] = coordinates;
    BoundaryRestrictionChart::try_new(
        format!("axis-{axis}-{}-{}-{}", point[0], point[1], point[2]),
        enclosure,
        x,
        y,
        z,
    )
    .map_err(CadFailure::from)
}

fn candidate_seed(
    atom_id: &str,
    point: [i32; 3],
    axis: usize,
    intersection: Option<BoundaryIntersectionSelector>,
) -> Result<BoundaryCandidateSeed, CadFailure> {
    BoundaryCandidateSeed::try_new(
        String::from(atom_id),
        centered_chart(point, axis)?,
        vec![dyadic(0)?],
        vec![
            BoundarySideSample::new(dyadic(-1)?),
            BoundarySideSample::new(dyadic(1)?),
        ],
        intersection,
    )
    .map_err(CadFailure::from)
}

fn implicit_operand(polynomial: &RegionPolynomial3) -> Result<IntersectionOperand, CadFailure> {
    let mut terms = Vec::new();
    for term in polynomial.terms() {
        terms.push(
            PolynomialTerm3::new(
                term.coefficient().try_clone().map_err(algebra)?,
                term.exponent_x(),
                term.exponent_y(),
                term.exponent_z(),
            )
            .map_err(CadFailure::from)?,
        );
    }
    Ok(IntersectionOperand::Declaration(
        ShapeDeclaration::ImplicitSurface(ImplicitSurfaceDeclaration::new(
            MultivariatePolynomial3::try_from_terms(terms).map_err(CadFailure::from)?,
        )),
    ))
}

fn evaluated_point(point: [i32; 3]) -> Result<IntersectionOperand, CadFailure> {
    Ok(IntersectionOperand::Evaluated(ShapeEvaluation::Exact(
        ExactShapeEvaluation::ParametricSurface {
            coordinate: Point3 {
                x: algebraic(point[0])?,
                y: algebraic(point[1])?,
                z: algebraic(point[2])?,
            },
        },
    )))
}

fn isolation_policy() -> Result<RootIsolationPolicy, CadFailure> {
    let enclosure = Enclosure::new(dyadic(-1)?, dyadic(1)?).map_err(|error| {
        CadFailure::Geometry(GeometryFailure::CalculusFailure {
            error: CalculusFailure::Interval(error),
        })
    })?;
    RootIsolationPolicy::new(8, RootRefinementMethod::Bisection, enclosure)
        .map_err(CadFailure::from)
}

fn intersection_proves(set: &IntersectionSet) -> bool {
    matches!(
        set.kind(),
        IntersectionKind::Point
            | IntersectionKind::FinitePoints
            | IntersectionKind::Curve
            | IntersectionKind::CurveBranches
    )
}

fn branch_intersection(
    left: &RegionPolynomial3,
    right: &RegionPolynomial3,
    point: [i32; 3],
) -> Result<Option<IntersectionSet>, CadFailure> {
    let _ = left;
    let contact = construct_intersection(IntersectionConstructionRequest::new(
        IntersectionConstructionRequestBody::new(
            evaluated_point(point)?,
            implicit_operand(right)?,
            isolation_policy()?,
        ),
    ));
    match contact {
        Ok(set) if intersection_proves(&set) => Ok(Some(set)),
        _ => Ok(None),
    }
}

struct Pinch {
    point: [i32; 3],
    axis: usize,
    atom_id: String,
}

struct ZeroScan<'a> {
    region: &'a AlgebraicRegion,
    atom: &'a CanonicalAtom,
    polynomial: &'a RegionPolynomial3,
    face: Option<BoundaryCandidateSeed>,
    pinches: Vec<Pinch>,
    singular: bool,
}

impl ZeroScan<'_> {
    fn visit(
        &mut self,
        point: [i32; 3],
        axis: usize,
        record_pinch: bool,
    ) -> Result<(), CadFailure> {
        if !evaluate(self.polynomial, point[0], point[1], point[2])?.is_zero() {
            return Ok(());
        }
        if gradient_vanishes(self.polynomial, point[0], point[1], point[2])? {
            self.singular = true;
        }
        if opposite_region_sides(self.region, point, axis)? {
            if self.face.is_none() {
                self.face = Some(candidate_seed(self.atom.atom_id(), point, axis, None)?);
            }
        } else if record_pinch {
            self.pinches.push(Pinch {
                point,
                axis,
                atom_id: String::from(self.atom.atom_id()),
            });
        }
        Ok(())
    }
}

fn scan_polynomial(
    region: &AlgebraicRegion,
    atom: &CanonicalAtom,
    polynomial: &RegionPolynomial3,
) -> Result<(Option<BoundaryCandidateSeed>, Vec<Pinch>, bool), CadFailure> {
    let mut scan = ZeroScan {
        region,
        atom,
        polynomial,
        face: None,
        pinches: Vec::new(),
        singular: false,
    };
    for axis in 0..3 {
        for parameter in -LATTICE..=LATTICE {
            scan.visit(point_on_offset_axis(axis, parameter, 0, 0), axis, true)?;
        }
    }
    if scan.face.is_some() {
        return Ok((scan.face, scan.pinches, scan.singular));
    }
    for axis in 0..3 {
        for offset_a in -LATTICE..=LATTICE {
            for offset_b in -LATTICE..=LATTICE {
                if offset_a == 0 && offset_b == 0 {
                    continue;
                }
                for parameter in -LATTICE..=LATTICE {
                    scan.visit(
                        point_on_offset_axis(axis, parameter, offset_a, offset_b),
                        axis,
                        false,
                    )?;
                    if scan.face.is_some() {
                        return Ok((scan.face, scan.pinches, scan.singular));
                    }
                }
            }
        }
    }
    Ok((scan.face, scan.pinches, scan.singular))
}

fn formula_is_empty(formula: &NormalizedRegionFormula) -> bool {
    matches!(formula, NormalizedRegionFormula::False)
}

fn polynomial_of(atom: &CanonicalAtom) -> Option<&RegionPolynomial3> {
    match atom.function() {
        RegionFunction::Polynomial(polynomial) => Some(polynomial),
        RegionFunction::RationalFunction(_) => None,
    }
}

pub fn derive_complete_boundary(
    region: AlgebraicRegion,
) -> Result<BoundaryStratification, CadFailure> {
    if region.canonical_atoms().is_empty() || formula_is_empty(region.normalized_region_formula()) {
        return Err(failure(BoundaryDerivationCondition::EmptyRegion, None));
    }
    let mut seeds = Vec::new();
    let mut pinches = Vec::new();
    let mut singular_uncovered = Vec::new();
    for atom in region.canonical_atoms() {
        match atom.function() {
            RegionFunction::Polynomial(polynomial) => {
                let (face, atom_pinches, singular) = scan_polynomial(&region, atom, polynomial)?;
                if face.is_none() && singular {
                    singular_uncovered.push(String::from(atom.atom_id()));
                }
                if let Some(seed) = face {
                    seeds.push(seed);
                }
                pinches.extend(atom_pinches);
            }
            RegionFunction::RationalFunction(_) => {
                return Err(failure(
                    BoundaryDerivationCondition::SeamUnsupported,
                    Some(String::from(atom.atom_id())),
                ));
            }
        }
    }
    if let Some(atom_id) = singular_uncovered.into_iter().next() {
        return Err(failure(
            BoundaryDerivationCondition::SingularityUnsupported,
            Some(atom_id),
        ));
    }
    let mut intersections = Vec::new();
    let mut used_points = Vec::new();
    for pinch in &pinches {
        if used_points.contains(&pinch.point) {
            continue;
        }
        let mates: Vec<&Pinch> = pinches
            .iter()
            .filter(|candidate| {
                candidate.point == pinch.point && candidate.atom_id != pinch.atom_id
            })
            .collect();
        let Some(mate) = mates.first() else {
            continue;
        };
        let Some(left) = region
            .canonical_atoms()
            .iter()
            .find(|atom| atom.atom_id() == pinch.atom_id)
            .and_then(polynomial_of)
        else {
            continue;
        };
        let Some(right) = region
            .canonical_atoms()
            .iter()
            .find(|atom| atom.atom_id() == mate.atom_id)
            .and_then(polynomial_of)
        else {
            continue;
        };
        let mut axis = None;
        for candidate in [pinch.axis, 0, 1, 2] {
            if !chart_vanishes_identically(left, pinch.point, candidate)? {
                axis = Some(candidate);
                break;
            }
        }
        let Some(axis) = axis else {
            used_points.push(pinch.point);
            continue;
        };
        let selector = match branch_intersection(left, right, pinch.point)? {
            Some(set) => {
                let selector = BoundaryIntersectionSelector::try_new(intersections.len(), None)
                    .map_err(CadFailure::from)?;
                intersections.push(set);
                Some(selector)
            }
            None => None,
        };
        seeds.push(candidate_seed(
            pinch.atom_id.as_str(),
            pinch.point,
            axis,
            selector,
        )?);
        used_points.push(pinch.point);
    }
    if seeds.is_empty() {
        return Err(failure(
            BoundaryDerivationCondition::IncompleteCoverage,
            None,
        ));
    }
    let expect_regular = region.region_kind() == RegionKind::RegularClosedVolume;
    let atom_ids: Vec<String> = region
        .canonical_atoms()
        .iter()
        .map(|atom| String::from(atom.atom_id()))
        .collect();
    let derived = neco_geometry::derive_boundary(BoundaryDerivationRequest {
        region,
        intersections,
        candidate_loci: seeds,
        max_order: DerivativeOrder::new(3),
    })?;
    let stratification = derived.into_parts().1;
    if expect_regular {
        for atom_id in &atom_ids {
            let covered = stratification.strata().iter().any(|stratum| {
                matches!(
                    stratum.local_evidence(),
                    BoundaryLocalEvidence::Proved { .. }
                ) && stratum
                    .candidates()
                    .iter()
                    .any(|candidate| candidate.locus().atom_id() == atom_id)
            });
            if !covered {
                return Err(failure(
                    BoundaryDerivationCondition::IncompleteCoverage,
                    Some(atom_id.clone()),
                ));
            }
        }
        if stratification.strata().is_empty() {
            return Err(failure(
                BoundaryDerivationCondition::IncompleteCoverage,
                None,
            ));
        }
    }
    Ok(stratification)
}
