use crate::{
    protocol::*,
    reconciliation,
    runtime::{CorpusRun, Runtime},
};
use serde_json::{json, Value};
use std::{collections::BTreeMap, fs};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Q {
    n: i128,
    d: i128,
}
fn overflow() -> Failure {
    Failure::new(
        "controlPreconditionFailure",
        "exactControlArithmetic",
        "exact i128 operation overflow",
    )
}
fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}
impl Q {
    fn new(n: i128, d: i128) -> Result<Self> {
        require(
            d > 0,
            "controlPreconditionFailure",
            "exactControlArithmetic",
            "positive denominator required",
        )?;
        let g = gcd(n.unsigned_abs(), d as u128) as i128;
        Ok(Self { n: n / g, d: d / g })
    }
    fn from(v: &Value) -> Result<Self> {
        if let Some(n) = v["numerator"].as_str() {
            Self::new(
                n.parse().map_err(|_| overflow())?,
                string(v, "denominator", "exactControlArithmetic")?
                    .parse()
                    .map_err(|_| overflow())?,
            )
        } else {
            let n = string(v, "integer", "exactControlArithmetic")?
                .parse()
                .map_err(|_| overflow())?;
            let e = v["exponent"]
                .as_u64()
                .filter(|e| *e < 127)
                .ok_or_else(overflow)?;
            Self::new(n, 1i128.checked_shl(e as u32).ok_or_else(overflow)?)
        }
    }
    fn add(self, b: Self) -> Result<Self> {
        let g = gcd(self.d as u128, b.d as u128) as i128;
        let x = b.d / g;
        let y = self.d / g;
        Self::new(
            self.n
                .checked_mul(x)
                .and_then(|a| b.n.checked_mul(y).and_then(|b| a.checked_add(b)))
                .ok_or_else(overflow)?,
            self.d.checked_mul(x).ok_or_else(overflow)?,
        )
    }
    fn sub(self, b: Self) -> Result<Self> {
        self.add(Self {
            n: b.n.checked_neg().ok_or_else(overflow)?,
            d: b.d,
        })
    }
    fn mul(self, b: Self) -> Result<Self> {
        let a = gcd(self.n.unsigned_abs(), b.d as u128) as i128;
        let c = gcd(b.n.unsigned_abs(), self.d as u128) as i128;
        Self::new(
            (self.n / a).checked_mul(b.n / c).ok_or_else(overflow)?,
            (self.d / c).checked_mul(b.d / a).ok_or_else(overflow)?,
        )
    }
    fn div(self, b: Self) -> Result<Self> {
        require(
            b.n != 0,
            "controlPreconditionFailure",
            "exactControlArithmetic",
            "denominator polynomial must be nonzero",
        )?;
        self.mul(Self::new(
            if b.n < 0 { -b.d } else { b.d },
            b.n.checked_abs().ok_or_else(overflow)?,
        )?)
    }
    fn pow(self, n: u64) -> Result<Self> {
        let mut x = Self { n: 1, d: 1 };
        for _ in 0..n {
            x = x.mul(self)?;
        }
        Ok(x)
    }
    fn value(self) -> Value {
        json!({"numerator":self.n.to_string(),"denominator":self.d.to_string()})
    }
    fn abs_greater(self, b: Self) -> Result<bool> {
        Ok(self
            .n
            .checked_abs()
            .and_then(|n| n.checked_mul(b.d))
            .ok_or_else(overflow)?
            > b.n
                .checked_abs()
                .and_then(|n| n.checked_mul(self.d))
                .ok_or_else(overflow)?)
    }
}
fn polynomial(terms: &Value, variables: &[(&str, Q)]) -> Result<Q> {
    let mut sum = Q { n: 0, d: 1 };
    for term in terms.as_array().ok_or_else(|| {
        Failure::new(
            "controlPreconditionFailure",
            "exactControlArithmetic",
            "polynomial array required",
        )
    })? {
        let mut term_value = Q::from(&term["coefficient"])?;
        for (key, x) in variables {
            term_value = term_value.mul(x.pow(term[*key].as_u64().ok_or_else(overflow)?)?)?;
        }
        sum = sum.add(term_value)?;
    }
    Ok(sum)
}
fn zero_atom() -> Value {
    json!({"coefficient":{"numerator":"0","denominator":"1"},"exponentX":0,"exponentY":0,"exponentZ":0})
}
fn zero_surface() -> Value {
    json!({"coefficient":{"numerator":"0","denominator":"1"},"exponentU":0,"exponentV":0})
}
fn push(poly: &mut Value, term: Value) -> Result<()> {
    poly.as_array_mut()
        .ok_or_else(|| {
            Failure::new(
                "controlPreconditionFailure",
                "mutateObservation",
                "polynomial array required",
            )
        })?
        .push(term);
    Ok(())
}
pub fn mutations(original: &Value, family: &str) -> Result<(Value, Value, Value)> {
    let mut bad = original.clone();
    let mut equivalent = original.clone();
    let mut witness = json!({"mathematicalChange":"non-equivalent","equivalentChange":"zero coefficient insertion"});
    let b = &mut bad["cases"][0]["publicObservation"]["value"];
    let e = &mut equivalent["cases"][0]["publicObservation"]["value"];
    match family {
        "exact-chart" => {
            let mut one = zero_atom();
            one["coefficient"]["numerator"] = json!("1");
            push(&mut b["atom"], one)?;
            push(&mut e["atom"], zero_atom())?;
            witness["changedField"] = json!("atom:+1");
        }
        "face-atom-covering" => {
            let v = &mut b["clauses"][0]["imageQuotients"]["x"][0]["coefficient"];
            let changed = Q::from(v)?.add(Q { n: 1, d: 1 })?;
            *v = changed.value();
            push(&mut e["clauses"][0]["guards"][0], zero_atom())?;
            witness["changedField"] = json!("clauses[0].imageQuotients.x[0].coefficient:+1");
        }
        "finite-surface-representation" => {
            let triangle = b["faceTriangles"][0]["triangleIndices"][0]
                .as_u64()
                .ok_or_else(overflow)? as usize;
            let vertex = b["mesh"]["triangles"][triangle][0]
                .as_u64()
                .ok_or_else(overflow)? as usize;
            let bound = Q::from(&b["faceTriangles"][0]["residualBound"])?;
            let mut chosen = None;
            for power in 1..=16 {
                let x = Q {
                    n: 1i128 << power,
                    d: 1,
                };
                let residual = polynomial(
                    &b["faceTriangles"][0]["atom"],
                    &[
                        ("exponentX", x),
                        ("exponentY", Q { n: 0, d: 1 }),
                        ("exponentZ", Q { n: 0, d: 1 }),
                    ],
                )?;
                if residual.abs_greater(bound)? {
                    chosen = Some((x, residual));
                    break;
                }
            }
            let (x, residual) = chosen.ok_or_else(|| {
                Failure::new(
                    "coverageMissing",
                    "finiteSurfaceMutation",
                    "no actual atom point beyond residual certificate found",
                )
            })?;
            b["mesh"]["vertices"][vertex] = json!({"x":{"integer":x.n.to_string(),"exponent":0},"y":{"integer":"0","exponent":0},"z":{"integer":"0","exponent":0}});
            push(&mut e["faceTriangles"][0]["atom"], zero_atom())?;
            witness = json!({"changedField":format!("mesh.vertices[{vertex}]"),"exactAtomResidual":residual.value(),"declaredResidualBound":bound.value(),"residualExceedsBound":true,"equivalentChange":"zero atom term insertion"});
        }
        "certified-chord-bound" => {
            let mut selected = None;
            let mut edges: Vec<_> = array(b, "edges", "chordMutation")?
                .iter()
                .enumerate()
                .collect();
            edges.sort_by_key(|(_, edge)| {
                ["left", "right"]
                    .iter()
                    .flat_map(|end| {
                        ["u", "v"]
                            .iter()
                            .map(move |axis| canonical(&edge[*end][*axis]).len())
                    })
                    .sum::<usize>()
            });
            for (index, edge) in edges {
                if edge["parameterization"]["kind"] != "line" {
                    continue;
                }
                let patch = edge["patchIndex"].as_u64().ok_or_else(overflow)? as usize;
                let chart = &b["patches"][patch]["chart"];
                let u = Q::from(&edge["left"]["u"])?
                    .add(Q::from(&edge["right"]["u"])?)?
                    .div(Q { n: 2, d: 1 })?;
                let v = Q::from(&edge["left"]["v"])?
                    .add(Q::from(&edge["right"]["v"])?)?
                    .div(Q { n: 2, d: 1 })?;
                let vars = [("exponentU", u), ("exponentV", v)];
                let denominator = polynomial(&chart["denominator"], &vars)?;
                let mut squared = Q { n: 0, d: 1 };
                for axis in ["x", "y", "z"] {
                    let point = polynomial(&chart[axis], &vars)?.div(denominator)?;
                    let chord = Q::from(&edge["left"]["point"][axis])?
                        .add(Q::from(&edge["right"]["point"][axis])?)?
                        .div(Q { n: 2, d: 1 })?;
                    let deviation = point.sub(chord)?;
                    squared = squared.add(deviation.mul(deviation)?)?;
                }
                if squared.n > 0 {
                    selected = Some((index, squared));
                    break;
                }
            }
            let (index, squared) = selected.ok_or_else(|| {
                Failure::new(
                    "coverageMissing",
                    "chordMutation",
                    "actual line edge has no positive exact midpoint deviation",
                )
            })?;
            b["edges"][index]["establishedSquaredBound"] = json!({"integer":"0","exponent":0});
            push(&mut e["patches"][0]["chart"]["denominator"], zero_surface())?;
            witness = json!({"changedField":format!("edges[{index}].establishedSquaredBound"),"exactMidpointSquaredDeviation":squared.value(),"changedSquaredBound":"0","equivalentChange":"zero chart denominator term insertion"});
        }
        "mesh-judgment" => {
            let a = b["mesh"]["triangles"][0]
                .as_array_mut()
                .ok_or_else(overflow)?;
            a.swap(1, 2);
            for t in e["mesh"]["triangles"].as_array_mut().ok_or_else(overflow)? {
                t.as_array_mut().ok_or_else(overflow)?.rotate_left(1);
            }
            witness = json!({"changedField":"mesh.triangles[0]:orientation reversal","equivalentChange":"cyclic triangle rotations, real public API claim required"});
        }
        _ => {
            return Err(Failure::new(
                "coverageMissing",
                "mutateObservation",
                "unknown family",
            ))
        }
    }
    reconciliation::validate_case(&bad["cases"][0])?;
    reconciliation::validate_case(&equivalent["cases"][0])?;
    require(
        canonical(original) != canonical(&bad) && canonical(original) != canonical(&equivalent),
        "controlPreconditionFailure",
        "mutateObservation",
        "both mathematical controls must change canonical bytes",
    )?;
    Ok((bad, equivalent, witness))
}
fn expected(checker: &Value, verdict: &str) -> Result<()> {
    require(
        checker["cases"]
            .as_array()
            .is_some_and(|a| a.len() == 1 && a[0]["verdict"] == verdict),
        "unexpectedVerdict",
        "mutationRejection",
        "mathematical control must return requested per-case verdict",
    )
}
pub fn mutation_lane(runtime: &Runtime, run: &CorpusRun) -> Result<Value> {
    let mut rows = serde_json::Map::new();
    for family in FAMILIES {
        let original = run.samples.get(family).ok_or_else(|| {
            Failure::new(
                "coverageMissing",
                "mutationRejection",
                format!("no actual normal {family} case"),
            )
        })?;
        let (bad, mut equivalent, witness) = mutations(original, family)?;
        let mut api = Value::Null;
        if family == "mesh-judgment" {
            let mesh = &equivalent["cases"][0]["publicObservation"]["value"]["mesh"];
            let input = json!({"schema":"neco.cad.mesh-api-probe-request.v1","mesh":mesh,"source":{"identity":"neco-cad-equivalent-mesh","revision":runtime.descriptor["adapterRevision"]}});
            let path = runtime.temp.join("equivalent-mesh-probe-input.json");
            write(&path, &input)?;
            let program = runtime.control_probe.clone();
            let bytes = runtime.call(
                &program,
                &["mesh", "--input", path.to_str().unwrap()],
                "meshPublicApiProbe",
            )?;
            api = decode(&bytes, "meshPublicApiProbe")?;
            require(
                api["schema"] == "neco.cad.mesh-api-probe-result.v1"
                    && api["inputDigest"] == digest(&canonical(&input))
                    && api["mesh"] == *mesh
                    && api["outcome"]["kind"] == "validated",
                "parityMismatch",
                "meshPublicApiProbe",
                "cyclic equivalent API observation failed",
            )?;
            equivalent["cases"][0]["publicObservation"]["value"]["outcome"] =
                api["outcome"].clone();
        }
        let (checked, observer) = runtime.check(&canonical(&bad), &format!("mutation-{family}"))?;
        expected(&checked, "refuted")?;
        let (equal_checked, equal_observer) =
            runtime.check(&canonical(&equivalent), &format!("equivalent-{family}"))?;
        expected(&equal_checked, "verified")?;
        rows.insert(family.into(),json!({"classification":{"observed":"refuted","expected":"refuted"},"originalDigest":digest(&canonical(original)),"mutatedDigest":digest(&canonical(&bad)),"equivalentDigest":digest(&canonical(&equivalent)),"equivalentVerified":true,"equivalentClassification":{"observed":"verified","expected":"verified"},"checkerResult":checked,"observerResult":observer,"equivalentCheckerResult":equal_checked,"equivalentObserverResult":equal_observer,"controlPrecondition":witness,"publicApiObservation":api}));
    }
    Ok(Value::Object(rows))
}
fn corrupt_number(value: &mut Value) -> bool {
    match value {
        Value::Object(fields) => {
            for key in ["numerator", "integer"] {
                if fields.get(key).is_some_and(Value::is_string) {
                    fields.insert(key.into(), json!("01"));
                    return true;
                }
            }
            fields.values_mut().any(corrupt_number)
        }
        Value::Array(items) => items.iter_mut().any(corrupt_number),
        _ => false,
    }
}
pub fn invalid_lane(runtime: &Runtime, run: &CorpusRun) -> Result<Value> {
    let mut rows = Vec::new();
    for family in FAMILIES {
        let original = run.samples.get(family).ok_or_else(|| {
            Failure::new(
                "coverageMissing",
                "invalidProtocol",
                "missing family sample",
            )
        })?;
        let bytes = canonical(original);
        let mut vectors = BTreeMap::<String, Vec<u8>>::new();
        vectors.insert("nonUtf8".into(), vec![0xff, 0xfe, 0]);
        let mut whitespace = bytes.clone();
        whitespace.push(b' ');
        vectors.insert("noncanonicalBytes".into(), whitespace);
        let mut duplicate = bytes.clone();
        duplicate.splice(
            1..1,
            b"\"schema\":\"neco.cad.observation-batch.v1\","
                .iter()
                .copied(),
        );
        vectors.insert("duplicateKey".into(), duplicate);
        for (name, field, value) in [
            ("wrongSchema", "schema", json!("wrong")),
            ("unknownFamily", "operationFamily", json!("unknown")),
            ("familyKindMismatch", "kind", json!("wrong")),
            ("emptyId", "caseId", json!("")),
        ] {
            let mut v = original.clone();
            if field == "schema" {
                v[field] = value;
            } else if field == "kind" {
                v["cases"][0]["publicObservation"][field] = value;
            } else {
                v["cases"][0][field] = value;
            }
            vectors.insert(name.into(), canonical(&v));
        }
        let mut duplicate_id = original.clone();
        let c = duplicate_id["cases"][0].clone();
        duplicate_id["cases"].as_array_mut().unwrap().push(c);
        vectors.insert("duplicateId".into(), canonical(&duplicate_id));
        let mut malformed = original.clone();
        require(
            corrupt_number(&mut malformed["cases"][0]["publicObservation"]["value"]),
            "controlPreconditionFailure",
            "invalidProtocol",
            "no actual public numeric field to corrupt",
        )?;
        vectors.insert("malformedNumber".into(), canonical(&malformed));
        for (name, bytes) in vectors {
            let (checker, observer) = runtime.check(&bytes, &format!("invalid-{family}-{name}"))?;
            let classification = if checker["batchVerdict"] == "invalid" {
                "batchInvalid"
            } else {
                require(
                    checker["cases"]
                        .as_array()
                        .is_some_and(|a| a.iter().all(|c| c["verdict"] == "invalid")),
                    "unexpectedVerdict",
                    "invalidProtocol",
                    "invalid protocol control was not rejected",
                )?;
                "caseInvalid"
            };
            rows.push(json!({"family":family,"vector":name,"expected":"invalid","observed":"invalid","classification":classification,"inputDigest":digest(&bytes),"checkerResult":checker,"observerResult":observer}));
        }
        let (valid_checker, _) = runtime.check(
            &canonical(original),
            &format!("invalid-control-original-{family}"),
        )?;
        let mut wrong_digest = valid_checker.clone();
        wrong_digest["digest"] = json!("0".repeat(64));
        let mut missing = valid_checker.clone();
        missing["cases"] = json!([]);
        let mut unknown = valid_checker.clone();
        unknown["cases"][0]["verdict"] = json!("unknown");
        let mut reversed_batch = original.clone();
        let mut c = reversed_batch["cases"][0].clone();
        c["caseId"] = json!("control-reordered-second");
        reversed_batch["cases"].as_array_mut().unwrap().push(c);
        let (mut reversed_checker, _) = runtime.check(
            &canonical(&reversed_batch),
            &format!("reorder-original-{family}"),
        )?;
        reversed_checker["cases"].as_array_mut().unwrap().reverse();
        for (name, batch, result) in [
            ("wrongResultDigest", original.clone(), wrong_digest),
            ("missingResultCase", original.clone(), missing),
            ("unknownVerdict", original.clone(), unknown),
            ("reorderedResultCases", reversed_batch, reversed_checker),
        ] {
            let bp = runtime
                .temp
                .join(format!("result-control-{family}-{name}-batch.json"));
            let kp = runtime
                .temp
                .join(format!("result-control-{family}-{name}-checker.json"));
            write(&bp, &batch)?;
            write(&kp, &result)?;
            let output = runtime.invoke(
                &runtime.observer,
                &[
                    "--batch",
                    bp.to_str().unwrap(),
                    "--checker-result",
                    kp.to_str().unwrap(),
                ],
                "invalidObserverResult",
            )?;
            require(
                output.code == Some(2),
                "unexpectedProcessStatus",
                "invalidProtocol",
                "malformed checker result must be rejected with structured code2",
            )?;
            let failure = decode(&output.stderr, "invalidObserverResult")?;
            rows.push(json!({"family":family,"vector":name,"expected":"observerRejection","observed":"observerRejection","inputDigest":digest(&canonical(&batch)),"resultDigest":digest(&canonical(&result)),"exitCode":output.code,"failure":failure}));
        }
        let bp = runtime
            .temp
            .join(format!("result-bytes-{family}-batch.json"));
        write(&bp, original)?;
        let mut whitespace = canonical(&valid_checker);
        whitespace.push(b' ');
        let mut duplicate = canonical(&valid_checker);
        duplicate.splice(1..1, b"\"digest\":\"0\",".iter().copied());
        for (name, bytes) in [
            ("noncanonicalResultBytes", whitespace),
            ("duplicateResultKey", duplicate),
        ] {
            let kp = runtime
                .temp
                .join(format!("result-bytes-{family}-{name}.json"));
            fs::write(&kp, &bytes)
                .map_err(|e| Failure::new("processFailure", "invalidProtocol", e))?;
            let output = runtime.invoke(
                &runtime.observer,
                &[
                    "--batch",
                    bp.to_str().unwrap(),
                    "--checker-result",
                    kp.to_str().unwrap(),
                ],
                "invalidObserverResultBytes",
            )?;
            require(
                output.code == Some(2),
                "unexpectedProcessStatus",
                "invalidProtocol",
                "noncanonical result bytes must be rejected",
            )?;
            let failure = decode(&output.stderr, "invalidObserverResultBytes")?;
            rows.push(json!({"family":family,"vector":name,"expected":"observerRejection","observed":"observerRejection","resultDigest":digest(&bytes),"exitCode":output.code,"failure":failure}));
        }
    }
    Ok(Value::Array(rows))
}
