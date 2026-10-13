use alloc::collections::{BTreeMap, BTreeSet};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use core::cmp::Ordering;

use neco_geometry::{IntersectionComponentView, IntersectionWitness, WitnessStatus};

use crate::types::*;

pub fn construct_full_domain_arrangement(
    vertex_keys: Vec<String>,
) -> Result<Arrangement, ArrangementFailure> {
    let mut vertices = Vec::new();
    for vertex_key in vertex_keys {
        vertices.push(PlanarVertexDeclaration::isolated(vertex_key)?);
    }
    let walk_id = String::from("full-domain-walk");
    let face_id = String::from("full-domain-face");
    Ok(Arrangement {
        vertices,
        curves: Vec::new(),
        segments: Vec::new(),
        overlaps: Vec::new(),
        directed_incidences: Vec::new(),
        closed_walks: vec![ClosedWalk::new(
            walk_id.clone(),
            Vec::new(),
            Vec::new(),
            PlanarWalkRole::FiniteOuter,
            None,
        )],
        faces: vec![Face {
            face_id: face_id.clone(),
            outer_walk_id: walk_id,
            hole_ids: Vec::new(),
        }],
        holes: Vec::new(),
        finite_cells: vec![FiniteCell::new(
            String::from("full-domain-cell"),
            vec![face_id],
            Vec::new(),
        )],
    })
}

pub fn construct_arrangement(
    request: ArrangementConstructionRequest,
) -> Result<Arrangement, ArrangementFailure> {
    let (curves, bindings, topology) = request.into_parts();
    validate_curves(&curves)?;
    classify_intersections(&curves, &bindings)?;
    validate_parameter_associations(&curves, &bindings)?;
    let mut events = endpoint_events(&curves)?;
    events.extend(point_events(&curves, &bindings)?);
    canonicalize_events(&mut events)?;
    let segments = build_segments(&curves, &events)?;
    let overlaps = build_overlaps(&bindings)?;
    let membership = resolve_membership(&topology, &events, segments.is_empty())?;
    let incidences = build_incidences(&segments, &membership)?;
    let walks = build_walks(&topology, &incidences)?;
    let (faces, holes) = build_faces_and_holes(&walks, &topology)?;
    let cells = build_cells(&faces, &holes, &walks, &incidences)?;
    Ok(Arrangement {
        vertices: topology.into_vertices(),
        curves,
        segments,
        overlaps,
        directed_incidences: incidences,
        closed_walks: walks,
        faces,
        holes,
        finite_cells: cells,
    })
}

fn validate_curves(curves: &[ArrangementCurve]) -> Result<(), ArrangementFailure> {
    if curves.is_empty() {
        return Err(ArrangementFailure::EmptyCurveInput);
    }
    let mut identities = BTreeSet::new();
    for curve in curves {
        if !identities.insert(curve.identity().curve_id()) {
            return Err(ArrangementFailure::DuplicateCurveIdentity {
                curve_id: curve.identity().curve_id().to_string(),
            });
        }
    }
    Ok(())
}
fn ordered_pair(left: &str, right: &str) -> (String, String) {
    if left < right {
        (left.to_string(), right.to_string())
    } else {
        (right.to_string(), left.to_string())
    }
}
fn classify_intersections(
    curves: &[ArrangementCurve],
    bindings: &[ArrangementIntersectionBinding],
) -> Result<(), ArrangementFailure> {
    let curve_ids: BTreeSet<&str> = curves
        .iter()
        .map(|curve| curve.identity().curve_id())
        .collect();
    let mut pairs = BTreeSet::new();
    for binding in bindings {
        for curve_id in [binding.left_curve_id(), binding.right_curve_id()] {
            if !curve_ids.contains(curve_id) {
                return Err(ArrangementFailure::UnknownIntersectionCurve {
                    curve_id: curve_id.to_string(),
                });
            }
        }
        if binding.left_curve_id() == binding.right_curve_id() {
            return Err(ArrangementFailure::IntersectionBindingSelfPair {
                curve_id: binding.left_curve_id().to_string(),
            });
        }
        if !pairs.insert(ordered_pair(
            binding.left_curve_id(),
            binding.right_curve_id(),
        )) {
            return Err(ArrangementFailure::DuplicateIntersectionBinding {
                left_curve_id: binding.left_curve_id().to_string(),
                right_curve_id: binding.right_curve_id().to_string(),
            });
        }
        validate_intersection_set(binding)?;
    }
    for (left_index, left) in curves.iter().enumerate() {
        for right in &curves[left_index + 1..] {
            if !pairs.contains(&ordered_pair(
                left.identity().curve_id(),
                right.identity().curve_id(),
            )) {
                return Err(ArrangementFailure::MissingIntersectionBinding {
                    left_curve_id: left.identity().curve_id().to_string(),
                    right_curve_id: right.identity().curve_id().to_string(),
                });
            }
        }
    }
    Ok(())
}

fn validate_witness_status(
    witness: &IntersectionWitness,
    binding: &ArrangementIntersectionBinding,
) -> Result<(), ArrangementFailure> {
    if !matches!(witness.existence().status(), WitnessStatus::Proved)
        || !matches!(witness.uniqueness().status(), WitnessStatus::Proved)
    {
        return Err(ArrangementFailure::IntersectionSetWitnessNotProved {
            left_curve_id: binding.left_curve_id().to_string(),
            right_curve_id: binding.right_curve_id().to_string(),
        });
    }
    Ok(())
}

fn component_identities(
    binding: &ArrangementIntersectionBinding,
) -> Result<BTreeSet<neco_geometry::IntersectionComponentIdentity>, ArrangementFailure> {
    match binding.intersection().component_view() {
        IntersectionComponentView::Components(entries) => {
            Ok(entries.into_iter().map(|entry| entry.identity()).collect())
        }
        IntersectionComponentView::NoComponents => Ok(BTreeSet::new()),
        IntersectionComponentView::Undetermined(reason) => {
            Err(ArrangementFailure::IntersectionUndetermined { reason })
        }
    }
}

fn validate_intersection_set(
    binding: &ArrangementIntersectionBinding,
) -> Result<(), ArrangementFailure> {
    component_identities(binding)?;
    validate_witness_status(binding.intersection().witness(), binding)
}

fn validate_parameter_associations(
    curves: &[ArrangementCurve],
    bindings: &[ArrangementIntersectionBinding],
) -> Result<(), ArrangementFailure> {
    for binding in bindings {
        let expected = component_identities(binding)?;
        let point_required = matches!(
            binding.intersection().kind(),
            neco_geometry::IntersectionKind::Point | neco_geometry::IntersectionKind::FinitePoints
        );
        let span_required = matches!(
            binding.intersection().kind(),
            neco_geometry::IntersectionKind::Curve
                | neco_geometry::IntersectionKind::CurveBranches
                | neco_geometry::IntersectionKind::Coincident
        );

        if point_required {
            validate_point_associations(curves, binding, &expected)?;
        } else if !binding.point_associations().is_empty() {
            return Err(ArrangementFailure::UnknownPointAssociation {
                component_id: binding.point_associations()[0]
                    .intersection_id()
                    .to_string(),
            });
        }

        if span_required {
            validate_span_associations(curves, binding, &expected)?;
        } else if !binding.span_associations().is_empty() {
            return Err(ArrangementFailure::UnknownSpanAssociation {
                span_id: binding.span_associations()[0].span_id().to_string(),
            });
        }
    }
    Ok(())
}

fn validate_point_associations(
    curves: &[ArrangementCurve],
    binding: &ArrangementIntersectionBinding,
    expected: &BTreeSet<neco_geometry::IntersectionComponentIdentity>,
) -> Result<(), ArrangementFailure> {
    for association in binding.point_associations() {
        if !expected.contains(&association.component_identity()) {
            return Err(ArrangementFailure::AssociationComponentIdentityMismatch {
                component_id: association.intersection_id().to_string(),
            });
        }
    }
    for component_identity in expected {
        for curve_id in [binding.left_curve_id(), binding.right_curve_id()] {
            let values: Vec<&CurveParameterAssociation> = binding
                .point_associations()
                .iter()
                .filter(|association| {
                    association.component_identity() == *component_identity
                        && association.curve_id() == curve_id
                })
                .collect();
            if values.is_empty() {
                return Err(ArrangementFailure::IncompletePointAssociation {
                    component_id: String::new(),
                });
            }
            if values.len() > 1 {
                return Err(ArrangementFailure::DuplicatePointAssociation {
                    component_id: values[0].intersection_id().to_string(),
                    curve_id: curve_id.to_string(),
                });
            }
            validate_point_association(curves, binding, values[0])?;
        }
    }
    Ok(())
}

fn validate_span_associations(
    curves: &[ArrangementCurve],
    binding: &ArrangementIntersectionBinding,
    expected: &BTreeSet<neco_geometry::IntersectionComponentIdentity>,
) -> Result<(), ArrangementFailure> {
    let mut span_ids = BTreeSet::new();
    for span in binding.span_associations() {
        if !expected.contains(&span.component_identity()) {
            return Err(ArrangementFailure::AssociationComponentIdentityMismatch {
                component_id: span.intersection_id().to_string(),
            });
        }
        if !span_ids.insert(span.span_id()) {
            return Err(ArrangementFailure::DuplicateSpanAssociation {
                span_id: span.span_id().to_string(),
            });
        }
        validate_span_association(curves, binding, span)?;
    }
    for component_identity in expected {
        let spans: Vec<&PairedSpanAssociation> = binding
            .span_associations()
            .iter()
            .filter(|span| span.component_identity() == *component_identity)
            .collect();
        if spans.is_empty() {
            return Err(ArrangementFailure::MissingSpanAssociation {
                component_id: String::new(),
            });
        }
        if spans.len() > 1 {
            return Err(ArrangementFailure::DuplicateSpanAssociation {
                span_id: spans[1].span_id().to_string(),
            });
        }
    }
    Ok(())
}
fn validate_point_association(
    curves: &[ArrangementCurve],
    binding: &ArrangementIntersectionBinding,
    association: &CurveParameterAssociation,
) -> Result<(), ArrangementFailure> {
    if association.curve_id() != binding.left_curve_id()
        && association.curve_id() != binding.right_curve_id()
    {
        return Err(ArrangementFailure::UnknownPointAssociation {
            component_id: association.intersection_id().to_string(),
        });
    }
    if association.endpoint_role() != Some(EndpointRole::Point) {
        return Err(ArrangementFailure::AssociationEndpointRoleMismatch {
            span_id: association.intersection_id().to_string(),
        });
    }
    let curve = curves
        .iter()
        .find(|curve| curve.identity().curve_id() == association.curve_id())
        .ok_or_else(|| ArrangementFailure::UnknownPointAssociation {
            component_id: association.intersection_id().to_string(),
        })?;
    if !curve
        .parameter_interval()
        .contains(association.parameter())?
    {
        return Err(ArrangementFailure::AssociationParameterOutOfDomain {
            curve_id: association.curve_id().to_string(),
        });
    }
    Ok(())
}
fn validate_span_association(
    curves: &[ArrangementCurve],
    binding: &ArrangementIntersectionBinding,
    span: &PairedSpanAssociation,
) -> Result<(), ArrangementFailure> {
    if span.left_curve_id() != binding.left_curve_id()
        || span.right_curve_id() != binding.right_curve_id()
    {
        return Err(ArrangementFailure::AssociationCurveMismatch {
            span_id: span.span_id().to_string(),
        });
    }
    for association in [
        span.left_lower(),
        span.left_upper(),
        span.right_lower(),
        span.right_upper(),
    ] {
        if association.intersection_id() != span.intersection_id()
            || association.component_identity() != span.component_identity()
        {
            return Err(ArrangementFailure::AssociationComponentIdentityMismatch {
                component_id: span.intersection_id().to_string(),
            });
        }
    }
    for (curve_id, lower, upper) in [
        (span.left_curve_id(), span.left_lower(), span.left_upper()),
        (
            span.right_curve_id(),
            span.right_lower(),
            span.right_upper(),
        ),
    ] {
        if lower.curve_id() != curve_id || upper.curve_id() != curve_id {
            return Err(ArrangementFailure::AssociationCurveMismatch {
                span_id: span.span_id().to_string(),
            });
        }
        if lower.endpoint_role() != Some(EndpointRole::OverlapLower)
            || upper.endpoint_role() != Some(EndpointRole::OverlapUpper)
            || lower.parameter().compare(upper.parameter())? != Ordering::Less
        {
            return Err(ArrangementFailure::AssociationEndpointRoleMismatch {
                span_id: span.span_id().to_string(),
            });
        }
        let curve = curves
            .iter()
            .find(|curve| curve.identity().curve_id() == curve_id)
            .ok_or_else(|| ArrangementFailure::AssociationCurveMismatch {
                span_id: span.span_id().to_string(),
            })?;
        if !curve.parameter_interval().contains(lower.parameter())?
            || !curve.parameter_interval().contains(upper.parameter())?
        {
            return Err(ArrangementFailure::AssociationParameterOutOfDomain {
                curve_id: curve_id.to_string(),
            });
        }
    }
    Ok(())
}

fn endpoint_events(curves: &[ArrangementCurve]) -> Result<Vec<SplitEvent>, ArrangementFailure> {
    curves
        .iter()
        .flat_map(|curve| {
            let id = curve.identity().curve_id().to_string();
            let key = curve.identity().stable_key().to_string();
            [
                (curve.parameter_interval().lower(), Endpoint::Lower),
                (curve.parameter_interval().upper(), Endpoint::Upper),
            ]
            .into_iter()
            .map(move |(parameter, endpoint)| {
                Ok(SplitEvent {
                    curve_id: id.clone(),
                    stable_key: key.clone(),
                    parameter: parameter.try_clone()?,
                    locator: CurveEventLocator::endpoint(id.clone(), endpoint)?,
                    source_rank: 0,
                })
            })
        })
        .collect()
}
fn point_events(
    curves: &[ArrangementCurve],
    bindings: &[ArrangementIntersectionBinding],
) -> Result<Vec<SplitEvent>, ArrangementFailure> {
    let mut events = Vec::new();
    for binding in bindings {
        if matches!(
            binding.intersection().kind(),
            neco_geometry::IntersectionKind::Point | neco_geometry::IntersectionKind::FinitePoints
        ) {
            for value in binding.point_associations() {
                let stable_key = curves
                    .iter()
                    .find(|curve| curve.identity().curve_id() == value.curve_id())
                    .ok_or_else(|| ArrangementFailure::UnknownPointAssociation {
                        component_id: value.intersection_id().to_string(),
                    })?
                    .identity()
                    .stable_key()
                    .to_string();
                events.push(SplitEvent {
                    curve_id: value.curve_id().to_string(),
                    stable_key,
                    parameter: value.parameter().try_clone()?,
                    locator: CurveEventLocator::point_intersection(
                        value.curve_id().to_string(),
                        value.intersection_id().to_string(),
                        value.component_identity(),
                        value.parameter().try_clone()?,
                    )?,
                    source_rank: 1,
                });
            }
        }
    }
    Ok(events)
}
fn compare_events(left: &SplitEvent, right: &SplitEvent) -> Result<Ordering, ArrangementFailure> {
    let mut order = left.stable_key.cmp(&right.stable_key);
    if order == Ordering::Equal {
        order = left.parameter.compare(&right.parameter)?;
    }
    if order == Ordering::Equal {
        order = left.source_rank.cmp(&right.source_rank);
    }
    if order == Ordering::Equal {
        order = left.locator.compare(&right.locator)?;
    }
    Ok(order)
}
fn canonicalize_events(events: &mut [SplitEvent]) -> Result<(), ArrangementFailure> {
    for index in 1..events.len() {
        let mut current = index;
        while current > 0
            && compare_events(&events[current], &events[current - 1])? == Ordering::Less
        {
            events.swap(current, current - 1);
            current -= 1;
        }
    }
    Ok(())
}
fn clone_locator(locator: &CurveEventLocator) -> Result<CurveEventLocator, ArrangementFailure> {
    locator.try_clone()
}

fn build_segments(
    curves: &[ArrangementCurve],
    events: &[SplitEvent],
) -> Result<Vec<Segment>, ArrangementFailure> {
    let mut segments = Vec::new();
    for curve in curves {
        let indices: Vec<usize> = events
            .iter()
            .enumerate()
            .filter_map(|(index, event)| {
                (event.curve_id == curve.identity().curve_id()).then_some(index)
            })
            .collect();
        let (first_index, last_index) = match (indices.first(), indices.last()) {
            (Some(first), Some(last)) => (*first, *last),
            _ => {
                return Err(ArrangementFailure::SegmentCoverageFailure {
                    curve_id: curve.identity().curve_id().to_string(),
                });
            }
        };
        if indices.len() < 2
            || events[first_index]
                .parameter
                .compare(curve.parameter_interval().lower())?
                != Ordering::Equal
            || events[last_index]
                .parameter
                .compare(curve.parameter_interval().upper())?
                != Ordering::Equal
        {
            return Err(ArrangementFailure::SegmentCoverageFailure {
                curve_id: curve.identity().curve_id().to_string(),
            });
        }
        for pair in indices.windows(2) {
            let lower = events[pair[0]].parameter.try_clone()?;
            let upper = events[pair[1]].parameter.try_clone()?;
            if lower.compare(&upper)? == Ordering::Equal {
                continue;
            }
            let span = ParameterSpan::new(lower, upper, true, true)?;
            segments.push(Segment::new(
                format!("{}:{}", curve.identity().stable_key(), segments.len()),
                curve.identity().curve_id().to_string(),
                span,
                pair[0],
                pair[1],
            ));
        }
    }
    Ok(segments)
}
fn build_overlaps(
    bindings: &[ArrangementIntersectionBinding],
) -> Result<Vec<Overlap>, ArrangementFailure> {
    let mut overlaps = Vec::new();
    for binding in bindings {
        for span in binding.span_associations() {
            let overlap_id = span.span_id().to_string();
            if overlaps
                .iter()
                .any(|value: &Overlap| value.overlap_id == overlap_id)
            {
                return Err(ArrangementFailure::DuplicateOverlap { overlap_id });
            }
            overlaps.push(Overlap {
                overlap_id,
                left_curve_id: span.left_curve_id().to_string(),
                right_curve_id: span.right_curve_id().to_string(),
                left_span: ParameterSpan::new(
                    span.left_lower().parameter().try_clone()?,
                    span.left_upper().parameter().try_clone()?,
                    true,
                    true,
                )?,
                right_span: ParameterSpan::new(
                    span.right_lower().parameter().try_clone()?,
                    span.right_upper().parameter().try_clone()?,
                    true,
                    true,
                )?,
                orientation: match span.correspondence() {
                    SpanEndpointCorrespondence::LowerToLower => OverlapOrientation::SameDirection,
                    SpanEndpointCorrespondence::LowerToUpper => {
                        OverlapOrientation::OppositeDirection
                    }
                },
            });
        }
    }
    Ok(overlaps)
}

struct Membership {
    event_vertices: Vec<String>,
}
fn resolve_membership(
    topology: &PlanarTopologyDeclaration,
    events: &[SplitEvent],
    allow_empty: bool,
) -> Result<Membership, ArrangementFailure> {
    if allow_empty
        && topology.vertices().is_empty()
        && topology.event_rotations().is_empty()
        && topology.walk_classifications().is_empty()
    {
        return Ok(Membership {
            event_vertices: Vec::new(),
        });
    }
    let mut vertices = BTreeSet::new();
    let mut assignments = vec![None; events.len()];
    for vertex in topology.vertices() {
        if !vertices.insert(vertex.vertex_key()) {
            return Err(ArrangementFailure::DuplicatePlanarVertex {
                vertex_key: vertex.vertex_key().to_string(),
            });
        }
        for locator in vertex.incident_event_locators() {
            let mut indices = Vec::new();
            for (index, event) in events.iter().enumerate() {
                if locator.compare(&event.locator)? == Ordering::Equal {
                    indices.push(index);
                }
            }
            if indices.is_empty() {
                return Err(ArrangementFailure::UnknownVertexEventLocator {
                    vertex_key: vertex.vertex_key().to_string(),
                    locator: clone_locator(locator)?,
                });
            }
            if indices.len() != 1 {
                return Err(ArrangementFailure::AmbiguousVertexEventLocator {
                    vertex_key: vertex.vertex_key().to_string(),
                    locator: clone_locator(locator)?,
                });
            }
            let index = indices[0];
            if assignments[index].is_some() {
                return Err(ArrangementFailure::DuplicateVertexEventLocator {
                    vertex_key: vertex.vertex_key().to_string(),
                    locator: clone_locator(locator)?,
                });
            }
            assignments[index] = Some(vertex.vertex_key().to_string());
        }
    }
    let mut event_vertices = Vec::new();
    for (index, value) in assignments.into_iter().enumerate() {
        match value {
            Some(vertex_key) => event_vertices.push(vertex_key),
            None => {
                return Err(ArrangementFailure::UncoveredVertexEventLocator {
                    locator: clone_locator(&events[index].locator)?,
                });
            }
        }
    }
    Ok(Membership { event_vertices })
}
fn build_incidences(
    segments: &[Segment],
    membership: &Membership,
) -> Result<Vec<DirectedIncidence>, ArrangementFailure> {
    let mut values = Vec::new();
    for segment in segments {
        let from = membership
            .event_vertices
            .get(segment.start_event)
            .ok_or_else(|| ArrangementFailure::IncidenceConstructionFailure {
                incidence_id: segment.segment_id().to_string(),
            })?
            .to_string();
        let to = membership
            .event_vertices
            .get(segment.end_event)
            .ok_or_else(|| ArrangementFailure::IncidenceConstructionFailure {
                incidence_id: segment.segment_id().to_string(),
            })?
            .to_string();
        values.push(DirectedIncidence::new(
            format!("{}:forward", segment.segment_id()),
            segment.segment_id().to_string(),
            from.clone(),
            to.clone(),
            DirectedIncidenceDirection::Forward,
        ));
        values.push(DirectedIncidence::new(
            format!("{}:reverse", segment.segment_id()),
            segment.segment_id().to_string(),
            to,
            from,
            DirectedIncidenceDirection::Reverse,
        ));
    }
    Ok(values)
}

fn build_walks(
    topology: &PlanarTopologyDeclaration,
    incidences: &[DirectedIncidence],
) -> Result<Vec<ClosedWalk>, ArrangementFailure> {
    if incidences.is_empty() {
        if topology.event_rotations().is_empty() && topology.walk_classifications().is_empty() {
            return Ok(Vec::new());
        }
        return Err(ArrangementFailure::UnknownEventRotation {
            vertex_key: String::new(),
        });
    }
    let ids: BTreeMap<&str, &DirectedIncidence> = incidences
        .iter()
        .map(|item| (item.incidence_id(), item))
        .collect();
    let vertices: BTreeSet<&str> = incidences
        .iter()
        .map(|item| item.from_vertex_key())
        .collect();
    let mut rotations = BTreeMap::new();
    for rotation in topology.event_rotations() {
        if !vertices.contains(rotation.vertex_key()) {
            return Err(ArrangementFailure::UnknownEventRotation {
                vertex_key: rotation.vertex_key().to_string(),
            });
        }
        if rotations
            .insert(
                rotation.vertex_key(),
                rotation.outgoing_incidence_ids_counterclockwise(),
            )
            .is_some()
        {
            return Err(ArrangementFailure::DuplicateEventRotation {
                vertex_key: rotation.vertex_key().to_string(),
            });
        }
        let mut listed = BTreeSet::new();
        for id in rotation.outgoing_incidence_ids_counterclockwise() {
            let incidence = ids.get(id.as_str()).ok_or_else(|| {
                ArrangementFailure::RotationUnknownIncidence {
                    vertex_key: rotation.vertex_key().to_string(),
                    incidence_id: id.clone(),
                }
            })?;
            if !listed.insert(id.as_str()) {
                return Err(ArrangementFailure::RotationDuplicateIncidence {
                    vertex_key: rotation.vertex_key().to_string(),
                    incidence_id: id.clone(),
                });
            }
            if incidence.from_vertex_key() != rotation.vertex_key() {
                return Err(ArrangementFailure::RotationIncidenceMismatch {
                    vertex_key: rotation.vertex_key().to_string(),
                    incidence_id: id.clone(),
                });
            }
        }
        let expected: BTreeSet<&str> = incidences
            .iter()
            .filter(|item| item.from_vertex_key() == rotation.vertex_key())
            .map(|item| item.incidence_id())
            .collect();
        if listed != expected {
            return Err(ArrangementFailure::RotationIncidenceCompletenessFailure {
                vertex_key: rotation.vertex_key().to_string(),
            });
        }
    }
    for vertex in vertices {
        if !rotations.contains_key(vertex) {
            return Err(ArrangementFailure::UnknownEventRotation {
                vertex_key: vertex.to_string(),
            });
        }
    }
    let mut successor = BTreeMap::new();
    for incidence in incidences {
        let rotation = rotations.get(incidence.to_vertex_key()).ok_or_else(|| {
            ArrangementFailure::UnknownEventRotation {
                vertex_key: incidence.to_vertex_key().to_string(),
            }
        })?;
        let reverse = format!(
            "{}:{}",
            incidence.segment_id(),
            if incidence.direction() == DirectedIncidenceDirection::Forward {
                "reverse"
            } else {
                "forward"
            }
        );
        let position = rotation
            .iter()
            .position(|id| id == &reverse)
            .ok_or_else(
                || ArrangementFailure::RotationIncidenceCompletenessFailure {
                    vertex_key: incidence.to_vertex_key().to_string(),
                },
            )?;
        successor.insert(
            incidence.incidence_id(),
            rotation[(position + rotation.len() - 1) % rotation.len()].clone(),
        );
    }
    let mut classifications = BTreeMap::new();
    let mut sequences = BTreeSet::new();
    for item in topology.walk_classifications() {
        if classifications.insert(item.walk_key(), item).is_some() {
            return Err(ArrangementFailure::DuplicateWalkClassification {
                walk_key: item.walk_key().to_string(),
            });
        }
        if !sequences.insert(item.canonical_incidence_ids()) {
            return Err(ArrangementFailure::DuplicateWalkClassificationSequence {
                canonical_incidence_ids: item.canonical_incidence_ids().to_vec(),
            });
        }
        for id in item.canonical_incidence_ids() {
            if !ids.contains_key(id.as_str()) {
                return Err(ArrangementFailure::WalkClassificationUnknownIncidence {
                    walk_key: item.walk_key().to_string(),
                    incidence_id: id.clone(),
                });
            }
        }
    }
    let mut visited = BTreeSet::new();
    let mut walks = Vec::new();
    for start in incidences {
        if visited.contains(start.incidence_id()) {
            continue;
        }
        let mut sequence = Vec::new();
        let mut local = BTreeSet::new();
        let mut cursor = start.incidence_id().to_string();
        loop {
            if !local.insert(cursor.clone()) {
                if cursor != start.incidence_id() {
                    return Err(ArrangementFailure::NonClosedWalk {
                        incidence_id: cursor,
                    });
                }
                break;
            }
            visited.insert(cursor.clone());
            sequence.push(cursor.clone());
            cursor = successor
                .get(cursor.as_str())
                .ok_or_else(|| ArrangementFailure::NonClosedWalk {
                    incidence_id: cursor.clone(),
                })?
                .clone();
        }
        let classification = topology
            .walk_classifications()
            .iter()
            .find(|value| value.canonical_incidence_ids() == sequence.as_slice())
            .ok_or_else(|| ArrangementFailure::UnknownWalkClassification {
                walk_key: sequence[0].clone(),
            })?;
        validate_role_form(classification)?;
        let vertex_keys = sequence
            .iter()
            .map(|id| ids[id.as_str()].from_vertex_key().to_string())
            .collect();
        walks.push(ClosedWalk::new(
            classification.walk_key().to_string(),
            sequence,
            vertex_keys,
            classification.role(),
            classification.winding(),
        ));
    }
    Ok(walks)
}
fn validate_role_form(value: &WalkClassification) -> Result<(), ArrangementFailure> {
    match value.role() {
        PlanarWalkRole::Exterior => {
            if value.winding().is_some() {
                Err(ArrangementFailure::WalkClassificationWindingMismatch {
                    walk_key: value.walk_key().to_string(),
                })
            } else if value.containing_outer_walk_key().is_some() {
                Err(ArrangementFailure::WalkClassificationContainmentMismatch {
                    walk_key: value.walk_key().to_string(),
                })
            } else {
                Ok(())
            }
        }
        PlanarWalkRole::FiniteOuter => {
            if value.winding() != Some(PlanarWinding::Counterclockwise) {
                Err(ArrangementFailure::WalkClassificationWindingMismatch {
                    walk_key: value.walk_key().to_string(),
                })
            } else if value.containing_outer_walk_key().is_some() {
                Err(ArrangementFailure::WalkClassificationContainmentMismatch {
                    walk_key: value.walk_key().to_string(),
                })
            } else {
                Ok(())
            }
        }
        PlanarWalkRole::Hole => {
            if value.winding() != Some(PlanarWinding::Clockwise) {
                Err(ArrangementFailure::WalkClassificationWindingMismatch {
                    walk_key: value.walk_key().to_string(),
                })
            } else if value.containing_outer_walk_key().is_none() {
                Err(ArrangementFailure::WalkClassificationContainmentMismatch {
                    walk_key: value.walk_key().to_string(),
                })
            } else {
                Ok(())
            }
        }
    }
}
fn build_faces_and_holes(
    walks: &[ClosedWalk],
    topology: &PlanarTopologyDeclaration,
) -> Result<(Vec<Face>, Vec<Hole>), ArrangementFailure> {
    let mut faces: Vec<Face> = walks
        .iter()
        .filter(|walk| walk.role() == PlanarWalkRole::FiniteOuter)
        .map(|walk| Face {
            face_id: format!("face:{}", walk.walk_id()),
            outer_walk_id: walk.walk_id().to_string(),
            hole_ids: Vec::new(),
        })
        .collect();
    let mut holes = Vec::new();
    for classification in topology
        .walk_classifications()
        .iter()
        .filter(|value| value.role() == PlanarWalkRole::Hole)
    {
        let outer = classification.containing_outer_walk_key().ok_or_else(|| {
            ArrangementFailure::HoleContainmentFailure {
                walk_key: classification.walk_key().to_string(),
            }
        })?;
        if outer == classification.walk_key() {
            return Err(ArrangementFailure::ContainingOuterWalkSelfReference {
                walk_key: classification.walk_key().to_string(),
            });
        }
        let face = faces
            .iter_mut()
            .find(|face| face.outer_walk_id == outer)
            .ok_or_else(|| ArrangementFailure::ContainingOuterWalkUnknown {
                walk_key: classification.walk_key().to_string(),
                containing_outer_walk_key: outer.to_string(),
            })?;
        let hole_id = format!("hole:{}", classification.walk_key());
        face.hole_ids.push(hole_id.clone());
        holes.push(Hole {
            hole_id,
            walk_id: classification.walk_key().to_string(),
            containing_face_id: face.face_id.clone(),
        });
    }
    Ok((faces, holes))
}
fn build_cells(
    faces: &[Face],
    holes: &[Hole],
    walks: &[ClosedWalk],
    incidences: &[DirectedIncidence],
) -> Result<Vec<FiniteCell>, ArrangementFailure> {
    let walk_ids: BTreeSet<&str> = walks.iter().map(|walk| walk.walk_id()).collect();
    let incidence_ids: BTreeSet<&str> = incidences
        .iter()
        .map(|incidence| incidence.incidence_id())
        .collect();
    let mut cells = Vec::new();
    for face in faces {
        if !walk_ids.contains(face.outer_walk_id()) {
            return Err(ArrangementFailure::FiniteCellConstructionFailure {
                face_id: face.face_id().to_string(),
            });
        }
        for hole_id in face.hole_ids() {
            let Some(hole) = holes.iter().find(|hole| hole.hole_id() == hole_id) else {
                return Err(ArrangementFailure::FiniteCellConstructionFailure {
                    face_id: face.face_id().to_string(),
                });
            };
            if hole.containing_face_id() != face.face_id()
                || !walk_ids.contains(hole.walk_id())
                || walks
                    .iter()
                    .find(|walk| walk.walk_id() == hole.walk_id())
                    .is_some_and(|walk| {
                        walk.incidence_ids()
                            .iter()
                            .any(|id| !incidence_ids.contains(id.as_str()))
                    })
            {
                return Err(ArrangementFailure::FiniteCellConstructionFailure {
                    face_id: face.face_id().to_string(),
                });
            }
        }
        cells.push(FiniteCell::new(
            format!("cell:{}", face.face_id()),
            vec![face.face_id().to_string()],
            face.hole_ids().to_vec(),
        ));
    }
    Ok(cells)
}
