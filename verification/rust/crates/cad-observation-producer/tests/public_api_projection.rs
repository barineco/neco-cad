use cad_observation_producer::{generate, self_check};
use cad_observation_types::{canonical_json, CadObservationFailure};
use serde_json::Value;

fn mesh_request() -> Value {
    let mut request: Value = serde_json::from_slice(include_bytes!(
        "../../../../vectors/requests/cad-request-batch.json"
    )).unwrap();
    let mesh = request["cases"].as_array().unwrap().iter()
        .find(|case| case["family"] == "mesh-judgment").unwrap().clone();
    request["cases"] = serde_json::json!([mesh]);
    request
}

fn dyadic_value(value: &Value) -> f64 {
    let integer = value["integer"].as_str().unwrap().parse::<i64>().unwrap();
    let exponent = value["exponent"].as_u64().unwrap();
    integer as f64 / 2.0_f64.powi(i32::try_from(exponent).unwrap())
}

#[test]
fn transformed_mesh_and_public_api_replay_preserve_values_and_detect_changes() {
    let request = canonical_json(&mesh_request()).unwrap();
    let batch = generate(&request).unwrap();
    let mut observed: Value = serde_json::from_slice(&batch).unwrap();
    let value = &observed["cases"][0]["publicObservation"]["value"];
    assert_eq!(value["outcome"]["kind"], "validated");
    let vertices: Vec<_> = value["mesh"]["vertices"].as_array().unwrap().iter()
        .map(|v| [dyadic_value(&v["x"]), dyadic_value(&v["y"]), dyadic_value(&v["z"])]).collect();
    assert_eq!(vertices, vec![[-1.5, 0.0, 0.0], [0.25, 0.0, 0.0],
        [-1.5, 1.75, 0.0], [-1.5, 0.0, 1.75]]);
    let parity: Value = serde_json::from_slice(&self_check(&request, &batch).unwrap()).unwrap();
    assert_eq!(parity["verified"], true);
    observed["cases"][0]["publicObservation"]["value"]["mesh"]["vertices"][0]["x"]["integer"]
        = Value::String("-2".to_owned());
    let altered = canonical_json(&observed).unwrap();
    let parity: Value = serde_json::from_slice(&self_check(&request, &altered).unwrap()).unwrap();
    assert_eq!(parity["verified"], false);
    assert_eq!(parity["cases"][0]["verified"], false);
}

#[test]
fn over_limit_batches_are_rejected_before_public_api_execution() {
    let mut request = mesh_request();
    let template = request["cases"][0].clone();
    request["cases"] = Value::Array((0..201_u64).map(|index| {
        let mut case = template.clone();
        case["caseId"] = Value::String(format!("case-{index:016x}"));
        case["globalCaseIndex"] = Value::String(format!("{index:016x}"));
        case["domainCaseIndex"] = Value::String(format!("{index:016x}"));
        case
    }).collect());
    assert_eq!(generate(&canonical_json(&request).unwrap()),
        Err(CadObservationFailure::BatchResourceLimit { count: 201, maximum: 200 }));
}

fn rational_value(value: &Value) -> f64 {
    value["numerator"].as_str().unwrap().parse::<f64>().unwrap()
        / value["denominator"].as_str().unwrap().parse::<f64>().unwrap()
}

fn surface_value(terms: &Value, u: f64, v: f64) -> f64 {
    terms.as_array().unwrap().iter().map(|term| {
        rational_value(&term["coefficient"])
            * u.powi(i32::try_from(term["exponentU"].as_u64().unwrap()).unwrap())
            * v.powi(i32::try_from(term["exponentV"].as_u64().unwrap()).unwrap())
    }).sum()
}

fn chart_requests() -> Vec<Value> {
    let base: Value = serde_json::from_slice(include_bytes!(
        "../../../../vectors/requests/cad-request-batch.json"
    )).unwrap();
    let template = base["cases"][0].clone();
    let mut requests = Vec::new();
    for family in 0..3_u64 {
        for scale in 0..4_u64 {
            for translation in 0..4_u64 {
                for sign in 0..2_u64 {
                    if translation == 0 && sign == 1 {
                        continue;
                    }
                    let index = requests.len();
                    let mut case = template.clone();
                    case["caseId"] = Value::String(format!("case-{index:016x}"));
                    case["globalCaseIndex"] = Value::String(format!("{index:016x}"));
                    case["domainCaseIndex"] = Value::String(format!("{index:016x}"));
                    for (name, selector) in [("familyParameter", family), ("multiplicity", scale),
                        ("value", translation), ("sign", sign)]
                    {
                        case["selectors"][name] = Value::String(format!("{selector:016x}"));
                    }
                    let mut request = base.clone();
                    request["cases"] = Value::Array(vec![case]);
                    requests.push(request);
                }
            }
        }
    }
    requests
}

fn observe_in_parallel(requests: &[Value]) -> Vec<Value> {
    let next = std::sync::atomic::AtomicUsize::new(0);
    let results = std::sync::Mutex::new(vec![Value::Null; requests.len()]);
    let workers = std::thread::available_parallelism().map_or(1, usize::from);
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| loop {
                let index = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let Some(request) = requests.get(index) else { break };
                let batch = generate(&canonical_json(request).unwrap()).unwrap();
                let mut observed: Value = serde_json::from_slice(&batch).unwrap();
                results.lock().unwrap()[index] = observed["cases"][0].take();
            });
        }
    });
    results.into_inner().unwrap()
}

#[test]
fn all_chart_selector_classes_preserve_the_constructed_unit_extent_and_center() {
    for case in &observe_in_parallel(&chart_requests()) {
        assert_eq!(case["publicObservation"]["kind"], "exactChart");
        let input = &case["input"];
        let value = &case["publicObservation"]["value"];
        let midpoint = |axis: &str| {
            (rational_value(&value["domain"][axis]["lower"])
                + rational_value(&value["domain"][axis]["upper"])) / 2.0
        };
        let (u, v) = (midpoint("u"), midpoint("v"));
        let denominator = surface_value(&value["chart"]["denominator"], u, v);
        let displacement: Vec<_> = ["x", "y", "z"].iter().enumerate().map(|(index, axis)| {
            surface_value(&value["chart"][*axis], u, v) / denominator
                - input["translation"][index].as_f64().unwrap()
        }).collect();
        assert_eq!(displacement.iter().map(|coordinate| coordinate.abs()).sum::<f64>(), 1.0,
            "actual public chart extent differs for {}", case["caseId"]);
        assert_eq!(displacement.iter().filter(|coordinate| **coordinate != 0.0).count(), 1,
            "actual public chart center differs for {}", case["caseId"]);
    }
}
