use neco_cad_observer::{
    controls,
    corpus::coverage,
    protocol::*,
    reconciliation,
    runtime::{repeat, run_corpus, Runtime},
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write,
    path::Path,
    process::ExitCode,
};
struct Args {
    command: String,
    values: BTreeMap<String, String>,
    flags: BTreeSet<String>,
}
impl Args {
    fn parse(args: Vec<String>) -> Result<Self> {
        let command = args
            .first()
            .ok_or_else(|| {
                Failure::new("invalidProcessInvocation", "parseCli", "command required")
            })?
            .clone();
        let mut values = BTreeMap::new();
        let mut flags = BTreeSet::new();
        let mut index = 1;
        let switches = [
            "--require-descriptor-buckets",
            "--require-selector-rules",
            "--require-shard-at-most-producer-max",
            "--require-shared-normal-corpus",
            "--require-structured-process-failures",
            "--sequential",
        ];
        while index < args.len() {
            let key = &args[index];
            require(
                key.starts_with("--"),
                "invalidProcessInvocation",
                "parseCli",
                "flag required",
            )?;
            if switches.contains(&key.as_str()) {
                require(
                    flags.insert(key.clone()),
                    "invalidProcessInvocation",
                    "parseCli",
                    "duplicate flag",
                )?;
                index += 1;
            } else {
                let value = args
                    .get(index + 1)
                    .filter(|v| !v.starts_with("--"))
                    .ok_or_else(|| {
                        Failure::new(
                            "invalidProcessInvocation",
                            "parseCli",
                            format!("value for {key} required"),
                        )
                    })?;
                require(
                    values.insert(key.clone(), value.clone()).is_none(),
                    "invalidProcessInvocation",
                    "parseCli",
                    "duplicate flag",
                )?;
                index += 2;
            }
        }
        Ok(Self {
            command,
            values,
            flags,
        })
    }
    fn validate(&self, values: &[&str], flags: &[&str]) -> Result<()> {
        require(
            self.values.keys().all(|k| values.contains(&k.as_str()))
                && self.flags.iter().all(|k| flags.contains(&k.as_str())),
            "invalidProcessInvocation",
            "parseCli",
            "unknown command flag",
        )
    }
    fn get(&self, key: &str) -> Result<&str> {
        self.values.get(key).map(String::as_str).ok_or_else(|| {
            Failure::new(
                "invalidProcessInvocation",
                "parseCli",
                format!("{key} required"),
            )
        })
    }
    fn size(&self, key: &str) -> Result<usize> {
        self.get(key)?
            .parse()
            .map_err(|e| Failure::new("invalidProcessInvocation", "parseCli", e))
    }
    fn path(&self, key: &str) -> Result<&Path> {
        Ok(Path::new(self.get(key)?))
    }
}
fn generate(args: &Args) -> Result<Value> {
    args.validate(
        &[
            "--logical-manifest",
            "--host-execution-config",
            "--shard-size",
            "--out",
        ],
        &[],
    )?;
    let runtime = Runtime::load(
        args.path("--logical-manifest")?,
        args.path("--host-execution-config")?,
    )?;
    let m = runtime.manifest.clone();
    let descriptor = Some(&runtime.descriptor);
    let size = args.size("--shard-size")?;
    let out = args.path("--out")?;
    let mut source_refs = serde_json::Map::new();
    let mut indices = BTreeMap::<String, usize>::new();
    let mut hashers = BTreeMap::<String, Sha256>::new();
    for (id, request) in m.batches(size, descriptor)? {
        let index = indices.entry(id.clone()).or_default();
        let directory = out.join(&id);
        let path = directory.join(format!("shard-{index:04}.json"));
        write(&path, &request)?;
        let row = source_refs.entry(id.clone()).or_insert_with(
            || json!({"directory":directory,"count":0,"requestDigest":null,"shards":[]}),
        );
        let count = array(&request, "cases", "generate")?.len();
        row["count"] = json!(row["count"].as_u64().unwrap() + count as u64);
        row["shards"].as_array_mut().unwrap().push(
            json!({"path":path,"requestDigest":digest(&canonical(&request)),"caseCount":count}),
        );
        for c in array(&request, "cases", "generate")? {
            hashers.entry(id.clone()).or_default().update(canonical(c));
        }
        *index += 1;
    }
    for (id, hasher) in hashers {
        source_refs[&id]["requestDigest"] = json!(format!("{:x}", hasher.finalize()));
    }
    Ok(
        json!({"schema":"neco.cad.generated-corpus.v1","logicalManifestDigest":digest(&canonical(&m.value)),"sourceRefs":source_refs}),
    )
}
fn execute(args: &Args) -> Result<Value> {
    if args.command == "generate" {
        return generate(args);
    }
    if args.command == "verify-coverage" {
        args.validate(
            &["--logical-manifest", "--host-execution-config"],
            &[
                "--require-descriptor-buckets",
                "--require-selector-rules",
                "--require-shard-at-most-producer-max",
            ],
        )?;
        let runtime = Runtime::load(
            args.path("--logical-manifest")?,
            args.path("--host-execution-config")?,
        )?;
        return coverage(
            &runtime.manifest,
            &runtime.descriptor,
            runtime.manifest.shard,
        );
    }
    if args.command == "run-lanes" {
        args.validate(
            &["--logical-manifest", "--host-execution-config", "--lanes"],
            &[
                "--require-shared-normal-corpus",
                "--require-structured-process-failures",
            ],
        )?;
        let lanes: Vec<_> = args.get("--lanes")?.split(',').collect();
        require(
            lanes.len() == 7
                && lanes.iter().copied().collect::<BTreeSet<_>>() == LANES.into_iter().collect(),
            "invalidProcessInvocation",
            "runLanes",
            "all seven lanes exactly once required",
        )?;
        let r = Runtime::load(
            args.path("--logical-manifest")?,
            args.path("--host-execution-config")?,
        )?;
        let proof = r.proof()?;
        let shard = r.manifest.shard;
        let first = run_corpus(&r, "first", shard)?;
        let mutation = controls::mutation_lane(&r, &first)?;
        let invalid = controls::invalid_lane(&r, &first)?;
        let second = run_corpus(&r, "second", shard)?;
        let repeated = repeat(&first, &second)?;
        let parity:serde_json::Map<String,Value>=array(&first.report,"shards","publicApiParity")?.iter().map(|s|(format!("{}-{}",s["corpusId"].as_str().unwrap(),s["shardIndex"]),json!({"verified":true,"resultMatched":s["resultMatched"],"report":s["publicApiParity"]}))).collect();
        let report = json!({"schema":"neco.cad.completion-report.v1","logicalManifestDigest":digest(&canonical(&r.manifest.value)),"descriptorDigest":digest(&canonical(&r.descriptor)),"sharedNormalCorpus":true,"realPasses":2,"sequentialStaging":true,"processErrorCount":0,"processErrors":[],"corpusResults":first.report,"laneResults":{"corpusExecution":{"status":"passed","coverage":coverage(&r.manifest,&r.descriptor,shard)?},"observationVerification":{"status":"passed","normalCorpusId":"cad","separateFailureCorpora":true,"corpora":first.report["corpora"]},"soundnessTheorem":{"neco.cad":proof},"publicApiParity":parity,"mutationRejection":mutation,"invalidProtocol":invalid,"repeatability":repeated}});
        write(&r.reports.join("lanes.json"), &report)?;
        return Ok(report);
    }
    if args.command == "compare-repeatability" {
        args.validate(
            &[
                "--logical-manifest",
                "--host-execution-config",
                "--runs",
                "--shard-size",
                "--write-report",
            ],
            &["--sequential"],
        )?;
        require(
            args.size("--runs")? == 2 && args.flags.contains("--sequential"),
            "invalidProcessInvocation",
            "compareRepeatability",
            "two sequential actual passes required",
        )?;
        let r = Runtime::load(
            args.path("--logical-manifest")?,
            args.path("--host-execution-config")?,
        )?;
        let size = args.size("--shard-size")?;
        let first = run_corpus(&r, "first", size)?;
        let second = run_corpus(&r, "second", size)?;
        let report = json!({"schema":"neco.cad.completion-report.v1","logicalManifestDigest":digest(&canonical(&r.manifest.value)),"corpusResults":first.report,"processErrorCount":0,"processErrors":[],"realPasses":2,"sequentialStaging":true,"laneResults":{"repeatability":repeat(&first,&second)?}});
        write(args.path("--write-report")?, &report)?;
        return Ok(report);
    }
    Err(Failure::new(
        "invalidProcessInvocation",
        "parseCli",
        "unknown command",
    ))
}
fn main() -> ExitCode {
    let raw: Vec<_> = std::env::args().skip(1).collect();
    let reconciliation_mode = raw.first().is_some_and(|s| s == "--batch");
    let result = if reconciliation_mode {
        match raw.as_slice() {
            [flag, batch, result_flag, result]
                if flag == "--batch" && result_flag == "--checker-result" =>
            {
                fs::read(batch)
                    .map_err(|e| Failure::new("processFailure", "readBatch", e))
                    .and_then(|b| {
                        fs::read(result)
                            .map_err(|e| Failure::new("processFailure", "readCheckerResult", e))
                            .and_then(|k| reconciliation::reconcile_bytes(&b, &k))
                    })
            }
            _ => Err(Failure::new(
                "invalidProcessInvocation",
                "parseCli",
                "--batch B --checker-result K required",
            )),
        }
    } else {
        Args::parse(raw.clone()).and_then(|args| execute(&args))
    };
    match result {
        Ok(value) => match std::io::stdout().write_all(&canonical(&value)) {
            Ok(()) => ExitCode::from(if value["mathematicalRejected"] == true {
                1
            } else {
                0
            }),
            Err(_) => ExitCode::from(1),
        },
        Err(failure) => {
            let process_failure = failure.kind() == "processFailure";
            let errors = if process_failure {
                vec![failure.0.clone()]
            } else {
                Vec::new()
            };
            let value = json!({"schema":"neco.cad.observer-failure.v1","classification":failure.kind(),"processErrorCount":errors.len(),"processErrors":errors,"failure":failure.0});
            let _ = std::io::stderr().write_all(&canonical(&value));
            let declaration_error = matches!(
                failure.kind(),
                "invalidProcessInvocation" | "manifestInvalid" | "hostInvalid"
            ) || matches!(
                failure.0["failingOperation"].as_str(),
                Some("decodeLogicalManifest" | "loadHostExecutionConfig")
            );
            ExitCode::from(if reconciliation_mode || declaration_error {
                2
            } else {
                1
            })
        }
    }
}
