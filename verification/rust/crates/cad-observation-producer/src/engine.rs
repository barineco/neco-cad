use neco_cad::*;
use serde_json::Value;

#[derive(Debug)]
pub enum EngineError {
    MissingFace,
    MissingSurface,
    Construction(Box<CadConstructionFailure>),
    Cad(Box<CadFailure>),
    InvalidInput,
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingFace => formatter.write_str("requested face is absent from the public result"),
            Self::MissingSurface => formatter.write_str("requested surface is absent from the public result"),
            Self::Construction(error) => write!(formatter, "construct: {error:?}"),
            Self::Cad(error) => write!(formatter, "CAD operation: {error:?}"),
            Self::InvalidInput => formatter.write_str("projected public API input is malformed"),
        }
    }
}

impl From<CadConstructionFailure> for EngineError {
    fn from(error: CadConstructionFailure) -> Self { Self::Construction(Box::new(error)) }
}
impl From<CadFailure> for EngineError {
    fn from(error: CadFailure) -> Self { Self::Cad(Box::new(error)) }
}

fn op_id(value: &str) -> OperationIdentity {
    OperationIdentity::new(value).expect("constant operation identity")
}
fn source_id(value: &str) -> SourceIdentity {
    SourceIdentity::new(value).expect("constant source identity")
}

fn operation(name: &str, request: OperationRequest) -> ConstructionOperation {
    ConstructionOperation {
        operation: op_id(name), source_object: source_id(name), request,
        expected_output: GeometryOutputExpectation {
            category: GeometryOutputCategory::Solid, ambient_dimension: 3,
            manifold_dimension: 3, closure: GeometryClosure::Closed,
        },
    }
}

fn construction_request(operations: Vec<ConstructionOperation>, revision: &str) -> ConstructionRequest {
    let source_revision = SourceRevision::new(revision.to_owned()).expect("projected nonempty revision");
    let ledger = IdentityLedger::new(
        operations.iter().map(|op| OperationIdentityBinding {
            source: op.source_object.clone(), operation: op.operation.clone(), output: op.output_identity(),
        }).collect(),
        operations.iter().map(|op| EntityIdentityBinding {
            source: op.source_object.clone(), operation: op.operation.clone(), output: op.output_identity(),
            produced: format!("{}:primary", op.operation.as_str()),
        }).collect(),
    ).expect("distinct constant operation identities");
    ConstructionRequest {
        source_revision: source_revision.clone(), precision: GeometryAccuracyPolicy::ExactOnly,
        identity_ledger: ledger, operations, roots: vec![],
        provenance: Provenance::new("cad-verification", "cad", source_revision,
            "generated-request-v1", "exact", vec![], vec![]),
    }
}

fn derive(operations: Vec<ConstructionOperation>, output_name: &str, revision: &str)
    -> Result<DerivedAnalyticBrep, EngineError>
{
    let result = construct(construction_request(operations, revision))?;
    let region = result.into_outputs().into_iter().find_map(|(identity, output)| {
        if identity.operation != op_id(output_name) || identity.slot != OutputSlot::Primary {
            return None;
        }
        match output { ConstructionOutput::Region(region) => Some(region), _ => None }
    }).ok_or(EngineError::MissingSurface)?;
    let boundary = derive_complete_boundary(region)?;
    Ok(derive_analytic_brep(boundary, SourceRevision::new(revision.to_owned())
        .map_err(|_| EngineError::InvalidInput)?)?)
}

pub fn revision(input: &Value) -> Result<&str, EngineError> {
    input["sourceRevision"].as_str().filter(|s| !s.is_empty()).ok_or(EngineError::InvalidInput)
}

pub fn sphere_derived(revision: &str) -> Result<DerivedAnalyticBrep, EngineError> {
    derive(vec![operation("s", OperationRequest::Primitive(PrimitiveDeclaration::Sphere {
        radius: ExactScalar::new(1.0)?,
    }))], "s", revision)
}

pub fn chart_derived(input: &Value) -> Result<DerivedAnalyticBrep, EngineError> {
    let extent = input["primitiveExtent"].as_f64().ok_or(EngineError::InvalidInput)?;
    let primitive = if input["surfaceFamily"] == "plane" {
        PrimitiveDeclaration::Box { lengths: ExactVector3::new([2.0 * extent; 3])? }
    } else { PrimitiveDeclaration::Sphere { radius: ExactScalar::new(extent)? } };
    let translation = input["translation"].as_array().ok_or(EngineError::InvalidInput)?;
    if translation.len() != 3 { return Err(EngineError::InvalidInput); }
    let mut origin = [0.0; 3];
    for (coordinate, value) in origin.iter_mut().zip(translation) {
        *coordinate = value.as_f64().ok_or(EngineError::InvalidInput)?;
    }
    let scale = input["scale"].as_f64().ok_or(EngineError::InvalidInput)?;
    let operations = vec![
        operation("s", OperationRequest::Primitive(primitive)),
        operation("t", OperationRequest::Transform {
            source: OperationOutputRef::primary(op_id("s")),
            placement: ExactRigidPlacement::new(ExactPoint3::new(origin)?, [1.0, 0.0, 0.0, 0.0])?,
            scale: ExactScalar::new(scale)?,
        }),
    ];
    derive(operations, "t", revision(input)?)
}

pub fn sphere_tessellation(input: &Value) -> Result<AdaptiveTessellation, EngineError> {
    let derived = sphere_derived(revision(input)?)?;
    let chord = input["maximumChordError"].as_f64().ok_or(EngineError::InvalidInput)?;
    let surface = input["maximumSurfaceError"].as_f64().ok_or(EngineError::InvalidInput)?;
    let precision = AdaptivePrecision::new(TessellationConsumer::Analysis, chord, surface)
        .map_err(|_| EngineError::InvalidInput)?;
    let request = derived.into_adaptive_tessellation_request(source_id("cad-verification"), precision, 48)?;
    Ok(tessellate_adaptive(request)?)
}
