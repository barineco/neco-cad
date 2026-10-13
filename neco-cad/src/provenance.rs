use crate::construction_types::{
    ConstructionOperation, GeometryOutputCategory, InputRole, OperationOutputIdentity, OutputSlot,
};
use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;
use neco_brep::SourceRevision;
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct SourceIdentity(String);
impl SourceIdentity {
    pub fn new(v: impl Into<String>) -> Result<Self, IdentityFailure> {
        let v = v.into();
        if v.is_empty() {
            Err(IdentityFailure::EmptyIdentity)
        } else {
            Ok(Self(v))
        }
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct OperationIdentity(String);
impl OperationIdentity {
    pub fn new(v: impl Into<String>) -> Result<Self, IdentityFailure> {
        SourceIdentity::new(v).map(|x| Self(x.0))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BodyComposition {
    pub policy_id: String,
    pub obstacle_operation: String,
    pub material_operation: String,
    pub overlap_precedence: String,
    pub tolerance_policy: String,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationProvenance {
    pub source_object: SourceIdentity,
    pub declaration_kind: &'static str,
    pub input_roles: Vec<InputRole>,
    pub source_revision: SourceRevision,
    pub adapter_revision: String,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Provenance {
    pub declaration_schema: String,
    pub declaration_namespace: String,
    pub source_revision: SourceRevision,
    pub adapter_revision: String,
    pub precision: String,
    pub operations: Vec<OperationProvenance>,
    pub composition: Vec<BodyComposition>,
}
impl Provenance {
    pub fn new(
        schema: impl Into<String>,
        namespace: impl Into<String>,
        revision: SourceRevision,
        adapter: impl Into<String>,
        precision: impl Into<String>,
        operations: Vec<OperationProvenance>,
        composition: Vec<BodyComposition>,
    ) -> Self {
        Self {
            declaration_schema: schema.into(),
            declaration_namespace: namespace.into(),
            source_revision: revision,
            adapter_revision: adapter.into(),
            precision: precision.into(),
            operations,
            composition,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationIdentityBinding {
    pub source: SourceIdentity,
    pub operation: OperationIdentity,
    pub output: OperationOutputIdentity,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntityIdentityBinding {
    pub source: SourceIdentity,
    pub operation: OperationIdentity,
    pub output: OperationOutputIdentity,
    pub produced: String,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdentityLedger {
    pub operations: Vec<OperationIdentityBinding>,
    pub entities: Vec<EntityIdentityBinding>,
}
fn valid_produced_identifier(value: &str) -> bool {
    let mut parts = value.split(':');
    matches!(
        (parts.next(), parts.next(), parts.next()),
        (Some(prefix), Some(name), None)
            if !prefix.is_empty()
                && !name.is_empty()
                && prefix.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
                && name.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    )
}
impl IdentityLedger {
    pub fn new(
        operations: Vec<OperationIdentityBinding>,
        entities: Vec<EntityIdentityBinding>,
    ) -> Result<Self, IdentityFailure> {
        if operations.is_empty() || entities.is_empty() {
            return Err(IdentityFailure::EmptyLedger);
        }
        let ledger = Self {
            operations,
            entities,
        };
        ledger.validate_internal()?;
        Ok(ledger)
    }
    fn validate_internal(&self) -> Result<(), IdentityFailure> {
        let mut operation_ids = BTreeSet::new();
        let mut operation_outputs = BTreeSet::new();
        for binding in &self.operations {
            if !operation_ids.insert(binding.operation.clone()) {
                return Err(IdentityFailure::DuplicateOperation(
                    binding.operation.clone(),
                ));
            }
            if !operation_outputs.insert(binding.output.clone()) {
                return Err(IdentityFailure::DuplicateOutput(binding.output.clone()));
            }
        }
        let mut entity_operations = BTreeSet::new();
        let mut produced = BTreeSet::new();
        for binding in &self.entities {
            if !entity_operations.insert(binding.operation.clone()) {
                return Err(IdentityFailure::DuplicateEntityBinding(
                    binding.operation.clone(),
                ));
            }
            if !valid_produced_identifier(&binding.produced) {
                return Err(IdentityFailure::UnknownProducedEntity(
                    binding.produced.clone(),
                ));
            }
            if !produced.insert(binding.produced.clone()) {
                return Err(IdentityFailure::DuplicateProducedEntity(
                    binding.produced.clone(),
                ));
            }
            let operation = self
                .operations
                .iter()
                .find(|candidate| candidate.operation == binding.operation)
                .ok_or_else(|| IdentityFailure::ExtraEntityBinding(binding.operation.clone()))?;
            if binding.source != operation.source {
                return Err(IdentityFailure::SourceMismatch {
                    request: operation.source.clone(),
                    ledger: binding.source.clone(),
                });
            }
            if binding.output != operation.output {
                return Err(IdentityFailure::OutputMismatch {
                    request: operation.output.clone(),
                    ledger: binding.output.clone(),
                });
            }
        }
        for operation in &self.operations {
            if !entity_operations.contains(&operation.operation) {
                return Err(IdentityFailure::MissingEntityBinding(
                    operation.operation.clone(),
                ));
            }
        }
        Ok(())
    }
    pub fn validate(&self, request: &[ConstructionOperation]) -> Result<(), IdentityFailure> {
        self.validate_internal()?;
        let mut requested = BTreeSet::new();
        for operation in request {
            if !requested.insert(operation.operation.clone()) {
                return Err(IdentityFailure::DuplicateOperation(
                    operation.operation.clone(),
                ));
            }
        }
        let mut sorted: Vec<_> = request.iter().collect();
        sorted.sort_by_key(|o| o.operation.clone());
        for operation in sorted {
            let binding = self
                .operations
                .iter()
                .find(|b| b.operation == operation.operation)
                .ok_or_else(|| {
                    IdentityFailure::MissingOperationBinding(operation.operation.clone())
                })?;
            if binding.source != operation.source_object {
                return Err(IdentityFailure::SourceMismatch {
                    request: operation.source_object.clone(),
                    ledger: binding.source.clone(),
                });
            }
            if binding.output != operation.output_identity() {
                return Err(IdentityFailure::OutputMismatch {
                    request: operation.output_identity(),
                    ledger: binding.output.clone(),
                });
            }
        }
        for binding in &self.operations {
            if !requested.contains(&binding.operation) {
                return Err(IdentityFailure::ExtraOperationBinding(
                    binding.operation.clone(),
                ));
            }
        }
        let mut entity_operations = BTreeSet::new();
        let mut produced = BTreeSet::new();
        for binding in &self.entities {
            if !entity_operations.insert(binding.operation.clone()) {
                return Err(IdentityFailure::DuplicateEntityBinding(
                    binding.operation.clone(),
                ));
            }
            if !produced.insert(binding.produced.clone()) {
                return Err(IdentityFailure::DuplicateProducedEntity(
                    binding.produced.clone(),
                ));
            }
            let operation = request
                .iter()
                .find(|o| o.operation == binding.operation)
                .ok_or_else(|| IdentityFailure::ExtraEntityBinding(binding.operation.clone()))?;
            if binding.produced.is_empty() {
                return Err(IdentityFailure::UnknownProducedEntity(
                    binding.produced.clone(),
                ));
            }
            if binding.source != operation.source_object {
                return Err(IdentityFailure::SourceMismatch {
                    request: operation.source_object.clone(),
                    ledger: binding.source.clone(),
                });
            }
            if binding.output != operation.output_identity() {
                return Err(IdentityFailure::OutputMismatch {
                    request: operation.output_identity(),
                    ledger: binding.output.clone(),
                });
            }
        }
        for operation in request {
            if !entity_operations.contains(&operation.operation) {
                return Err(IdentityFailure::MissingEntityBinding(
                    operation.operation.clone(),
                ));
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConstructionNode {
    pub operation: OperationIdentity,
    pub kind: &'static str,
    pub ordinal: u32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConstructionDependency {
    pub consumer: OperationIdentity,
    pub role: InputRole,
    pub producer: OperationOutputIdentity,
    pub ordinal: Option<u32>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConstructionGraph {
    nodes: Vec<ConstructionNode>,
    dependencies: Vec<ConstructionDependency>,
}
impl ConstructionGraph {
    pub fn build(ops: &[ConstructionOperation]) -> Result<Self, ConstructionGraphFailure> {
        let mut canonical: Vec<_> = ops.iter().collect();
        canonical.sort_by_key(|operation| operation.operation.clone());
        let mut ids = BTreeSet::new();
        for operation in &canonical {
            if !ids.insert(operation.operation.clone()) {
                return Err(ConstructionGraphFailure::DuplicateOperation(
                    operation.operation.clone(),
                ));
            }
        }
        let mut deps = Vec::new();
        for o in &canonical {
            for (role, source, ordinal) in o.request.dependencies() {
                if source.output.slot != OutputSlot::Primary {
                    return Err(ConstructionGraphFailure::UnknownOutputSlot(
                        source.output.clone(),
                    ));
                }
                let producer = canonical
                    .iter()
                    .find(|x| x.operation == source.output.operation)
                    .ok_or_else(|| {
                        ConstructionGraphFailure::UnknownDependency(source.output.operation.clone())
                    })?;
                let expected = match role {
                    InputRole::Left | InputRole::Right => Some(GeometryOutputCategory::Solid),
                    other => required(other),
                };
                if let Some(expected) = expected {
                    if producer.expected_output.category != expected {
                        return Err(ConstructionGraphFailure::InputKindMismatch {
                            consumer: o.operation.clone(),
                            role,
                            expected,
                            actual: producer.expected_output.category,
                        });
                    }
                }
                deps.push(ConstructionDependency {
                    consumer: o.operation.clone(),
                    role,
                    producer: source.output,
                    ordinal,
                })
            }
        }
        let mut remaining: Vec<_> = canonical.iter().map(|o| o.operation.clone()).collect();
        remaining.sort();
        let mut ordered = Vec::new();
        while !remaining.is_empty() {
            let next = remaining
                .iter()
                .find(|id| {
                    deps.iter()
                        .filter(|d| d.consumer == **id)
                        .all(|d| ordered.contains(&d.producer.operation))
                })
                .cloned();
            match next {
                Some(id) => {
                    remaining.retain(|x| x != &id);
                    ordered.push(id)
                }
                None => return Err(ConstructionGraphFailure::DependencyCycle(remaining)),
            }
        }
        let mut nodes = Vec::new();
        for (ordinal, operation) in ordered.into_iter().enumerate() {
            let index = canonical
                .binary_search_by_key(&operation, |candidate| candidate.operation.clone())
                .map_err(|_| ConstructionGraphFailure::UnknownDependency(operation.clone()))?;
            nodes.push(ConstructionNode {
                operation,
                kind: canonical[index].request.kind(),
                ordinal: ordinal as u32,
            });
        }
        Ok(Self {
            nodes,
            dependencies: deps,
        })
    }
    pub fn nodes(&self) -> &[ConstructionNode] {
        &self.nodes
    }
    pub fn dependencies(&self) -> &[ConstructionDependency] {
        &self.dependencies
    }
}
fn required(role: InputRole) -> Option<GeometryOutputCategory> {
    match role {
        InputRole::Profile | InputRole::Section => Some(GeometryOutputCategory::Profile),
        InputRole::Spine => Some(GeometryOutputCategory::Curve),
        InputRole::Left | InputRole::Right => None,
        InputRole::InstanceGeometry | InputRole::BodyGeometry => {
            Some(GeometryOutputCategory::Solid)
        }
        InputRole::Source => None,
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConstructionGraphFailure {
    DuplicateOperation(OperationIdentity),
    UnknownDependency(OperationIdentity),
    DependencyCycle(Vec<OperationIdentity>),
    UnknownOutputSlot(OperationOutputIdentity),
    InputKindMismatch {
        consumer: OperationIdentity,
        role: InputRole,
        expected: GeometryOutputCategory,
        actual: GeometryOutputCategory,
    },
    UnknownRoot(OperationIdentity),
    DuplicateRoot(SourceIdentity),
    DuplicateBodyOrdinal(u32),
    DomainBodyMismatch(SourceIdentity),
    CompositionMismatch(String),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IdentityFailure {
    EmptyIdentity,
    EmptyLedger,
    DuplicateOperation(OperationIdentity),
    DuplicateOutput(OperationOutputIdentity),
    MissingOperationBinding(OperationIdentity),
    ExtraOperationBinding(OperationIdentity),
    MissingEntityBinding(OperationIdentity),
    DuplicateEntityBinding(OperationIdentity),
    DuplicateProducedEntity(String),
    ExtraEntityBinding(OperationIdentity),
    SourceMismatch {
        request: SourceIdentity,
        ledger: SourceIdentity,
    },
    OutputMismatch {
        request: OperationOutputIdentity,
        ledger: OperationOutputIdentity,
    },
    UnknownProducedEntity(String),
}
