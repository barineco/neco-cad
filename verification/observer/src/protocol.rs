use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fmt, fs, path::Path};

pub const FAMILIES: [&str; 5] = [
    "exact-chart",
    "face-atom-covering",
    "finite-surface-representation",
    "certified-chord-bound",
    "mesh-judgment",
];
pub const KINDS: [&str; 5] = [
    "exactChart",
    "faceAtomCovering",
    "finiteSurfaceRepresentation",
    "certifiedChordBound",
    "meshJudgment",
];
pub const SELECTORS: [&str; 9] = [
    "value",
    "familyParameter",
    "observationClass",
    "sign",
    "multiplicity",
    "intervalWidth",
    "formulaVariant",
    "mutation",
    "reserved",
];
pub const LANES: [&str; 7] = [
    "observationVerification",
    "soundnessTheorem",
    "publicApiParity",
    "mutationRejection",
    "invalidProtocol",
    "corpusExecution",
    "repeatability",
];
pub type Result<T, E = Failure> = std::result::Result<T, E>;
#[derive(Debug, Clone)]
pub struct Failure(pub Value);
impl Failure {
    pub fn new(kind: &str, operation: &str, condition: impl fmt::Display) -> Self {
        Self(
            json!({"kind":kind,"failingOperation":operation,"violatedCondition":condition.to_string(),"operandProvenance":{},"witness":null,"contributingFailures":[]}),
        )
    }
    pub fn witness(mut self, value: Value) -> Self {
        self.0["witness"] = value;
        self
    }
    pub fn kind(&self) -> &str {
        self.0["kind"].as_str().unwrap_or("processFailure")
    }
}
impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for Failure {}
pub fn require(ok: bool, kind: &str, op: &str, condition: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(Failure::new(kind, op, condition))
    }
}
pub fn canonical(value: &Value) -> Vec<u8> {
    value.to_string().into_bytes()
}
pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn hex(text: &str, size: usize) -> bool {
    text.len() == size
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub fn fields(value: &Value, keys: &[&str], kind: &str, op: &str) -> Result<()> {
    require(
        value
            .as_object()
            .is_some_and(|v| v.len() == keys.len() && keys.iter().all(|k| v.contains_key(*k))),
        kind,
        op,
        "exact field set required",
    )
}
pub fn fields_with_optional(
    value: &Value,
    required: &[&str],
    optional: &[&str],
    kind: &str,
    op: &str,
) -> Result<()> {
    require(
        value.as_object().is_some_and(|v| {
            required.iter().all(|k| v.contains_key(*k))
                && v.keys()
                    .all(|k| required.contains(&k.as_str()) || optional.contains(&k.as_str()))
        }),
        kind,
        op,
        "exact field set required",
    )
}
pub fn decimal(text: &str, positive: bool) -> bool {
    let digits = text.strip_prefix('-').unwrap_or(text);
    !digits.is_empty()
        && digits.bytes().all(|b| b.is_ascii_digit())
        && (digits == "0" || !digits.starts_with('0'))
        && text != "-0"
        && (!positive || (!text.starts_with('-') && text != "0"))
}
pub fn string<'a>(value: &'a Value, key: &str, op: &str) -> Result<&'a str> {
    value[key]
        .as_str()
        .ok_or_else(|| Failure::new("schemaInvalid", op, format!("{key} must be a string")))
}
pub fn array<'a>(value: &'a Value, key: &str, op: &str) -> Result<&'a Vec<Value>> {
    value[key]
        .as_array()
        .ok_or_else(|| Failure::new("schemaInvalid", op, format!("{key} must be an array")))
}
struct Unique(Value);
impl<'de> Deserialize<'de> for Unique {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Unique;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("JSON with unique keys")
            }
            fn visit_bool<E: de::Error>(self, v: bool) -> std::result::Result<Unique, E> {
                Ok(Unique(v.into()))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> std::result::Result<Unique, E> {
                Ok(Unique(v.into()))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> std::result::Result<Unique, E> {
                Ok(Unique(v.into()))
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> std::result::Result<Unique, E> {
                serde_json::Number::from_f64(v)
                    .map(|n| Unique(Value::Number(n)))
                    .ok_or_else(|| E::custom("non-finite number"))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> std::result::Result<Unique, E> {
                Ok(Unique(v.into()))
            }
            fn visit_string<E: de::Error>(self, v: String) -> std::result::Result<Unique, E> {
                Ok(Unique(v.into()))
            }
            fn visit_unit<E: de::Error>(self) -> std::result::Result<Unique, E> {
                Ok(Unique(Value::Null))
            }
            fn visit_none<E: de::Error>(self) -> std::result::Result<Unique, E> {
                Ok(Unique(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> std::result::Result<Unique, A::Error> {
                let mut items = Vec::new();
                while let Some(Unique(v)) = seq.next_element()? {
                    items.push(v);
                }
                Ok(Unique(Value::Array(items)))
            }
            fn visit_map<A: MapAccess<'de>>(
                self,
                mut map: A,
            ) -> std::result::Result<Unique, A::Error> {
                let mut items = Map::new();
                while let Some((key, Unique(v))) = map.next_entry::<String, Unique>()? {
                    if items.insert(key, v).is_some() {
                        return Err(de::Error::custom("duplicateKey"));
                    }
                }
                Ok(Unique(Value::Object(items)))
            }
        }
        deserializer.deserialize_any(V)
    }
}
pub fn decode(bytes: &[u8], op: &str) -> Result<Value> {
    std::str::from_utf8(bytes).map_err(|e| Failure::new("nonUtf8", op, e))?;
    let mut decoder = serde_json::Deserializer::from_slice(bytes);
    let Unique(value) = Unique::deserialize(&mut decoder).map_err(|e| {
        Failure::new(
            if e.to_string().contains("duplicateKey") {
                "duplicateKey"
            } else {
                "parseInvalid"
            },
            op,
            e,
        )
    })?;
    decoder
        .end()
        .map_err(|e| Failure::new("parseInvalid", op, e))?;
    require(
        canonical(&value) == bytes,
        "noncanonicalBytes",
        op,
        "canonical sorted JSON bytes required",
    )?;
    Ok(value)
}
pub fn load(path: &Path, op: &str) -> Result<Value> {
    let bytes = fs::read(path).map_err(|e| Failure::new("processFailure", op, e))?;
    decode(&bytes, op)
}
pub fn write(path: &Path, value: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| Failure::new("processFailure", "writeReport", e))?;
    }
    fs::write(path, canonical(value)).map_err(|e| Failure::new("processFailure", "writeReport", e))
}
#[derive(Debug, PartialEq, Eq)]
pub enum CheckerForm {
    BatchVerdict,
    Cases,
}
pub fn checker_form(result: &Value) -> Result<CheckerForm> {
    let form = if result.get("batchVerdict").is_some() {
        fields(
            result,
            &["batchVerdict", "digest"],
            "resultInvalid",
            "decodeCheckerResult",
        )?;
        require(
            result["batchVerdict"] == "invalid",
            "resultInvalid",
            "decodeCheckerResult",
            "batchVerdict must be invalid",
        )?;
        CheckerForm::BatchVerdict
    } else {
        fields(
            result,
            &["cases", "digest"],
            "resultInvalid",
            "decodeCheckerResult",
        )?;
        let mut ids = BTreeSet::new();
        let cases = array(result, "cases", "decodeCheckerResult")?;
        require(
            !cases.is_empty(),
            "resultInvalid",
            "decodeCheckerResult",
            "result cases must be nonempty",
        )?;
        for c in cases {
            fields(
                c,
                &["caseId", "verdict"],
                "resultInvalid",
                "decodeCheckerResult",
            )?;
            let id = string(c, "caseId", "decodeCheckerResult")?;
            require(
                !id.is_empty() && id != "__digest__" && ids.insert(id),
                "resultInvalid",
                "decodeCheckerResult",
                "caseIds must be nonempty unique non-sentinel",
            )?;
            require(
                ["verified", "refuted", "undetermined", "invalid"].contains(&string(
                    c,
                    "verdict",
                    "decodeCheckerResult",
                )?),
                "resultInvalid",
                "decodeCheckerResult",
                "unknown verdict",
            )?;
        }
        CheckerForm::Cases
    };
    require(
        hex(string(result, "digest", "decodeCheckerResult")?, 64),
        "resultInvalid",
        "decodeCheckerResult",
        "digest must be lowercase SHA256",
    )?;
    Ok(form)
}
