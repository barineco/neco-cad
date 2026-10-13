use crate::protocol::*;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone)]
pub struct Manifest {
    pub value: Value,
    pub seed: u64,
    pub shard: usize,
    pub sources: Vec<Value>,
}
pub fn draw(seed: u64, index: u64, slot: u64) -> u64 {
    let x = seed.wrapping_add(
        0x9e3779b97f4a7c15u64
            .wrapping_mul(index.wrapping_mul(9).wrapping_add(slot).wrapping_add(1)),
    );
    let z = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    let z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}
const DERIVED_FAMILY_CASE_LIMIT: u64 = 1_000_000;
fn source_family(id: &str) -> &'static str {
    if id == "cad-typed-failure" {
        "exact-chart"
    } else {
        "mesh-judgment"
    }
}
fn source_families(source: &Value) -> Vec<String> {
    let id = source["id"].as_str().unwrap_or_default();
    if id == "cad" {
        let mut families: Vec<String> =
            FAMILIES.iter().map(|family| (*family).to_owned()).collect();
        families.sort();
        families
    } else {
        vec![source_family(id).to_owned()]
    }
}
struct BucketRule {
    slot: usize,
    modulus: u64,
    names: Vec<String>,
    required: BTreeMap<String, u64>,
}
fn forbidden(value: &Value) -> bool {
    match value {
        Value::Object(o) => o.iter().any(|(k, v)| {
            [
                "sourceOID",
                "sourceOid",
                "sourceOids",
                "gitOid",
                "gitObjectId",
                "hostPath",
                "producerPath",
                "checkerPath",
                "observerPath",
                "workingDirectory",
                "temporaryDirectory",
                "reportDirectory",
                "binding",
            ]
            .contains(&k.as_str())
                || forbidden(v)
        }),
        Value::Array(a) => a.iter().any(forbidden),
        Value::String(s) => s.starts_with('/') || s == "binding",
        _ => false,
    }
}
impl Manifest {
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let v = decode(bytes, "decodeLogicalManifest")?;
        fields(
            &v,
            &[
                "schema",
                "protocolRevision",
                "generator",
                "domains",
                "coverage",
                "lanes",
                "timeoutPolicy",
                "sourceRefs",
            ],
            "manifestInvalid",
            "decodeLogicalManifest",
        )?;
        require(
            !forbidden(&v)
                && v["schema"] == "neco.verification.logical-manifest.v1"
                && v["protocolRevision"] == "protocol-v1",
            "manifestInvalid",
            "decodeLogicalManifest",
            "logical manifest schema or forbidden host identity",
        )?;
        let g = &v["generator"];
        let explicit_count = g
            .get("normalCaseCount")
            .or_else(|| g.get("allocation"))
            .is_some();
        let mut generator_fields = vec![
            "revision",
            "algorithm",
            "drawScheduleRevision",
            "seed",
            "tier",
            "shardCaseCount",
        ];
        if explicit_count {
            generator_fields.extend(["normalCaseCount", "allocation"]);
        }
        fields(
            g,
            &generator_fields,
            "manifestInvalid",
            "decodeLogicalManifest",
        )?;
        require(
            g["revision"] == "generator-v1"
                && g["algorithm"] == "splitmix64-v1"
                && g["drawScheduleRevision"] == "draw-schedule-v1"
                && g["tier"] == "smoke"
                && (!explicit_count
                    || g["normalCaseCount"]
                        .as_u64()
                        .is_some_and(|n| n > 0 && g["allocation"] == json!({"neco.cad":n}))),
            "manifestInvalid",
            "decodeLogicalManifest",
            "CAD normal allocation must equal the declared normal case count",
        )?;
        let seed = string(g, "seed", "decodeLogicalManifest")?;
        require(
            hex(seed, 16),
            "manifestInvalid",
            "decodeLogicalManifest",
            "seed must be hex16",
        )?;
        let seed = u64::from_str_radix(seed, 16)
            .map_err(|e| Failure::new("manifestInvalid", "decodeLogicalManifest", e))?;
        let shard = g["shardCaseCount"].as_u64().ok_or_else(|| {
            Failure::new(
                "manifestInvalid",
                "decodeLogicalManifest",
                "shard size required",
            )
        })? as usize;
        require(
            shard > 0 && shard <= 200,
            "manifestInvalid",
            "decodeLogicalManifest",
            "shard must be 1..200",
        )?;
        let lanes: BTreeSet<_> = array(&v, "lanes", "decodeLogicalManifest")?
            .iter()
            .filter_map(Value::as_str)
            .collect();
        require(
            lanes == LANES.into_iter().collect()
                && array(&v, "lanes", "decodeLogicalManifest")?.len() == 7,
            "manifestInvalid",
            "decodeLogicalManifest",
            "all seven lanes exactly once required",
        )?;
        let domains = array(&v, "domains", "decodeLogicalManifest")?;
        require(
            domains.len() == 1 && domains[0]["domain"] == "neco.cad",
            "manifestInvalid",
            "decodeLogicalManifest",
            "CAD is an independent single-domain completion",
        )?;
        require(
            domains[0]["semanticsRef"]["declarationDigest"]
                .as_str()
                .is_some_and(|s| hex(s, 64)),
            "manifestInvalid",
            "decodeLogicalManifest",
            "real semantics declaration digest required",
        )?;
        let domain = &domains[0];
        fields(
            domain,
            &[
                "adapterRevision",
                "checkerApiRevision",
                "coverageMinimum",
                "domain",
                "families",
                "observationSchema",
                "producerApiRevision",
                "projectionRevision",
                "requestSchema",
                "semanticsRef",
            ],
            "manifestInvalid",
            "decodeLogicalManifest",
        )?;
        require(
            domain["observationSchema"] == "neco.cad.observation-batch.v1"
                && domain["requestSchema"] == "neco.verification.generated-request-batch.v1"
                && [
                    "adapterRevision",
                    "checkerApiRevision",
                    "producerApiRevision",
                ]
                .iter()
                .all(|k| domain[*k].as_str().is_some_and(|s| !s.is_empty()))
                && domain["projectionRevision"]
                    .as_str()
                    .is_some_and(|s| hex(s, 64)),
            "manifestInvalid",
            "decodeLogicalManifest",
            "selected domain schema and revision references required",
        )?;
        let semantics = &domain["semanticsRef"];
        fields(
            semantics,
            &[
                "checkerRevision",
                "declarationDigest",
                "protocolRevision",
                "sharedSemanticsRefs",
            ],
            "manifestInvalid",
            "decodeLogicalManifest",
        )?;
        require(
            semantics["protocolRevision"] == v["protocolRevision"]
                && semantics["checkerRevision"] == domain["checkerApiRevision"],
            "manifestInvalid",
            "decodeLogicalManifest",
            "nested protocol/checker revision differs",
        )?;
        let shared = array(semantics, "sharedSemanticsRefs", "decodeLogicalManifest")?;
        require(
            shared.len() == 3,
            "manifestInvalid",
            "decodeLogicalManifest",
            "three upstream shared semantic references required",
        )?;
        let mut revisions = BTreeSet::new();
        for reference in shared {
            fields(
                reference,
                &["revision", "declarationDigest"],
                "manifestInvalid",
                "decodeLogicalManifest",
            )?;
            require(
                reference["declarationDigest"]
                    .as_str()
                    .is_some_and(|s| hex(s, 64))
                    && reference["revision"]
                        .as_str()
                        .is_some_and(|s| !s.is_empty() && revisions.insert(s)),
                "manifestInvalid",
                "decodeLogicalManifest",
                "shared references must have unique revisions and real digests",
            )?;
        }
        let families = array(domain, "families", "decodeLogicalManifest")?;
        require(
            families.len() == 5
                && families
                    .iter()
                    .filter_map(|f| f["family"].as_str())
                    .collect::<BTreeSet<_>>()
                    == FAMILIES.into_iter().collect(),
            "manifestInvalid",
            "decodeLogicalManifest",
            "all five normal families required",
        )?;
        for family in families {
            fields(
                family,
                &["family", "minimum", "requestedObservationKinds", "weight"],
                "manifestInvalid",
                "decodeLogicalManifest",
            )?;
            require(
                family["minimum"].as_u64().is_some_and(|n| n > 0)
                    && family["requestedObservationKinds"] == json!(["success"])
                    && family["weight"] == 1,
                "manifestInvalid",
                "decodeLogicalManifest",
                "normal family allocation differs",
            )?;
        }
        fields(
            &v["coverage"],
            &["invalidProtocolExpectedVerdict", "mutationExpectedVerdict"],
            "manifestInvalid",
            "decodeLogicalManifest",
        )?;
        require(
            v["coverage"]["invalidProtocolExpectedVerdict"] == "invalid"
                && v["coverage"]["mutationExpectedVerdict"] == "refuted",
            "manifestInvalid",
            "decodeLogicalManifest",
            "control verdict declarations differ",
        )?;
        fields(
            &v["timeoutPolicy"],
            &["laneTimeoutSeconds", "shardTimeoutSeconds"],
            "manifestInvalid",
            "decodeLogicalManifest",
        )?;
        require(
            v["timeoutPolicy"]["laneTimeoutSeconds"]
                .as_u64()
                .is_some_and(|n| n > 0)
                && v["timeoutPolicy"]["shardTimeoutSeconds"]
                    .as_u64()
                    .is_some_and(|n| n > 0),
            "manifestInvalid",
            "decodeLogicalManifest",
            "positive timeouts required",
        )?;
        let sources = array(&v, "sourceRefs", "decodeLogicalManifest")?.clone();
        let mut ids = BTreeSet::new();
        for source in &sources {
            let id = string(source, "id", "decodeLogicalManifest")?;
            let mut source_fields = vec!["id", "requestedObservationKind", "expectedVerdict"];
            if source.get("familyCounts").is_some() {
                source_fields.push("familyCounts");
            }
            fields(
                source,
                &source_fields,
                "manifestInvalid",
                "decodeLogicalManifest",
            )?;
            require(
                ids.insert(id) && ["cad", "cad-typed-failure", "cad-mesh-rejected"].contains(&id),
                "manifestInvalid",
                "decodeLogicalManifest",
                "unknown or duplicate corpus ID",
            )?;
            if id == "cad" {
                require(
                    source["requestedObservationKind"] == "success"
                        && source["expectedVerdict"] == "verified"
                        && source.get("familyCounts").is_some() == explicit_count,
                    "manifestInvalid",
                    "decodeLogicalManifest",
                    "normal corpus requires success requests, verified verdicts and counts exactly when the generator declares them",
                )?;
                if let Some(counts) = source.get("familyCounts") {
                    let counts = counts.as_object().ok_or_else(|| {
                        Failure::new(
                            "manifestInvalid",
                            "decodeLogicalManifest",
                            "familyCounts object required",
                        )
                    })?;
                    let mut total = 0u64;
                    for family in families {
                        let name = family["family"].as_str().unwrap_or_default();
                        let count = counts.get(name).and_then(Value::as_u64);
                        require(
                            count.is_some_and(|n| Some(n) >= family["minimum"].as_u64()),
                            "manifestInvalid",
                            "decodeLogicalManifest",
                            "explicit family count must reach the family minimum",
                        )?;
                        total += count.unwrap_or(0);
                    }
                    require(
                        counts.len() == FAMILIES.len()
                            && Some(total) == g["normalCaseCount"].as_u64(),
                        "manifestInvalid",
                        "decodeLogicalManifest",
                        "explicit family counts must sum to the declared normal case count",
                    )?;
                }
            } else {
                let family = source_family(id);
                require(
                    source["requestedObservationKind"] == "failure"
                        && source["expectedVerdict"]
                            == if id == "cad-typed-failure" {
                                "undetermined"
                            } else {
                                "verified"
                            },
                    "manifestInvalid",
                    "decodeLogicalManifest",
                    "failure corpus kind/verdict declaration differs",
                )?;
                if let Some(counts) = source.get("familyCounts") {
                    require(
                        counts.as_object().is_some_and(|counts| {
                            counts.len() == 1
                                && counts
                                    .get(family)
                                    .and_then(Value::as_u64)
                                    .is_some_and(|n| n > 0)
                        }),
                        "manifestInvalid",
                        "decodeLogicalManifest",
                        "failure corpus family count differs",
                    )?;
                }
            }
        }
        require(
            ids.contains("cad") && ids.contains("cad-typed-failure"),
            "manifestInvalid",
            "decodeLogicalManifest",
            "normal and typed failure corpora required",
        )?;
        Ok(Self {
            value: v,
            seed,
            shard,
            sources,
        })
    }
    fn bucket_rules(
        &self,
        descriptor: &Value,
        family: &str,
        kind: &Value,
    ) -> Result<Vec<BucketRule>> {
        let mut rules = Vec::new();
        let minima = array(&self.value["domains"][0], "coverageMinimum", "derive")?;
        for p in array(
            &descriptor["projectionClaim"],
            "projectionDeclarations",
            "derive",
        )? {
            if p["family"] != family {
                continue;
            }
            for b in array(p, "selectorBindings", "derive")? {
                if let Some(kinds) = b.get("requestedObservationKinds").and_then(Value::as_array) {
                    if !kinds.contains(kind) {
                        continue;
                    }
                }
                let rule = &b["selectorRule"];
                let slot = SELECTORS
                    .iter()
                    .position(|name| rule["selector"] == *name)
                    .ok_or_else(|| {
                        Failure::new(
                            "descriptorMismatch",
                            "derive",
                            "selectorRule selector is unbound",
                        )
                    })?;
                let modulus = rule["modulus"].as_u64().filter(|n| *n > 0).ok_or_else(|| {
                    Failure::new("descriptorMismatch", "derive", "positive modulus required")
                })?;
                let mut names = Vec::new();
                for remainder in 0..modulus {
                    let name = rule["bucketsByRemainder"][remainder.to_string()]
                        .as_str()
                        .ok_or_else(|| {
                            Failure::new(
                                "descriptorMismatch",
                                "derive",
                                "remainder map must be total",
                            )
                        })?;
                    names.push(name.to_owned());
                }
                let mut required = BTreeMap::new();
                for name in &names {
                    let declared = minima
                        .iter()
                        .filter(|m| {
                            m["family"] == family && m["axis"] == b["axis"] && m["bucket"] == *name
                        })
                        .filter_map(|m| m["minimum"].as_u64())
                        .max()
                        .unwrap_or(0);
                    required.insert(name.clone(), declared.max(1));
                }
                rules.push(BucketRule {
                    slot,
                    modulus,
                    names,
                    required,
                });
            }
        }
        Ok(rules)
    }
    fn derive_count(
        &self,
        descriptor: Option<&Value>,
        start: u64,
        family: &str,
        kind: &Value,
        floor: u64,
    ) -> Result<u64> {
        let rules = match descriptor {
            Some(descriptor) => self.bucket_rules(descriptor, family, kind)?,
            None => Vec::new(),
        };
        let floor = if descriptor.is_some() {
            floor
        } else {
            floor
                + array(&self.value["domains"][0], "coverageMinimum", "derive")?
                    .iter()
                    .filter(|m| m["family"] == family)
                    .filter_map(|m| m["minimum"].as_u64())
                    .sum::<u64>()
        };
        let mut seen: Vec<BTreeMap<String, u64>> = vec![BTreeMap::new(); rules.len()];
        let mut count = 0u64;
        loop {
            let covered = count >= floor
                && rules.iter().zip(&seen).all(|(rule, seen)| {
                    rule.required
                        .iter()
                        .all(|(name, need)| seen.get(name).copied().unwrap_or(0) >= *need)
                });
            if covered {
                return Ok(count);
            }
            require(
                count < DERIVED_FAMILY_CASE_LIMIT,
                "coverageMissing",
                "derive",
                "declared selector coverage is not reached within the derivation limit",
            )?;
            for (rule, seen) in rules.iter().zip(seen.iter_mut()) {
                let value = draw(self.seed, start + count, rule.slot as u64);
                *seen
                    .entry(rule.names[(value % rule.modulus) as usize].clone())
                    .or_default() += 1;
            }
            count += 1;
        }
    }
    pub fn plan(&self, descriptor: Option<&Value>) -> Result<Vec<(usize, String, u64)>> {
        let mut plan = Vec::new();
        let mut index = 0u64;
        for (position, source) in self.sources.iter().enumerate() {
            for family in source_families(source) {
                let count = match source["familyCounts"].get(&family).and_then(Value::as_u64) {
                    Some(count) => count,
                    None => {
                        let floor = if source["id"] == "cad" {
                            array(&self.value["domains"][0], "families", "plan")?
                                .iter()
                                .find(|f| f["family"] == family.as_str())
                                .and_then(|f| f["minimum"].as_u64())
                                .unwrap_or(1)
                        } else {
                            1
                        };
                        self.derive_count(
                            descriptor,
                            index,
                            &family,
                            &source["requestedObservationKind"],
                            floor,
                        )?
                    }
                };
                plan.push((position, family, count));
                index += count;
            }
        }
        Ok(plan)
    }
    pub fn batches(
        &self,
        shard: usize,
        descriptor: Option<&Value>,
    ) -> Result<Vec<(String, Value)>> {
        require(
            shard > 0 && shard <= 200,
            "shardExceedsProducerMax",
            "generate",
            "shard size must be 1..200",
        )?;
        let plan = self.plan(descriptor)?;
        let mut batches = Vec::new();
        let mut index = 0u64;
        let request = |cases: Vec<Value>| {
            let mut request = json!({"schema":"neco.verification.generated-request-batch.v1","domain":"neco.cad","adapterRevision":self.value["domains"][0]["adapterRevision"],"generator":{"algorithm":"splitmix64-v1","drawScheduleRevision":"draw-schedule-v1","seed":format!("{:016x}",self.seed)},"cases":[]});
            request["cases"] = Value::Array(cases);
            request
        };
        for (position, source) in self.sources.iter().enumerate() {
            let mut cases = Vec::new();
            for (_, family, count) in plan.iter().filter(|(p, _, _)| *p == position) {
                for _ in 0..*count {
                    let selectors: serde_json::Map<String, Value> = SELECTORS
                        .iter()
                        .enumerate()
                        .map(|(slot, name)| {
                            (
                                (*name).into(),
                                json!(format!("{:016x}", draw(self.seed, index, slot as u64))),
                            )
                        })
                        .collect();
                    let mut case = json!({"caseId":format!("case-{index:016x}"),"globalCaseIndex":format!("{index:016x}"),"domainCaseIndex":format!("{index:016x}"),"family":family,"requestedObservationKind":source["requestedObservationKind"],"selectors":{}});
                    case["selectors"] = Value::Object(selectors);
                    cases.push(case);
                    index = index.checked_add(1).ok_or_else(|| {
                        Failure::new("manifestInvalid", "generate", "case index overflow")
                    })?;
                    if cases.len() == shard {
                        batches.push((
                            source["id"].as_str().unwrap().to_owned(),
                            request(std::mem::take(&mut cases)),
                        ));
                    }
                }
            }
            if !cases.is_empty() {
                batches.push((source["id"].as_str().unwrap().to_owned(), request(cases)));
            }
        }
        Ok(batches)
    }
}
pub fn coverage(manifest: &Manifest, descriptor: &Value, shard: usize) -> Result<Value> {
    require(
        shard > 0
            && descriptor["resourceClaim"]["maxBatchCases"]
                .as_u64()
                .is_some_and(|n| shard as u64 <= n),
        "shardExceedsProducerMax",
        "verifyCoverage",
        "shard exceeds actual descriptor max",
    )?;
    let projections = array(
        &descriptor["projectionClaim"],
        "projectionDeclarations",
        "verifyCoverage",
    )?;
    require(
        projections.len() == 5,
        "descriptorMismatch",
        "verifyCoverage",
        "five projection families required",
    )?;
    let mut rows = Vec::new();
    let batches = manifest.batches(shard, Some(descriptor))?;
    for source in &manifest.sources {
        let source_families = source_families(source);
        let mut declared_axes = BTreeSet::new();
        for p in projections {
            let family = string(p, "family", "verifyCoverage")?;
            if !source_families.iter().any(|name| name == family) {
                continue;
            }
            let bindings = array(p, "selectorBindings", "verifyCoverage")?;
            let axes = array(p, "applicableCoverageAxes", "verifyCoverage")?;
            require(
                axes.iter()
                    .filter_map(Value::as_str)
                    .collect::<BTreeSet<_>>()
                    == bindings
                        .iter()
                        .filter_map(|b| b["axis"].as_str())
                        .collect::<BTreeSet<_>>(),
                "descriptorMismatch",
                "verifyCoverage",
                "every applicable coverage axis must have a selector rule",
            )?;
            for b in bindings {
                if let Some(kinds) = b.get("requestedObservationKinds") {
                    let kinds = kinds.as_array().filter(|a| !a.is_empty()).ok_or_else(|| {
                        Failure::new(
                            "descriptorMismatch",
                            "verifyCoverage",
                            "applicable request kinds must be a nonempty array",
                        )
                    })?;
                    require(
                        kinds.iter().all(|k| k == "success" || k == "failure"),
                        "descriptorMismatch",
                        "verifyCoverage",
                        "unknown applicability kind",
                    )?;
                    if !kinds.contains(&source["requestedObservationKind"]) {
                        continue;
                    }
                }
                let rule = &b["selectorRule"];
                fields(
                    rule,
                    &["selector", "modulus", "bucketsByRemainder"],
                    "descriptorMismatch",
                    "verifyCoverage",
                )?;
                let slot = string(rule, "selector", "verifyCoverage")?;
                require(
                    SELECTORS.contains(&slot) && b["selector"] == slot,
                    "descriptorMismatch",
                    "verifyCoverage",
                    "selectorRule selector is unbound",
                )?;
                let modulus = rule["modulus"].as_u64().filter(|n| *n > 0).ok_or_else(|| {
                    Failure::new(
                        "descriptorMismatch",
                        "verifyCoverage",
                        "positive modulus required",
                    )
                })?;
                let buckets = rule["bucketsByRemainder"].as_object().ok_or_else(|| {
                    Failure::new(
                        "descriptorMismatch",
                        "verifyCoverage",
                        "total remainder map required",
                    )
                })?;
                require(
                    buckets.len() as u64 == modulus
                        && (0..modulus)
                            .all(|n| buckets.get(&n.to_string()).is_some_and(Value::is_string)),
                    "descriptorMismatch",
                    "verifyCoverage",
                    "selectorRule remainder map must be total",
                )?;
                let mut counts = std::collections::BTreeMap::<String, u64>::new();
                for (id, batch) in &batches {
                    if id != source["id"].as_str().unwrap() {
                        continue;
                    }
                    for c in array(batch, "cases", "verifyCoverage")? {
                        if c["family"] != family {
                            continue;
                        }
                        let value = u64::from_str_radix(
                            string(&c["selectors"], slot, "verifyCoverage")?,
                            16,
                        )
                        .map_err(|e| Failure::new("descriptorMismatch", "verifyCoverage", e))?;
                        let bucket = buckets[&(value % modulus).to_string()].as_str().unwrap();
                        *counts.entry(bucket.into()).or_default() += 1;
                    }
                }
                require(
                    buckets
                        .values()
                        .all(|name| counts.get(name.as_str().unwrap()).copied().unwrap_or(0) > 0),
                    "coverageMissing",
                    "verifyCoverage",
                    "declared selector bucket was not generated",
                )?;
                for minimum in array(
                    &manifest.value["domains"][0],
                    "coverageMinimum",
                    "verifyCoverage",
                )? {
                    fields(
                        minimum,
                        &["family", "axis", "bucket", "minimum"],
                        "manifestInvalid",
                        "verifyCoverage",
                    )?;
                    if minimum["family"] == family && minimum["axis"] == b["axis"] {
                        let bucket = string(minimum, "bucket", "verifyCoverage")?;
                        let minimum = minimum["minimum"].as_u64().ok_or_else(|| {
                            Failure::new(
                                "manifestInvalid",
                                "verifyCoverage",
                                "coverage minimum must be u64",
                            )
                        })?;
                        require(
                            counts.get(bucket).copied().unwrap_or(0) >= minimum,
                            "coverageMissing",
                            "verifyCoverage",
                            "declared selector coverage minimum unmet",
                        )?;
                        declared_axes.insert((
                            family.to_owned(),
                            b["axis"].as_str().unwrap().to_owned(),
                            bucket.to_owned(),
                        ));
                    }
                }
                rows.push(json!({"corpusId":source["id"],"family":family,"axis":b["axis"],"selector":slot,"counts":counts,"evidenceKind":"requestSelectorCoverage"}));
            }
        }
        for minimum in array(
            &manifest.value["domains"][0],
            "coverageMinimum",
            "verifyCoverage",
        )? {
            let family = string(minimum, "family", "verifyCoverage")?;
            if source_families.iter().any(|name| name == family) {
                require(
                    declared_axes.contains(&(
                        family.to_owned(),
                        string(minimum, "axis", "verifyCoverage")?.to_owned(),
                        string(minimum, "bucket", "verifyCoverage")?.to_owned(),
                    )),
                    "coverageMissing",
                    "verifyCoverage",
                    "coverage minimum references an unbound descriptor bucket",
                )?;
            }
        }
    }
    Ok(
        json!({"schema":"neco.cad.coverage-report.v1","status":"passed","shardCaseCount":shard,"producerMaxBatchCases":descriptor["resourceClaim"]["maxBatchCases"],"buckets":rows}),
    )
}
