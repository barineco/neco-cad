use cad_observation_producer::cli::dispatch;
use std::env;
use std::io::{self, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    match dispatch(&arguments) {
        Ok(bytes) => {
            if io::stdout().write_all(&bytes).is_err() {
                return ExitCode::from(2);
            }
            let parity_failed = arguments.first().is_some_and(|command| command == "self-check")
                && serde_json::from_slice::<serde_json::Value>(&bytes)
                    .is_ok_and(|value| value["verified"] == false);
            if parity_failed { ExitCode::from(1) } else { ExitCode::SUCCESS }
        }
        Err(failure) => {
            let (kind, case_id, family) = match &failure {
                cad_observation_types::CadObservationFailure::BatchResourceLimit { .. } =>
                    ("batchResourceLimit", None, None),
                cad_observation_types::CadObservationFailure::EngineFailure { case_id, family, .. } =>
                    ("engineFailure", Some(case_id.as_str()), Some(family.as_str())),
                cad_observation_types::CadObservationFailure::InvalidProcessInvocation =>
                    ("invalidProcessInvocation", None, None),
                _ => ("invalidRequest", None, None),
            };
            let value = serde_json::json!({
                "schema": "neco.verification.adapter-failure.v1",
                "kind": kind,
                "failingOperation": "dispatch",
                "operandProvenance": {
                    "caseId": case_id, "family": family, "requestedObservationKind": null,
                    "selectors": {}, "projectionRevision": null, "resourceRevision": null,
                    "buildIdentity": null,
                },
                "violatedCondition": format!("{failure:?}"),
                "contributingFailures": [],
            });
            if let Ok(bytes) = cad_observation_types::canonical_json(&value) {
                let _ = io::stderr().write_all(&bytes);
            }
            ExitCode::from(2)
        }
    }
}
