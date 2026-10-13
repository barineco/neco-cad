use crate::construction_types::*;
use crate::provenance::{
    BodyComposition, ConstructionGraph, ConstructionGraphFailure, SourceIdentity,
};
use alloc::boxed::Box;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::cmp::Ordering;
use neco_algnum::RealAlgebraic;
use neco_bigint::{BigInt, Dyadic};
use neco_brep::{
    CurveRepresentation, CurveRepresentationId, SurfaceRepresentation, SurfaceRepresentationId,
};
use neco_cad_import::{region_from_mesh, RegionImportRequest};
use neco_geometry::{
    AlgebraicRegion, CanonicalAtom, Enclosure, GeometryFailure, MembershipClassification,
    NormalizedRegionFormula, Point3, RegionAffineMap3, RegionFunction, RegionKind,
    RegionMembershipRequest, RegionPoint, RegionPolynomial3, RegionPolynomialTerm,
    RootIsolationPolicy, RootRefinementMethod, SignRelation,
};
use neco_mesh::{validate_closed_oriented, validate_surface_mesh, MeshSource, SurfaceMesh};
use neco_nurbs::{define_nurbs, BezierPoint, NurbsDefinitionRequest};

fn referenced<'a>(
    operations: &'a [ConstructionOperation],
    reference: &OperationOutputRef,
) -> Result<&'a ConstructionOperation, CadConstructionFailure> {
    if reference.output.slot != OutputSlot::Primary {
        return Err(CadConstructionFailure::Graph(
            ConstructionGraphFailure::UnknownOutputSlot(reference.output.clone()),
        ));
    }
    operations
        .iter()
        .find(|operation| operation.operation == reference.output.operation)
        .ok_or_else(|| {
            CadConstructionFailure::Graph(ConstructionGraphFailure::UnknownRoot(
                reference.output.operation.clone(),
            ))
        })
}
fn mismatch(
    operation: &ConstructionOperation,
    expected_schema: GeometryOutputExpectation,
) -> CadConstructionFailure {
    CadConstructionFailure::OutputSchemaMismatch {
        operation: operation.operation.clone(),
        expected: expected_schema,
        actual: operation.expected_output.clone(),
    }
}
fn schema(category: GeometryOutputCategory, closure: GeometryClosure) -> GeometryOutputExpectation {
    expected(category, closure)
}
fn produced_from_profile(profile: &GeometryOutputExpectation) -> GeometryOutputExpectation {
    match profile.closure {
        GeometryClosure::Closed => schema(GeometryOutputCategory::Solid, GeometryClosure::Closed),
        GeometryClosure::Open => schema(GeometryOutputCategory::Sheet, GeometryClosure::Open),
    }
}
fn profile_input(
    operation: &ConstructionOperation,
    role: InputRole,
    input: &ConstructionOperation,
) -> Result<(), CadConstructionFailure> {
    if input.expected_output.category != GeometryOutputCategory::Profile {
        return Err(CadConstructionFailure::Graph(
            ConstructionGraphFailure::InputKindMismatch {
                consumer: operation.operation.clone(),
                role,
                expected: GeometryOutputCategory::Profile,
                actual: input.expected_output.category,
            },
        ));
    }
    Ok(())
}
fn valid_segment_curve(curve: &ProfileSegmentCurve) -> Result<(), &'static str> {
    match curve {
        ProfileSegmentCurve::Line => Ok(()),
        ProfileSegmentCurve::CircularArc { radius, .. } if radius.is_zero() => Err("profile.arc"),
        ProfileSegmentCurve::CircularArc { .. } => Ok(()),
        ProfileSegmentCurve::Nurbs {
            degree,
            knots,
            control_points,
            weights,
            parameter_domain,
        } => valid_nurbs(
            knots,
            *degree,
            control_points.len(),
            weights,
            parameter_domain,
        ),
    }
}
fn validate_curve(curve: &CurveDeclaration) -> Result<(), &'static str> {
    match curve {
        CurveDeclaration::Line { length, .. } if length.is_zero() => Err("curve.line"),
        CurveDeclaration::Polyline { points, closed } => {
            if points.len() < if *closed { 3 } else { 2 } {
                Err("curve.polyline")
            } else {
                Ok(())
            }
        }
        CurveDeclaration::CircularArc { radius, .. } if radius.is_zero() => Err("curve.arc"),
        CurveDeclaration::Nurbs {
            degree,
            knots,
            control_points,
            weights,
            parameter_domain,
            ..
        } => valid_nurbs(
            knots,
            *degree,
            control_points.len(),
            weights,
            parameter_domain,
        ),
        _ => Ok(()),
    }
}
fn positive(value: &ExactScalar) -> Result<bool, CadConstructionFailure> {
    Ok(value.compare_exact(&ExactScalar::new(0.)?)? == Ordering::Greater)
}
fn positive_vector(value: &ExactVector3) -> Result<bool, CadConstructionFailure> {
    value
        .0
        .iter()
        .try_fold(true, |valid, component| Ok(valid && positive(component)?))
}
fn valid_targets(targets: &[SubentityRef]) -> bool {
    !targets.is_empty()
        && targets
            .iter()
            .all(|target| !target.subentity.is_empty() && !target.expected_kind.is_empty())
}
fn validate_payload(operation: &ConstructionOperation) -> Result<(), CadConstructionFailure> {
    let bad = |field| CadConstructionFailure::PayloadInvalid {
        operation: operation.operation.clone(),
        field,
    };
    match &operation.request {
        OperationRequest::Curve(curve) => validate_curve(curve).map_err(bad)?,
        OperationRequest::Primitive(primitive) => {
            let valid = match primitive {
                PrimitiveDeclaration::Box { lengths }
                | PrimitiveDeclaration::Ellipsoid { radii: lengths } => positive_vector(lengths)?,
                PrimitiveDeclaration::Cylinder {
                    outer_radius,
                    inner_radius,
                    height,
                } => {
                    positive(outer_radius)?
                        && positive(height)?
                        && match inner_radius {
                            Some(inner) => {
                                positive(inner)?
                                    && inner.compare_exact(outer_radius)? == Ordering::Less
                            }
                            None => true,
                        }
                }
                PrimitiveDeclaration::Cone {
                    base_radius,
                    top_radius,
                    height,
                } => positive(base_radius)? && positive(top_radius)? && positive(height)?,
                PrimitiveDeclaration::Sphere { radius } => positive(radius)?,
                PrimitiveDeclaration::Torus {
                    major_radius,
                    minor_radius,
                } => {
                    positive(major_radius)?
                        && positive(minor_radius)?
                        && major_radius.compare_exact(minor_radius)? == Ordering::Greater
                }
            };
            if !valid {
                return Err(bad("primitive.dimensions"));
            }
        }
        OperationRequest::Profile(profile) => {
            let corners: BTreeSet<_> = profile.corners.iter().map(|x| &x.id).collect();
            let segments: BTreeSet<_> = profile.segments.iter().map(|x| &x.id).collect();
            let corner_order: BTreeSet<_> = profile.corner_order.iter().collect();
            let segment_order: BTreeSet<_> = profile.segment_order.iter().collect();
            if corners.len() != profile.corners.len()
                || segments.len() != profile.segments.len()
                || corner_order.len() != corners.len()
                || segment_order.len() != segments.len()
                || !profile.corner_order.iter().all(|id| corners.contains(id))
                || !profile.segment_order.iter().all(|id| segments.contains(id))
                || profile.segments.iter().any(|segment| {
                    !corners.contains(&segment.start_corner)
                        || !corners.contains(&segment.end_corner)
                        || valid_segment_curve(&segment.curve).is_err()
                })
            {
                return Err(bad("profile.topology"));
            }
        }
        OperationRequest::Fillet {
            targets, radius, ..
        } => {
            if !valid_targets(targets) || !positive(radius)? {
                return Err(bad("fillet"));
            }
        }
        OperationRequest::Chamfer {
            targets, distance, ..
        } => {
            if !valid_targets(targets) || !positive(distance)? {
                return Err(bad("chamfer"));
            }
        }
        OperationRequest::Transform { scale, .. } => {
            if !positive(scale)? {
                return Err(bad("transform.scale"));
            }
        }
        OperationRequest::Extrude { distance, .. } => {
            if !positive(distance)? {
                return Err(bad("extrude.distance"));
            }
        }
        OperationRequest::Revolve { .. } => {}
        OperationRequest::Sweep {
            scale_samples,
            twist_samples,
            ..
        } => {
            for samples in [scale_samples, twist_samples] {
                for pair in samples.windows(2) {
                    if pair[0]
                        .parameter
                        .compare_exact(&pair[1].parameter)
                        .map_err(|_| bad("sweep.samples"))?
                        != Ordering::Less
                    {
                        return Err(bad("sweep.samples"));
                    }
                }
            }
        }
        OperationRequest::Loft {
            sections,
            section_order,
            ..
        } => {
            let slots: BTreeSet<_> = sections.iter().map(|x| &x.slot).collect();
            let order: BTreeSet<_> = section_order.iter().collect();
            if sections.is_empty()
                || slots.len() != sections.len()
                || order.len() != slots.len()
                || !section_order.iter().all(|slot| slots.contains(slot))
            {
                return Err(bad("loft.order"));
            }
        }
        OperationRequest::ExplicitSolid { vertices, faces } => {
            let vertex_ids: BTreeSet<_> = vertices.iter().map(|x| &x.id).collect();
            let face_ids: BTreeSet<_> = faces.iter().map(|x| &x.id).collect();
            if vertices.is_empty()
                || faces.is_empty()
                || vertex_ids.len() != vertices.len()
                || face_ids.len() != faces.len()
                || faces.iter().any(|face| {
                    face.vertex_order.len() < 3
                        || face.vertex_order.iter().collect::<BTreeSet<_>>().len()
                            != face.vertex_order.len()
                        || face
                            .vertex_order
                            .iter()
                            .any(|vertex| !vertex_ids.contains(vertex))
                })
            {
                return Err(bad("explicit-solid.topology"));
            }
        }
        OperationRequest::NurbsSurfacePatch(patch) => {
            let rows = patch.control_net.len();
            let columns = patch.control_net.first().map_or(0, Vec::len);
            if rows == 0
                || columns == 0
                || patch.control_net.iter().any(|row| row.len() != columns)
                || patch.weight_net.len() != rows
                || patch.weight_net.iter().any(|row| row.len() != columns)
            {
                return Err(bad("surface.rectangular"));
            }
            let one = ExactScalar::new(1.).map_err(|_| bad("surface"))?;
            valid_nurbs(
                &patch.u_knots,
                patch.u_degree,
                rows,
                &vec![one.clone(); rows],
                &patch.parameter_domain[0],
            )
            .map_err(bad)?;
            valid_nurbs(
                &patch.v_knots,
                patch.v_degree,
                columns,
                &vec![one; columns],
                &patch.parameter_domain[1],
            )
            .map_err(bad)?;
            let zero = ExactScalar::new(0.).map_err(|_| bad("surface.weights"))?;
            if patch
                .weight_net
                .iter()
                .flatten()
                .any(|weight| weight.compare_exact(&zero) != Ok(Ordering::Greater))
            {
                return Err(bad("surface.weights"));
            }
            let mut trim_ids = BTreeSet::new();
            for trim in &patch.trims {
                if trim.trim_id.is_empty() || !trim_ids.insert(&trim.trim_id) {
                    return Err(bad("surface.trims"));
                }
                valid_nurbs(
                    &trim.knots,
                    trim.degree,
                    trim.control_points.len(),
                    &trim.weights,
                    &trim.parameter_domain,
                )
                .map_err(bad)?;
            }
        }
        OperationRequest::Boolean { left, right, .. } => {
            if left.output == right.output {
                return Err(bad("boolean.operands"));
            }
        }
        OperationRequest::ExternalGeometry(asset) => {
            if asset.asset_id.is_empty()
                || asset.source_relative_path.is_empty()
                || asset.adapter.is_empty()
                || !positive(&asset.unit_scale)?
            {
                return Err(bad("external.asset"));
            }
        }
    }
    Ok(())
}
fn required_schema(
    operation: &ConstructionOperation,
    operations: &[ConstructionOperation],
) -> Result<GeometryOutputExpectation, CadConstructionFailure> {
    Ok(match &operation.request {
        OperationRequest::Curve(_) => schema(GeometryOutputCategory::Curve, GeometryClosure::Open),
        OperationRequest::Primitive(_) | OperationRequest::ExplicitSolid { .. } => {
            schema(GeometryOutputCategory::Solid, GeometryClosure::Closed)
        }
        OperationRequest::Profile(profile) => {
            schema(GeometryOutputCategory::Profile, profile.closure)
        }
        OperationRequest::NurbsSurfacePatch(_) => {
            schema(GeometryOutputCategory::Sheet, GeometryClosure::Open)
        }
        OperationRequest::ExternalGeometry(asset) => asset.asset_schema.clone(),
        OperationRequest::Transform { source, .. } => {
            referenced(operations, source)?.expected_output.clone()
        }
        OperationRequest::Fillet { source, .. } | OperationRequest::Chamfer { source, .. } => {
            let source = referenced(operations, source)?;
            profile_input(operation, InputRole::Source, source)?;
            source.expected_output.clone()
        }
        OperationRequest::Extrude { profile, .. } | OperationRequest::Revolve { profile, .. } => {
            let profile = referenced(operations, profile)?;
            profile_input(operation, InputRole::Profile, profile)?;
            produced_from_profile(&profile.expected_output)
        }
        OperationRequest::Sweep { profile, spine, .. } => {
            let profile = referenced(operations, profile)?;
            profile_input(operation, InputRole::Profile, profile)?;
            let spine = referenced(operations, spine)?;
            if spine.expected_output != schema(GeometryOutputCategory::Curve, GeometryClosure::Open)
            {
                return Err(mismatch(
                    spine,
                    schema(GeometryOutputCategory::Curve, GeometryClosure::Open),
                ));
            }
            produced_from_profile(&profile.expected_output)
        }
        OperationRequest::Loft { sections, .. } => {
            let first = sections
                .first()
                .ok_or_else(|| CadConstructionFailure::PayloadInvalid {
                    operation: operation.operation.clone(),
                    field: "loft.sections",
                })?;
            let first_profile = referenced(operations, &first.profile)?;
            profile_input(operation, InputRole::Section, first_profile)?;
            for section in sections.iter().skip(1) {
                let profile = referenced(operations, &section.profile)?;
                profile_input(operation, InputRole::Section, profile)?;
                if profile.expected_output != first_profile.expected_output {
                    return Err(mismatch(
                        operation,
                        produced_from_profile(&first_profile.expected_output),
                    ));
                }
            }
            produced_from_profile(&first_profile.expected_output)
        }
        OperationRequest::Boolean { left, right, .. } => {
            let left = referenced(operations, left)?;
            let right = referenced(operations, right)?;
            if left.expected_output != right.expected_output
                || !matches!(
                    left.expected_output.category,
                    GeometryOutputCategory::Profile | GeometryOutputCategory::Solid
                )
            {
                return Err(CadConstructionFailure::Graph(
                    ConstructionGraphFailure::InputKindMismatch {
                        consumer: operation.operation.clone(),
                        role: InputRole::Left,
                        expected: GeometryOutputCategory::Solid,
                        actual: left.expected_output.category,
                    },
                ));
            }
            left.expected_output.clone()
        }
    })
}
fn validate_operations(operations: &[ConstructionOperation]) -> Result<(), CadConstructionFailure> {
    let mut sorted: Vec<_> = operations.iter().collect();
    sorted.sort_by(|a, b| a.operation.cmp(&b.operation));
    for operation in sorted {
        validate_payload(operation)?;
        let required = required_schema(operation, operations)?;
        if operation.expected_output != required {
            return Err(mismatch(operation, required));
        }
    }
    Ok(())
}
fn root_identity(root: &ConstructionRoot) -> &SourceIdentity {
    match root {
        ConstructionRoot::GeometryInstance { instance, .. } => instance,
        ConstructionRoot::PhysicalBody { body, .. } => body,
        ConstructionRoot::PhysicalDomain { domain, .. } => domain,
    }
}
fn validate_roots(
    request: &ConstructionRequest,
) -> Result<(Vec<ConstructionRoot>, Vec<BodyComposition>), CadConstructionFailure> {
    let mut roots = request.roots.clone();
    roots.sort_by_key(|root| root_identity(root).clone());
    let mut identities = BTreeSet::new();
    let mut body_ordinals = BTreeSet::new();
    let mut bodies = Vec::new();
    for root in &roots {
        if !identities.insert(root_identity(root).clone()) {
            return Err(CadConstructionFailure::Graph(
                ConstructionGraphFailure::DuplicateRoot(root_identity(root).clone()),
            ));
        }
        match root {
            ConstructionRoot::GeometryInstance { output, .. } => {
                referenced(&request.operations, output)?;
            }
            ConstructionRoot::PhysicalBody {
                body,
                output,
                ordinal,
            } => {
                if !body_ordinals.insert(*ordinal) {
                    return Err(CadConstructionFailure::Graph(
                        ConstructionGraphFailure::DuplicateBodyOrdinal(*ordinal),
                    ));
                }
                let operation = referenced(&request.operations, output)?;
                if operation.expected_output.category != GeometryOutputCategory::Solid {
                    return Err(CadConstructionFailure::Graph(
                        ConstructionGraphFailure::InputKindMismatch {
                            consumer: operation.operation.clone(),
                            role: InputRole::BodyGeometry,
                            expected: GeometryOutputCategory::Solid,
                            actual: operation.expected_output.category,
                        },
                    ));
                }
                bodies.push((*ordinal, body.clone()));
            }
            ConstructionRoot::PhysicalDomain { .. } => {}
        }
    }
    bodies.sort();
    let mut compositions = Vec::new();
    for root in &roots {
        if let ConstructionRoot::PhysicalDomain {
            domain,
            bodies: members,
            composition,
        } = root
        {
            if bodies.iter().map(|(_, body)| body).ne(members.iter())
                || !request.provenance.composition.contains(composition)
            {
                return Err(CadConstructionFailure::Graph(
                    ConstructionGraphFailure::DomainBodyMismatch(domain.clone()),
                ));
            }
            compositions.push(composition.clone());
        }
    }
    Ok((roots, compositions))
}

fn coefficient_operation() -> Result<crate::OperationIdentity, CadConstructionFailure> {
    crate::OperationIdentity::new("region-polynomial").map_err(CadConstructionFailure::Identity)
}

fn lowering(
    operation: &crate::OperationIdentity,
    condition: LoweringCondition,
) -> CadConstructionFailure {
    CadConstructionFailure::Lowering {
        operation: operation.clone(),
        condition,
        field: None,
        operand_provenance: None,
        lower: None,
    }
}

fn lowering_geometry(
    operation: &crate::OperationIdentity,
    condition: LoweringCondition,
    failure: GeometryFailure,
) -> CadConstructionFailure {
    CadConstructionFailure::Lowering {
        operation: operation.clone(),
        condition,
        field: None,
        operand_provenance: None,
        lower: Some(Box::new(CadConstructionLowerFailure::Geometry(failure))),
    }
}

fn geometry_condition(failure: &GeometryFailure) -> LoweringCondition {
    match failure {
        GeometryFailure::DuplicateRegionAtom | GeometryFailure::DuplicateRegionCanonicalKey => {
            LoweringCondition::DuplicateTerm
        }
        GeometryFailure::RegionAmbientDimensionMismatch
        | GeometryFailure::RegionPointDimensionMismatch => LoweringCondition::DimensionMismatch,
        GeometryFailure::MalformedRegionFormula => LoweringCondition::UnknownAtom,
        GeometryFailure::AlgebraFailure(_) => LoweringCondition::UnconvertibleCoefficient,
        GeometryFailure::CalculusFailure { .. } | GeometryFailure::DenominatorValidity { .. } => {
            LoweringCondition::ZeroDenominator
        }
        _ => LoweringCondition::ImplicitizationFailed,
    }
}

pub fn region_polynomial_from_exact_terms(
    terms: &[(ExactScalar, u32, u32, u32)],
) -> Result<RegionPolynomial3, CadConstructionFailure> {
    let operation = coefficient_operation()?;
    for left in 0..terms.len() {
        for right in 0..left {
            if terms[left].1 != terms[right].1
                || terms[left].2 != terms[right].2
                || terms[left].3 != terms[right].3
            {
                continue;
            }
            let condition = if terms[left].0.compare_exact(&terms[right].0)? == Ordering::Equal {
                LoweringCondition::DuplicateTerm
            } else {
                LoweringCondition::DuplicateExponent
            };
            return Err(lowering(&operation, condition));
        }
    }
    let mut converted = Vec::new();
    for (coefficient, exponent_x, exponent_y, exponent_z) in terms {
        converted.push(
            RegionPolynomialTerm::try_new(
                coefficient.to_real_algebraic()?,
                *exponent_x,
                *exponent_y,
                *exponent_z,
            )
            .map_err(|failure| {
                lowering_geometry(&operation, geometry_condition(&failure), failure)
            })?,
        );
    }
    RegionPolynomial3::try_from_terms(converted)
        .map_err(|failure| lowering_geometry(&operation, geometry_condition(&failure), failure))
}

fn poly_terms(
    operation: &crate::OperationIdentity,
    terms: &[(ExactScalar, u32, u32, u32)],
) -> Result<RegionPolynomial3, CadConstructionFailure> {
    match region_polynomial_from_exact_terms(terms) {
        Ok(polynomial) => Ok(polynomial),
        Err(CadConstructionFailure::Lowering {
            condition, lower, ..
        }) => Err(CadConstructionFailure::Lowering {
            operation: operation.clone(),
            condition,
            field: Some("coefficients"),
            operand_provenance: None,
            lower,
        }),
        Err(failure) => Err(failure),
    }
}

fn zero_scalar() -> Result<ExactScalar, CadConstructionFailure> {
    ExactScalar::new(0.)
}

fn one_scalar() -> Result<ExactScalar, CadConstructionFailure> {
    ExactScalar::new(1.)
}

fn two_scalar() -> Result<ExactScalar, CadConstructionFailure> {
    ExactScalar::new(2.)
}

fn negate(value: &ExactScalar) -> Result<ExactScalar, CadConstructionFailure> {
    zero_scalar()?.sub(value)
}

fn push_term(
    terms: &mut Vec<(ExactScalar, u32, u32, u32)>,
    coefficient: ExactScalar,
    exponent_x: u32,
    exponent_y: u32,
    exponent_z: u32,
) {
    if !coefficient.is_zero() {
        terms.push((coefficient, exponent_x, exponent_y, exponent_z));
    }
}

fn poly_atom(
    atom_id: String,
    polynomial: RegionPolynomial3,
    relation: SignRelation,
) -> Result<CanonicalAtom, GeometryFailure> {
    CanonicalAtom::try_new(atom_id, RegionFunction::Polynomial(polynomial), relation)
}

fn algebraic_region(
    operation: &crate::OperationIdentity,
    atoms: Vec<CanonicalAtom>,
    formula: NormalizedRegionFormula,
    kind: RegionKind,
) -> Result<AlgebraicRegion, CadConstructionFailure> {
    AlgebraicRegion::try_new(3, atoms, formula, kind)
        .map_err(|failure| lowering_geometry(operation, geometry_condition(&failure), failure))
}

fn formula_and(atoms: Vec<String>) -> NormalizedRegionFormula {
    let operands = atoms
        .into_iter()
        .map(NormalizedRegionFormula::Atom)
        .collect();
    NormalizedRegionFormula::And(operands)
}

fn lower_primitive(
    operation: &crate::OperationIdentity,
    primitive: &PrimitiveDeclaration,
) -> Result<ConstructionOutput, CadConstructionFailure> {
    let prefix = operation.as_str();
    match primitive {
        PrimitiveDeclaration::Sphere { radius } => {
            let one = one_scalar()?;
            let radius_sq = radius.mul(radius)?;
            let polynomial = poly_terms(
                operation,
                &[
                    (one.clone(), 2, 0, 0),
                    (one.clone(), 0, 2, 0),
                    (one, 0, 0, 2),
                    (negate(&radius_sq)?, 0, 0, 0),
                ],
            )?;
            let atom_id = format!("{prefix}:sphere");
            let atom = poly_atom(atom_id.clone(), polynomial, SignRelation::Negative).map_err(
                |failure| lowering_geometry(operation, geometry_condition(&failure), failure),
            )?;
            Ok(ConstructionOutput::Region(algebraic_region(
                operation,
                vec![atom],
                NormalizedRegionFormula::Atom(atom_id),
                RegionKind::RegularClosedVolume,
            )?))
        }
        PrimitiveDeclaration::Ellipsoid { radii } => {
            let rx = &radii.0[0];
            let ry = &radii.0[1];
            let rz = &radii.0[2];
            let rx2 = rx.mul(rx)?;
            let ry2 = ry.mul(ry)?;
            let rz2 = rz.mul(rz)?;
            let polynomial = poly_terms(
                operation,
                &[
                    (ry2.mul(&rz2)?, 2, 0, 0),
                    (rx2.mul(&rz2)?, 0, 2, 0),
                    (rx2.mul(&ry2)?, 0, 0, 2),
                    (negate(&rx2.mul(&ry2)?.mul(&rz2)?)?, 0, 0, 0),
                ],
            )?;
            let atom_id = format!("{prefix}:ellipsoid");
            let atom = poly_atom(atom_id.clone(), polynomial, SignRelation::Negative).map_err(
                |failure| lowering_geometry(operation, geometry_condition(&failure), failure),
            )?;
            Ok(ConstructionOutput::Region(algebraic_region(
                operation,
                vec![atom],
                NormalizedRegionFormula::Atom(atom_id),
                RegionKind::RegularClosedVolume,
            )?))
        }
        PrimitiveDeclaration::Box { lengths } => {
            let half = [
                lengths.0[0].div(&two_scalar()?)?,
                lengths.0[1].div(&two_scalar()?)?,
                lengths.0[2].div(&two_scalar()?)?,
            ];
            let minus_one = negate(&one_scalar()?)?;
            let specs = [
                ("box-x-pos", half[0].clone(), minus_one.clone(), 1, 0, 0),
                ("box-x-neg", half[0].clone(), one_scalar()?, 1, 0, 0),
                ("box-y-pos", half[1].clone(), minus_one.clone(), 0, 1, 0),
                ("box-y-neg", half[1].clone(), one_scalar()?, 0, 1, 0),
                ("box-z-pos", half[2].clone(), minus_one, 0, 0, 1),
                ("box-z-neg", half[2].clone(), one_scalar()?, 0, 0, 1),
            ];
            let mut atoms = Vec::new();
            let mut ids = Vec::new();
            for (name, constant, linear, x, y, z) in specs {
                let atom_id = format!("{prefix}:{name}");
                let polynomial = poly_terms(operation, &[(constant, 0, 0, 0), (linear, x, y, z)])?;
                atoms.push(
                    poly_atom(atom_id.clone(), polynomial, SignRelation::Positive).map_err(
                        |failure| {
                            lowering_geometry(operation, geometry_condition(&failure), failure)
                        },
                    )?,
                );
                ids.push(atom_id);
            }
            Ok(ConstructionOutput::Region(algebraic_region(
                operation,
                atoms,
                formula_and(ids),
                RegionKind::RegularClosedVolume,
            )?))
        }
        PrimitiveDeclaration::Cylinder {
            outer_radius,
            inner_radius,
            height,
        } => {
            let one = one_scalar()?;
            let outer = poly_terms(
                operation,
                &[
                    (one.clone(), 2, 0, 0),
                    (one.clone(), 0, 2, 0),
                    (negate(&outer_radius.mul(outer_radius)?)?, 0, 0, 0),
                ],
            )?;
            let z_poly = poly_terms(operation, &[(one.clone(), 0, 0, 1)])?;
            let height_poly = poly_terms(
                operation,
                &[(height.clone(), 0, 0, 0), (negate(&one)?, 0, 0, 1)],
            )?;
            let mut atoms = Vec::new();
            let mut ids = Vec::new();
            let names = [
                (
                    format!("{prefix}:cylinder-outer"),
                    outer,
                    SignRelation::Negative,
                ),
                (
                    format!("{prefix}:cylinder-bottom"),
                    z_poly,
                    SignRelation::Positive,
                ),
                (
                    format!("{prefix}:cylinder-top"),
                    height_poly,
                    SignRelation::Positive,
                ),
            ];
            for (atom_id, polynomial, relation) in names {
                atoms.push(poly_atom(atom_id.clone(), polynomial, relation).map_err(
                    |failure| lowering_geometry(operation, geometry_condition(&failure), failure),
                )?);
                ids.push(atom_id);
            }
            if let Some(inner_radius) = inner_radius {
                let inner = poly_terms(
                    operation,
                    &[
                        (one_scalar()?, 2, 0, 0),
                        (one_scalar()?, 0, 2, 0),
                        (negate(&inner_radius.mul(inner_radius)?)?, 0, 0, 0),
                    ],
                )?;
                let atom_id = format!("{prefix}:cylinder-inner");
                atoms.push(
                    poly_atom(atom_id.clone(), inner, SignRelation::Positive).map_err(
                        |failure| {
                            lowering_geometry(operation, geometry_condition(&failure), failure)
                        },
                    )?,
                );
                ids.push(atom_id);
            }
            Ok(ConstructionOutput::Region(algebraic_region(
                operation,
                atoms,
                formula_and(ids),
                RegionKind::RegularClosedVolume,
            )?))
        }
        PrimitiveDeclaration::Cone {
            base_radius,
            top_radius,
            height,
        } => {
            let slope = top_radius.sub(base_radius)?.div(height)?;
            let two_ab = two_scalar()?.mul(base_radius)?.mul(&slope)?;
            let slope_sq = slope.mul(&slope)?;
            let base_sq = base_radius.mul(base_radius)?;
            let one = one_scalar()?;
            let side = poly_terms(
                operation,
                &[
                    (one.clone(), 2, 0, 0),
                    (one.clone(), 0, 2, 0),
                    (negate(&two_ab)?, 0, 0, 1),
                    (negate(&slope_sq)?, 0, 0, 2),
                    (negate(&base_sq)?, 0, 0, 0),
                ],
            )?;
            let z_poly = poly_terms(operation, &[(one.clone(), 0, 0, 1)])?;
            let height_poly = poly_terms(
                operation,
                &[(height.clone(), 0, 0, 0), (negate(&one)?, 0, 0, 1)],
            )?;
            let mut atoms = Vec::new();
            let mut ids = Vec::new();
            for (name, polynomial, relation) in [
                (format!("{prefix}:cone-side"), side, SignRelation::Negative),
                (
                    format!("{prefix}:cone-bottom"),
                    z_poly,
                    SignRelation::Positive,
                ),
                (
                    format!("{prefix}:cone-top"),
                    height_poly,
                    SignRelation::Positive,
                ),
            ] {
                atoms.push(
                    poly_atom(name.clone(), polynomial, relation).map_err(|failure| {
                        lowering_geometry(operation, geometry_condition(&failure), failure)
                    })?,
                );
                ids.push(name);
            }
            Ok(ConstructionOutput::Region(algebraic_region(
                operation,
                atoms,
                formula_and(ids),
                RegionKind::RegularClosedVolume,
            )?))
        }
        PrimitiveDeclaration::Torus {
            major_radius,
            minor_radius,
        } => {
            let major_sq = major_radius.mul(major_radius)?;
            let minor_sq = minor_radius.mul(minor_radius)?;
            let shift = major_sq.sub(&minor_sq)?;
            let shift_sq = shift.mul(&shift)?;
            let two_shift = two_scalar()?.mul(&shift)?;
            let four_major = ExactScalar::new(4.)?.mul(&major_sq)?;
            let x2 = two_shift.sub(&four_major)?;
            let one = one_scalar()?;
            let two = two_scalar()?;
            let polynomial = poly_terms(
                operation,
                &[
                    (one.clone(), 4, 0, 0),
                    (one.clone(), 0, 4, 0),
                    (one.clone(), 0, 0, 4),
                    (two.clone(), 2, 2, 0),
                    (two.clone(), 2, 0, 2),
                    (two, 0, 2, 2),
                    (x2.clone(), 2, 0, 0),
                    (x2, 0, 2, 0),
                    (two_shift, 0, 0, 2),
                    (shift_sq, 0, 0, 0),
                ],
            )?;
            let atom_id = format!("{prefix}:torus");
            let atom = poly_atom(atom_id.clone(), polynomial, SignRelation::Negative).map_err(
                |failure| lowering_geometry(operation, geometry_condition(&failure), failure),
            )?;
            Ok(ConstructionOutput::Region(algebraic_region(
                operation,
                vec![atom],
                NormalizedRegionFormula::Atom(atom_id),
                RegionKind::RegularClosedVolume,
            )?))
        }
    }
}

fn dot3(
    left: &[ExactScalar; 3],
    right: &[ExactScalar; 3],
) -> Result<ExactScalar, CadConstructionFailure> {
    left[0]
        .mul(&right[0])?
        .add(&left[1].mul(&right[1])?)?
        .add(&left[2].mul(&right[2])?)
}

fn cross3(
    left: &[ExactScalar; 3],
    right: &[ExactScalar; 3],
) -> Result<[ExactScalar; 3], CadConstructionFailure> {
    Ok([
        left[1].mul(&right[2])?.sub(&left[2].mul(&right[1])?)?,
        left[2].mul(&right[0])?.sub(&left[0].mul(&right[2])?)?,
        left[0].mul(&right[1])?.sub(&left[1].mul(&right[0])?)?,
    ])
}

fn is_one(value: &ExactScalar) -> Result<bool, CadConstructionFailure> {
    Ok(value.compare_exact(&one_scalar()?)? == Ordering::Equal)
}

fn is_zero_value(value: &ExactScalar) -> bool {
    value.is_zero()
}

fn coordinate_poly(
    operation: &crate::OperationIdentity,
    axis: &[ExactScalar; 3],
    origin: &ExactPoint3,
) -> Result<RegionPolynomial3, CadConstructionFailure> {
    let shift = dot3(axis, &origin.0)?;
    let mut terms = Vec::new();
    push_term(&mut terms, axis[0].clone(), 1, 0, 0);
    push_term(&mut terms, axis[1].clone(), 0, 1, 0);
    push_term(&mut terms, axis[2].clone(), 0, 0, 1);
    push_term(&mut terms, negate(&shift)?, 0, 0, 0);
    if terms.is_empty() {
        poly_terms(operation, &[(zero_scalar()?, 0, 0, 0)])
    } else {
        poly_terms(operation, &terms)
    }
}

fn embed_uv(
    operation: &crate::OperationIdentity,
    uv: RegionPolynomial3,
    u: RegionPolynomial3,
    v: RegionPolynomial3,
) -> Result<RegionPolynomial3, CadConstructionFailure> {
    let map = RegionAffineMap3::try_new(u, v, RegionPolynomial3::zero())
        .map_err(|failure| lowering_geometry(operation, geometry_condition(&failure), failure))?;
    uv.try_affine_substitute(&map)
        .map_err(|failure| lowering_geometry(operation, geometry_condition(&failure), failure))
}

#[derive(Clone)]
struct UvLinear {
    alpha: ExactScalar,
    beta: ExactScalar,
    gamma: ExactScalar,
}

struct UvAtom {
    id: String,
    poly: RegionPolynomial3,
    relation: SignRelation,
    linear: Option<UvLinear>,
}

struct ProfileLowering {
    region: AlgebraicRegion,
    uv_atoms: Vec<UvAtom>,
    origin: ExactPoint3,
    u_axis: [ExactScalar; 3],
    v_axis: [ExactScalar; 3],
}

fn corner_map(profile: &ProfileRequest) -> BTreeMap<&str, &ProfileCorner> {
    profile
        .corners
        .iter()
        .map(|corner| (corner.id.as_str(), corner))
        .collect()
}

fn segment_map(profile: &ProfileRequest) -> BTreeMap<&str, &ProfileSegment> {
    profile
        .segments
        .iter()
        .map(|segment| (segment.id.as_str(), segment))
        .collect()
}

fn orient2(
    a: &[ExactScalar; 2],
    b: &[ExactScalar; 2],
    c: &[ExactScalar; 2],
) -> Result<Ordering, CadConstructionFailure> {
    let bx = b[0].sub(&a[0])?;
    let by = b[1].sub(&a[1])?;
    let cx = c[0].sub(&a[0])?;
    let cy = c[1].sub(&a[1])?;
    bx.mul(&cy)?
        .sub(&by.mul(&cx)?)?
        .compare_exact(&zero_scalar()?)
}

fn segments_intersect(
    a: &[ExactScalar; 2],
    b: &[ExactScalar; 2],
    c: &[ExactScalar; 2],
    d: &[ExactScalar; 2],
) -> Result<bool, CadConstructionFailure> {
    let abc = orient2(a, b, c)?;
    let abd = orient2(a, b, d)?;
    let cda = orient2(c, d, a)?;
    let cdb = orient2(c, d, b)?;
    Ok(abc != abd && cda != cdb && abc != Ordering::Equal && abd != Ordering::Equal)
}

fn collect_cycles<'a>(
    profile: &'a ProfileRequest,
    operation: &crate::OperationIdentity,
) -> Result<Vec<Vec<&'a ProfileSegment>>, CadConstructionFailure> {
    let segments = segment_map(profile);
    let mut remaining: Vec<&str> = profile.segment_order.iter().map(String::as_str).collect();
    let mut cycles = Vec::new();
    while !remaining.is_empty() {
        let first = *segments
            .get(remaining[0])
            .ok_or_else(|| lowering(operation, LoweringCondition::OrderMismatch))?;
        let mut cycle = vec![first];
        remaining.remove(0);
        let start = first.start_corner.as_str();
        let mut current = first.end_corner.as_str();
        while current != start {
            if remaining.is_empty() {
                return Err(lowering(operation, LoweringCondition::DisconnectedProfile));
            }
            let next = *segments
                .get(remaining[0])
                .ok_or_else(|| lowering(operation, LoweringCondition::OrderMismatch))?;
            if next.start_corner != current {
                let later_connects = remaining.iter().any(|id| {
                    segments
                        .get(*id)
                        .is_some_and(|segment| segment.start_corner == current)
                });
                return Err(lowering(
                    operation,
                    if later_connects {
                        LoweringCondition::OrderMismatch
                    } else {
                        LoweringCondition::DisconnectedProfile
                    },
                ));
            }
            remaining.remove(0);
            current = next.end_corner.as_str();
            cycle.push(next);
        }
        cycles.push(cycle);
    }
    Ok(cycles)
}

fn point_in_cycle(
    point: &[ExactScalar; 2],
    cycle: &[&ProfileSegment],
    corners: &BTreeMap<&str, &ProfileCorner>,
) -> Result<bool, CadConstructionFailure> {
    let operation = coefficient_operation()?;
    let mut winding = 0i32;
    for segment in cycle {
        let start = &corners
            .get(segment.start_corner.as_str())
            .ok_or_else(|| lowering(&operation, LoweringCondition::OrderMismatch))?
            .position;
        let end = &corners
            .get(segment.end_corner.as_str())
            .ok_or_else(|| lowering(&operation, LoweringCondition::OrderMismatch))?
            .position;
        if start[1].compare_exact(&point[1])? == Ordering::Greater
            && end[1].compare_exact(&point[1])? != Ordering::Greater
            || end[1].compare_exact(&point[1])? == Ordering::Greater
                && start[1].compare_exact(&point[1])? != Ordering::Greater
        {
            if orient2(start, end, point)? == Ordering::Greater {
                winding += 1;
            } else {
                winding -= 1;
            }
        }
    }
    Ok(winding != 0)
}

fn uv_linear_poly(
    operation: &crate::OperationIdentity,
    linear: &UvLinear,
) -> Result<RegionPolynomial3, CadConstructionFailure> {
    let mut terms = Vec::new();
    push_term(&mut terms, linear.alpha.clone(), 1, 0, 0);
    push_term(&mut terms, linear.beta.clone(), 0, 1, 0);
    push_term(&mut terms, linear.gamma.clone(), 0, 0, 0);
    if terms.is_empty() {
        poly_terms(operation, &[(zero_scalar()?, 0, 0, 0)])
    } else {
        poly_terms(operation, &terms)
    }
}

fn lower_profile_data(
    operation: &crate::OperationIdentity,
    profile: &ProfileRequest,
) -> Result<ProfileLowering, CadConstructionFailure> {
    if profile.closure != GeometryClosure::Closed {
        return Err(lowering(operation, LoweringCondition::OpenProfile));
    }
    let x_len = dot3(&profile.x_axis.0, &profile.x_axis.0)?;
    let n_len = dot3(&profile.normal.0, &profile.normal.0)?;
    let xn = dot3(&profile.x_axis.0, &profile.normal.0)?;
    if !is_one(&x_len)? || !is_one(&n_len)? || !is_zero_value(&xn) {
        return Err(lowering(operation, LoweringCondition::NonOrthonormalFrame));
    }
    let y_axis = cross3(&profile.normal.0, &profile.x_axis.0)?;
    if !is_one(&dot3(&y_axis, &y_axis)?)? {
        return Err(lowering(operation, LoweringCondition::NonOrthonormalFrame));
    }
    if profile
        .segments
        .iter()
        .any(|segment| matches!(segment.curve, ProfileSegmentCurve::Nurbs { .. }))
    {
        return Err(lowering(operation, LoweringCondition::GeneralNurbsSegment));
    }
    let corners = corner_map(profile);
    let cycles = collect_cycles(profile, operation)?;
    if cycles.is_empty() {
        return Err(lowering(operation, LoweringCondition::DisconnectedProfile));
    }
    if cycles.len() > 1 {
        let first = &cycles[0];
        let probe = &corners
            .get(cycles[1][0].start_corner.as_str())
            .ok_or_else(|| lowering(operation, LoweringCondition::OrderMismatch))?
            .position;
        return Err(if point_in_cycle(probe, first, &corners)? {
            lowering(operation, LoweringCondition::ProfileHole)
        } else {
            lowering(operation, LoweringCondition::DisconnectedProfile)
        });
    }
    let cycle = &cycles[0];
    if cycle.len() < 3
        && !cycle
            .iter()
            .all(|segment| matches!(segment.curve, ProfileSegmentCurve::CircularArc { .. }))
    {
        return Err(lowering(operation, LoweringCondition::DisconnectedProfile));
    }
    for (index, segment) in cycle.iter().enumerate() {
        let next = cycle[(index + 1) % cycle.len()];
        if segment.end_corner != next.start_corner {
            return Err(lowering(operation, LoweringCondition::OrderMismatch));
        }
    }
    for (left_index, left) in cycle.iter().enumerate() {
        for right in cycle.iter().skip(left_index + 2) {
            if left.start_corner == right.end_corner || left.end_corner == right.start_corner {
                continue;
            }
            if matches!(left.curve, ProfileSegmentCurve::Line)
                && matches!(right.curve, ProfileSegmentCurve::Line)
            {
                let a = &corners[left.start_corner.as_str()].position;
                let b = &corners[left.end_corner.as_str()].position;
                let c = &corners[right.start_corner.as_str()].position;
                let d = &corners[right.end_corner.as_str()].position;
                if segments_intersect(a, b, c, d)? {
                    return Err(lowering(operation, LoweringCondition::SelfIntersection));
                }
            }
        }
    }
    let all_arcs = cycle
        .iter()
        .all(|segment| matches!(segment.curve, ProfileSegmentCurve::CircularArc { .. }));
    let mut uv_atoms = Vec::new();
    if all_arcs {
        let ProfileSegmentCurve::CircularArc { center, radius, .. } = &cycle[0].curve else {
            return Err(lowering(
                operation,
                LoweringCondition::ImplicitizationFailed,
            ));
        };
        for segment in cycle {
            let ProfileSegmentCurve::CircularArc {
                center: other_center,
                radius: other_radius,
                ..
            } = &segment.curve
            else {
                return Err(lowering(
                    operation,
                    LoweringCondition::ImplicitizationFailed,
                ));
            };
            if other_center[0].compare_exact(&center[0])? != Ordering::Equal
                || other_center[1].compare_exact(&center[1])? != Ordering::Equal
                || other_radius.compare_exact(radius)? != Ordering::Equal
            {
                return Err(lowering(
                    operation,
                    LoweringCondition::ImplicitizationFailed,
                ));
            }
            for corner_id in [&segment.start_corner, &segment.end_corner] {
                let position = &corners[corner_id.as_str()].position;
                let dx = position[0].sub(&center[0])?;
                let dy = position[1].sub(&center[1])?;
                let on_circle = dx
                    .mul(&dx)?
                    .add(&dy.mul(&dy)?)?
                    .compare_exact(&radius.mul(radius)?)?;
                if on_circle != Ordering::Equal {
                    return Err(lowering(
                        operation,
                        LoweringCondition::ImplicitizationFailed,
                    ));
                }
            }
        }
        let two_cx = two_scalar()?.mul(&center[0])?;
        let two_cy = two_scalar()?.mul(&center[1])?;
        let constant = center[0]
            .mul(&center[0])?
            .add(&center[1].mul(&center[1])?)?
            .sub(&radius.mul(radius)?)?;
        let poly = poly_terms(
            operation,
            &[
                (one_scalar()?, 2, 0, 0),
                (one_scalar()?, 0, 2, 0),
                (negate(&two_cx)?, 1, 0, 0),
                (negate(&two_cy)?, 0, 1, 0),
                (constant, 0, 0, 0),
            ],
        )?;
        uv_atoms.push(UvAtom {
            id: format!("{}:disk", operation.as_str()),
            poly,
            relation: SignRelation::Negative,
            linear: None,
        });
    } else {
        let mut turn = None;
        for index in 0..cycle.len() {
            if !matches!(cycle[index].curve, ProfileSegmentCurve::Line)
                || !matches!(
                    cycle[(index + 1) % cycle.len()].curve,
                    ProfileSegmentCurve::Line
                )
            {
                continue;
            }
            let a = &corners[cycle[index].start_corner.as_str()].position;
            let b = &corners[cycle[index].end_corner.as_str()].position;
            let c = &corners[cycle[(index + 1) % cycle.len()].end_corner.as_str()].position;
            let sign = orient2(a, b, c)?;
            if sign == Ordering::Equal {
                continue;
            }
            match turn {
                None => turn = Some(sign),
                Some(expected) if expected != sign => {
                    return Err(lowering(
                        operation,
                        LoweringCondition::ImplicitizationFailed,
                    ));
                }
                Some(_) => {}
            }
        }
        for (index, segment) in cycle.iter().enumerate() {
            match &segment.curve {
                ProfileSegmentCurve::Line => {
                    let start = &corners[segment.start_corner.as_str()].position;
                    let end = &corners[segment.end_corner.as_str()].position;
                    let dx = end[0].sub(&start[0])?;
                    let dy = end[1].sub(&start[1])?;
                    let inward_x = negate(&dy)?;
                    let inward_y = dx;
                    let gamma = negate(&inward_x.mul(&start[0])?.add(&inward_y.mul(&start[1])?)?)?;
                    let linear = UvLinear {
                        alpha: inward_x,
                        beta: inward_y,
                        gamma,
                    };
                    let poly = uv_linear_poly(operation, &linear)?;
                    uv_atoms.push(UvAtom {
                        id: format!("{}:edge-{index}", operation.as_str()),
                        poly,
                        relation: SignRelation::Positive,
                        linear: Some(linear),
                    });
                }
                ProfileSegmentCurve::CircularArc { center, radius, .. } => {
                    let start = &corners[segment.start_corner.as_str()].position;
                    let end = &corners[segment.end_corner.as_str()].position;
                    let start_d = start[0]
                        .sub(&center[0])?
                        .mul(&start[0].sub(&center[0])?)?
                        .add(&start[1].sub(&center[1])?.mul(&start[1].sub(&center[1])?)?)?;
                    let end_d = end[0]
                        .sub(&center[0])?
                        .mul(&end[0].sub(&center[0])?)?
                        .add(&end[1].sub(&center[1])?.mul(&end[1].sub(&center[1])?)?)?;
                    let radius_sq = radius.mul(radius)?;
                    if start_d.compare_exact(&radius_sq)? != Ordering::Equal
                        || end_d.compare_exact(&radius_sq)? != Ordering::Equal
                    {
                        return Err(lowering(
                            operation,
                            LoweringCondition::ImplicitizationFailed,
                        ));
                    }
                    let two_cx = two_scalar()?.mul(&center[0])?;
                    let two_cy = two_scalar()?.mul(&center[1])?;
                    let constant = center[0]
                        .mul(&center[0])?
                        .add(&center[1].mul(&center[1])?)?
                        .sub(&radius_sq)?;
                    let poly = poly_terms(
                        operation,
                        &[
                            (one_scalar()?, 2, 0, 0),
                            (one_scalar()?, 0, 2, 0),
                            (negate(&two_cx)?, 1, 0, 0),
                            (negate(&two_cy)?, 0, 1, 0),
                            (constant, 0, 0, 0),
                        ],
                    )?;
                    let toward_center = orient2(start, end, center)?;
                    let relation = if toward_center == Ordering::Greater {
                        SignRelation::Negative
                    } else {
                        SignRelation::Positive
                    };
                    uv_atoms.push(UvAtom {
                        id: format!("{}:arc-{index}", operation.as_str()),
                        poly,
                        relation,
                        linear: None,
                    });
                }
                ProfileSegmentCurve::Nurbs { .. } => {
                    return Err(lowering(operation, LoweringCondition::GeneralNurbsSegment));
                }
            }
        }
    }
    if uv_atoms.is_empty() {
        return Err(lowering(
            operation,
            LoweringCondition::ImplicitizationFailed,
        ));
    }
    let u = coordinate_poly(operation, &profile.x_axis.0, &profile.origin)?;
    let v = coordinate_poly(operation, &y_axis, &profile.origin)?;
    let mut atoms = Vec::new();
    let mut ids = Vec::new();
    for atom in &uv_atoms {
        let embedded = embed_uv(
            operation,
            atom.poly.try_clone().map_err(|failure| {
                lowering_geometry(operation, geometry_condition(&failure), failure)
            })?,
            u.try_clone().map_err(|failure| {
                lowering_geometry(operation, geometry_condition(&failure), failure)
            })?,
            v.try_clone().map_err(|failure| {
                lowering_geometry(operation, geometry_condition(&failure), failure)
            })?,
        )?;
        atoms.push(
            poly_atom(atom.id.clone(), embedded, atom.relation).map_err(|failure| {
                lowering_geometry(operation, geometry_condition(&failure), failure)
            })?,
        );
        ids.push(atom.id.clone());
    }
    let formula = if ids.len() == 1 {
        NormalizedRegionFormula::Atom(ids[0].clone())
    } else {
        formula_and(ids)
    };
    let region = algebraic_region(
        operation,
        atoms,
        formula.clone(),
        RegionKind::GeneralSemiAlgebraicSet,
    )?;
    Ok(ProfileLowering {
        region,
        uv_atoms,
        origin: profile.origin.clone(),
        u_axis: profile.x_axis.0.clone(),
        v_axis: y_axis,
    })
}

fn rewrite_formula(formula: &NormalizedRegionFormula, prefix: &str) -> NormalizedRegionFormula {
    match formula {
        NormalizedRegionFormula::Atom(atom) => {
            NormalizedRegionFormula::Atom(format!("{prefix}:{atom}"))
        }
        NormalizedRegionFormula::Not(inner) => {
            NormalizedRegionFormula::Not(Box::new(rewrite_formula(inner, prefix)))
        }
        NormalizedRegionFormula::And(operands) => NormalizedRegionFormula::And(
            operands
                .iter()
                .map(|operand| rewrite_formula(operand, prefix))
                .collect(),
        ),
        NormalizedRegionFormula::Or(operands) => NormalizedRegionFormula::Or(
            operands
                .iter()
                .map(|operand| rewrite_formula(operand, prefix))
                .collect(),
        ),
        NormalizedRegionFormula::True => NormalizedRegionFormula::True,
        NormalizedRegionFormula::False => NormalizedRegionFormula::False,
    }
}

fn prefix_atoms(
    operation: &crate::OperationIdentity,
    region: &AlgebraicRegion,
    prefix: &str,
) -> Result<(Vec<CanonicalAtom>, NormalizedRegionFormula), CadConstructionFailure> {
    let mut atoms = Vec::new();
    for atom in region.canonical_atoms() {
        let function = atom.function().try_clone().map_err(|failure| {
            lowering_geometry(operation, geometry_condition(&failure), failure)
        })?;
        atoms.push(
            CanonicalAtom::try_new(
                format!("{prefix}:{}", atom.atom_id()),
                function,
                atom.relation(),
            )
            .map_err(|failure| {
                lowering_geometry(operation, geometry_condition(&failure), failure)
            })?,
        );
    }
    Ok((
        atoms,
        rewrite_formula(region.normalized_region_formula(), prefix),
    ))
}

fn lower_extrude(
    operation: &crate::OperationIdentity,
    profile_region: &AlgebraicRegion,
    profile: &ProfileRequest,
    direction: &ExactVector3,
    distance: &ExactScalar,
    symmetric: bool,
) -> Result<ConstructionOutput, CadConstructionFailure> {
    let t = coordinate_poly(operation, &direction.0, &profile.origin)?;
    let (mut atoms, mut formula) = prefix_atoms(operation, profile_region, operation.as_str())?;
    let slabs = if symmetric {
        let half = distance.div(&two_scalar()?)?;
        let lower = t
            .try_add(&poly_terms(operation, &[(half.clone(), 0, 0, 0)])?)
            .map_err(|failure| {
                lowering_geometry(operation, geometry_condition(&failure), failure)
            })?;
        let upper = poly_terms(operation, &[(half, 0, 0, 0)])?
            .try_add(
                &t.try_scale(&negate(&one_scalar()?)?.to_real_algebraic()?)
                    .map_err(|failure| {
                        lowering_geometry(operation, geometry_condition(&failure), failure)
                    })?,
            )
            .map_err(|failure| {
                lowering_geometry(operation, geometry_condition(&failure), failure)
            })?;
        vec![
            (format!("{}:extrude-lower", operation.as_str()), lower),
            (format!("{}:extrude-upper", operation.as_str()), upper),
        ]
    } else {
        let upper = poly_terms(operation, &[(distance.clone(), 0, 0, 0)])?
            .try_add(
                &t.try_scale(&negate(&one_scalar()?)?.to_real_algebraic()?)
                    .map_err(|failure| {
                        lowering_geometry(operation, geometry_condition(&failure), failure)
                    })?,
            )
            .map_err(|failure| {
                lowering_geometry(operation, geometry_condition(&failure), failure)
            })?;
        vec![
            (format!("{}:extrude-lower", operation.as_str()), t),
            (format!("{}:extrude-upper", operation.as_str()), upper),
        ]
    };
    let mut slab_formula = Vec::new();
    for (atom_id, polynomial) in slabs {
        atoms.push(
            poly_atom(atom_id.clone(), polynomial, SignRelation::Positive).map_err(|failure| {
                lowering_geometry(operation, geometry_condition(&failure), failure)
            })?,
        );
        slab_formula.push(NormalizedRegionFormula::Atom(atom_id));
    }
    let mut operands = vec![formula];
    operands.extend(slab_formula);
    formula = NormalizedRegionFormula::And(operands);
    let _ = profile;
    Ok(ConstructionOutput::Region(algebraic_region(
        operation,
        atoms,
        formula,
        RegionKind::RegularClosedVolume,
    )?))
}

fn squared_norm_poly(
    operation: &crate::OperationIdentity,
    origin: &ExactPoint3,
) -> Result<RegionPolynomial3, CadConstructionFailure> {
    let mut terms = Vec::new();
    let one = one_scalar()?;
    push_term(&mut terms, one.clone(), 2, 0, 0);
    push_term(&mut terms, one.clone(), 0, 2, 0);
    push_term(&mut terms, one, 0, 0, 2);
    push_term(
        &mut terms,
        negate(&two_scalar()?.mul(&origin.0[0])?)?,
        1,
        0,
        0,
    );
    push_term(
        &mut terms,
        negate(&two_scalar()?.mul(&origin.0[1])?)?,
        0,
        1,
        0,
    );
    push_term(
        &mut terms,
        negate(&two_scalar()?.mul(&origin.0[2])?)?,
        0,
        0,
        1,
    );
    let origin_sq = origin.0[0]
        .mul(&origin.0[0])?
        .add(&origin.0[1].mul(&origin.0[1])?)?
        .add(&origin.0[2].mul(&origin.0[2])?)?;
    push_term(&mut terms, origin_sq, 0, 0, 0);
    poly_terms(operation, &terms)
}

fn axial_poly(
    operation: &crate::OperationIdentity,
    origin: &ExactPoint3,
    axis: &[ExactScalar; 3],
) -> Result<RegionPolynomial3, CadConstructionFailure> {
    coordinate_poly(operation, axis, origin)
}

fn multiply_repeat(
    operation: &crate::OperationIdentity,
    base: &RegionPolynomial3,
    exponent: u32,
) -> Result<RegionPolynomial3, CadConstructionFailure> {
    if exponent == 0 {
        return poly_terms(operation, &[(one_scalar()?, 0, 0, 0)]);
    }
    let mut result = base
        .try_clone()
        .map_err(|failure| lowering_geometry(operation, geometry_condition(&failure), failure))?;
    for _ in 1..exponent {
        result = result.try_multiply(base).map_err(|failure| {
            lowering_geometry(operation, geometry_condition(&failure), failure)
        })?;
    }
    Ok(result)
}

fn revolve_even(
    operation: &crate::OperationIdentity,
    uv: &RegionPolynomial3,
    u_is_radial: bool,
    radius_sq: &RegionPolynomial3,
    axial: &RegionPolynomial3,
) -> Result<RegionPolynomial3, CadConstructionFailure> {
    let mut result = RegionPolynomial3::zero();
    for term in uv.terms() {
        let (radial_exp, axial_exp) = if u_is_radial {
            (term.exponent_x(), term.exponent_y())
        } else {
            (term.exponent_y(), term.exponent_x())
        };
        if radial_exp % 2 == 1 || term.exponent_z() != 0 {
            return Err(lowering(
                operation,
                LoweringCondition::ImplicitizationFailed,
            ));
        }
        let coefficient = ExactScalar::new(0.)
            .and_then(|_| ExactScalar::from_f64_exact_binary(0., "revolve").map(|_| ()));
        let _ = coefficient;
        let constant = RegionPolynomial3::try_from_terms(vec![RegionPolynomialTerm::try_new(
            term.coefficient()
                .try_clone()
                .map_err(|_| lowering(operation, LoweringCondition::UnconvertibleCoefficient))?,
            0,
            0,
            0,
        )
        .map_err(|failure| lowering_geometry(operation, geometry_condition(&failure), failure))?])
        .map_err(|failure| lowering_geometry(operation, geometry_condition(&failure), failure))?;
        let radial = multiply_repeat(operation, radius_sq, radial_exp / 2)?;
        let axial_power = multiply_repeat(operation, axial, axial_exp)?;
        let piece = constant
            .try_multiply(&radial)
            .map_err(|failure| lowering_geometry(operation, geometry_condition(&failure), failure))?
            .try_multiply(&axial_power)
            .map_err(|failure| {
                lowering_geometry(operation, geometry_condition(&failure), failure)
            })?;
        result = result.try_add(&piece).map_err(|failure| {
            lowering_geometry(operation, geometry_condition(&failure), failure)
        })?;
    }
    Ok(result)
}

fn lower_revolve(
    operation: &crate::OperationIdentity,
    profile: &ProfileRequest,
    axis_origin: &ExactPoint3,
    axis_direction: &ExactVector3,
    extent: &RevolveExtent,
) -> Result<ConstructionOutput, CadConstructionFailure> {
    match extent {
        RevolveExtent::FiniteAngle(_) => {
            return Err(lowering(operation, LoweringCondition::FiniteRevolveAngle));
        }
        RevolveExtent::Full => {}
    }
    let lowered = lower_profile_data(operation, profile)?;
    if !is_one(&dot3(&axis_direction.0, &axis_direction.0)?)? {
        return Err(lowering(
            operation,
            LoweringCondition::ImplicitizationFailed,
        ));
    }
    let u_dot = dot3(&lowered.u_axis, &axis_direction.0)?;
    let v_dot = dot3(&lowered.v_axis, &axis_direction.0)?;
    let u_is_radial = is_zero_value(&u_dot) && is_one(&v_dot.mul(&v_dot)?)?;
    let v_is_radial = is_zero_value(&v_dot) && is_one(&u_dot.mul(&u_dot)?)?;
    if !u_is_radial && !v_is_radial {
        return Err(lowering(
            operation,
            LoweringCondition::ImplicitizationFailed,
        ));
    }
    let axial = axial_poly(operation, axis_origin, &axis_direction.0)?;
    let r2 = squared_norm_poly(operation, axis_origin)?
        .try_add(
            &axial
                .try_multiply(&axial)
                .map_err(|failure| {
                    lowering_geometry(operation, geometry_condition(&failure), failure)
                })?
                .try_scale(&negate(&one_scalar()?)?.to_real_algebraic()?)
                .map_err(|failure| {
                    lowering_geometry(operation, geometry_condition(&failure), failure)
                })?,
        )
        .map_err(|failure| lowering_geometry(operation, geometry_condition(&failure), failure))?;
    let mut atoms = Vec::new();
    let mut ids = Vec::new();
    for atom in &lowered.uv_atoms {
        if let Some(linear) = &atom.linear {
            let radial = if u_is_radial {
                &linear.alpha
            } else {
                &linear.beta
            };
            let axial_coeff = if u_is_radial {
                &linear.beta
            } else {
                &linear.alpha
            };
            if is_zero_value(axial_coeff) {
                if is_zero_value(radial) {
                    continue;
                }
                let threshold = negate(&linear.gamma)?.div(radial)?;
                if radial.compare_exact(&zero_scalar()?)? == Ordering::Greater {
                    if threshold.compare_exact(&zero_scalar()?)? != Ordering::Greater {
                        continue;
                    }
                    let threshold_sq = threshold.mul(&threshold)?;
                    let poly = r2
                        .try_add(&poly_terms(
                            operation,
                            &[(negate(&threshold_sq)?, 0, 0, 0)],
                        )?)
                        .map_err(|failure| {
                            lowering_geometry(operation, geometry_condition(&failure), failure)
                        })?;
                    let atom_id = format!("{}:revolve-{}", operation.as_str(), atom.id);
                    atoms.push(
                        poly_atom(atom_id.clone(), poly, SignRelation::Positive).map_err(
                            |failure| {
                                lowering_geometry(operation, geometry_condition(&failure), failure)
                            },
                        )?,
                    );
                    ids.push(atom_id);
                } else {
                    let threshold_sq = threshold.mul(&threshold)?;
                    let poly = r2
                        .try_add(&poly_terms(
                            operation,
                            &[(negate(&threshold_sq)?, 0, 0, 0)],
                        )?)
                        .map_err(|failure| {
                            lowering_geometry(operation, geometry_condition(&failure), failure)
                        })?;
                    let atom_id = format!("{}:revolve-{}", operation.as_str(), atom.id);
                    atoms.push(
                        poly_atom(atom_id.clone(), poly, SignRelation::Negative).map_err(
                            |failure| {
                                lowering_geometry(operation, geometry_condition(&failure), failure)
                            },
                        )?,
                    );
                    ids.push(atom_id);
                }
                continue;
            }
            if is_zero_value(radial) {
                let poly = embed_uv(
                    operation,
                    atom.poly.try_clone().map_err(|failure| {
                        lowering_geometry(operation, geometry_condition(&failure), failure)
                    })?,
                    coordinate_poly(operation, &lowered.u_axis, &lowered.origin)?,
                    axial.try_clone().map_err(|failure| {
                        lowering_geometry(operation, geometry_condition(&failure), failure)
                    })?,
                )?;
                let atom_id = format!("{}:revolve-{}", operation.as_str(), atom.id);
                atoms.push(
                    poly_atom(atom_id.clone(), poly, atom.relation).map_err(|failure| {
                        lowering_geometry(operation, geometry_condition(&failure), failure)
                    })?,
                );
                ids.push(atom_id);
                continue;
            }
            return Err(lowering(
                operation,
                LoweringCondition::ImplicitizationFailed,
            ));
        }
        let revolved = revolve_even(operation, &atom.poly, u_is_radial, &r2, &axial)?;
        let atom_id = format!("{}:revolve-{}", operation.as_str(), atom.id);
        atoms.push(
            poly_atom(atom_id.clone(), revolved, atom.relation).map_err(|failure| {
                lowering_geometry(operation, geometry_condition(&failure), failure)
            })?,
        );
        ids.push(atom_id);
    }
    if atoms.is_empty() {
        return Err(lowering(
            operation,
            LoweringCondition::ImplicitizationFailed,
        ));
    }
    let formula = if ids.len() == 1 {
        NormalizedRegionFormula::Atom(ids[0].clone())
    } else {
        formula_and(ids)
    };
    Ok(ConstructionOutput::Region(algebraic_region(
        operation,
        atoms,
        formula,
        RegionKind::RegularClosedVolume,
    )?))
}

fn rotation_matrix(
    quaternion: &[ExactScalar; 4],
) -> Result<[[ExactScalar; 3]; 3], CadConstructionFailure> {
    let w = &quaternion[0];
    let x = &quaternion[1];
    let y = &quaternion[2];
    let z = &quaternion[3];
    let two = two_scalar()?;
    let xx = x.mul(x)?;
    let yy = y.mul(y)?;
    let zz = z.mul(z)?;
    let xy = x.mul(y)?;
    let xz = x.mul(z)?;
    let yz = y.mul(z)?;
    let wx = w.mul(x)?;
    let wy = w.mul(y)?;
    let wz = w.mul(z)?;
    Ok([
        [
            one_scalar()?.sub(&two.mul(&yy.add(&zz)?)?)?,
            two.mul(&xy.sub(&wz)?)?,
            two.mul(&xz.add(&wy)?)?,
        ],
        [
            two.mul(&xy.add(&wz)?)?,
            one_scalar()?.sub(&two.mul(&xx.add(&zz)?)?)?,
            two.mul(&yz.sub(&wx)?)?,
        ],
        [
            two.mul(&xz.sub(&wy)?)?,
            two.mul(&yz.add(&wx)?)?,
            one_scalar()?.sub(&two.mul(&xx.add(&yy)?)?)?,
        ],
    ])
}

fn lower_transform(
    operation: &crate::OperationIdentity,
    source: &AlgebraicRegion,
    placement: &ExactRigidPlacement,
    scale: &ExactScalar,
) -> Result<ConstructionOutput, CadConstructionFailure> {
    if !positive(scale)? {
        return Err(lowering(operation, LoweringCondition::NonPositiveScale));
    }
    let rotation = rotation_matrix(&placement.quaternion)?;
    let translation = &placement.origin.0;
    let inv_scale = one_scalar()?.div(scale)?;
    let mut axes = Vec::new();
    let columns = [
        [
            rotation[0][0].clone(),
            rotation[1][0].clone(),
            rotation[2][0].clone(),
        ],
        [
            rotation[0][1].clone(),
            rotation[1][1].clone(),
            rotation[2][1].clone(),
        ],
        [
            rotation[0][2].clone(),
            rotation[1][2].clone(),
            rotation[2][2].clone(),
        ],
    ];
    for column in columns {
        let axis = [
            column[0].mul(&inv_scale)?,
            column[1].mul(&inv_scale)?,
            column[2].mul(&inv_scale)?,
        ];
        let shift = axis[0]
            .mul(&translation[0])?
            .add(&axis[1].mul(&translation[1])?)?
            .add(&axis[2].mul(&translation[2])?)?;
        let mut terms = Vec::new();
        push_term(&mut terms, axis[0].clone(), 1, 0, 0);
        push_term(&mut terms, axis[1].clone(), 0, 1, 0);
        push_term(&mut terms, axis[2].clone(), 0, 0, 1);
        push_term(&mut terms, negate(&shift)?, 0, 0, 0);
        axes.push(poly_terms(operation, &terms)?);
    }
    let map = RegionAffineMap3::try_new(axes.remove(0), axes.remove(0), axes.remove(0))
        .map_err(|failure| lowering_geometry(operation, geometry_condition(&failure), failure))?;
    let mut atoms = Vec::new();
    for atom in source.canonical_atoms() {
        let function = atom
            .function()
            .try_affine_substitute(&map)
            .map_err(|failure| {
                lowering_geometry(operation, geometry_condition(&failure), failure)
            })?;
        atoms.push(
            CanonicalAtom::try_new(String::from(atom.atom_id()), function, atom.relation())
                .map_err(|failure| {
                    lowering_geometry(operation, geometry_condition(&failure), failure)
                })?,
        );
    }
    Ok(ConstructionOutput::Region(algebraic_region(
        operation,
        atoms,
        source.normalized_region_formula().clone(),
        source.region_kind(),
    )?))
}

fn boolean_formula(
    operator: BooleanOperator,
    left: &NormalizedRegionFormula,
    right: &NormalizedRegionFormula,
) -> NormalizedRegionFormula {
    match operator {
        BooleanOperator::Union => NormalizedRegionFormula::Or(vec![left.clone(), right.clone()]),
        BooleanOperator::Intersection => {
            NormalizedRegionFormula::And(vec![left.clone(), right.clone()])
        }
        BooleanOperator::Difference => NormalizedRegionFormula::And(vec![
            left.clone(),
            NormalizedRegionFormula::Not(Box::new(right.clone())),
        ]),
    }
}

fn lower_boolean(
    operation: &crate::OperationIdentity,
    request: &BooleanRequest,
    left: &ConstructionOutput,
    right: &ConstructionOutput,
) -> Result<ConstructionOutput, CadConstructionFailure> {
    let (left_region, right_region) = match (left, right) {
        (ConstructionOutput::ImportedRegion(_), _) | (_, ConstructionOutput::ImportedRegion(_)) => {
            return Err(lowering(operation, LoweringCondition::ImportedBooleanChild));
        }
        (ConstructionOutput::Region(left), ConstructionOutput::Region(right)) => (left, right),
        _ => {
            return Err(lowering(
                operation,
                LoweringCondition::ImplicitizationFailed,
            ))
        }
    };
    let formula = boolean_formula(
        request.operator,
        left_region.normalized_region_formula(),
        right_region.normalized_region_formula(),
    );
    let region = match request.kind {
        BooleanKind::Set => left_region.set_boolean(right_region, formula),
        BooleanKind::Solid => left_region.solid_boolean(right_region, formula),
    }
    .map_err(|failure| lowering_geometry(operation, geometry_condition(&failure), failure))?;
    Ok(ConstructionOutput::Region(region))
}

fn point3(point: &ExactPoint3) -> Result<Point3, CadConstructionFailure> {
    Ok(Point3 {
        x: point.0[0].to_real_algebraic()?,
        y: point.0[1].to_real_algebraic()?,
        z: point.0[2].to_real_algebraic()?,
    })
}

fn point_plus(
    origin: &ExactPoint3,
    direction: &ExactVector3,
    length: &ExactScalar,
) -> Result<ExactPoint3, CadConstructionFailure> {
    Ok(ExactPoint3([
        origin.0[0].add(&direction.0[0].mul(length)?)?,
        origin.0[1].add(&direction.0[1].mul(length)?)?,
        origin.0[2].add(&direction.0[2].mul(length)?)?,
    ]))
}

fn algebraic_knots(knots: &[ExactScalar]) -> Result<Vec<RealAlgebraic>, CadConstructionFailure> {
    knots.iter().map(ExactScalar::to_real_algebraic).collect()
}

fn lower_curve(
    operation: &crate::OperationIdentity,
    curve: &CurveDeclaration,
) -> Result<ConstructionOutput, CadConstructionFailure> {
    let identity = CurveRepresentationId::new(operation.as_str().into()).map_err(|failure| {
        CadConstructionFailure::Lowering {
            operation: operation.clone(),
            condition: LoweringCondition::ImplicitizationFailed,
            field: Some("curve"),
            operand_provenance: None,
            lower: Some(Box::new(CadConstructionLowerFailure::Nurbs(format!(
                "{failure:?}"
            )))),
        }
    })?;
    let value = match curve {
        CurveDeclaration::Line {
            origin,
            direction,
            length,
        } => define_nurbs(NurbsDefinitionRequest::PolynomialCurve {
            control_points: vec![
                BezierPoint::Point3(point3(origin)?),
                BezierPoint::Point3(point3(&point_plus(origin, direction, length)?)?),
            ],
            knots: vec![
                zero_scalar()?.to_real_algebraic()?,
                zero_scalar()?.to_real_algebraic()?,
                one_scalar()?.to_real_algebraic()?,
                one_scalar()?.to_real_algebraic()?,
            ],
            degree: 1,
        }),
        CurveDeclaration::Polyline { points, .. } => {
            let mut knots = vec![
                zero_scalar()?.to_real_algebraic()?,
                zero_scalar()?.to_real_algebraic()?,
            ];
            let last = points.len().saturating_sub(1).max(1);
            for index in 1..last {
                knots.push(ExactScalar::new(index as f64)?.to_real_algebraic()?);
            }
            knots.push(ExactScalar::new(last as f64)?.to_real_algebraic()?);
            knots.push(ExactScalar::new(last as f64)?.to_real_algebraic()?);
            define_nurbs(NurbsDefinitionRequest::PolynomialCurve {
                control_points: points
                    .iter()
                    .map(|point| point3(point).map(BezierPoint::Point3))
                    .collect::<Result<Vec<_>, _>>()?,
                knots,
                degree: 1,
            })
        }
        CurveDeclaration::Nurbs {
            degree,
            knots,
            control_points,
            weights,
            ..
        } => define_nurbs(NurbsDefinitionRequest::RationalCurve {
            control_points: control_points
                .iter()
                .map(|point| point3(point).map(BezierPoint::Point3))
                .collect::<Result<Vec<_>, _>>()?,
            weights: weights
                .iter()
                .map(ExactScalar::to_real_algebraic)
                .collect::<Result<Vec<_>, _>>()?,
            knots: algebraic_knots(knots)?,
            degree: *degree as usize,
        }),
        CurveDeclaration::CircularArc { .. } => {
            return Err(lowering(
                operation,
                LoweringCondition::ImplicitizationFailed,
            ));
        }
    }
    .map_err(|_| CadConstructionFailure::Lowering {
        operation: operation.clone(),
        condition: LoweringCondition::ImplicitizationFailed,
        field: Some("curve"),
        operand_provenance: None,
        lower: Some(Box::new(CadConstructionLowerFailure::Nurbs(String::from(
            "definition",
        )))),
    })?;
    let representation = CurveRepresentation::new(identity, value).map_err(|failure| {
        CadConstructionFailure::Lowering {
            operation: operation.clone(),
            condition: LoweringCondition::ImplicitizationFailed,
            field: Some("curve"),
            operand_provenance: None,
            lower: Some(Box::new(CadConstructionLowerFailure::Nurbs(format!(
                "{failure:?}"
            )))),
        }
    })?;
    Ok(ConstructionOutput::AnalyticRepresentation(
        CurveOrSurfaceRepresentation::Curve(representation),
    ))
}

fn lower_surface(
    operation: &crate::OperationIdentity,
    patch: &NurbsSurfacePatchRequest,
) -> Result<ConstructionOutput, CadConstructionFailure> {
    let identity = SurfaceRepresentationId::new(operation.as_str().into()).map_err(|failure| {
        CadConstructionFailure::Lowering {
            operation: operation.clone(),
            condition: LoweringCondition::ImplicitizationFailed,
            field: Some("surface"),
            operand_provenance: None,
            lower: Some(Box::new(CadConstructionLowerFailure::Nurbs(format!(
                "{failure:?}"
            )))),
        }
    })?;
    let control_net = patch
        .control_net
        .iter()
        .map(|row| {
            row.iter()
                .map(|point| point3(point).map(BezierPoint::Point3))
                .collect::<Result<Vec<_>, _>>()
        })
        .collect::<Result<Vec<_>, _>>()?;
    let weights = patch
        .weight_net
        .iter()
        .map(|row| {
            row.iter()
                .map(ExactScalar::to_real_algebraic)
                .collect::<Result<Vec<_>, _>>()
        })
        .collect::<Result<Vec<_>, _>>()?;
    let value = define_nurbs(NurbsDefinitionRequest::RationalSurface {
        control_net,
        weights,
        u_knots: algebraic_knots(&patch.u_knots)?,
        v_knots: algebraic_knots(&patch.v_knots)?,
        u_degree: patch.u_degree as usize,
        v_degree: patch.v_degree as usize,
    })
    .map_err(|_| CadConstructionFailure::Lowering {
        operation: operation.clone(),
        condition: LoweringCondition::ImplicitizationFailed,
        field: Some("surface"),
        operand_provenance: None,
        lower: Some(Box::new(CadConstructionLowerFailure::Nurbs(String::from(
            "definition",
        )))),
    })?;
    let representation = SurfaceRepresentation::new(identity, value).map_err(|failure| {
        CadConstructionFailure::Lowering {
            operation: operation.clone(),
            condition: LoweringCondition::ImplicitizationFailed,
            field: Some("surface"),
            operand_provenance: None,
            lower: Some(Box::new(CadConstructionLowerFailure::Nurbs(format!(
                "{failure:?}"
            )))),
        }
    })?;
    Ok(ConstructionOutput::AnalyticRepresentation(
        CurveOrSurfaceRepresentation::Surface(representation),
    ))
}

fn exact_to_f64(value: &ExactScalar) -> Result<f64, CadConstructionFailure> {
    for integer in -64i32..=64 {
        let candidate = ExactScalar::new(f64::from(integer))?;
        if value.compare_exact(&candidate)? == Ordering::Equal {
            return Ok(f64::from(integer));
        }
    }
    Err(lowering(
        &coefficient_operation()?,
        LoweringCondition::UnconvertibleCoefficient,
    ))
}

fn import_request() -> Result<RegionImportRequest, CadConstructionFailure> {
    let operation = coefficient_operation()?;
    let enclosure = Enclosure::new(
        Dyadic::new(
            BigInt::try_from(0)
                .map_err(|_| lowering(&operation, LoweringCondition::UnconvertibleCoefficient))?,
            0,
        ),
        Dyadic::new(
            BigInt::try_from(1)
                .map_err(|_| lowering(&operation, LoweringCondition::UnconvertibleCoefficient))?,
            8,
        ),
    )
    .map_err(|_| lowering(&operation, LoweringCondition::UnconvertibleCoefficient))?;
    let isolation = RootIsolationPolicy::new(8, RootRefinementMethod::Bisection, enclosure)
        .map_err(|failure| lowering_geometry(&operation, geometry_condition(&failure), failure))?;
    let policy = neco_geometry::RayCrossingPolicy::new(isolation, 64, 64)
        .map_err(|failure| lowering_geometry(&operation, geometry_condition(&failure), failure))?;
    RegionImportRequest::new(0.01, policy).map_err(|failure| CadConstructionFailure::Lowering {
        operation,
        condition: LoweringCondition::ImplicitizationFailed,
        field: Some("import"),
        operand_provenance: None,
        lower: Some(Box::new(CadConstructionLowerFailure::Import(Box::new(
            failure,
        )))),
    })
}

fn lower_explicit_solid(
    operation: &crate::OperationIdentity,
    vertices: &[ExplicitSolidVertex],
    faces: &[ExplicitSolidFace],
    source_revision: &str,
) -> Result<ConstructionOutput, CadConstructionFailure> {
    let mut positions = BTreeMap::new();
    let mut coordinates = Vec::new();
    for (index, vertex) in vertices.iter().enumerate() {
        positions.insert(vertex.id.as_str(), index);
        coordinates.push([
            exact_to_f64(&vertex.position.0[0])?,
            exact_to_f64(&vertex.position.0[1])?,
            exact_to_f64(&vertex.position.0[2])?,
        ]);
    }
    let mut triangles = Vec::new();
    for face in faces {
        let indices =
            face.vertex_order
                .iter()
                .map(|id| {
                    positions.get(id.as_str()).copied().ok_or_else(|| {
                        lowering(operation, LoweringCondition::ImplicitizationFailed)
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
        for window in 1..indices.len() - 1 {
            triangles.push([indices[0], indices[window], indices[window + 1]]);
        }
    }
    let mesh = SurfaceMesh::new(coordinates, triangles, Vec::new()).map_err(|failure| {
        CadConstructionFailure::Lowering {
            operation: operation.clone(),
            condition: LoweringCondition::ImplicitizationFailed,
            field: Some("explicit-solid"),
            operand_provenance: None,
            lower: Some(Box::new(CadConstructionLowerFailure::Mesh(failure))),
        }
    })?;
    let validated =
        validate_surface_mesh(mesh).map_err(|failure| CadConstructionFailure::Lowering {
            operation: operation.clone(),
            condition: LoweringCondition::ImplicitizationFailed,
            field: Some("explicit-solid"),
            operand_provenance: None,
            lower: Some(Box::new(CadConstructionLowerFailure::Mesh(failure))),
        })?;
    let source = MeshSource::new(
        String::from(operation.as_str()),
        String::from(source_revision),
    )
    .map_err(|failure| CadConstructionFailure::Lowering {
        operation: operation.clone(),
        condition: LoweringCondition::ImplicitizationFailed,
        field: Some("explicit-solid"),
        operand_provenance: None,
        lower: Some(Box::new(CadConstructionLowerFailure::Mesh(failure))),
    })?;
    let closed = validate_closed_oriented(validated, source).map_err(|failure| {
        CadConstructionFailure::Lowering {
            operation: operation.clone(),
            condition: LoweringCondition::ImplicitizationFailed,
            field: Some("explicit-solid"),
            operand_provenance: None,
            lower: Some(Box::new(CadConstructionLowerFailure::Mesh(failure))),
        }
    })?;
    let imported = region_from_mesh(closed, import_request()?).map_err(|failure| {
        CadConstructionFailure::Lowering {
            operation: operation.clone(),
            condition: LoweringCondition::ImplicitizationFailed,
            field: Some("explicit-solid"),
            operand_provenance: None,
            lower: Some(Box::new(CadConstructionLowerFailure::Import(Box::new(
                failure,
            )))),
        }
    })?;
    Ok(ConstructionOutput::ImportedRegion(Box::new(imported)))
}

fn child_output<'a>(
    outputs: &'a BTreeMap<crate::OperationIdentity, ConstructionOutput>,
    reference: &OperationOutputRef,
) -> Result<&'a ConstructionOutput, CadConstructionFailure> {
    outputs.get(&reference.output.operation).ok_or_else(|| {
        CadConstructionFailure::Graph(ConstructionGraphFailure::UnknownDependency(
            reference.output.operation.clone(),
        ))
    })
}

fn lower_operation(
    operation: &ConstructionOperation,
    operations: &[ConstructionOperation],
    outputs: &BTreeMap<crate::OperationIdentity, ConstructionOutput>,
    source_revision: &str,
) -> Result<ConstructionOutput, CadConstructionFailure> {
    match &operation.request {
        OperationRequest::Primitive(primitive) => lower_primitive(&operation.operation, primitive),
        OperationRequest::Profile(profile) => Ok(ConstructionOutput::Region(
            lower_profile_data(&operation.operation, profile)?.region,
        )),
        OperationRequest::Extrude {
            profile,
            direction,
            distance,
            symmetric,
        } => {
            let profile_operation = referenced(operations, profile)?;
            let OperationRequest::Profile(profile_request) = &profile_operation.request else {
                return Err(lowering(
                    &operation.operation,
                    LoweringCondition::ImplicitizationFailed,
                ));
            };
            let ConstructionOutput::Region(profile_region) = child_output(outputs, profile)? else {
                return Err(lowering(
                    &operation.operation,
                    LoweringCondition::ImplicitizationFailed,
                ));
            };
            lower_extrude(
                &operation.operation,
                profile_region,
                profile_request,
                direction,
                distance,
                *symmetric,
            )
        }
        OperationRequest::Revolve {
            profile,
            axis_origin,
            axis_direction,
            extent,
        } => {
            let profile_operation = referenced(operations, profile)?;
            let OperationRequest::Profile(profile_request) = &profile_operation.request else {
                return Err(lowering(
                    &operation.operation,
                    LoweringCondition::ImplicitizationFailed,
                ));
            };
            let _ = child_output(outputs, profile)?;
            lower_revolve(
                &operation.operation,
                profile_request,
                axis_origin,
                axis_direction,
                extent,
            )
        }
        OperationRequest::Transform {
            source,
            placement,
            scale,
        } => {
            let ConstructionOutput::Region(region) = child_output(outputs, source)? else {
                return Err(lowering(
                    &operation.operation,
                    LoweringCondition::ImplicitizationFailed,
                ));
            };
            lower_transform(&operation.operation, region, placement, scale)
        }
        OperationRequest::Boolean {
            request,
            left,
            right,
        } => {
            let left_output = child_output(outputs, left)?;
            let right_output = child_output(outputs, right)?;
            lower_boolean(&operation.operation, request, left_output, right_output)
        }
        OperationRequest::Curve(curve) => lower_curve(&operation.operation, curve),
        OperationRequest::NurbsSurfacePatch(patch) => lower_surface(&operation.operation, patch),
        OperationRequest::ExplicitSolid { vertices, faces } => {
            lower_explicit_solid(&operation.operation, vertices, faces, source_revision)
        }
        OperationRequest::ExternalGeometry(asset) => Err(CadConstructionFailure::ExternalAdapter {
            operation: operation.operation.clone(),
            asset_id: asset.asset_id.clone(),
            source_relative_path: asset.source_relative_path.clone(),
            adapter: asset.adapter.clone(),
            asset_schema: asset.asset_schema.clone(),
            condition: String::from("adapter-unavailable"),
        }),
        OperationRequest::Sweep { .. } => Err(lowering(
            &operation.operation,
            LoweringCondition::SweepUnsupported,
        )),
        OperationRequest::Loft { .. } => Err(lowering(
            &operation.operation,
            LoweringCondition::LoftUnsupported,
        )),
        OperationRequest::Fillet { .. } => Err(lowering(
            &operation.operation,
            LoweringCondition::FilletUnsupported,
        )),
        OperationRequest::Chamfer { .. } => Err(lowering(
            &operation.operation,
            LoweringCondition::ChamferUnsupported,
        )),
    }
}

fn dummy_membership_region() -> Result<AlgebraicRegion, CadConstructionFailure> {
    let operation = coefficient_operation()?;
    AlgebraicRegion::try_new(
        3,
        Vec::new(),
        NormalizedRegionFormula::True,
        RegionKind::GeneralSemiAlgebraicSet,
    )
    .map_err(|failure| lowering_geometry(&operation, geometry_condition(&failure), failure))
}

pub fn classify_region_membership(
    region: &AlgebraicRegion,
    point: [ExactScalar; 3],
) -> Result<MembershipClassification, CadConstructionFailure> {
    let operation = coefficient_operation()?;
    let query = RegionPoint::Spatial(Point3 {
        x: point[0].to_real_algebraic()?,
        y: point[1].to_real_algebraic()?,
        z: point[2].to_real_algebraic()?,
    });
    region
        .membership(RegionMembershipRequest {
            region: dummy_membership_region()?,
            point: query,
        })
        .map(|result| result.classification)
        .map_err(|failure| lowering_geometry(&operation, geometry_condition(&failure), failure))
}

pub fn construct(
    request: ConstructionRequest,
) -> Result<ConstructionResult, CadConstructionFailure> {
    validate_operations(&request.operations)?;
    let graph = ConstructionGraph::build(&request.operations)?;
    let (roots, composition) = validate_roots(&request)?;
    request.identity_ledger.validate(&request.operations)?;
    if request.source_revision.as_str() != request.provenance.source_revision.as_str() {
        return Err(CadConstructionFailure::SourceRevisionMismatch {
            request: request.source_revision.as_str().into(),
            artifact: request.provenance.source_revision.as_str().into(),
        });
    }
    let mut outputs = BTreeMap::new();
    for node in graph.nodes() {
        let operation = request
            .operations
            .iter()
            .find(|candidate| candidate.operation == node.operation)
            .ok_or_else(|| {
                CadConstructionFailure::Graph(ConstructionGraphFailure::UnknownDependency(
                    node.operation.clone(),
                ))
            })?;
        let output = lower_operation(
            operation,
            &request.operations,
            &outputs,
            request.source_revision.as_str(),
        )?;
        outputs.insert(operation.operation.clone(), output);
    }
    let mut ordered = Vec::new();
    for node in graph.nodes() {
        if let Some(output) = outputs.remove(&node.operation) {
            ordered.push((
                OperationOutputIdentity {
                    operation: node.operation.clone(),
                    slot: OutputSlot::Primary,
                },
                output,
            ));
        }
    }
    Ok(ConstructionResult::new(
        graph,
        ordered,
        roots,
        composition,
        request.provenance,
        request.identity_ledger,
        request.source_revision,
    ))
}
