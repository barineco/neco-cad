use neco_brep::{
    evaluate_shell_crossing, materialize_analytic_brep, validate_closed_oriented_shell,
    AnalyticBrep, BrepAssignments, BrepFailure, BrepMaterializationRequest, ChartArrangement,
    ClosedOrientedShell, CurveRepresentation, CurveRepresentationId, FaceAssignment,
    ShellCrossingRequest, ShellTopologyWitness, SourceRevision, SurfaceRepresentation,
    SurfaceRepresentationId, Trim, TrimPCurve, VertexAssignment,
};

#[test]
fn exports_the_owned_materialization_entrypoint() {
    let materialize: fn(BrepMaterializationRequest) -> Result<AnalyticBrep, BrepFailure> =
        materialize_analytic_brep;
    let curve: fn(
        CurveRepresentationId,
        neco_nurbs::NurbsTypes,
    ) -> Result<CurveRepresentation, BrepFailure> = CurveRepresentation::new;
    let surface: fn(
        SurfaceRepresentationId,
        neco_nurbs::NurbsTypes,
    ) -> Result<SurfaceRepresentation, BrepFailure> = SurfaceRepresentation::new;
    let _ = (materialize, curve, surface);
}

#[test]
fn request_components_are_public_values() {
    fn observe(
        _: &BrepAssignments,
        _: &ChartArrangement,
        _: &VertexAssignment,
        _: &FaceAssignment,
        _: &SourceRevision,
    ) {
    }
    let _ = observe;
}

#[test]
fn analytic_brep_exports_complete_observation_collections() {
    fn observe(value: &AnalyticBrep) {
        let _ = (
            value.vertices(),
            value.edges(),
            value.faces(),
            value.shells(),
            value.trims(),
            value.incidences(),
            value.curve_representations(),
            value.surface_representations(),
            value.correspondence(),
            value.orientation(),
            value.witnesses(),
        );
    }
    let _ = observe;
}

#[test]
fn exposes_shell_topology_trim_crossing_and_provenance() {
    fn observe_trim(trim: &Trim) {
        let p_curve: TrimPCurve<'_> = trim.p_curve();
        let _ = (
            trim.loop_role(),
            p_curve.curve_representation_id(),
            p_curve.parameter_span(),
        );
    }
    fn observe_face(face: &neco_brep::Face) {
        let _ = (
            face.outer_trim_ids(),
            face.hole_trim_ids(),
            face.orientation(),
        );
    }
    fn observe_witness(witness: &ShellTopologyWitness) {
        let _ = (
            witness.shell_id(),
            witness.edge_count(),
            witness.closed(),
            witness.orientation(),
            witness.manifold(),
        );
    }
    type ShellCrossingFn = fn(
        &AnalyticBrep,
        &str,
        ShellCrossingRequest<'_>,
    ) -> Result<neco_geometry::CheckedRayCrossingResult, BrepFailure>;
    fn observe_crossing(_: ShellCrossingFn) {}
    fn observe_validate(_: fn(&AnalyticBrep, &str) -> Result<ClosedOrientedShell, BrepFailure>) {}
    let _ = (
        observe_trim,
        observe_face,
        observe_witness,
        observe_crossing,
        observe_validate,
    );
    observe_crossing(evaluate_shell_crossing);
    observe_validate(validate_closed_oriented_shell);
}
