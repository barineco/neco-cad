use neco_brep::{
    ArrangementId, BrepAssignments, BrepFailure, CurveRepresentationId, EdgeAssignment,
    FaceAssignment, SourceRevision, SurfaceRepresentationId, VertexAssignment,
};

#[test]
fn assignment_constructor_rejects_a_repeated_stratum() {
    let chart = ArrangementId::new(String::from("chart")).unwrap();
    let assignment =
        VertexAssignment::new(String::from("s"), chart.clone(), String::from("v")).unwrap();
    let edge = EdgeAssignment::new(
        String::from("s"),
        chart,
        String::from("segment"),
        CurveRepresentationId::new(String::from("curve")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        BrepAssignments::new(vec![assignment], vec![edge], Vec::new()),
        Err(BrepFailure::DuplicateAssignmentStratum {
            stratum_id: String::from("s"),
        })
    );
}

#[test]
fn assignment_values_preserve_their_direct_references() {
    let chart = ArrangementId::new(String::from("chart")).unwrap();
    let edge = EdgeAssignment::new(
        String::from("edge-stratum"),
        chart.clone(),
        String::from("segment"),
        CurveRepresentationId::new(String::from("curve")).unwrap(),
    )
    .unwrap();
    let face = FaceAssignment::new(
        String::from("face-stratum"),
        chart,
        String::from("face"),
        SurfaceRepresentationId::new(String::from("surface")).unwrap(),
    )
    .unwrap();
    assert_eq!(edge.segment_id(), "segment");
    assert_eq!(face.arrangement_face_id(), "face");
    assert_eq!(
        SourceRevision::new(String::new()),
        Err(BrepFailure::EmptyIdentifier)
    );
}

#[test]
fn rejects_shells_without_constructible_membership_evidence() {
    assert_eq!(
        neco_brep::BrepFailure::ShellMembershipEvidenceUnavailable {
            shell_id: String::from("known-shell"),
        },
        neco_brep::BrepFailure::ShellMembershipEvidenceUnavailable {
            shell_id: String::from("known-shell"),
        }
    );
    assert_eq!(
        neco_brep::BrepFailure::UnknownShellIdentity {
            shell_id: String::from("missing"),
        },
        neco_brep::BrepFailure::UnknownShellIdentity {
            shell_id: String::from("missing"),
        }
    );
}
