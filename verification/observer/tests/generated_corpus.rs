use neco_cad_observer::{
    corpus::{draw, Manifest},
    protocol::*,
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
fn manifest() -> Manifest {
    Manifest::decode(include_bytes!("../manifests/smoke.json")).unwrap()
}
#[test]
fn shards_preserve_all_case_rows_and_separate_verdict_categories() {
    let m = manifest();
    let flatten = |size| {
        m.batches(size, None)
            .unwrap()
            .into_iter()
            .flat_map(|(id, b)| {
                b["cases"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .cloned()
                    .map(move |c| (id.clone(), c))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    };
    let small = flatten(37);
    let full = flatten(200);
    assert_eq!(small, full);
    let mut counts = BTreeMap::<(String, String), usize>::new();
    let mut ids = BTreeSet::new();
    for (id, c) in &full {
        assert!(ids.insert(c["caseId"].as_str().unwrap()));
        *counts
            .entry((id.clone(), c["family"].as_str().unwrap().into()))
            .or_default() += 1;
    }
    for (position, family, count) in m.plan(None).unwrap() {
        let id = m.sources[position]["id"].as_str().unwrap();
        assert_eq!(counts[&(id.to_owned(), family)] as u64, count);
    }
    assert!(m.batches(0, None).is_err());
    assert!(m.batches(201, None).is_err());
}
#[test]
fn draw_schedule_uses_exactly_nine_distinct_slots_per_global_index() {
    assert_eq!(draw(0, 0, 0), 0xe220a8397b1dcdaf);
    assert_eq!(draw(0, 0, 1), 0x6e789e6aa1b965f4);
    assert_eq!(draw(0, 1, 0), draw(0, 0, 9));
    let m = manifest();
    let batches = m.batches(200, None).unwrap();
    for (_, b) in batches {
        for c in b["cases"].as_array().unwrap() {
            let index = u64::from_str_radix(c["globalCaseIndex"].as_str().unwrap(), 16).unwrap();
            for (slot, name) in SELECTORS.iter().enumerate() {
                assert_eq!(
                    c["selectors"][*name],
                    format!("{:016x}", draw(m.seed, index, slot as u64))
                );
            }
        }
    }
}
fn explicit_manifest() -> Value {
    let mut v = manifest().value;
    v["generator"]["normalCaseCount"] = json!(1000);
    v["generator"]["allocation"] = json!({"neco.cad":1000});
    for family in v["domains"][0]["families"].as_array_mut().unwrap() {
        family["minimum"] = json!(200);
    }
    v["sourceRefs"][0]["familyCounts"] = json!({"certified-chord-bound":200,"exact-chart":200,"face-atom-covering":200,"finite-surface-representation":200,"mesh-judgment":200});
    v
}
#[test]
fn explicit_counts_are_used_only_when_declared() {
    let explicit = Manifest::decode(&canonical(&explicit_manifest())).unwrap();
    let plan = explicit.plan(None).unwrap();
    assert_eq!(
        plan.iter()
            .filter(|(p, _, _)| *p == 0)
            .map(|(_, _, n)| n)
            .sum::<u64>(),
        1000
    );
    assert!(plan
        .iter()
        .filter(|(p, _, _)| *p == 0)
        .all(|(_, _, n)| *n == 200));
    let mut short = explicit_manifest();
    short["sourceRefs"][0]["familyCounts"]["exact-chart"] = json!(199);
    assert!(Manifest::decode(&canonical(&short)).is_err());
    let mut mismatch = explicit_manifest();
    mismatch["generator"]["normalCaseCount"] = json!(999);
    assert!(Manifest::decode(&canonical(&mismatch)).is_err());
    let mut dangling = manifest().value;
    dangling["sourceRefs"][0]["familyCounts"] =
        explicit_manifest()["sourceRefs"][0]["familyCounts"].clone();
    assert!(Manifest::decode(&canonical(&dangling)).is_err());
}
#[test]
fn derived_counts_are_the_smallest_selector_sequences_covering_every_declared_bucket() {
    let m = manifest();
    let projection: Value = serde_json::from_slice(include_bytes!(
        "../../manifest/cad-projection-declaration.json"
    ))
    .unwrap();
    let descriptor = json!({"projectionClaim":{"projectionDeclarations":projection["families"]}});
    let mut start = 0u64;
    for (position, family, count) in m.plan(Some(&descriptor)).unwrap() {
        let kind = &m.sources[position]["requestedObservationKind"];
        let bindings: Vec<Value> = projection["families"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|p| p["family"] == family.as_str())
            .flat_map(|p| p["selectorBindings"].as_array().unwrap().clone())
            .filter(|b| {
                b.get("requestedObservationKinds")
                    .is_none_or(|k| k.as_array().unwrap().contains(kind))
            })
            .collect();
        assert!(!bindings.is_empty());
        let covered = |upto: u64| {
            bindings.iter().all(|b| {
                let rule = &b["selectorRule"];
                let slot = SELECTORS
                    .iter()
                    .position(|n| rule["selector"] == *n)
                    .unwrap() as u64;
                let modulus = rule["modulus"].as_u64().unwrap();
                let seen: BTreeSet<&str> = (0..upto)
                    .map(|i| {
                        rule["bucketsByRemainder"]
                            [(draw(m.seed, start + i, slot) % modulus).to_string()]
                        .as_str()
                        .unwrap()
                    })
                    .collect();
                let declared: BTreeSet<&str> = rule["bucketsByRemainder"]
                    .as_object()
                    .unwrap()
                    .values()
                    .map(|n| n.as_str().unwrap())
                    .collect();
                seen == declared
            })
        };
        assert!(covered(count), "{family} is not covered");
        assert!(count == 1 || !covered(count - 1), "{family} is not minimal");
        start += count;
    }
}
#[test]
fn manifest_rejects_scope_shrinking_host_identity_and_verified_typed_failure() {
    let original = manifest().value;
    for change in 0..4 {
        let mut v = original.clone();
        match change {
            0 => v["generator"]["normalCaseCount"] = json!(0),
            1 => v["sourceRefs"][1]["expectedVerdict"] = json!("verified"),
            2 => v["domains"][0]["sourceOid"] = json!("a".repeat(40)),
            _ => v["generator"]["tier"] = json!("standard"),
        };
        assert!(Manifest::decode(&canonical(&v)).is_err());
    }
    let mut duplicate = original.clone();
    let s = duplicate["sourceRefs"][1].clone();
    duplicate["sourceRefs"].as_array_mut().unwrap().push(s);
    assert!(Manifest::decode(&canonical(&duplicate)).is_err());
    let mut bad = original;
    bad["domains"][0]["semanticsRef"]["declarationDigest"] = Value::String("binding".into());
    assert!(Manifest::decode(&canonical(&bad)).is_err());
}
