use super::*;

fn relative_import(parent: &Path, import: &str) -> Result<String, String> {
    let mut result = parent.to_path_buf();
    for c in Path::new(import).components() {
        match c {
            std::path::Component::CurDir => {}
            std::path::Component::Normal(p) => result.push(p),
            std::path::Component::ParentDir => {
                if !result.pop() {
                    return Err("proof import escapes selected root".into());
                }
            }
            _ => return Err("absolute/invalid proof import".into()),
        }
    }
    result
        .to_str()
        .map(str::to_string)
        .ok_or("proof path UTF-8".into())
}
fn unknown(reason: String) -> (Value, i32) {
    (
        json!({"schema":"ultragoal/1","state":"unknown","reason":reason}),
        2,
    )
}
pub fn verify(args: &[String]) -> Result<(Value, i32), String> {
    let input = PathBuf::from(option(args, "--bend-proof")?.ok_or("--bend-proof required")?);
    let parent =
        fs::canonicalize(input.parent().unwrap_or(Path::new("."))).map_err(|e| e.to_string())?;
    let root = if let Some(path) = option(args, "--root")? {
        fs::canonicalize(path).map_err(|e| e.to_string())?
    } else {
        parent.clone()
    };
    let absolute = parent.join(input.file_name().ok_or("proof name")?);
    let selected = absolute
        .strip_prefix(&root)
        .map_err(|_| "proof outside selected root")?
        .to_str()
        .ok_or("proof path UTF-8")?
        .to_string();
    let observer = inventory::Session::open(&root)?;
    let listing = observer.enumerate()?;
    let mut pending = vec![selected.clone()];
    let adjacent = relative_import(
        Path::new(&selected).parent().unwrap_or(Path::new("")),
        "./LAWS.bend",
    )?;
    if listing
        .members
        .iter()
        .any(|m| m.path == adjacent.as_bytes())
    {
        pending.push(adjacent);
    }
    let mut sources = BTreeMap::new();
    let mut identities = BTreeMap::new();
    let mut total = 0usize;
    let mut declarations = Vec::new();
    while let Some(path) = pending.pop() {
        if sources.contains_key(&path) {
            continue;
        }
        if sources.len() >= 64 {
            return Ok(unknown("proof closure exceeds64 files".into()));
        }
        let wanted = std::collections::BTreeSet::from([path.clone()]);
        let observation = observer.capture(&listing, Some(&wanted))?;
        if let Some(problem) = observation.problems.get(&path) {
            return Ok(unknown(format!("proof input {path}: {problem:?}")));
        }
        let Some(bytes) = observation.files.get(&path) else {
            return Ok(unknown(format!("missing proof import {path}")));
        };
        total += bytes.len();
        if total > 4 * 1024 * 1024 {
            return Ok(unknown("proof closure exceeds4MiB".into()));
        }
        let source = std::str::from_utf8(bytes).map_err(|_| "proof UTF-8")?;
        let plan = evaluate("PROOF_SOURCE\n".to_string() + &row(&[source]))?;
        if plan
            .iter()
            .any(|r| r.first().is_some_and(|s| s == "REFUSED"))
        {
            return Ok(unknown(format!(
                "proof closure {path} contains unsupported imports, unsafe definitions or incomplete holes"
            )));
        }
        if plan.first() != Some(&vec!["SOURCE".into(), "admitted".into()]) {
            return Err("proof source admission schema".into());
        }
        for r in plan.iter().skip(1) {
            if r.len() != 2 || r[0] != "IMPORT" {
                return Err("proof import schema".into());
            }
            pending.push(relative_import(
                Path::new(&path).parent().unwrap_or(Path::new("")),
                &r[1],
            )?);
        }
        for line in source.lines() {
            if let Some(law) = line.trim().strip_prefix("law ") {
                declarations.push(format!("{path}:{}", law.trim_end_matches(':')));
            }
        }
        identities.insert(path.clone(), observation.identities.get(&path).copied());
        sources.insert(path, bytes.clone());
    }
    if declarations.is_empty() {
        return Ok(unknown("proof closure declares no laws".into()));
    }
    let tool = env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or("host home unavailable")?
        .join(".bend/bin/bend");
    let tool_hash = hash(&read(&tool, 256 * 1024 * 1024)?);
    let base = tool
        .parent()
        .and_then(Path::parent)
        .ok_or("compiler root")?
        .join("bend2/base.bend");
    let base_hash = hash(&read(&base, MAX)?);
    let operation = op_id()?;
    let dir = env::temp_dir().join(format!("ug-proof-{operation}"));
    fs::create_dir(&dir).map_err(|e| e.to_string())?;
    struct Cleanup(PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(dir.clone());
    for (path, bytes) in &sources {
        let target = dir.join(path);
        fs::create_dir_all(target.parent().ok_or("proof parent")?).map_err(|e| e.to_string())?;
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(target)
            .map_err(|e| e.to_string())?;
        f.write_all(bytes).map_err(|e| e.to_string())?;
        f.sync_all().map_err(|e| e.to_string())?;
    }
    let mut command = Command::new(&tool);
    command
        .env("BEND_NO_TELEMETRY", "1")
        .arg(dir.join(&selected))
        .arg("--check-only");
    let p = run(command, Vec::new(), Duration::from_secs(30))?;
    let output = String::from_utf8_lossy(&p.out);
    let errors = String::from_utf8_lossy(&p.err);
    let wanted = sources.keys().cloned().collect();
    let after = observer.observe(Some(&wanted))?;
    let stable = sources.iter().all(|(path, bytes)| {
        after.files.get(path) == Some(bytes)
            && after.identities.get(path).copied() == identities[path]
            && !after.problems.contains_key(path)
            && read(&dir.join(path), MAX).ok().as_ref() == Some(bytes)
    }) && read(&tool, 256 * 1024 * 1024)
        .map(|b| hash(&b))
        .ok()
        .as_deref()
        == Some(tool_hash.as_str())
        && read(&base, MAX).map(|b| hash(&b)).ok().as_deref() == Some(base_hash.as_str());
    let disposition = evaluate(
        "PROOF_RESULT\n".to_string()
            + &row(&[
                &p.code.map(|x| x.to_string()).unwrap_or_default(),
                if p.timeout || p.truncated || p.cancelled || !stable {
                    "incomplete"
                } else {
                    "complete"
                },
                &output,
                &errors,
            ]),
    )?;
    let state = disposition.first().ok_or("proof disposition")?;
    if state.len() != 3 {
        return Err("proof disposition schema".into());
    }
    let closure: BTreeMap<_, _> = sources.iter().map(|(p, b)| (p, hash(b))).collect();
    Ok((
        json!({"schema":"ultragoal/1","operation":operation,"state":state[1],"surface":"Bend checker result for exact bounded local import closure; per-law names are source-line candidates, not a separate parsed theorem inventory","entry":selected,"source_closure":closure,"declared_law_line_candidates":declarations,"compiler_sha256":tool_hash,"base_sha256":base_hash,"stdout":output,"stderr":errors,"trust_assumptions":["installed compiler/Base/runtime correctness","temporary input remains same-user writable; no host attestation or atomic tool identity"],"unsupported":"remote/content-addressed packages, foreign imports, unsafe definitions, unsupported local import syntax, closure beyond64 files/4MiB","latency_ms":p.ms}),
        state[2].parse().map_err(|_| "proof exit")?,
    ))
}
