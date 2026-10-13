use crate::protocol::*;
use serde_json::{json, Value};
use std::collections::BTreeSet;

pub fn enumerate(batch: &Value) -> Result<Vec<&Value>> {
    fields(
        batch,
        &["cases", "proofSubjectRef", "schema", "testSetRef"],
        "batchInvalid",
        "decodeObservation",
    )?;
    require(
        batch["schema"] == "neco.cad.observation-batch.v1",
        "batchInvalid",
        "decodeObservation",
        "wrong observation schema",
    )?;
    for key in ["proofSubjectRef", "testSetRef"] {
        let r = &batch[key];
        fields(
            r,
            &["contentDigest", "versionIdentity"],
            "batchInvalid",
            "decodeObservation",
        )?;
        require(
            r["contentDigest"].as_str().is_some_and(|s| hex(s, 64))
                && r["versionIdentity"].as_str().is_some_and(|s| !s.is_empty()),
            "batchInvalid",
            "decodeObservation",
            "unbound declaration reference",
        )?;
    }
    let cases = array(batch, "cases", "decodeObservation")?;
    require(
        !cases.is_empty(),
        "batchInvalid",
        "decodeObservation",
        "empty observation batch",
    )?;
    let mut ids = BTreeSet::new();
    for c in cases {
        let id = c["caseId"].as_str().ok_or_else(|| {
            Failure::new(
                "batchInvalid",
                "decodeObservation",
                "every case must have a caseId",
            )
        })?;
        require(
            !id.is_empty() && id != "__digest__" && ids.insert(id),
            "batchInvalid",
            "decodeObservation",
            "every caseId must be nonempty unique non-sentinel",
        )?;
    }
    Ok(cases.iter().collect())
}
fn numeric_fields(value: &Value, keys: &[&str]) -> bool {
    value
        .as_object()
        .is_some_and(|map| map.len() == keys.len() && keys.iter().all(|key| map.contains_key(*key)))
}

fn decimal_coprime(numerator: &str, denominator: &str) -> bool {
    if denominator == "1" {
        return true;
    }
    let magnitude = numerator.strip_prefix('-').unwrap_or(numerator);
    if magnitude == "0" {
        return false;
    }
    if let (Ok(mut a), Ok(mut b)) = (magnitude.parse::<u128>(), denominator.parse::<u128>()) {
        while b != 0 {
            let remainder = a % b;
            a = b;
            b = remainder;
        }
        return a == 1;
    }
    let mut a: Vec<u8> = magnitude.bytes().map(|digit| digit - b'0').collect();
    let mut b: Vec<u8> = denominator.bytes().map(|digit| digit - b'0').collect();
    let mut remainder = Vec::with_capacity(b.len() + 1);
    while !b.is_empty() {
        remainder.clear();
        for &digit in &a {
            if remainder.is_empty() && digit == 0 {
                continue;
            }
            remainder.push(digit);
            while remainder.len() > b.len()
                || (remainder.len() == b.len() && remainder.as_slice() >= b.as_slice())
            {
                let mut borrow = 0i16;
                for offset in 0..remainder.len() {
                    let index = remainder.len() - 1 - offset;
                    let subtrahend = if offset < b.len() {
                        b[b.len() - 1 - offset]
                    } else {
                        0
                    };
                    let difference = i16::from(remainder[index]) - i16::from(subtrahend) - borrow;
                    if difference < 0 {
                        remainder[index] = (difference + 10) as u8;
                        borrow = 1;
                    } else {
                        remainder[index] = difference as u8;
                        borrow = 0;
                    }
                }
                let first = remainder
                    .iter()
                    .position(|digit| *digit != 0)
                    .unwrap_or(remainder.len());
                remainder.copy_within(first.., 0);
                remainder.truncate(remainder.len() - first);
            }
        }
        std::mem::swap(&mut a, &mut b);
        std::mem::swap(&mut b, &mut remainder);
    }
    a == [1]
}

fn rational_schema(value: &Value) -> bool {
    if !numeric_fields(value, &["numerator", "denominator"]) {
        return false;
    }
    match (value["numerator"].as_str(), value["denominator"].as_str()) {
        (Some(numerator), Some(denominator)) => {
            decimal(numerator, false)
                && decimal(denominator, true)
                && decimal_coprime(numerator, denominator)
        }
        _ => false,
    }
}

fn dyadic_schema(value: &Value) -> bool {
    if !numeric_fields(value, &["integer", "exponent"]) {
        return false;
    }
    match (value["integer"].as_str(), value["exponent"].as_u64()) {
        (Some(integer), Some(exponent))
            if decimal(integer, false) && exponent <= u64::from(u32::MAX) =>
        {
            if integer == "0" {
                exponent == 0
            } else {
                exponent == 0
                    || integer
                        .as_bytes()
                        .last()
                        .is_some_and(|digit| (*digit - b'0') % 2 == 1)
            }
        }
        _ => false,
    }
}

fn term_schema(value: &Value, exponents: &[&str]) -> bool {
    value.as_object().is_some_and(|map| {
        map.len() == exponents.len() + 1
            && map.contains_key("coefficient")
            && rational_schema(&value["coefficient"])
            && exponents.iter().all(|key| {
                value[*key]
                    .as_u64()
                    .is_some_and(|n| n <= u64::from(u32::MAX))
            })
    })
}

fn polynomial_schema(value: &Value, exponents: &[&str]) -> bool {
    value
        .as_array()
        .is_some_and(|terms| terms.iter().all(|term| term_schema(term, exponents)))
}

fn chart_schema(value: &Value) -> bool {
    numeric_fields(value, &["denominator", "x", "y", "z"])
        && ["denominator", "x", "y", "z"]
            .iter()
            .all(|key| polynomial_schema(&value[*key], &["exponentU", "exponentV"]))
}

fn generic_polynomial_schema(value: &Value) -> bool {
    let Some(terms) = value.as_array() else {
        return false;
    };
    let Some(first) = terms.first() else {
        return true;
    };
    if first.get("exponentU").is_some() || first.get("exponentV").is_some() {
        polynomial_schema(value, &["exponentU", "exponentV"])
    } else {
        polynomial_schema(value, &["exponentX", "exponentY", "exponentZ"])
    }
}

fn point_schema(value: &Value, dyadic: bool) -> bool {
    numeric_fields(value, &["x", "y", "z"])
        && ["x", "y", "z"].iter().all(|key| {
            if dyadic {
                dyadic_schema(&value[*key])
            } else {
                rational_schema(&value[*key])
            }
        })
}

fn numeric_member(key: &str, value: &Value) -> bool {
    match key {
        "chart" => chart_schema(value),
        "coefficient" | "constant" => rational_schema(value),
        "atom" | "atomQuotient" => {
            polynomial_schema(value, &["exponentX", "exponentY", "exponentZ"])
        }
        "polynomial" => generic_polynomial_schema(value),
        "guards" | "outerGuards" => value.as_array().is_some_and(|polynomials| {
            polynomials.iter().all(|polynomial| {
                polynomial_schema(polynomial, &["exponentX", "exponentY", "exponentZ"])
            })
        }),
        "imageQuotients" => {
            numeric_fields(value, &["x", "y", "z"])
                && ["x", "y", "z"].iter().all(|axis| {
                    polynomial_schema(&value[*axis], &["exponentX", "exponentY", "exponentZ"])
                })
        }
        "inverse" => {
            value.is_null()
                || (numeric_fields(
                    value,
                    &["uNumerator", "uDenominator", "vNumerator", "vDenominator"],
                ) && ["uNumerator", "uDenominator", "vNumerator", "vDenominator"]
                    .iter()
                    .all(|field| {
                        polynomial_schema(&value[*field], &["exponentX", "exponentY", "exponentZ"])
                    }))
        }
        "squares" => value.as_array().is_some_and(|entries| {
            entries.iter().all(|entry| {
                numeric_fields(entry, &["coefficient", "polynomial"])
                    && rational_schema(&entry["coefficient"])
                    && generic_polynomial_schema(&entry["polynomial"])
            })
        }),
        "domainPowers" | "guardPowers" => value.as_array().is_some_and(|entries| {
            entries.iter().all(|entry| {
                numeric_fields(entry, &["coefficient", "powers"])
                    && rational_schema(&entry["coefficient"])
                    && entry["powers"].as_array().is_some_and(|powers| {
                        powers
                            .iter()
                            .all(|power| power.as_u64().is_some_and(|n| n <= u64::from(u32::MAX)))
                    })
            })
        }),
        "domain" => {
            numeric_fields(value, &["u", "v"])
                && ["u", "v"].iter().all(|axis| {
                    let interval = &value[*axis];
                    numeric_fields(interval, &["lower", "upper", "lowerClosed", "upperClosed"])
                        && rational_schema(&interval["lower"])
                        && rational_schema(&interval["upper"])
                        && interval["lowerClosed"].is_boolean()
                        && interval["upperClosed"].is_boolean()
                })
        }
        "left" | "right" => {
            numeric_fields(value, &["u", "v", "point"])
                && rational_schema(&value["u"])
                && rational_schema(&value["v"])
                && point_schema(&value["point"], false)
        }
        "vertexParameters" => value.as_array().is_some_and(|entries| {
            entries.iter().all(|entry| {
                numeric_fields(entry, &["patchIndex", "u", "v", "point"])
                    && entry["patchIndex"].as_u64().is_some()
                    && rational_schema(&entry["u"])
                    && rational_schema(&entry["v"])
                    && point_schema(&entry["point"], false)
            })
        }),
        "parameterization" => {
            let form = if value["kind"] == "line" {
                numeric_fields(value, &["kind", "tLower", "tUpper"])
            } else {
                value["kind"] == "arc"
                    && numeric_fields(value, &["kind", "branch", "tLower", "tUpper"])
                    && (value["branch"] == "minusPole" || value["branch"] == "plusPole")
            };
            form && rational_schema(&value["tLower"]) && rational_schema(&value["tUpper"])
        }
        "shape" => {
            if value["kind"] == "line" {
                numeric_fields(value, &["kind"])
            } else {
                value["kind"] == "arc"
                    && numeric_fields(value, &["kind", "uLower", "uUpper", "vLower", "vUpper"])
                    && ["uLower", "uUpper", "vLower", "vUpper"]
                        .iter()
                        .all(|field| rational_schema(&value[*field]))
            }
        }
        "midpointU" | "midpointV" | "uLower" | "uUpper" | "vLower" | "vUpper" | "tLower"
        | "tUpper" => rational_schema(value),
        "point" => point_schema(value, false),
        "vertices" => value
            .as_array()
            .is_some_and(|points| points.iter().all(|point| point_schema(point, true))),
        "residualBound"
        | "requestedChordError"
        | "establishedChordError"
        | "requestedSurfaceError"
        | "establishedSurfaceError" => dyadic_schema(value),
        "establishedSquaredBound" => value.is_null() || dyadic_schema(value),
        _ => numeric_schema(value),
    }
}

fn numeric_schema(v: &Value) -> bool {
    match v {
        Value::Object(map) => {
            if map.contains_key("denominator")
                && ["x", "y", "z"].iter().any(|key| map.contains_key(*key))
            {
                return chart_schema(v);
            }
            if map.contains_key("numerator") || map.contains_key("denominator") {
                return rational_schema(v);
            }
            if map.contains_key("integer") || map.contains_key("exponent") {
                return dyadic_schema(v);
            }
            if map.contains_key("exponentU") || map.contains_key("exponentV") {
                return term_schema(v, &["exponentU", "exponentV"]);
            }
            if ["exponentX", "exponentY", "exponentZ"]
                .iter()
                .any(|key| map.contains_key(*key))
            {
                return term_schema(v, &["exponentX", "exponentY", "exponentZ"]);
            }
            map.iter().all(|(key, value)| numeric_member(key, value))
        }
        Value::Array(items) => items.iter().all(numeric_schema),
        _ => true,
    }
}
pub fn validate_case(c: &Value) -> Result<()> {
    fields(
        c,
        &["caseId", "input", "operationFamily", "publicObservation"],
        "caseInvalid",
        "decodeCase",
    )?;
    require(
        c["input"].is_object(),
        "caseInvalid",
        "decodeCase",
        "input must be an object",
    )?;
    let family = c["operationFamily"]
        .as_str()
        .ok_or_else(|| Failure::new("caseInvalid", "decodeCase", "family must be a string"))?;
    let index = FAMILIES
        .iter()
        .position(|f| *f == family)
        .ok_or_else(|| Failure::new("caseInvalid", "decodeCase", "unknown operation family"))?;
    let o = &c["publicObservation"];
    fields(o, &["kind", "value"], "caseInvalid", "decodeCase")?;
    require(
        o["value"].is_object() && (o["kind"] == KINDS[index] || o["kind"] == "typedFailure"),
        "caseInvalid",
        "decodeCase",
        "family-kind mismatch or malformed value",
    )?;
    require(
        numeric_schema(&o["value"]),
        "caseInvalid",
        "decodeCase",
        "noncanonical integer/rational/dyadic representation",
    )
    .map_err(|failure| failure.witness(c.clone()))?;
    if o["kind"] == "typedFailure" {
        let v = &o["value"];
        fields(
            v,
            &["failureKind", "provenance", "reason"],
            "caseInvalid",
            "decodeCase",
        )?;
        fields(
            &v["provenance"],
            &["caseId", "failingOperation", "input", "violatedCondition"],
            "caseInvalid",
            "decodeCase",
        )?;
        let construct = v["failureKind"] == "engineRejection"
            && v["provenance"]["failingOperation"] == "construct"
            && v["provenance"]["violatedCondition"] == "transform.scale must be positive";
        let boundary = v["failureKind"] == "boundaryRejection"
            && v["provenance"]["failingOperation"] == "derive_complete_boundary"
            && v["provenance"]["violatedCondition"]
                == "no exact boundary seed in integer lattice [-3,3]^3";
        require(
            (construct || boundary)
                && v["reason"].as_str().is_some_and(|s| !s.is_empty())
                && v["provenance"]["caseId"] == c["caseId"]
                && v["provenance"]["input"] == c["input"],
            "caseInvalid",
            "decodeCase",
            "typed failure provenance is inconsistent",
        )
        .map_err(|failure| failure.witness(c.clone()))?;
        let input = &c["input"];
        let condition_matches = if construct {
            input["scale"].as_f64() == Some(0.0)
        } else {
            input["scale"].as_f64() == Some(1.25)
                && input["primitiveExtent"].as_f64() == Some(1.0)
                && input["surfaceFamily"] == "sphere-south"
                && input["translation"].as_array().is_some_and(|coordinates| {
                    coordinates.len() == 3
                        && coordinates
                            .iter()
                            .zip([-0.5, 0.0, 0.0])
                            .all(|(coordinate, expected)| coordinate.as_f64() == Some(expected))
                })
        };
        require(
            family == "exact-chart" && condition_matches,
            "caseInvalid",
            "decodeCase",
            "typed failure input does not satisfy its declared failure condition",
        )
        .map_err(|failure| failure.witness(c.clone()))?;
    }
    Ok(())
}
pub fn reconcile(batch: &[u8], checker: &Value) -> Result<()> {
    let form = checker_form(checker)?;
    require(
        checker["digest"] == digest(batch),
        "resultDigestMismatch",
        "reconcile",
        "checker digest differs from actual input bytes",
    )?;
    let decoded = decode(batch, "decodeObservation");
    let enumerated = decoded.as_ref().map_err(Clone::clone).and_then(enumerate);
    match (form, enumerated) {
        (CheckerForm::BatchVerdict, Err(_)) => Ok(()),
        (CheckerForm::BatchVerdict, Ok(_)) => Err(Failure::new(
            "resultContradiction",
            "reconcile",
            "batchVerdict invalid cannot describe an enumerated canonical batch",
        )),
        (CheckerForm::Cases, Err(e)) => Err(e),
        (CheckerForm::Cases, Ok(cases)) => {
            let results = array(checker, "cases", "reconcile")?;
            require(
                cases.len() == results.len(),
                "caseCorrespondenceMismatch",
                "reconcile",
                "case count differs",
            )?;
            for (c, r) in cases.iter().zip(results) {
                require(
                    c["caseId"] == r["caseId"],
                    "caseCorrespondenceMismatch",
                    "reconcile",
                    "caseId order differs",
                )?;
                if let Err(e) = validate_case(c) {
                    require(
                        r["verdict"] == "invalid",
                        "resultContradiction",
                        "reconcile",
                        "invalid case requires caseInvalid verdict",
                    )
                    .map_err(|f| f.witness(e.0))?;
                } else if c["publicObservation"]["kind"] == "typedFailure" {
                    require(
                        r["verdict"] == "undetermined",
                        "resultContradiction",
                        "reconcile",
                        "typedFailure must remain undetermined",
                    )?;
                }
            }
            Ok(())
        }
    }
}
pub fn reconcile_bytes(batch: &[u8], result: &[u8]) -> Result<Value> {
    let checker = decode(result, "decodeCheckerResult")?;
    reconcile(batch, &checker)?;
    let classification = if checker.get("batchVerdict").is_some() {
        "batchInvalid"
    } else if checker["cases"]
        .as_array()
        .is_some_and(|a| a.iter().any(|c| c["verdict"] == "invalid"))
    {
        "caseInvalid"
    } else {
        "caseVerdicts"
    };
    Ok(
        json!({"schema":"neco.cad.reconciliation-result.v1","classification":classification,"digest":digest(batch),"checkerResultDigest":digest(result),"resultMatched":true}),
    )
}
