use crate::adaptive_tessellation::{
    affine_plane_nurbs_surface, affine_plane_surface_covering, sphere_nurbs_surface,
    sphere_surface_covering, AdaptiveTessellationRequest, FaceExactSurface, SurfaceCovering,
    SurfaceWeldVertex,
};
use crate::provenance::SourceIdentity;
use crate::types::{BrepDerivationCondition, CadFailure};
use alloc::collections::BTreeSet;
use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::cmp::Ordering;
use neco_algnum::RealAlgebraic;
use neco_bigint::{BigInt, Sign};
use neco_brep::{
    construct_full_domain_arrangement, materialize_analytic_brep, AnalyticBrep, ArrangementId,
    BrepAssignments, BrepMaterializationRequest, ChartArrangement, FaceAssignment, SourceRevision,
    SurfaceRepresentation, SurfaceRepresentationId, VertexAssignment,
};
use neco_geometry::{
    BoundaryLocalEvidence, BoundaryStratification, BoundaryStratum, BoundaryStratumDimension,
    CanonicalAtom, NormalizedRegionFormula, ParameterInterval, Point3, RegionFunction, RegionKind,
    RegionPolynomial3, SignRelation, SurfaceOrientation, SurfaceParameterDomain,
    SurfaceParameterPoint, SurfaceTrimLoop2, TrimLoopRole,
};
use neco_tessellation::AdaptivePrecision;

pub struct DerivedAnalyticBrep {
    brep: AnalyticBrep,
    face_surfaces: Vec<FaceExactSurface>,
}

impl DerivedAnalyticBrep {
    pub fn brep(&self) -> &AnalyticBrep {
        &self.brep
    }

    pub fn face_surfaces(&self) -> &[FaceExactSurface] {
        &self.face_surfaces
    }

    pub fn into_parts(self) -> (AnalyticBrep, Vec<FaceExactSurface>) {
        (self.brep, self.face_surfaces)
    }

    pub fn into_adaptive_tessellation_request(
        self,
        identity: SourceIdentity,
        precision: AdaptivePrecision,
        enclosure_bits: u32,
    ) -> Result<AdaptiveTessellationRequest, CadFailure> {
        AdaptiveTessellationRequest::new(
            String::from(identity.as_str()),
            self.brep,
            self.face_surfaces,
            precision,
            enclosure_bits,
        )
    }
}

fn failure(condition: BrepDerivationCondition, stratum_id: Option<String>) -> CadFailure {
    CadFailure::BrepDerivation {
        condition,
        stratum_id,
    }
}

fn unsupported_surface_covering(stratum_id: &str) -> CadFailure {
    failure(
        BrepDerivationCondition::UnsupportedSurfaceCovering,
        Some(String::from(stratum_id)),
    )
}

fn surface_representation_failure(stratum_id: &str) -> CadFailure {
    failure(
        BrepDerivationCondition::SurfaceRepresentation,
        Some(String::from(stratum_id)),
    )
}

fn algebraic(value: i32, stratum_id: &str) -> Result<RealAlgebraic, CadFailure> {
    RealAlgebraic::from_integer(
        BigInt::try_from(value).map_err(|_| unsupported_surface_covering(stratum_id))?,
    )
    .map_err(|_| unsupported_surface_covering(stratum_id))
}

fn clone_algebraic(value: &RealAlgebraic, stratum_id: &str) -> Result<RealAlgebraic, CadFailure> {
    value
        .try_clone()
        .map_err(|_| unsupported_surface_covering(stratum_id))
}

fn rational_algebraic(value: &RealAlgebraic) -> bool {
    let coefficients = value.minimal_polynomial().polynomial().coefficients();
    coefficients.len() == 2 && coefficients[1].sign() != Sign::Zero
}

fn proved_dimension(stratum: &BoundaryStratum, expected: i64) -> bool {
    matches!(
        stratum.local_evidence(),
        BoundaryLocalEvidence::Proved { .. }
    ) && matches!(
        stratum.dimension(),
        BoundaryStratumDimension::Dimension(dimension) if *dimension == expected
    )
}

fn support_atom_ids(stratum: &BoundaryStratum) -> BTreeSet<String> {
    stratum
        .candidates()
        .iter()
        .map(|candidate| String::from(candidate.locus().atom_id()))
        .collect()
}

fn unique_atom_id(atom_ids: BTreeSet<String>, stratum_id: &str) -> Result<String, CadFailure> {
    match atom_ids.len() {
        0 => Err(failure(
            BrepDerivationCondition::IncompleteCoverage,
            Some(String::from(stratum_id)),
        )),
        1 => atom_ids.into_iter().next().ok_or_else(|| {
            failure(
                BrepDerivationCondition::IncompleteCoverage,
                Some(String::from(stratum_id)),
            )
        }),
        _ => Err(failure(
            BrepDerivationCondition::AmbiguousSupportAtom,
            Some(String::from(stratum_id)),
        )),
    }
}

fn unique_support_atom_id(stratum: &BoundaryStratum) -> Result<String, CadFailure> {
    unique_atom_id(support_atom_ids(stratum), stratum.stratum_id())
}

fn atom_has_proved_face(strata: &[BoundaryStratum], atom_id: &str) -> bool {
    strata
        .iter()
        .any(|stratum| proved_dimension(stratum, 2) && support_atom_ids(stratum).contains(atom_id))
}

struct AffinePlane {
    atom_id: String,
    normal: [RealAlgebraic; 3],
    constant: RealAlgebraic,
    pivot_axis: usize,
    relation: SignRelation,
    orientation: SurfaceOrientation,
}

struct SphereSurface {
    atom_id: String,
    center: Point3,
    radius: RealAlgebraic,
}

enum SupportedSurface {
    AffinePlane(AffinePlane),
    Sphere(SphereSurface),
}

fn affine_coefficients(
    polynomial: &RegionPolynomial3,
    stratum_id: &str,
) -> Result<Option<([RealAlgebraic; 3], RealAlgebraic)>, CadFailure> {
    let zero = algebraic(0, stratum_id)?;
    let mut normal = [
        clone_algebraic(&zero, stratum_id)?,
        clone_algebraic(&zero, stratum_id)?,
        clone_algebraic(&zero, stratum_id)?,
    ];
    let mut constant = clone_algebraic(&zero, stratum_id)?;
    for term in polynomial.terms() {
        match (term.exponent_x(), term.exponent_y(), term.exponent_z()) {
            (0, 0, 0) => constant = clone_algebraic(term.coefficient(), stratum_id)?,
            (1, 0, 0) => normal[0] = clone_algebraic(term.coefficient(), stratum_id)?,
            (0, 1, 0) => normal[1] = clone_algebraic(term.coefficient(), stratum_id)?,
            (0, 0, 1) => normal[2] = clone_algebraic(term.coefficient(), stratum_id)?,
            _ => return Ok(None),
        }
    }
    if normal.iter().all(RealAlgebraic::is_zero)
        || normal.iter().any(|value| !rational_algebraic(value))
        || !rational_algebraic(&constant)
    {
        return Ok(None);
    }
    Ok(Some((normal, constant)))
}

fn plane_orientation(
    relation: SignRelation,
    pivot_axis: usize,
    pivot_sign: Sign,
) -> Option<SurfaceOrientation> {
    let chart_normal_sign = if pivot_axis == 1 {
        match pivot_sign {
            Sign::Positive => Sign::Negative,
            Sign::Negative => Sign::Positive,
            Sign::Zero => Sign::Zero,
        }
    } else {
        pivot_sign
    };
    match (relation, chart_normal_sign) {
        (SignRelation::Negative, Sign::Positive) | (SignRelation::Positive, Sign::Negative) => {
            Some(SurfaceOrientation::Preserve)
        }
        (SignRelation::Negative, Sign::Negative) | (SignRelation::Positive, Sign::Positive) => {
            Some(SurfaceOrientation::Reverse)
        }
        _ => None,
    }
}

fn plane_surface(
    atom_id: &str,
    polynomial: &RegionPolynomial3,
    relation: SignRelation,
    stratum_id: &str,
) -> Result<Option<SupportedSurface>, CadFailure> {
    let Some((normal, constant)) = affine_coefficients(polynomial, stratum_id)? else {
        return Ok(None);
    };
    let pivot_axis = normal
        .iter()
        .position(|coefficient| !coefficient.is_zero())
        .ok_or_else(|| unsupported_surface_covering(stratum_id))?;
    let pivot_sign = normal[pivot_axis]
        .sign()
        .map_err(|_| unsupported_surface_covering(stratum_id))?;
    let Some(orientation) = plane_orientation(relation, pivot_axis, pivot_sign) else {
        return Ok(None);
    };
    Ok(Some(SupportedSurface::AffinePlane(AffinePlane {
        atom_id: String::from(atom_id),
        normal,
        constant,
        pivot_axis,
        relation,
        orientation,
    })))
}

fn sphere_surface(
    atom_id: &str,
    polynomial: &RegionPolynomial3,
    relation: SignRelation,
    stratum_id: &str,
) -> Result<Option<SupportedSurface>, CadFailure> {
    let zero = algebraic(0, stratum_id)?;
    let mut quadratic = [None, None, None];
    let mut linear = [
        clone_algebraic(&zero, stratum_id)?,
        clone_algebraic(&zero, stratum_id)?,
        clone_algebraic(&zero, stratum_id)?,
    ];
    let mut constant = clone_algebraic(&zero, stratum_id)?;
    for term in polynomial.terms() {
        match (term.exponent_x(), term.exponent_y(), term.exponent_z()) {
            (2, 0, 0) => quadratic[0] = Some(clone_algebraic(term.coefficient(), stratum_id)?),
            (0, 2, 0) => quadratic[1] = Some(clone_algebraic(term.coefficient(), stratum_id)?),
            (0, 0, 2) => quadratic[2] = Some(clone_algebraic(term.coefficient(), stratum_id)?),
            (1, 0, 0) => linear[0] = clone_algebraic(term.coefficient(), stratum_id)?,
            (0, 1, 0) => linear[1] = clone_algebraic(term.coefficient(), stratum_id)?,
            (0, 0, 1) => linear[2] = clone_algebraic(term.coefficient(), stratum_id)?,
            (0, 0, 0) => constant = clone_algebraic(term.coefficient(), stratum_id)?,
            _ => return Ok(None),
        }
    }
    let (Some(a_x), Some(a_y), Some(a_z)) = (&quadratic[0], &quadratic[1], &quadratic[2]) else {
        return Ok(None);
    };
    if a_x.is_zero()
        || a_x
            .compare(a_y)
            .map_err(|_| unsupported_surface_covering(stratum_id))?
            != Ordering::Equal
        || a_x
            .compare(a_z)
            .map_err(|_| unsupported_surface_covering(stratum_id))?
            != Ordering::Equal
    {
        return Ok(None);
    }
    let coefficient_sign = a_x
        .sign()
        .map_err(|_| unsupported_surface_covering(stratum_id))?;
    if !matches!(
        (relation, coefficient_sign),
        (SignRelation::Negative, Sign::Positive) | (SignRelation::Positive, Sign::Negative)
    ) {
        return Ok(None);
    }
    let two = algebraic(2, stratum_id)?;
    let denominator = a_x
        .mul(&two)
        .map_err(|_| unsupported_surface_covering(stratum_id))?;
    let center_values = linear
        .iter()
        .map(|coefficient| {
            zero.sub(coefficient)
                .and_then(|numerator| numerator.div(&denominator))
                .map_err(|_| unsupported_surface_covering(stratum_id))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let center = Point3 {
        x: clone_algebraic(&center_values[0], stratum_id)?,
        y: clone_algebraic(&center_values[1], stratum_id)?,
        z: clone_algebraic(&center_values[2], stratum_id)?,
    };
    let radius_squared = center_values
        .iter()
        .try_fold(clone_algebraic(&zero, stratum_id)?, |sum, component| {
            component
                .mul(component)
                .and_then(|square| sum.add(&square))
                .map_err(|_| unsupported_surface_covering(stratum_id))
        })?
        .sub(
            &constant
                .div(a_x)
                .map_err(|_| unsupported_surface_covering(stratum_id))?,
        )
        .map_err(|_| unsupported_surface_covering(stratum_id))?;
    if radius_squared
        .compare(&zero)
        .map_err(|_| unsupported_surface_covering(stratum_id))?
        != Ordering::Greater
    {
        return Ok(None);
    }
    let radius = radius_squared
        .nth_root(2)
        .map_err(|_| unsupported_surface_covering(stratum_id))?;
    if !rational_algebraic(&center.x)
        || !rational_algebraic(&center.y)
        || !rational_algebraic(&center.z)
        || !rational_algebraic(&radius)
    {
        return Ok(None);
    }
    Ok(Some(SupportedSurface::Sphere(SphereSurface {
        atom_id: String::from(atom_id),
        center,
        radius,
    })))
}

fn classify_surface(
    atom: &CanonicalAtom,
    stratum_id: &str,
) -> Result<SupportedSurface, CadFailure> {
    let RegionFunction::Polynomial(polynomial) = atom.function() else {
        return Err(unsupported_surface_covering(stratum_id));
    };
    if let Some(surface) = plane_surface(atom.atom_id(), polynomial, atom.relation(), stratum_id)? {
        return Ok(surface);
    }
    if let Some(surface) = sphere_surface(atom.atom_id(), polynomial, atom.relation(), stratum_id)?
    {
        return Ok(surface);
    }
    Err(unsupported_surface_covering(stratum_id))
}

fn determinant(
    first: &[RealAlgebraic; 3],
    second: &[RealAlgebraic; 3],
    third: &[RealAlgebraic; 3],
    stratum_id: &str,
) -> Result<RealAlgebraic, CadFailure> {
    let first_minor = second[1]
        .mul(&third[2])
        .and_then(|value| value.sub(&second[2].mul(&third[1])?))
        .map_err(|_| unsupported_surface_covering(stratum_id))?;
    let second_minor = second[0]
        .mul(&third[2])
        .and_then(|value| value.sub(&second[2].mul(&third[0])?))
        .map_err(|_| unsupported_surface_covering(stratum_id))?;
    let third_minor = second[0]
        .mul(&third[1])
        .and_then(|value| value.sub(&second[1].mul(&third[0])?))
        .map_err(|_| unsupported_surface_covering(stratum_id))?;
    first[0]
        .mul(&first_minor)
        .and_then(|value| value.sub(&first[1].mul(&second_minor)?))
        .and_then(|value| value.add(&first[2].mul(&third_minor)?))
        .map_err(|_| unsupported_surface_covering(stratum_id))
}

fn intersect_planes(
    first: &AffinePlane,
    second: &AffinePlane,
    third: &AffinePlane,
    stratum_id: &str,
) -> Result<Option<Point3>, CadFailure> {
    let denominator = determinant(&first.normal, &second.normal, &third.normal, stratum_id)?;
    if denominator.is_zero() {
        return Ok(None);
    }
    let zero = algebraic(0, stratum_id)?;
    let constants = [
        zero.sub(&first.constant)
            .map_err(|_| unsupported_surface_covering(stratum_id))?,
        zero.sub(&second.constant)
            .map_err(|_| unsupported_surface_covering(stratum_id))?,
        zero.sub(&third.constant)
            .map_err(|_| unsupported_surface_covering(stratum_id))?,
    ];
    let x_rows = [
        clone_algebraic(&constants[0], stratum_id)?,
        clone_algebraic(&constants[1], stratum_id)?,
        clone_algebraic(&constants[2], stratum_id)?,
    ];
    let y_rows = x_rows
        .iter()
        .map(|value| clone_algebraic(value, stratum_id))
        .collect::<Result<Vec<_>, _>>()?;
    let z_rows = x_rows
        .iter()
        .map(|value| clone_algebraic(value, stratum_id))
        .collect::<Result<Vec<_>, _>>()?;
    let x = determinant(
        &x_rows,
        &[
            clone_algebraic(&first.normal[1], stratum_id)?,
            clone_algebraic(&second.normal[1], stratum_id)?,
            clone_algebraic(&third.normal[1], stratum_id)?,
        ],
        &[
            clone_algebraic(&first.normal[2], stratum_id)?,
            clone_algebraic(&second.normal[2], stratum_id)?,
            clone_algebraic(&third.normal[2], stratum_id)?,
        ],
        stratum_id,
    )?
    .div(&denominator)
    .map_err(|_| unsupported_surface_covering(stratum_id))?;
    let y = determinant(
        &[
            clone_algebraic(&first.normal[0], stratum_id)?,
            clone_algebraic(&second.normal[0], stratum_id)?,
            clone_algebraic(&third.normal[0], stratum_id)?,
        ],
        &[
            y_rows[0]
                .try_clone()
                .map_err(|_| unsupported_surface_covering(stratum_id))?,
            y_rows[1]
                .try_clone()
                .map_err(|_| unsupported_surface_covering(stratum_id))?,
            y_rows[2]
                .try_clone()
                .map_err(|_| unsupported_surface_covering(stratum_id))?,
        ],
        &[
            clone_algebraic(&first.normal[2], stratum_id)?,
            clone_algebraic(&second.normal[2], stratum_id)?,
            clone_algebraic(&third.normal[2], stratum_id)?,
        ],
        stratum_id,
    )?
    .div(&denominator)
    .map_err(|_| unsupported_surface_covering(stratum_id))?;
    let z = determinant(
        &[
            clone_algebraic(&first.normal[0], stratum_id)?,
            clone_algebraic(&second.normal[0], stratum_id)?,
            clone_algebraic(&third.normal[0], stratum_id)?,
        ],
        &[
            clone_algebraic(&first.normal[1], stratum_id)?,
            clone_algebraic(&second.normal[1], stratum_id)?,
            clone_algebraic(&third.normal[1], stratum_id)?,
        ],
        &[
            z_rows[0]
                .try_clone()
                .map_err(|_| unsupported_surface_covering(stratum_id))?,
            z_rows[1]
                .try_clone()
                .map_err(|_| unsupported_surface_covering(stratum_id))?,
            z_rows[2]
                .try_clone()
                .map_err(|_| unsupported_surface_covering(stratum_id))?,
        ],
        stratum_id,
    )?
    .div(&denominator)
    .map_err(|_| unsupported_surface_covering(stratum_id))?;
    Ok(Some(Point3 { x, y, z }))
}

fn plane_value(
    plane: &AffinePlane,
    point: &Point3,
    stratum_id: &str,
) -> Result<RealAlgebraic, CadFailure> {
    let coordinates = [&point.x, &point.y, &point.z];
    let mut value = clone_algebraic(&plane.constant, stratum_id)?;
    for (coefficient, coordinate) in plane.normal.iter().zip(coordinates) {
        value = value
            .add(
                &coefficient
                    .mul(coordinate)
                    .map_err(|_| unsupported_surface_covering(stratum_id))?,
            )
            .map_err(|_| unsupported_surface_covering(stratum_id))?;
    }
    Ok(value)
}

fn satisfies_plane(
    plane: &AffinePlane,
    point: &Point3,
    stratum_id: &str,
) -> Result<bool, CadFailure> {
    let sign = plane_value(plane, point, stratum_id)?
        .sign()
        .map_err(|_| unsupported_surface_covering(stratum_id))?;
    Ok(match sign {
        Sign::Zero => true,
        Sign::Positive => plane.relation == SignRelation::Positive,
        Sign::Negative => plane.relation == SignRelation::Negative,
    })
}

fn points_equal(left: &Point3, right: &Point3, stratum_id: &str) -> Result<bool, CadFailure> {
    for (left_coordinate, right_coordinate) in [
        (&left.x, &right.x),
        (&left.y, &right.y),
        (&left.z, &right.z),
    ] {
        if left_coordinate
            .compare(right_coordinate)
            .map_err(|_| unsupported_surface_covering(stratum_id))?
            != Ordering::Equal
        {
            return Ok(false);
        }
    }
    Ok(true)
}

fn collect_conjunctive_atoms<'a>(
    formula: &'a NormalizedRegionFormula,
    atoms: &mut Vec<&'a str>,
) -> bool {
    match formula {
        NormalizedRegionFormula::Atom(atom_id) => {
            atoms.push(atom_id.as_str());
            true
        }
        NormalizedRegionFormula::And(operands) => operands
            .iter()
            .all(|operand| collect_conjunctive_atoms(operand, atoms)),
        NormalizedRegionFormula::True => true,
        NormalizedRegionFormula::Not(_)
        | NormalizedRegionFormula::Or(_)
        | NormalizedRegionFormula::False => false,
    }
}

fn parameter_point(
    plane: &AffinePlane,
    point: &Point3,
    stratum_id: &str,
) -> Result<SurfaceParameterPoint, CadFailure> {
    let parameter_axes: Vec<usize> = (0..3).filter(|axis| *axis != plane.pivot_axis).collect();
    let coordinates = [&point.x, &point.y, &point.z];
    Ok(SurfaceParameterPoint::new(
        clone_algebraic(coordinates[parameter_axes[0]], stratum_id)?,
        clone_algebraic(coordinates[parameter_axes[1]], stratum_id)?,
    ))
}

fn compare_around(
    left: &SurfaceParameterPoint,
    right: &SurfaceParameterPoint,
    center_u: &RealAlgebraic,
    center_v: &RealAlgebraic,
    stratum_id: &str,
) -> Result<Ordering, CadFailure> {
    let left_u = left
        .u()
        .sub(center_u)
        .map_err(|_| unsupported_surface_covering(stratum_id))?;
    let left_v = left
        .v()
        .sub(center_v)
        .map_err(|_| unsupported_surface_covering(stratum_id))?;
    let right_u = right
        .u()
        .sub(center_u)
        .map_err(|_| unsupported_surface_covering(stratum_id))?;
    let right_v = right
        .v()
        .sub(center_v)
        .map_err(|_| unsupported_surface_covering(stratum_id))?;
    let upper_half = |u: &RealAlgebraic, v: &RealAlgebraic| -> Result<bool, CadFailure> {
        let v_sign = v
            .sign()
            .map_err(|_| unsupported_surface_covering(stratum_id))?;
        let u_sign = u
            .sign()
            .map_err(|_| unsupported_surface_covering(stratum_id))?;
        Ok(v_sign == Sign::Positive || v_sign == Sign::Zero && u_sign != Sign::Negative)
    };
    let left_half = upper_half(&left_u, &left_v)?;
    let right_half = upper_half(&right_u, &right_v)?;
    if left_half != right_half {
        return Ok(if left_half {
            Ordering::Less
        } else {
            Ordering::Greater
        });
    }
    let cross = left_u
        .mul(&right_v)
        .and_then(|value| value.sub(&left_v.mul(&right_u)?))
        .map_err(|_| unsupported_surface_covering(stratum_id))?;
    match cross
        .sign()
        .map_err(|_| unsupported_surface_covering(stratum_id))?
    {
        Sign::Positive => Ok(Ordering::Less),
        Sign::Negative => Ok(Ordering::Greater),
        Sign::Zero => {
            let left_distance = left_u
                .mul(&left_u)
                .and_then(|value| value.add(&left_v.mul(&left_v)?))
                .map_err(|_| unsupported_surface_covering(stratum_id))?;
            let right_distance = right_u
                .mul(&right_u)
                .and_then(|value| value.add(&right_v.mul(&right_v)?))
                .map_err(|_| unsupported_surface_covering(stratum_id))?;
            left_distance
                .compare(&right_distance)
                .map_err(|_| unsupported_surface_covering(stratum_id))
        }
    }
}

fn sort_parameter_points(
    points: &mut [SurfaceParameterPoint],
    stratum_id: &str,
) -> Result<(), CadFailure> {
    let count =
        i32::try_from(points.len()).map_err(|_| unsupported_surface_covering(stratum_id))?;
    let divisor = algebraic(count, stratum_id)?;
    let mut center_u = algebraic(0, stratum_id)?;
    let mut center_v = algebraic(0, stratum_id)?;
    for point in points.iter() {
        center_u = center_u
            .add(point.u())
            .map_err(|_| unsupported_surface_covering(stratum_id))?;
        center_v = center_v
            .add(point.v())
            .map_err(|_| unsupported_surface_covering(stratum_id))?;
    }
    center_u = center_u
        .div(&divisor)
        .map_err(|_| unsupported_surface_covering(stratum_id))?;
    center_v = center_v
        .div(&divisor)
        .map_err(|_| unsupported_surface_covering(stratum_id))?;
    for index in 1..points.len() {
        let mut position = index;
        while position > 0
            && compare_around(
                &points[position],
                &points[position - 1],
                &center_u,
                &center_v,
                stratum_id,
            )? == Ordering::Less
        {
            points.swap(position, position - 1);
            position -= 1;
        }
    }
    Ok(())
}

fn planar_trim(
    region: &neco_geometry::AlgebraicRegion,
    face_plane: &AffinePlane,
    stratum_id: &str,
) -> Result<
    (
        SurfaceParameterDomain,
        SurfaceTrimLoop2,
        Vec<SurfaceWeldVertex>,
    ),
    CadFailure,
> {
    let mut atom_ids = Vec::new();
    if !collect_conjunctive_atoms(region.normalized_region_formula(), &mut atom_ids) {
        return Err(unsupported_surface_covering(stratum_id));
    }
    let mut constraints = Vec::new();
    for atom_id in atom_ids {
        let atom = region
            .canonical_atoms()
            .iter()
            .find(|atom| atom.atom_id() == atom_id)
            .ok_or_else(|| unsupported_surface_covering(stratum_id))?;
        let RegionFunction::Polynomial(polynomial) = atom.function() else {
            return Err(unsupported_surface_covering(stratum_id));
        };
        let Some((normal, constant)) = affine_coefficients(polynomial, stratum_id)? else {
            return Err(unsupported_surface_covering(stratum_id));
        };
        let pivot_axis = normal
            .iter()
            .position(|coefficient| !coefficient.is_zero())
            .ok_or_else(|| unsupported_surface_covering(stratum_id))?;
        let pivot_sign = normal[pivot_axis]
            .sign()
            .map_err(|_| unsupported_surface_covering(stratum_id))?;
        let orientation = plane_orientation(atom.relation(), pivot_axis, pivot_sign)
            .ok_or_else(|| unsupported_surface_covering(stratum_id))?;
        constraints.push(AffinePlane {
            atom_id: String::from(atom.atom_id()),
            normal,
            constant,
            pivot_axis,
            relation: atom.relation(),
            orientation,
        });
    }
    let mut vertices = Vec::new();
    for first in 0..constraints.len() {
        for second in (first + 1)..constraints.len() {
            let Some(point) = intersect_planes(
                face_plane,
                &constraints[first],
                &constraints[second],
                stratum_id,
            )?
            else {
                continue;
            };
            let mut inside = true;
            for constraint in &constraints {
                if !satisfies_plane(constraint, &point, stratum_id)? {
                    inside = false;
                    break;
                }
            }
            if inside
                && !vertices
                    .iter()
                    .map(|existing| points_equal(existing, &point, stratum_id))
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .any(|equal| equal)
            {
                vertices.push(point);
            }
        }
    }
    if vertices.len() < 3 {
        return Err(unsupported_surface_covering(stratum_id));
    }
    let mut parameters = vertices
        .iter()
        .map(|point| parameter_point(face_plane, point, stratum_id))
        .collect::<Result<Vec<_>, _>>()?;
    sort_parameter_points(&mut parameters, stratum_id)?;
    let mut minimum_u = clone_algebraic(parameters[0].u(), stratum_id)?;
    let mut maximum_u = clone_algebraic(parameters[0].u(), stratum_id)?;
    let mut minimum_v = clone_algebraic(parameters[0].v(), stratum_id)?;
    let mut maximum_v = clone_algebraic(parameters[0].v(), stratum_id)?;
    for point in &parameters[1..] {
        if point
            .u()
            .compare(&minimum_u)
            .map_err(|_| unsupported_surface_covering(stratum_id))?
            == Ordering::Less
        {
            minimum_u = clone_algebraic(point.u(), stratum_id)?;
        }
        if point
            .u()
            .compare(&maximum_u)
            .map_err(|_| unsupported_surface_covering(stratum_id))?
            == Ordering::Greater
        {
            maximum_u = clone_algebraic(point.u(), stratum_id)?;
        }
        if point
            .v()
            .compare(&minimum_v)
            .map_err(|_| unsupported_surface_covering(stratum_id))?
            == Ordering::Less
        {
            minimum_v = clone_algebraic(point.v(), stratum_id)?;
        }
        if point
            .v()
            .compare(&maximum_v)
            .map_err(|_| unsupported_surface_covering(stratum_id))?
            == Ordering::Greater
        {
            maximum_v = clone_algebraic(point.v(), stratum_id)?;
        }
    }
    let domain = SurfaceParameterDomain::new(
        ParameterInterval::new(minimum_u, maximum_u, true, true).map_err(CadFailure::from)?,
        ParameterInterval::new(minimum_v, maximum_v, true, true).map_err(CadFailure::from)?,
    );
    let outer = SurfaceTrimLoop2::try_new(TrimLoopRole::Outer, parameters, &domain)
        .map_err(CadFailure::from)?;
    let mut weld_vertices = Vec::new();
    for point in vertices {
        let mut active_atoms = BTreeSet::new();
        for constraint in &constraints {
            if plane_value(constraint, &point, stratum_id)?
                .sign()
                .map_err(|_| unsupported_surface_covering(stratum_id))?
                == Sign::Zero
            {
                active_atoms.insert(constraint.atom_id.as_str());
            }
        }
        if active_atoms.len() < 3 {
            return Err(unsupported_surface_covering(stratum_id));
        }
        let weld_identity = active_atoms.into_iter().map(String::from).collect();
        weld_vertices.push(SurfaceWeldVertex::new(weld_identity, point));
    }
    Ok((domain, outer, weld_vertices))
}

fn surface_representation(
    atom_id: &str,
    surface: &SupportedSurface,
    stratum_id: &str,
) -> Result<SurfaceRepresentation, CadFailure> {
    let value = match surface {
        SupportedSurface::AffinePlane(plane) => {
            affine_plane_nurbs_surface(&plane.normal, &plane.constant, plane.pivot_axis)
        }
        SupportedSurface::Sphere(sphere) => sphere_nurbs_surface(&sphere.center, &sphere.radius),
    }
    .map_err(|_| surface_representation_failure(stratum_id))?;
    SurfaceRepresentation::new(
        SurfaceRepresentationId::new(format!("surface:{atom_id}"))?,
        value,
    )
    .map_err(|_| surface_representation_failure(stratum_id))
}

fn sphere_union_interiors_overlap(
    left: &SphereSurface,
    right: &SphereSurface,
    stratum_id: &str,
) -> Result<bool, CadFailure> {
    let differences = [
        left.center
            .x
            .sub(&right.center.x)
            .map_err(|_| unsupported_surface_covering(stratum_id))?,
        left.center
            .y
            .sub(&right.center.y)
            .map_err(|_| unsupported_surface_covering(stratum_id))?,
        left.center
            .z
            .sub(&right.center.z)
            .map_err(|_| unsupported_surface_covering(stratum_id))?,
    ];
    let zero = algebraic(0, stratum_id)?;
    let distance_squared =
        differences
            .iter()
            .try_fold(clone_algebraic(&zero, stratum_id)?, |sum, difference| {
                difference
                    .mul(difference)
                    .and_then(|square| sum.add(&square))
                    .map_err(|_| unsupported_surface_covering(stratum_id))
            })?;
    let radius_sum = left
        .radius
        .add(&right.radius)
        .map_err(|_| unsupported_surface_covering(stratum_id))?;
    let sum_squared = radius_sum
        .mul(&radius_sum)
        .map_err(|_| unsupported_surface_covering(stratum_id))?;
    Ok(distance_squared
        .compare(&sum_squared)
        .map_err(|_| unsupported_surface_covering(stratum_id))?
        == Ordering::Less)
}

fn full_sphere_covering_supported(
    region: &neco_geometry::AlgebraicRegion,
    current: &SphereSurface,
    stratum_id: &str,
) -> Result<bool, CadFailure> {
    match region.normalized_region_formula() {
        NormalizedRegionFormula::Atom(atom_id) => Ok(atom_id == &current.atom_id),
        NormalizedRegionFormula::Or(operands) => {
            for operand in operands {
                let NormalizedRegionFormula::Atom(atom_id) = operand else {
                    return Ok(false);
                };
                let atom = region
                    .canonical_atoms()
                    .iter()
                    .find(|atom| atom.atom_id() == atom_id)
                    .ok_or_else(|| unsupported_surface_covering(stratum_id))?;
                let RegionFunction::Polynomial(polynomial) = atom.function() else {
                    return Ok(false);
                };
                let Some(SupportedSurface::Sphere(other)) =
                    sphere_surface(atom.atom_id(), polynomial, atom.relation(), stratum_id)?
                else {
                    return Ok(false);
                };
                if atom_id != &current.atom_id
                    && sphere_union_interiors_overlap(current, &other, stratum_id)?
                {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        NormalizedRegionFormula::Not(_)
        | NormalizedRegionFormula::And(_)
        | NormalizedRegionFormula::True
        | NormalizedRegionFormula::False => Ok(false),
    }
}

fn surface_covering(
    surface: &SupportedSurface,
    region: &neco_geometry::AlgebraicRegion,
    stratum_id: &str,
) -> Result<(SurfaceCovering, Vec<SurfaceWeldVertex>), CadFailure> {
    match surface {
        SupportedSurface::AffinePlane(plane) => {
            let (domain, outer, weld_vertices) = planar_trim(region, plane, stratum_id)?;
            let covering = affine_plane_surface_covering(
                &plane.normal,
                &plane.constant,
                plane.pivot_axis,
                plane.orientation,
                domain,
                outer,
            )
            .map_err(|_| unsupported_surface_covering(stratum_id))?;
            Ok((covering, weld_vertices))
        }
        SupportedSurface::Sphere(sphere) => {
            if !full_sphere_covering_supported(region, sphere, stratum_id)? {
                return Err(unsupported_surface_covering(stratum_id));
            }
            Ok((
                sphere_surface_covering(&sphere.center, &sphere.radius)
                    .map_err(|_| unsupported_surface_covering(stratum_id))?,
                Vec::new(),
            ))
        }
    }
}

struct PendingFaceSurface {
    stratum_id: String,
    exact_surface: CanonicalAtom,
    covering: SurfaceCovering,
    weld_vertices: Vec<SurfaceWeldVertex>,
}

pub fn derive_analytic_brep(
    stratification: BoundaryStratification,
    source_revision: SourceRevision,
) -> Result<DerivedAnalyticBrep, CadFailure> {
    let region_kind = stratification.boundary().source_region().region_kind();
    if region_kind == RegionKind::LowerDimensionalSet {
        return Err(failure(BrepDerivationCondition::OpenBoundary, None));
    }
    let atom_ids: Vec<String> = stratification
        .boundary()
        .source_region()
        .canonical_atoms()
        .iter()
        .map(|atom| String::from(atom.atom_id()))
        .collect();
    for stratum in stratification.strata() {
        if matches!(
            stratum.dimension(),
            BoundaryStratumDimension::UndeterminedDimension { .. }
        ) && region_kind == RegionKind::RegularClosedVolume
        {
            for atom_id in support_atom_ids(stratum) {
                if !atom_has_proved_face(stratification.strata(), &atom_id) {
                    return Err(failure(
                        BrepDerivationCondition::UndeterminedDimension,
                        Some(String::from(stratum.stratum_id())),
                    ));
                }
            }
        }
    }
    let face_strata: Vec<&BoundaryStratum> = stratification
        .strata()
        .iter()
        .filter(|stratum| proved_dimension(stratum, 2))
        .collect();
    if face_strata.is_empty() {
        return Err(failure(BrepDerivationCondition::IncompleteCoverage, None));
    }
    if region_kind == RegionKind::RegularClosedVolume {
        for atom_id in &atom_ids {
            if !atom_has_proved_face(stratification.strata(), atom_id) {
                return Err(failure(
                    BrepDerivationCondition::IncompleteCoverage,
                    Some(atom_id.clone()),
                ));
            }
        }
    }
    let vertex_ids: Vec<String> = stratification
        .strata()
        .iter()
        .filter(|stratum| proved_dimension(stratum, 0))
        .map(|stratum| String::from(stratum.stratum_id()))
        .collect();
    let chart = ArrangementId::new(String::from("full-domain-chart"))?;
    let arrangement = construct_full_domain_arrangement(vertex_ids.clone())
        .map_err(|_| failure(BrepDerivationCondition::IncompleteCoverage, None))?;
    let source_region = stratification.boundary().source_region();
    let source_atoms = source_region.canonical_atoms();
    let mut surfaces = Vec::new();
    let mut face_assignments = Vec::new();
    let mut pending_face_surfaces = Vec::new();
    let mut represented_atoms = BTreeSet::new();
    for stratum in face_strata {
        let stratum_id = stratum.stratum_id();
        let atom_id = unique_support_atom_id(stratum)?;
        let source_atom = source_atoms
            .iter()
            .find(|atom| atom.atom_id() == atom_id)
            .ok_or_else(|| {
                failure(
                    BrepDerivationCondition::IncompleteCoverage,
                    Some(String::from(stratum_id)),
                )
            })?;
        let surface = classify_surface(source_atom, stratum_id)?;
        if represented_atoms.insert(atom_id.clone()) {
            surfaces.push(surface_representation(&atom_id, &surface, stratum_id)?);
        }
        let (covering, weld_vertices) = surface_covering(&surface, source_region, stratum_id)?;
        pending_face_surfaces.push(PendingFaceSurface {
            stratum_id: String::from(stratum_id),
            exact_surface: source_atom.try_clone().map_err(CadFailure::from)?,
            covering,
            weld_vertices,
        });
        face_assignments.push(FaceAssignment::new(
            String::from(stratum_id),
            chart.clone(),
            String::from("full-domain-face"),
            SurfaceRepresentationId::new(format!("surface:{atom_id}"))?,
        )?);
    }
    let vertex_assignments = vertex_ids
        .iter()
        .map(|stratum_id| {
            VertexAssignment::new(stratum_id.clone(), chart.clone(), stratum_id.clone())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let request = BrepMaterializationRequest::new(
        stratification,
        vec![ChartArrangement::new(chart, arrangement)],
        Vec::new(),
        surfaces,
        BrepAssignments::new(vertex_assignments, Vec::new(), face_assignments)?,
        source_revision,
    )?;
    let brep = materialize_analytic_brep(request)?;
    if brep.faces().is_empty() {
        return Err(failure(BrepDerivationCondition::IncompleteCoverage, None));
    }
    if region_kind == RegionKind::RegularClosedVolume
        && brep.witnesses().closed_shell_ids().is_empty()
    {
        return Err(failure(BrepDerivationCondition::OpenBoundary, None));
    }
    let mut face_surfaces = Vec::new();
    for pending in pending_face_surfaces {
        let face_id = brep
            .faces()
            .iter()
            .find(|face| face.stratum_id() == pending.stratum_id)
            .map(|face| String::from(face.id()))
            .ok_or_else(|| {
                failure(
                    BrepDerivationCondition::IncompleteCoverage,
                    Some(pending.stratum_id.clone()),
                )
            })?;
        face_surfaces.push(
            FaceExactSurface::new(face_id, pending.exact_surface, pending.covering)
                .with_weld_vertices(pending.weld_vertices),
        );
    }
    face_surfaces.sort_by(|left, right| left.face_id().cmp(right.face_id()));
    Ok(DerivedAnalyticBrep {
        brep,
        face_surfaces,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiple_support_atoms_are_classified_as_ambiguous() {
        let atom_ids = BTreeSet::from([String::from("first"), String::from("second")]);
        assert_eq!(
            unique_atom_id(atom_ids, "face-stratum"),
            Err(CadFailure::BrepDerivation {
                condition: BrepDerivationCondition::AmbiguousSupportAtom,
                stratum_id: Some(String::from("face-stratum")),
            })
        );
    }

    #[test]
    fn rational_function_surface_is_classified_as_unsupported() {
        let term = |coefficient, x| {
            neco_geometry::RegionPolynomialTerm::try_new(
                algebraic(coefficient, "rational").unwrap(),
                x,
                0,
                0,
            )
            .unwrap()
        };
        let numerator = RegionPolynomial3::try_from_terms(vec![term(1, 0)]).unwrap();
        let denominator = RegionPolynomial3::try_from_terms(vec![term(1, 1)]).unwrap();
        let domain = neco_geometry::RationalFunctionDomain3::try_new(
            RegionPolynomial3::try_from_terms(vec![term(1, 1)]).unwrap(),
        )
        .unwrap();
        let atom = CanonicalAtom::try_new(
            "rational",
            RegionFunction::RationalFunction(
                neco_geometry::RationalFunction3::try_new(numerator, denominator, domain).unwrap(),
            ),
            SignRelation::Positive,
        )
        .unwrap();
        assert_eq!(
            classify_surface(&atom, "rational-stratum").err(),
            Some(CadFailure::BrepDerivation {
                condition: BrepDerivationCondition::UnsupportedSurfaceCovering,
                stratum_id: Some(String::from("rational-stratum")),
            })
        );
    }

    #[test]
    fn full_sphere_union_requires_disjoint_interiors() {
        let sphere = |center_x, radius| SphereSurface {
            atom_id: String::from("sphere"),
            center: Point3 {
                x: algebraic(center_x, "sphere").unwrap(),
                y: algebraic(0, "sphere").unwrap(),
                z: algebraic(0, "sphere").unwrap(),
            },
            radius: algebraic(radius, "sphere").unwrap(),
        };
        assert!(
            !sphere_union_interiors_overlap(&sphere(0, 1), &sphere(2, 1), "external-contact")
                .unwrap()
        );
        assert!(
            sphere_union_interiors_overlap(&sphere(0, 2), &sphere(0, 1), "containment").unwrap()
        );
    }
}
