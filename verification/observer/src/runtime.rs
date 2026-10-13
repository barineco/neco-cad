use crate::{
    corpus::{coverage, Manifest},
    protocol::*,
    reconciliation,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
    thread,
    time::{Duration, Instant},
};

pub struct Runtime {
    pub host: Value,
    pub cwd: PathBuf,
    pub temp: PathBuf,
    pub reports: PathBuf,
    pub producer: PathBuf,
    pub checker: PathBuf,
    pub control_probe: PathBuf,
    pub observer: PathBuf,
    pub descriptor: Value,
    pub manifest: Manifest,
    pub parallelism: usize,
    started: Instant,
}
pub struct ChildOutput {
    pub bytes: Vec<u8>,
    pub stderr: Vec<u8>,
    pub code: Option<i32>,
}
fn read_limited<R: Read>(mut reader: R) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 65536];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            return Ok(bytes);
        }
        if bytes.len() + count > 536870912 {
            return Err(std::io::Error::other("stdoutPayloadBytes exceeded"));
        }
        bytes.extend_from_slice(&buffer[..count]);
    }
}
pub fn child(
    program: &Path,
    args: &[&str],
    cwd: &Path,
    seconds: u64,
    operation: &str,
) -> Result<ChildOutput> {
    let mut child = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            Failure::new("processFailure", operation, e)
                .witness(json!({"command":program,"args":args}))
        })?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| Failure::new("processFailure", operation, "missing stdout pipe"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| Failure::new("processFailure", operation, "missing stderr pipe"))?;
    let out = thread::spawn(move || read_limited(stdout));
    let err = thread::spawn(move || read_limited(stderr));
    let start = Instant::now();
    let status;
    loop {
        if let Some(s) = child
            .try_wait()
            .map_err(|e| Failure::new("processFailure", operation, e))?
        {
            status = s;
            break;
        }
        if start.elapsed() > Duration::from_secs(seconds) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(Failure::new(
                "processFailure",
                operation,
                "declared child timeout exceeded",
            )
            .witness(json!({"command":program,"args":args,"timeoutSeconds":seconds})));
        }
        thread::sleep(Duration::from_millis(20));
    }
    let bytes = out
        .join()
        .map_err(|_| Failure::new("processFailure", operation, "stdout reader panicked"))?
        .map_err(|e| Failure::new("processFailure", operation, e))?;
    let stderr = err
        .join()
        .map_err(|_| Failure::new("processFailure", operation, "stderr reader panicked"))?
        .map_err(|e| Failure::new("processFailure", operation, e))?;
    Ok(ChildOutput {
        bytes,
        stderr,
        code: status.code(),
    })
}
impl ChildOutput {
    pub fn success(self, op: &str) -> Result<Vec<u8>> {
        if self.code == Some(0) {
            Ok(self.bytes)
        } else {
            Err(Failure::new("processFailure",op,"child exited unsuccessfully").witness(json!({"exitCode":self.code,"stdoutDigest":digest(&self.bytes),"stderrDigest":digest(&self.stderr),"stderr":String::from_utf8_lossy(&self.stderr)})))
        }
    }
}
fn stdout_value(bytes: &[u8], op: &str) -> Result<Value> {
    decode(bytes.strip_suffix(b"\n").unwrap_or(bytes), op)
}

pub fn host_parallelism(host: &Value) -> Result<usize> {
    fields_with_optional(
        host,
        &[
            "workingDirectory",
            "temporaryDirectory",
            "reportDirectory",
            "domains",
            "controlProbePath",
        ],
        &["shardParallelism"],
        "hostInvalid",
        "loadHostExecutionConfig",
    )?;
    Ok(match host.get("shardParallelism") {
        None => thread::available_parallelism().map_or(1, usize::from),
        Some(v) => v.as_u64().filter(|n| *n > 0).ok_or_else(|| {
            Failure::new(
                "hostInvalid",
                "loadHostExecutionConfig",
                "shardParallelism must be a positive integer",
            )
        })? as usize,
    })
}

impl Runtime {
    pub fn load(manifest_path: &Path, host_path: &Path) -> Result<Self> {
        let manifest = Manifest::decode(
            &fs::read(manifest_path)
                .map_err(|e| Failure::new("processFailure", "loadManifest", e))?,
        )?;
        let started = Instant::now();
        let host = load(host_path, "loadHostExecutionConfig")?;
        let parallelism = host_parallelism(&host)?;
        let cwd = PathBuf::from(string(
            &host,
            "workingDirectory",
            "loadHostExecutionConfig",
        )?);
        require(
            cwd.is_absolute() && cwd.is_dir(),
            "hostInvalid",
            "loadHostExecutionConfig",
            "workingDirectory must be an explicit existing absolute directory",
        )?;
        let resolve = |value: &Value| -> Result<PathBuf> {
            let p = Path::new(value.as_str().ok_or_else(|| {
                Failure::new("hostInvalid", "loadHostExecutionConfig", "path required")
            })?);
            Ok(if p.is_absolute() {
                p.to_owned()
            } else {
                cwd.join(p)
            })
        };
        fields(
            &host["domains"],
            &["neco.cad"],
            "hostInvalid",
            "loadHostExecutionConfig",
        )?;
        let paths = &host["domains"]["neco.cad"];
        fields(
            paths,
            &["producerPath", "checkerPath"],
            "hostInvalid",
            "loadHostExecutionConfig",
        )?;
        let producer = resolve(&paths["producerPath"])?;
        let checker = resolve(&paths["checkerPath"])?;
        let control_probe = resolve(&host["controlProbePath"])?;
        let temp = resolve(&host["temporaryDirectory"])?;
        let reports = resolve(&host["reportDirectory"])?;
        let observer = std::env::current_exe()
            .map_err(|e| Failure::new("processFailure", "loadHostExecutionConfig", e))?;
        let descriptor_bytes = child(
            &producer,
            &["describe"],
            &cwd,
            manifest.value["timeoutPolicy"]["shardTimeoutSeconds"]
                .as_u64()
                .unwrap_or(7200),
            "producerDescribe",
        )?
        .success("producerDescribe")?;
        let descriptor = stdout_value(&descriptor_bytes, "producerDescribe")?;
        fields(
            &descriptor,
            &[
                "schema",
                "domain",
                "semanticsRef",
                "producerApiRevision",
                "adapterRevision",
                "rustcIdentity",
                "targetTriple",
                "projectionClaim",
                "resourceClaim",
            ],
            "descriptorMismatch",
            "producerDescribe",
        )?;
        fields(
            &descriptor["semanticsRef"],
            &["declarationDigest", "revision"],
            "descriptorMismatch",
            "producerDescribe",
        )?;
        fields(
            &descriptor["projectionClaim"],
            &[
                "projectionDeclarationDigest",
                "projectionDeclarations",
                "projectionRevision",
            ],
            "descriptorMismatch",
            "producerDescribe",
        )?;
        fields(
            &descriptor["resourceClaim"],
            &[
                "maxBatchCases",
                "resourceDeclarationDigest",
                "resourceRevision",
            ],
            "descriptorMismatch",
            "producerDescribe",
        )?;
        require(
            descriptor["schema"] == "neco.verification.producer-description.v1"
                && descriptor["domain"] == "neco.cad",
            "descriptorMismatch",
            "producerDescribe",
            "actual descriptor schema/domain differs",
        )?;
        let domain = &manifest.value["domains"][0];
        require(
            descriptor["adapterRevision"] == domain["adapterRevision"]
                && descriptor["producerApiRevision"] == domain["producerApiRevision"]
                && descriptor["projectionClaim"]["projectionRevision"]
                    == domain["projectionRevision"]
                && descriptor["semanticsRef"]["declarationDigest"]
                    == domain["semanticsRef"]["declarationDigest"],
            "descriptorMismatch",
            "producerDescribe",
            "selected manifest and actual descriptor differ",
        )?;
        let rustc = string(&descriptor, "rustcIdentity", "producerDescribe")?;
        let target = string(&descriptor, "targetTriple", "producerDescribe")?;
        require(
            rustc.starts_with("rustc ")
                && target.starts_with(std::env::consts::ARCH)
                && target.contains(if std::env::consts::OS == "macos" {
                    "darwin"
                } else {
                    std::env::consts::OS
                }),
            "descriptorMismatch",
            "producerDescribe",
            "actual rustc identity/target incompatible",
        )?;
        let r = Self {
            host,
            cwd,
            temp,
            reports,
            producer,
            checker,
            control_probe,
            observer,
            descriptor,
            manifest,
            parallelism,
            started,
        };
        coverage(&r.manifest, &r.descriptor, r.manifest.shard)?;
        fs::create_dir_all(&r.temp)
            .map_err(|e| Failure::new("processFailure", "createRuntime", e))?;
        fs::create_dir_all(&r.reports)
            .map_err(|e| Failure::new("processFailure", "createRuntime", e))?;
        Ok(r)
    }
    pub fn invoke(&self, program: &Path, args: &[&str], op: &str) -> Result<ChildOutput> {
        let policy = &self.manifest.value["timeoutPolicy"];
        let total = policy["laneTimeoutSeconds"]
            .as_u64()
            .ok_or_else(|| Failure::new("manifestInvalid", op, "laneTimeoutSeconds required"))?;
        let shard_timeout = policy["shardTimeoutSeconds"]
            .as_u64()
            .ok_or_else(|| Failure::new("manifestInvalid", op, "shardTimeoutSeconds required"))?;
        let remaining = total.saturating_sub(self.started.elapsed().as_secs());
        require(
            remaining > 0,
            "processFailure",
            op,
            "declared lane timeout exceeded",
        )?;
        child(program, args, &self.cwd, shard_timeout.min(remaining), op)
    }
    pub fn call(&self, program: &Path, args: &[&str], op: &str) -> Result<Vec<u8>> {
        self.invoke(program, args, op)?.success(op)
    }
    pub fn check(&self, bytes: &[u8], name: &str) -> Result<(Value, Value)> {
        let batch = self.temp.join(format!("{name}-batch.json"));
        fs::write(&batch, bytes)
            .map_err(|e| Failure::new("processFailure", "writeObservation", e))?;
        let stdout = self.call(
            &self.checker,
            &[
                "check",
                batch.to_str().ok_or_else(|| {
                    Failure::new("hostInvalid", "checkerCheck", "UTF8 path required")
                })?,
            ],
            "checkerCheck",
        )?;
        let checker = stdout_value(&stdout, "checkerCheck")?;
        let reconciliation = reconciliation::reconcile_bytes(bytes, &canonical(&checker))?;
        Ok((checker, reconciliation))
    }
    pub fn proof(&self) -> Result<Value> {
        let stdout = self.call(&self.checker, &["soundness"], "checkerSoundness")?;
        let text = std::str::from_utf8(&stdout)
            .map_err(|e| Failure::new("processFailure", "checkerSoundness", e))?;
        Ok(json!({"outputDigest":digest(&stdout),"status":"passed","text":text.trim()}))
    }
    fn parity(
        &self,
        request: &Value,
        observation: &Value,
        request_path: &Path,
        batch_path: &Path,
    ) -> Result<Value> {
        let output = self.invoke(
            &self.producer,
            &[
                "self-check",
                "--request",
                request_path.to_str().unwrap(),
                "--batch",
                batch_path.to_str().unwrap(),
            ],
            "producerSelfCheck",
        )?;
        if output.code != Some(0) {
            if output.code == Some(1) {
                if let Ok(report) = stdout_value(&output.bytes, "producerSelfCheck") {
                    if report["schema"] == "neco.verification.public-api-parity-result.v1"
                        && report["verified"] == false
                    {
                        return Err(Failure::new(
                            "parityMismatch",
                            "publicApiParity",
                            "actual public API replay disagreed",
                        )
                        .witness(report));
                    }
                }
            }
            return output
                .success("producerSelfCheck")
                .and_then(|b| stdout_value(&b, "producerSelfCheck"));
        }
        let parity = stdout_value(&output.bytes, "producerSelfCheck")?;
        fields(
            &parity,
            &[
                "schema",
                "verified",
                "requestDigest",
                "observationDigest",
                "cases",
            ],
            "parityMismatch",
            "publicApiParity",
        )?;
        require(
            parity["schema"] == "neco.verification.public-api-parity-result.v1"
                && parity["verified"] == true
                && parity["requestDigest"] == digest(&canonical(request))
                && parity["observationDigest"] == digest(&canonical(observation)),
            "parityMismatch",
            "publicApiParity",
            "actual API parity digest/verdict differs",
        )?;
        let cases = reconciliation::enumerate(observation)?;
        let replayed = array(&parity, "cases", "publicApiParity")?;
        require(
            replayed.len() == cases.len(),
            "parityMismatch",
            "publicApiParity",
            "parity case count differs",
        )?;
        for (p, c) in replayed.iter().zip(cases) {
            fields(
                p,
                &[
                    "caseId",
                    "verified",
                    "replayedObservationDigest",
                    "suppliedObservationDigest",
                ],
                "parityMismatch",
                "publicApiParity",
            )?;
            let d = digest(&canonical(c));
            require(
                p["caseId"] == c["caseId"]
                    && p["verified"] == true
                    && p["suppliedObservationDigest"] == d
                    && p["replayedObservationDigest"] == d,
                "parityMismatch",
                "publicApiParity",
                "public API per-case replay differs",
            )?;
        }
        Ok(parity)
    }
}

pub struct CorpusRun {
    pub report: Value,
    pub samples: BTreeMap<String, Value>,
}
struct ShardOutput {
    categories: BTreeMap<String, u64>,
    verdicts: BTreeMap<String, u64>,
    rows: Vec<[Vec<u8>; 3]>,
    samples: BTreeMap<String, Value>,
    report: Value,
}
fn run_shard(
    runtime: &Runtime,
    pass: &str,
    index: usize,
    id: &String,
    request: &Value,
) -> Result<ShardOutput> {
    let manifest = &runtime.manifest;
    let source = manifest.sources.iter().find(|s| s["id"] == *id).unwrap();
    let mut categories = BTreeMap::<String, u64>::new();
    let mut verdicts = BTreeMap::<String, u64>::new();
    let mut samples = BTreeMap::<String, Value>::new();
    let name = format!("{pass}-{id}-{index:04}");
    let request_path = runtime.temp.join(format!("{name}-request.json"));
    write(&request_path, request)?;
    let observation = stdout_value(
        &runtime.call(
            &runtime.producer,
            &["generate", "--request", request_path.to_str().unwrap()],
            "producerGenerate",
        )?,
        "producerGenerate",
    )?;
    let observation_bytes = canonical(&observation);
    let cases = reconciliation::enumerate(&observation)?;
    let requested = array(request, "cases", "runCorpus")?;
    require(
        cases.len() == requested.len(),
        "caseCorrespondenceMismatch",
        "runCorpus",
        "producer case count differs",
    )?;
    for (c, r) in cases.iter().zip(requested) {
        reconciliation::validate_case(c)?;
        require(
            c["caseId"] == r["caseId"] && c["operationFamily"] == r["family"],
            "caseCorrespondenceMismatch",
            "runCorpus",
            "producer family/order differs",
        )?;
        require(
            if *id == "cad-typed-failure" {
                c["publicObservation"]["kind"] == "typedFailure"
            } else if *id == "cad-mesh-rejected" {
                c["publicObservation"]["kind"] == "meshJudgment"
                    && c["publicObservation"]["value"]["outcome"]["kind"] == "rejected"
            } else {
                c["publicObservation"]["kind"] != "typedFailure"
                    && !(c["publicObservation"]["kind"] == "meshJudgment"
                        && c["publicObservation"]["value"]["outcome"]["kind"] != "validated")
            },
            "unexpectedObservation",
            "runCorpus",
            "normal/failure corpus category differs",
        )?;
        let kind = c["publicObservation"]["kind"].as_str().unwrap();
        *categories.entry(kind.into()).or_default() += 1;
        if kind == "typedFailure" {
            let category = string(&c["publicObservation"]["value"], "failureKind", "runCorpus")?;
            *categories.entry(category.into()).or_default() += 1;
        } else if kind == "meshJudgment" && *id == "cad-mesh-rejected" {
            let category = string(
                &c["publicObservation"]["value"]["outcome"],
                "failureKind",
                "runCorpus",
            )?;
            *categories.entry(category.into()).or_default() += 1;
        } else if kind == "certifiedChordBound" {
            for edge in array(&c["publicObservation"]["value"], "edges", "runCorpus")? {
                let category = string(&edge["parameterization"], "kind", "runCorpus")?;
                *categories.entry(format!("chord:{category}")).or_default() += 1;
            }
        }
    }
    let (checker, reconciliation) = runtime.check(&observation_bytes, &name)?;
    require(
        checker.get("cases").is_some(),
        "resultContradiction",
        "runCorpus",
        "generated corpus requires per-case verdicts",
    )?;
    let checked = array(&checker, "cases", "runCorpus")?;
    for c in checked {
        require(
            c["verdict"] == source["expectedVerdict"],
            "unexpectedVerdict",
            "runCorpus",
            "corpus verdict differs from separate declared category",
        )?;
        *verdicts
            .entry(c["verdict"].as_str().unwrap().into())
            .or_default() += 1;
    }
    let observation_path = runtime.temp.join(format!("{name}-batch.json"));
    let parity = runtime.parity(request, &observation, &request_path, &observation_path)?;
    let rows = requested
        .iter()
        .zip(&cases)
        .zip(checked)
        .map(|((r, o), k)| [canonical(r), canonical(o), canonical(k)])
        .collect::<Vec<_>>();
    if *id == "cad" {
        for c in &cases {
            let f = c["operationFamily"].as_str().unwrap();
            if !samples.contains_key(f) {
                samples.insert(f.to_owned(),json!({"schema":observation["schema"],"proofSubjectRef":observation["proofSubjectRef"],"testSetRef":observation["testSetRef"],"cases":[c]}));
            }
        }
    }
    let shard_report = json!({"corpusId":id,"shardIndex":index,"caseCount":cases.len(),"requestDigest":digest(&canonical(request)),"observationDigest":digest(&observation_bytes),"checkerDigest":digest(&canonical(&checker)),"checkerCases":checked,"publicApiParity":parity,"resultMatched":reconciliation["resultMatched"]});
    write(
        &runtime.reports.join(format!("{name}-request.json")),
        request,
    )?;
    write(
        &runtime.reports.join(format!("{name}-observation.json")),
        &observation,
    )?;
    write(
        &runtime.reports.join(format!("{name}-checker.json")),
        &checker,
    )?;
    Ok(ShardOutput {
        categories,
        verdicts,
        rows,
        samples,
        report: shard_report,
    })
}
pub fn run_corpus(runtime: &Runtime, pass: &str, shard: usize) -> Result<CorpusRun> {
    let manifest = &runtime.manifest;
    coverage(manifest, &runtime.descriptor, shard)?;
    let batches = manifest.batches(shard, Some(&runtime.descriptor))?;
    let next = AtomicUsize::new(0);
    let slots = Mutex::new(
        (0..batches.len())
            .map(|_| None)
            .collect::<Vec<Option<Result<ShardOutput>>>>(),
    );
    let workers = runtime.parallelism.min(batches.len()).max(1);
    thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| loop {
                let index = next.fetch_add(1, Ordering::SeqCst);
                let Some((id, request)) = batches.get(index) else {
                    break;
                };
                let output = run_shard(runtime, pass, index, id, request);
                slots.lock().unwrap()[index] = Some(output);
            });
        }
    });
    let mut digesters = BTreeMap::<String, (Sha256, Sha256, Sha256)>::new();
    let mut counts = BTreeMap::<String, BTreeMap<String, u64>>::new();
    let mut shards = Vec::new();
    let mut samples = BTreeMap::new();
    let mut observation_categories = BTreeMap::<String, BTreeMap<String, u64>>::new();
    for ((id, _), slot) in batches.iter().zip(slots.into_inner().unwrap()) {
        let output = slot.unwrap()?;
        let digesters = digesters
            .entry(id.clone())
            .or_insert_with(|| (Sha256::new(), Sha256::new(), Sha256::new()));
        for [r, o, k] in &output.rows {
            digesters.0.update(r);
            digesters.1.update(o);
            digesters.2.update(k);
        }
        for (key, n) in output.verdicts {
            *counts
                .entry(id.clone())
                .or_default()
                .entry(key)
                .or_default() += n;
        }
        for (key, n) in output.categories {
            *observation_categories
                .entry(id.clone())
                .or_default()
                .entry(key)
                .or_default() += n;
        }
        for (family, sample) in output.samples {
            samples.entry(family).or_insert(sample);
        }
        shards.push(output.report);
    }
    for (id, counts) in &observation_categories {
        if id == "cad" {
            require(
                counts.get("chord:line").copied().unwrap_or(0) > 0
                    && counts.get("chord:arc").copied().unwrap_or(0) > 0,
                "coverageMissing",
                "runCorpus",
                "actual normal chord corpus must cover line and arc parameterizations",
            )?;
        }
        if id == "cad-mesh-rejected" {
            require(
                ["openEdge", "orientationMismatch", "degenerateTriangle"]
                    .iter()
                    .all(|k| counts.get(*k).copied().unwrap_or(0) > 0),
                "coverageMissing",
                "runCorpus",
                "actual rejected mesh transition classes are missing",
            )?;
        }
    }
    let corpora: serde_json::Map<String,Value> = digesters.into_iter().map(|(id,(r,o,c))|(id.clone(),json!({"requestDigest":format!("{:x}",r.finalize()),"observationDigest":format!("{:x}",o.finalize()),"checkerDigest":format!("{:x}",c.finalize()),"verdictCounts":counts[&id],"observationCategoryCounts":observation_categories[&id]}))).collect();
    Ok(CorpusRun {
        report: json!({"corpora":corpora,"shards":shards,"processErrorCount":0}),
        samples,
    })
}
pub fn repeat(first: &CorpusRun, second: &CorpusRun) -> Result<Value> {
    require(
        first.report == second.report,
        "repeatabilityMismatch",
        "compareRepeatability",
        "request/observation/checker full case semantics, parity, or verdict categories differ",
    )?;
    Ok(
        json!({"status":"passed","realPasses":2,"sequential":true,"corpora":first.report["corpora"],"checkerCaseRowsMatched":true,"shardSemanticsMatched":true}),
    )
}
