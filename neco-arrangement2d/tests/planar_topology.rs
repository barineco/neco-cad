use neco_arrangement2d::{
    CurveEventLocator, Endpoint, EventRotation, PlanarTopologyDeclaration, PlanarVertexDeclaration,
    PlanarWalkRole, PlanarWinding, WalkClassification,
};

#[test]
fn vertex_locator_accepts_endpoint_and_point_forms() {
    let endpoint = CurveEventLocator::endpoint(String::from("curve-a"), Endpoint::Lower).unwrap();
    let vertex = PlanarVertexDeclaration::new(String::from("vertex-a"), vec![endpoint]).unwrap();
    assert_eq!(vertex.incident_event_locators().len(), 1);
}

#[test]
fn topology_declarations_preserve_rotation_and_role() {
    let rotation = EventRotation::new(String::from("vertex-a"), vec![String::from("i-a")]).unwrap();
    let classification = WalkClassification::new(
        String::from("walk-a"),
        vec![String::from("i-a")],
        PlanarWalkRole::FiniteOuter,
        Some(PlanarWinding::Counterclockwise),
        None,
    )
    .unwrap();
    let topology =
        PlanarTopologyDeclaration::new(Vec::new(), vec![rotation], vec![classification]).unwrap();
    assert_eq!(topology.event_rotations().len(), 1);
    assert_eq!(
        topology.walk_classifications()[0].role(),
        PlanarWalkRole::FiniteOuter
    );
}
