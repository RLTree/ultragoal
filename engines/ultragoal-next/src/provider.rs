use super::*;
mod credential;
mod dispatch;
use credential::CredentialError;
#[cfg(test)] use credential::validate_credential;
use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use std::fmt;
use std::os::fd::AsRawFd;

// JSON compatibility decoding is mechanical. Bend validates numeric outputs and
// decides their semantic disposition. Duplicate keys are rejected at every depth.
pub(crate) struct Strict(pub(crate) Value);
impl<'de> Deserialize<'de> for Strict {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Strict;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("unique-key JSON")
            }
            fn visit_bool<E: de::Error>(self, v: bool) -> Result<Strict, E> {
                Ok(Strict(json!(v)))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Strict, E> {
                Ok(Strict(json!(v)))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Strict, E> {
                Ok(Strict(json!(v)))
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Strict, E> {
                if !v.is_finite() {
                    return Err(E::custom("nonfinite"));
                }
                Ok(Strict(json!(v)))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Strict, E> {
                Ok(Strict(json!(v)))
            }
            fn visit_string<E: de::Error>(self, v: String) -> Result<Strict, E> {
                Ok(Strict(json!(v)))
            }
            fn visit_unit<E: de::Error>(self) -> Result<Strict, E> {
                Ok(Strict(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> Result<Strict, A::Error> {
                let mut v = Vec::new();
                while let Some(x) = a.next_element::<Strict>()? {
                    v.push(x.0)
                }
                Ok(Strict(Value::Array(v)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> Result<Strict, A::Error> {
                let mut m = serde_json::Map::new();
                while let Some(k) = a.next_key::<String>()? {
                    if m.contains_key(&k) {
                        return Err(de::Error::custom("duplicate key"));
                    }
                    m.insert(k, a.next_value::<Strict>()?.0);
                }
                Ok(Strict(Value::Object(m)))
            }
        }
        d.deserialize_any(V)
    }
}
pub fn decode(bytes: &[u8]) -> Result<Value, String> {
    serde_json::from_slice::<Strict>(bytes)
        .map(|s| s.0)
        .map_err(|e| e.to_string())
}

/// A provider response echoes request ids and option keys plus one number per
/// option; 512 KiB, depth 32 and 8,192 values cover every jev-1.13 answer shape.
const MAX_RESPONSE: usize = 512 * 1024;
fn decode_response(bytes: &[u8]) -> Result<Value, String> {
    if bytes.len() > MAX_RESPONSE {
        return Err("provider response exceeds 512 KiB".into());
    }
    let value = decode(bytes)?;
    let mut pending = vec![(&value, 0usize)];
    let mut count = 0usize;
    while let Some((node, depth)) = pending.pop() {
        count += 1;
        if depth > 32 || count > 8192 {
            return Err("provider response exceeds depth 32 or 8192 values".into());
        }
        match node {
            Value::Array(items) => pending.extend(items.iter().map(|v| (v, depth + 1))),
            Value::Object(items) => pending.extend(items.values().map(|v| (v, depth + 1))),
            _ => {}
        }
    }
    Ok(value)
}

/// The provider is fixed at compile time. A synthetic build posts to a loopback
/// port named at run time and never reads the credential; any other value fails
/// closed instead of falling back to the real provider.
const PROVIDER: Option<&str> = option_env!("UG_TYPESAFE_PROVIDER");
const SYNTHETIC_BEARER: &str = "UG-SYNTHETIC-BEARER";
const DEFAULT_KEYCHAIN_SERVICE: &str = "research-run.typesafe";
struct Target {
    name: &'static str,
    url: String,
    protocol: &'static str,
}
fn target() -> Result<Target, String> {
    match PROVIDER {
        None => Ok(Target { name: "typesafe", url: "https://api.typesafe.ai/v1/systemone".into(), protocol: "=https" }),
        Some("synthetic-loopback-v1") => {
            let raw = env::var("UG_SYNTHETIC_PROVIDER_PORT").map_err(|_| "synthetic provider port")?;
            loopback_target(&raw)
        }
        Some(_) => Err("synthetic provider unknown".into()),
    }
}
fn loopback_target(raw: &str) -> Result<Target, String> {
    let port = Some(raw)
        .filter(|r| !r.is_empty() && r.bytes().all(|b| b.is_ascii_digit()))
        .and_then(|r| r.parse::<u16>().ok())
        .filter(|p| *p >= 1024)
        .ok_or("synthetic provider port")?;
    Ok(Target { name: "synthetic-loopback-v1", url: format!("http://127.0.0.1:{port}/v1/systemone"), protocol: "=http" })
}

fn contains_credential(value: &Value, key: &str) -> bool {
    match value {
        Value::String(s) => s.contains(key),
        Value::Array(xs) => xs.iter().any(|v| contains_credential(v, key)),
        Value::Object(m) => m
            .iter()
            .any(|(k, v)| k.contains(key) || contains_credential(v, key)),
        _ => false,
    }
}

fn raw_credential_echo(raw: &str, key: &str) -> bool {
    if raw.contains(key) {
        return true;
    }
    let mut start = None;
    let mut escaped = false;
    for (i, c) in raw.char_indices() {
        if let Some(begin) = start {
            if escaped {
                escaped = false;
                continue;
            }
            if c == '\\' {
                escaped = true;
                continue;
            }
            if c == '"' {
                if serde_json::from_str::<String>(&raw[begin..i + 1]).is_ok_and(|s| s.contains(key))
                {
                    return true;
                }
                start = None;
            }
        } else if c == '"' {
            start = Some(i);
        }
    }
    false
}
fn object<'a>(v: &'a Value, keys: &[&str]) -> Result<&'a serde_json::Map<String, Value>, String> {
    let m = v.as_object().ok_or("expected object")?;
    if m.len() != keys.len() || keys.iter().any(|k| !m.contains_key(*k)) {
        return Err(format!("unexpected object shape; expected {keys:?}"));
    }
    Ok(m)
}
fn numeric(v: &Value) -> Result<String, String> {
    v.as_number()
        .map(ToString::to_string)
        .ok_or("expected number".into())
}
fn s(v: &Value) -> Result<&str, String> {
    v.as_str().ok_or("expected string".into())
}
fn individually_validated(response: &Value, validation: &[Vec<String>]) -> Value {
    let mut answers = serde_json::Map::new();
    for r in validation {
        if r.len() == 3
            && r[2] != "malformed"
            && let Some(answer) = response["answers"].get(&r[1])
        {
            answers.insert(r[1].clone(), answer.clone());
        }
    }
    Value::Object(answers)
}
#[derive(Clone)]
struct ChoiceLayout { tag: String, names: [String;3] }
type ChoiceLayouts = BTreeMap<String,ChoiceLayout>;

fn decode_layouts(request: &Value, rows: Vec<Vec<String>>) -> Result<ChoiceLayouts,String> {
    let questions=request["questions"].as_object().ok_or("questions object")?;
    let expected=questions.iter().filter(|(_,q)|q["type"]=="choice").count();
    if rows.len()!=expected {return Err("Bend Choice layout coverage mismatch".into());}
    let mut layouts=ChoiceLayouts::new();
    for r in rows {
        if r.len()!=6 || r[0]!="CHOICE_LAYOUT" {return Err("Bend refused Choice schema".into());}
        let q=questions.get(&r[1]).ok_or("unrequested layout identity")?;
        let criteria=q["criteria"].as_object().ok_or("Choice criteria")?;
        let names=[r[3].clone(),r[4].clone(),r[5].clone()];
        let distinct=names.iter().collect::<std::collections::BTreeSet<_>>();
        if q["type"]!="choice" || criteria.len()!=names.len() || distinct.len()!=names.len()
            || names.iter().any(|n|!criteria.contains_key(n)) || r[2].is_empty()
            || layouts.insert(r[1].clone(),ChoiceLayout{tag:r[2].clone(),names}).is_some() {
            return Err("Bend layout does not bind the requested option fields".into());
        }
    }
    Ok(layouts)
}

fn plan_choices(request: &Value, deadline: Instant) -> Result<ChoiceLayouts,String> {
    let questions=request["questions"].as_object().ok_or("questions object")?;
    let mut frame="JEV_LAYOUTS\n".to_string();let mut count=0;
    for (id,q) in questions {
        if q["type"]=="choice" {
            let criteria=q["criteria"].as_object().ok_or("Choice criteria object")?;
            let mut fields=vec![id.as_str()];
            for (name,description) in criteria {
                fields.push(name);fields.push(description.as_str().ok_or("Choice description text")?);
            }
            frame+=&row(&fields);count+=1;
        }
    }
    if count==0 {return Ok(ChoiceLayouts::new());}
    decode_layouts(request,evaluate_until(frame,deadline)?)
}

fn response_frame_planned(request: &Value, response: &Value, layouts: &ChoiceLayouts) -> Result<String, String> {
    object(response, &["model", "answers", "usage"])?;
    if !response["usage"].is_null() {
        object(&response["usage"], &["input_tokens", "output_tokens"])?;
        if response["usage"]["input_tokens"].as_u64().is_none()
            || response["usage"]["output_tokens"].as_u64().is_none()
        {
            return Err("invalid provider usage counts".into());
        }
    }
    if response["model"] != request["model"] {
        return Err("wrong model".into());
    }
    let questions = request["questions"].as_object().ok_or("questions")?;
    let answers = response["answers"].as_object().ok_or("answers")?;
    if questions.len() != answers.len() || questions.keys().any(|k| !answers.contains_key(k)) {
        return Err("missing/extra question ID".into());
    }
    let mut frame = "JEV\n".to_string();
    for (id, q) in questions {
        let a = &answers[id];
        if q["type"] != a["type"] {
            return Err("wrong answer type".into());
        }
        match s(&q["type"])? {
            "choice" => {
                object(a, &["type", "choice", "confidence", "probabilities"])?;
                let opts = q["criteria"].as_object().ok_or("choice criteria")?;
                let probabilities = a["probabilities"]
                    .as_object()
                    .ok_or("choice probabilities")?;
                let layout = layouts.get(id).ok_or("missing Bend Choice layout")?;
                let tag = layout.tag.as_str();
                let names = &layout.names;
                if probabilities.len() != opts.len()
                    || opts.keys().any(|k| !probabilities.contains_key(k))
                {
                    return Err("wrong option set".into());
                }
                frame += &row(&[
                    tag,
                    id,
                    s(&a["choice"])?,
                    &numeric(&a["confidence"])?,
                    &numeric(&probabilities[&names[0]])?,
                    &numeric(&probabilities[&names[1]])?,
                    &numeric(&probabilities[&names[2]])?,
                ]);
            }
            "noul" => {
                object(a, &["type", "noul"])?;
                frame += &row(&["NOUL", id, &numeric(&a["noul"])?]);
            }
            "score" => {
                object(
                    a,
                    &["type", "score", "confidence", "legend", "probabilities"],
                )?;
                let criteria = q["criteria"].as_array().ok_or("score criteria")?;
                let legend = a["legend"].as_object().ok_or("score legend")?;
                let probs = a["probabilities"]
                    .as_object()
                    .ok_or("score probabilities")?;
                if criteria.len() < 2
                    || criteria.len() > 10
                    || legend.len() != criteria.len()
                    || probs.len() != criteria.len()
                {
                    return Err("wrong score dimensions".into());
                }
                let mut cols = vec![
                    "SCORE".to_string(),
                    id.clone(),
                    numeric(&a["score"])?,
                    numeric(&a["confidence"])?,
                ];
                for (i, c) in criteria.iter().enumerate() {
                    let key = i.to_string();
                    if legend.get(&key) != Some(c) {
                        return Err("wrong score legend/order".into());
                    }
                    cols.push(numeric(probs.get(&key).ok_or("missing score level")?)?);
                }
                frame += &row(&cols.iter().map(String::as_str).collect::<Vec<_>>());
            }
            _ => return Err("unsupported question kind".into()),
        }
    }
    Ok(frame)
}
fn credential_refusal(operation: &str, error: CredentialError) -> (Value, i32) {
    (json!({"schema":"ultragoal/1","operation":operation,"state":"unknown",
            "error_code":"credential_refused","reason":error.to_string(),"attempts":[]}), 2)
}

pub fn assess(args: &[String]) -> Result<(Value, i32), String> {
    let path = option(args, "--request")?.ok_or("--request required")?;
    let raw = read(Path::new(&path), 64_000)?;
    if let Some(path) = option(args, "--response")? {
        return Ok((validate_reported(&raw, &read(Path::new(&path), MAX_RESPONSE)?)?, 2));
    }
    assess_bytes(args, raw)
}

/// Validates preserved response bytes against their request with the current Bend
/// checks; no provider call, freshness or host provenance is implied.
fn validate_reported(raw: &[u8], response: &[u8]) -> Result<Value, String> {
    let text = std::str::from_utf8(response).map_err(|_| "response UTF-8")?;
    let numeric = evaluate("NUMERIC\n".to_string() + &row(&[text]))?;
    if numeric != vec![vec!["NUMERIC".to_string(), "valid".to_string()]] {
        return Err("raw numeric domain rejected".into());
    }
    let decoded_response = decode_response(response)?;
    let request = decode(raw)?;
    let layouts = plan_choices(&request, Instant::now() + Duration::from_secs(3))?;
    let validation = evaluate(response_frame_planned(&request, &decoded_response, &layouts)?)?;
    let validated_answers = individually_validated(&decoded_response, &validation);
    Ok(json!({"schema":"ultragoal-reported-response/1","state":"reported-advisory","request_sha256":hash(raw),"response_sha256":hash(response),"raw_response_json":text,"validation":validation,"validated_answers":validated_answers,"assurance":"imported bytes validated only; no provider execution, freshness or host provenance claimed","attempts":[]}))
}

/// Prior semantic responses under an explicit `--cache DIR`, keyed by the digest of
/// the exact request bytes (requirement, probe version and evidence content). An
/// identical request reuses the stored response, revalidated and decided by the
/// current Bend policy, and is reported as prior advice; `--semantic` asks afresh.
fn semantic_store(args: &[String]) -> Result<Option<PathBuf>, String> {
    Ok(option(args, "--cache")?.map(|dir| PathBuf::from(dir).join("semantic")))
}

fn prior_assessment(dir: &Path, raw: &[u8]) -> Result<Option<Value>, String> {
    let path = dir.join(format!("{}.json", hash(raw)));
    if !path.exists() {
        return Ok(None);
    }
    let stored = decode(&read(&path, MAX)?)?;
    let response = stored["raw_response_json"].as_str().ok_or("stored semantic response")?;
    let mut assessment = validate_reported(raw, response.as_bytes())?;
    assessment["state"] = json!("reused-prior");
    assessment["prior"] = json!({"recorded_unix_ms":stored["recorded_unix_ms"],"attempt_ids":stored["attempt_ids"],"provider":stored["provider"]});
    Ok(Some(assessment))
}

fn store_assessment(dir: &Path, raw: &[u8], assessment: &Value) -> Result<(), String> {
    let (Some(response), Some(rows)) = (assessment["raw_response_json"].as_str(), assessment["validation"].as_array()) else {
        return Ok(());
    };
    if rows.is_empty() || !rows.iter().all(|r| r[2].as_str().is_some_and(|l| l.starts_with("advisory-"))) {
        return Ok(());
    }
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{}.json", hash(raw)));
    let tmp = dir.join(format!(".{}.{}.tmp", hash(raw), std::process::id()));
    let attempt = assessment["attempts"].as_array().and_then(|a| a.last());
    let body = json!({"request_sha256":hash(raw),"raw_response_json":response,"recorded_unix_ms":std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e| e.to_string())?.as_millis(),"attempt_ids":assessment["attempts"].as_array().map(|a| a.iter().map(|x| x["attempt_id"].clone()).collect::<Vec<_>>()),"provider":attempt.map(|a| a["provider"].clone())});
    let mut f = OpenOptions::new().write(true).create_new(true).mode(0o600).custom_flags(libc::O_NOFOLLOW).open(&tmp).map_err(|e| e.to_string())?;
    f.write_all(&serde_json::to_vec(&body).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    f.sync_all().map_err(|e| e.to_string())?;
    fs::rename(tmp, path).map_err(|e| e.to_string())
}

/// Advisory ordering only. All candidates survive; no candidate is executed.
/// Candidates are ranked deterministically by Bend and judged in a ranked head
/// plus slices, so related candidates are judged beside the top-ranked ones.
fn advice_stage(mode: &str, initial: &str, rows: Vec<String>) -> Result<Vec<Vec<String>>, String> {
    let ordinary = if mode == "order" { "ADVICE_ORDER\n".to_string() + &row(&[initial]) } else { "ADVICE_RANK\n".to_string() };
    let bytes = ordinary.len() + rows.iter().map(String::len).sum::<usize>();
    if bytes <= MAX && env::var_os("UG_ADVICE_PAGE_BYTES").is_none() {
        return evaluate(ordinary + &rows.concat());
    }
    let mut lane = core_session::Lane::new("advice-page")?;
    let operation = format!("advice-{}-{mode}", std::process::id());
    let mut send = |fields: &[&str]| -> Result<Vec<Vec<String>>, String> {
        decode_core_rows(lane.request_one(row(fields), MAX)?)
    };
    let ack = send(&["ADV_BEGIN", &operation, mode, initial, &rows.len().to_string()])?;
    if ack != vec![vec!["ADV_ACK".to_string(), "begin".to_string()]] { return Err("Bend advice begin refused".into()); }
    let forced = env::var("UG_ADVICE_PAGE_BYTES").ok().and_then(|v| v.parse::<usize>().ok());
    let (mut sequence, mut page) = (0usize, String::new());
    let mut limit = resources::current().map(|h| h.frame_bytes(MAX)).unwrap_or(2 * 1024 * 1024);
    if limit == 0 { return Err("advice paused by current memory pressure".into()); }
    limit = limit.min(forced.unwrap_or(MAX)).min(MAX).max(1024);
    for item in rows {
        if !page.is_empty() && row(&["ADV_PAGE", &operation, &sequence.to_string(), &(page.clone() + &item)]).len() > limit {
            let ack = send(&["ADV_PAGE", &operation, &sequence.to_string(), &page])?;
            if ack != vec![vec!["ADV_ACK".to_string(), "page".to_string()]] { return Err("Bend advice page refused".into()); }
            sequence += 1;
            page.clear();
            limit = resources::current().map(|h| h.frame_bytes(MAX)).unwrap_or(2 * 1024 * 1024);
            if limit == 0 { return Err("advice paused by current memory pressure".into()); }
            limit = limit.min(forced.unwrap_or(MAX)).min(MAX).max(1024);
        }
        page += &item;
        if row(&["ADV_PAGE", &operation, &sequence.to_string(), &page]).len() > MAX {
            let _ = send(&["ADV_ABORT"]);
            return Err("one advice item exceeds the core's physical frame bound".into());
        }
    }
    if !page.is_empty() {
        let ack = send(&["ADV_PAGE", &operation, &sequence.to_string(), &page])?;
        if ack != vec![vec!["ADV_ACK".to_string(), "page".to_string()]] { return Err("Bend advice page refused".into()); }
        sequence += 1;
    }
    let result = send(&["ADV_END", &operation, &sequence.to_string()])?;
    lane.finish()?;
    if result.iter().any(|r| r.first().is_some_and(|x| x == "RANK_ERROR" || x == "ERROR")) {
        return Err("Bend advice end refused".into());
    }
    Ok(result)
}

pub fn advise(args: &[String]) -> Result<(Value, i32), String> {
    let input = option(args, "--request")?.ok_or("--request context file required")?;
    // Screen the target actually opened, not a caller-selected relative spelling.
    // A relative path can otherwise hide a .codex/.agents ancestor from Bend.
    let resolved = fs::canonicalize(&input).map_err(|e| e.to_string())?;
    let request_bytes = fs::metadata(&resolved).map_err(|e| e.to_string())?.len();
    let raw_context = if request_bytes > (MAX / 4) as u64 {
        segmented::read_disclosed(&resolved)?
    } else {
        let raw = read(&resolved, MAX)?;
        let disclosure = evaluate("DISCLOSURE_CHECK\n".to_string() + &row(&[
            resolved.to_str().ok_or("context path UTF-8")?,
            std::str::from_utf8(&raw).map_err(|_| "context UTF-8")?,
        ]))?;
        if disclosure != vec![vec!["DISCLOSURE".to_string(), "eligible".to_string()]] {
            return Err("advice context excluded by disclosure policy".into());
        }
        raw
    };
    let context = decode(&raw_context)?;
    let purpose = s(&context["purpose"])?;
    if s(&context["question"])?.is_empty() {
        return Err("advice question required".into());
    }
    let candidates = context["candidates"]
        .as_array()
        .ok_or("advice candidates array")?;
    let mut plan = Vec::new();
    for (start, batch) in candidates.chunks(256).enumerate() {
        let offset = start * 256;
        plan.extend(evaluate(
            "ADVICE_PLAN\n".to_string()
                + &row(&[purpose, &batch.len().to_string(), &offset.to_string()]),
        )?);
    }
    if plan.len() != candidates.len()
        || plan.iter().enumerate().any(|(i, r)| {
            r.len() < 4
                || r[0] != "ADVICE_QUESTION"
                || r[1] != format!("item{i}")
                || !matches!((r[2].as_str(), r.len()), ("score", 9) | ("noul", 4) | ("noul", 8))
        })
    {
        return Err("Bend refused advice purpose/candidate budget".into());
    }
    let order = advice_stage("order", &format!("{} {}", s(&context["question"])?, context["observations"]),
        candidates.iter().enumerate().map(|(i,c)| row(&[&i.to_string(), &c.to_string()])).collect())?;
    let mut seen = std::collections::BTreeSet::new();
    let mut ranked_indices = Vec::new();
    for r in &order {
        let i: usize = r.get(1).and_then(|v| v.parse().ok()).ok_or("advice order row")?;
        if r[0] != "ORDER" || i >= candidates.len() || !seen.insert(i) {
            return Err("Bend advice order identity mismatch".into());
        }
        ranked_indices.push(i);
    }
    if ranked_indices.len() != candidates.len() {
        return Err("Bend advice order omitted a candidate".into());
    }
    let items = ranked_indices
        .iter()
        .map(|&i| {
            let r = &plan[i];
            let question = match (r[2].as_str(), r.len()) {
                ("score", _) => json!({"type":"score","instructions":r[3],"criteria":r[4..]}),
                (_, 8) => json!({"type":"noul","instructions":r[3],"criteria":{r[4].clone():r[5],r[6].clone():r[7]}}),
                _ => json!({"type":"noul","instructions":r[3]}),
            };
            WindowItem { id: r[1].clone(), state: candidates[i].clone(), question }
        })
        .collect::<Vec<_>>();
    let start = Instant::now();
    let fixed = json!({"question":context["question"],"observations":context["observations"],"subject":context["subject"],"candidates":{}});
    let outcome = windowed(args, &fixed, &items, &budget(args)?)?;
    if outcome.suppressed {
        return Ok((
            json!({"schema":"ultragoal-advice/1","state":"unavailable","reason":"credential-bearing context suppressed","assessments":outcome.assessments,"ranked_candidates":[]}),
            2,
        ));
    }
    let mut ranked = Vec::new();
    for (index, r) in plan.iter().enumerate() {
        let judged = outcome.judgments.get(&r[1]).ok_or("window judgment missing")?;
        let (valid, answer, request, reason) = match judged {
            Ok((a, request)) => (true, a["answer"].clone(), json!(request), Value::Null),
            Err(reason) => (false, Value::Null, Value::Null, json!(reason)),
        };
        let scope = judged.as_ref().ok().map(|(_, request)| comparison_scope(outcome.head_request, *request));
        let ranking_value = if valid { answer[if r[2] == "score" { "score" } else { "noul" }].clone() } else { Value::Null };
        ranked.push(json!({"index":index,"candidate":candidates[index],"lexical_rank":ranked_indices.iter().position(|&i|i==index),"judgment_type":r[2],"ranking_value":ranking_value,"utility_probability":if valid{answer["noul"].clone()}else{Value::Null},"utility_score":if valid{answer["score"].clone()}else{Value::Null},"confidence":if valid{answer["confidence"].clone()}else{Value::Null},"window_request":request,"comparison_scope":scope,"question":r[1],"status":if valid{"advisory"}else{"uninspected"},"uninspected_reason":reason}));
    }
    let order = advice_stage("rank", "", ranked.iter().map(|r| row(&[&r["index"].to_string(), &r["ranking_value"].to_string()])).collect())?;
    let mut seen = std::collections::BTreeSet::new();
    let mut ordered = Vec::new();
    for r in order {
        if r.len() != 2 || r[0] != "RANK" {
            return Err("Bend advice rank shape".into());
        }
        let i: usize = r[1].parse().map_err(|_| "rank index")?;
        if i >= ranked.len() || !seen.insert(i) {
            return Err("Bend rank identity mismatch".into());
        }
        ordered.push(ranked[i].clone());
    }
    if ordered.len() != ranked.len() {
        return Err("Bend rank omitted a candidate".into());
    }
    let complete = ordered.iter().all(|r| r["status"] == "advisory");
    Ok((
        json!({"schema":"ultragoal-advice/2","state":if complete{"advisory"}else{"advisory-partial"},"purpose":purpose,"question":context["question"],"window":outcome.plan,"ranked_candidates":ordered,"assessments":outcome.assessments,"latency_ms":start.elapsed().as_millis(),"execution":"none; host/primary agent chooses and performs any action","mandatory_coverage":"unchanged; no candidate or requirement removed","ordering_scope":"validated suggestions first; uninspected candidates are named with their reason, not ruled out or necessarily less useful","assurance":"proposed usefulness only; no factual, permission, freshness or native-verification admission","calibration":"uncalibrated ordering; all original candidates and raw typed judgments retained"}),
        if complete { 0 } else { 2 },
    ))
}

/// One obligation per request: unrelated requirements cannot influence an answer.
pub fn assess_rows(args: &[String], rows: &[Vec<String>]) -> Result<Value, String> {
    let budget = budget(args)?;
    let results = in_flight(rows, budget.concurrency, |r| {
        assess_group(args, r, Instant::now() + budget.per_request).unwrap_or_else(
            |error| json!({"state":"unknown","reason":error,"obligation":r.get(1),"attempts":[]}),
        )
    });
    Ok(
        json!({"packet_policy":"one-obligation/3","group_count":rows.len(),"groups":results,"budget":budget.report(),"assurance":"uncalibrated advisory; each atomic request has its own deadline"}),
    )
}

fn prepare_obligation(r: &[String], probes: &[Vec<String>]) -> Result<(Value, Value), String> {
    if r.len() != 8 || r[0] != "REQUEST" || r[7].is_empty() {
        return Err("unsupported or malformed Bend question".into());
    }
    let mut excerpts = Vec::new();
    for line in r[5].lines() {
        let cols = line.split('\t').map(dec).collect::<Result<Vec<_>, _>>()?;
        if cols.len() != 3 {
            return Err("evidence projection shape".into());
        }
        excerpts.push(json!({"id":format!("source{}",excerpts.len()),"role":"unclassified-source","material_type":"unclassified","version":cols[1],"path":cols[0],"input_sha256":cols[1],"content":cols[2],"clause_ids":[],"provenance":"exact captured source bytes; source claims and role remain unauthenticated"}));
    }
    let mut context = json!({"schema":"ultragoal-source-context/3","target":r[4],"scope":"selected exact source excerpts only","clauses":[],"required_surfaces":"not separately declared; derive only from the original requirement, and report missing required evidence","evidence":excerpts});
    if excerpts.len() == 1 {
        let raw = s(&excerpts[0]["content"])?;
        if let Ok(declared) = decode(raw.as_bytes())
            && declared
                .get("schema")
                .and_then(Value::as_str)
                .is_some_and(|s| s.starts_with("ultragoal-semantic-context/"))
        {
            let validation = evaluate("SEMANTIC_CONTEXT\n".to_string() + &row(&[&r[4], raw]))?;
            if validation != vec![vec!["CONTEXT".to_string(), "valid".to_string()]] {
                return Err("declared semantic context rejected by Bend".into());
            }
            context = declared;
        }
    }
    let mut questions = serde_json::Map::new();
    // The rubric version is Bend's; both probes must declare the same one.
    let version = probes.first().and_then(|p| p.get(2)).ok_or("Bend semantic probe shape")?.clone();
    for p in probes {
        if p.len() != 10
            || p[0] != "PROBE"
            || version.is_empty()
            || !version.bytes().all(|b| b.is_ascii_digit())
            || p[2] != version
            || p[3].is_empty()
            || !matches!(p[1].as_str(), "conflict" | "support")
            || questions.contains_key(&p[1])
        {
            return Err("Bend semantic probe shape".into());
        }
        let criteria = p[4..]
            .chunks_exact(2)
            .map(|v| (v[0].clone(), json!(v[1])))
            .collect::<serde_json::Map<_, _>>();
        questions.insert(
            p[1].clone(),
            json!({"type":"choice","instructions":p[3],"criteria":criteria}),
        );
    }
    if questions.len() != 2 {
        return Err("both semantic probes required".into());
    }
    let binding = json!({"obligation":r[1],"rubric":r[4],"rubric_version":version,"context_validation":"declared schema and links only; not semantic completeness or native authority","questions":["conflict","support"],"combination":"conflict-first/1","input_key":digest_id(&r[6])?,"source_bindings":excerpts.iter().map(|e|json!({"path":e["path"],"input_sha256":e["input_sha256"]})).collect::<Vec<_>>()});
    let packet = json!({"model":"jev-1.13.0","state":{"schema":"ultragoal-semantic-state/3","admission_boundary":"This assesses supplied material meaning only. Native authenticity, identity, freshness, permission and adoption remain independently unqualified by this packet.","requirement":r[2],"requirement_origin":r[3],"context":context},"questions":questions});
    Ok((packet, binding))
}

fn assess_group(args: &[String], r: &[String], deadline: Instant) -> Result<Value, String> {
    let family = r.get(4).ok_or("missing semantic target")?;
    let probes = evaluate("SEMANTIC_PROBES\n".to_string() + &row(&[family]))?;
    let (packet, binding) = prepare_obligation(r, &probes)?;
    let raw = serde_json::to_vec(&packet).map_err(|e| e.to_string())?;
    let store = semantic_store(args)?;
    let prior = match &store {
        Some(dir) if !args.contains(&"--semantic".to_string()) => prior_assessment(dir, &raw)?,
        _ => None,
    };
    let assessment = match prior {
        Some(prior) => prior,
        None => {
            let (mut live, _) = assess_until(args, raw.clone(), deadline)?;
            if let Some(dir) = &store
                && let Err(error) = store_assessment(dir, &raw, &live)
            {
                live["prior_store_error"] = json!(error);
            }
            live
        }
    };
    let label = |id: &str| {
        assessment["validation"]
            .as_array()
            .and_then(|rows| rows.iter().find(|r| r[0] == "ANSWER" && r[1] == id))
            .and_then(|r| r[2].as_str())
            .unwrap_or("unavailable")
    };
    // Bend decides with its digest-bound fitted policy; the bridge only forwards the
    // option probabilities from the response it already validated.
    let response = assessment["raw_response_json"].as_str().and_then(|text| serde_json::from_str::<Value>(text).ok());
    let option = |id: &str, name: &str| {
        response.as_ref()
            .and_then(|v| v["answers"][id]["probabilities"][name].as_number().map(|n| n.to_string()))
            .unwrap_or_default()
    };
    let version = binding["rubric_version"].as_str().expect("binding carries the rubric version").to_string();
    let decision = evaluate(
        "SEMANTIC_CALIBRATED\n".to_string()
            + &row(&[
                &version,
                label("conflict"),
                label("support"),
                &option("conflict", "explicit-conflict"),
                &option("conflict", "no-explicit-conflict"),
                &option("conflict", "uncertain"),
                &option("support", "complete-support"),
                &option("support", "missing-support"),
                &option("support", "uncertain"),
                binding["rubric"].as_str().expect("binding carries the rubric family"),
            ]),
    )?;
    Ok(
        json!({"bindings":[binding],"distinct_evidence_packets":1,"question_count":2,"decision":decision,"review_signal":decision.first().and_then(|r|r.get(4)),"assessment":assessment,"combined_probability":null,"probability_note":"separate probe distributions retained; no statistical independence or calibrated combined probability assumed"}),
    )
}

fn assess_bytes(args: &[String], raw: Vec<u8>) -> Result<(Value, i32), String> {
    assess_until(args, raw, Instant::now() + budget(args)?.per_request)
}

/// Per-request deadline and requests in flight: runtime inputs (`--semantic-deadline-ms`,
/// default 3000; `--semantic-concurrency`, default 2) that Bend admits within its bounds.
pub struct Budget {
    pub per_request: Duration,
    pub concurrency: usize,
}

impl Budget {
    fn report(&self) -> Value {
        json!({"per_request_deadline_ms":self.per_request.as_millis(),"concurrency":self.concurrency})
    }
}

pub fn budget(args: &[String]) -> Result<Budget, String> {
    let ms = option(args, "--semantic-deadline-ms")?.unwrap_or_else(|| "3000".into());
    let k = option(args, "--semantic-concurrency")?.unwrap_or_else(|| "2".into());
    match evaluate("SEMANTIC_BUDGET\n".to_string() + &row(&[&ms, &k]))?.as_slice() {
        [r] if r.len() == 3 && r[0] == "BUDGET" => Ok(Budget {
            per_request: Duration::from_millis(r[1].parse().map_err(|_| "budget deadline")?),
            concurrency: r[2].parse().map_err(|_| "budget concurrency")?,
        }),
        _ => Err("semantic budget refused: --semantic-deadline-ms 500-15000, --semantic-concurrency 1-4".into()),
    }
}

pub fn assess_until(
    args: &[String],
    raw: Vec<u8>,
    absolute_deadline: Instant,
) -> Result<(Value, i32), String> {
    let mut record = ReceiptRecord {
        payload: None,
        operation: op_id()?,
        request_sha256: hash(&raw),
        attempts: Vec::new(),
        raw_response: None,
        raw_request: None,
        response_unvalidated: None,
    };
    let start = Instant::now();
    let result = assess_operation(args, raw, absolute_deadline, &mut record);
    finish_receipt(record, result, start.elapsed().as_millis())
}

fn finish_receipt(
    record: ReceiptRecord,
    result: Result<(Value, i32), String>,
    latency_ms: u128,
) -> Result<(Value, i32), String> {
    let (mut value, code) = match result {
        Ok(v) => v,
        Err(e) => (
            json!({"schema":"ultragoal/1","state":"unknown","reason":"local assessment boundary unavailable","detail":e}),
            2,
        ),
    };
    value["operation"] = json!(record.operation);
    value["request_sha256"] = json!(record.request_sha256);
    if let Some(raw) = record.raw_request {
        value["raw_request_json"] = json!(raw);
    }
    value["usage"] = usage_summary(&record.attempts);
    value["attempts"] = json!(record.attempts);
    if let Some(payload) = record.payload {
        value["payload"] = payload;
    }
    value["latency_ms"] = json!(latency_ms);
    value["assurance"] = json!("uncalibrated advisory");
    value["assurance_code"] = json!("semantic-advisory");
    value["assurance_detail"] =
        json!("Cannot discharge native, formal, permission or exact-identity obligations.");
    value["calibration"] = json!("none");
    value["calibration_code"] = json!("shadow-unqualified");
    value["calibration_detail"] =
        json!("No consequential rule promotion; original attempts and response bytes retained.");
    value["numeric_contract"] = json!({"authoritative":"raw_response_json","normalized":"IEEE754 binary64 nearest representable; float_roundtrip parsing; absolute equivalence tolerance1e-12 for reporting only","validation":"raw decimal domain checks before approximate mass/weighted-score checks; no tolerance permits out-of-domain raw values"});
    if let Some(raw) = record.raw_response {
        value["raw_response_json"] = json!(raw);
    }
    if value.get("response").is_none()
        && let Some(response) = record.response_unvalidated
    {
        value["usage_reported_unvalidated"] = response.get("usage").cloned().unwrap_or(Value::Null);
        value["response_unvalidated"] = response;
        value["response_validation"] = json!("rejected-or-unavailable; retained for evidence only");
    }
    Ok((value, code))
}

struct ReceiptRecord {
    payload: Option<Value>,
    operation: String,
    request_sha256: String,
    attempts: Vec<Value>,
    raw_response: Option<String>,
    raw_request: Option<String>,
    response_unvalidated: Option<Value>,
}

fn assess_operation(
    args: &[String],
    raw: Vec<u8>,
    absolute_deadline: Instant,
    record: &mut ReceiptRecord,
) -> Result<(Value, i32), String> {
    if raw.len() > MAX {
        return Err("hard transport byte limit".into());
    }
    let request = decode(&raw)?;
    object(&request, &["model", "state", "questions"])?;
    let qs = request["questions"].as_object().ok_or("questions object")?;
    // Payload composition, measured on the exact serialization Jev receives.
    let state_bytes = serde_json::to_vec(&request["state"]).map_err(|e| e.to_string())?.len();
    let question_sizes = qs
        .values()
        .map(|q| serde_json::to_vec(q).map(|b| b.len()).map_err(|e| e.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    let longest_question = question_sizes.iter().copied().max().unwrap_or(0);
    record.payload = Some(json!({"total_bytes":raw.len(),"state_bytes":state_bytes,"questions_bytes":question_sizes.iter().sum::<usize>(),"longest_question_bytes":longest_question,"question_count":qs.len(),"token_bound":"UTF-8 bytes bound tokens; jev-1.13 limits 64k per request and 32k for state plus the longest question"}));
    let admission = evaluate(
        "ADMIT_REQUEST\n".to_string()
            + &row(&[
                s(&request["model"])?,
                option(args, "--disclosure")?
                    .as_deref()
                    .unwrap_or("disabled"),
                &raw.len().to_string(),
                &state_bytes.saturating_add(longest_question).to_string(),
                &qs.len().to_string(),
                "running",
                &absolute_deadline
                    .saturating_duration_since(Instant::now())
                    .as_millis()
                    .to_string(),
            ]),
    )?;
    let admitted = admission.first().ok_or("missing Bend request admission")?;
    if admitted.len() != 5 || admitted[0] != "ADMITTED" {
        return Ok((
            json!({"schema":"ultragoal/1","state":"unknown","reason":"request not admitted","admission":admission,"attempts":[]}),
            2,
        ));
    }
    let approved_deadline =
        Instant::now() + Duration::from_millis(admitted[2].parse().map_err(|_| "deadline")?);
    let absolute_deadline = absolute_deadline.min(approved_deadline);
    let attempts_limit: u32 = admitted[3].parse().map_err(|_| "attempt limit")?;
    for q in qs.values() {
        let kind = s(&q["type"])?;
        object(
            q,
            if kind == "noul" && q.get("criteria").is_none() {
                &["type", "instructions"]
            } else {
                &["type", "instructions", "criteria"]
            },
        )?;
        if s(&q["instructions"])?.is_empty() {
            return Err("question instructions required".into());
        }

    }
    let layouts = plan_choices(&request,absolute_deadline)?;
    let target = target()?;
    let screened = match dispatch::prepare(args, &target, &request, &raw, absolute_deadline) {
        Ok(screened) => screened,
        Err(dispatch::PrepareError::Credential(error)) => return Ok(credential_refusal(&record.operation, error)),
        Err(dispatch::PrepareError::Disclosure(reason)) => return Err(reason.into()),
        Err(dispatch::PrepareError::Core(error)) => return Err(error),
    };
    let key = screened.key();
    if args.iter().any(|arg| arg == "--details") {
        record.raw_request = Some(String::from_utf8(raw.clone()).map_err(|_| "request UTF-8")?);
    }
    let operation = record.operation.clone();
    let start = Instant::now();
    let attempts = &mut record.attempts;
    let mut last = None;
    let mut raw_response = String::new();
    for attempt in 0..attempts_limit {
        let remaining = absolute_deadline.saturating_duration_since(Instant::now());
        if remaining < Duration::from_millis(50) {
            break;
        }
        // Every attempt is reserved in the ledger before dispatch with its
        // byte-bound token estimate, so an attempt whose outcome is unknown is
        // still counted; its settlement must match this reservation.
        attempts.push(json!({"attempt_id":attempt_id()?,"attempt":attempt+1,"attempts_limit":attempts_limit,"provider":target.name,"state":"dispatched","request_bytes":raw.len(),"input_token_upper_bound":raw.len(),"billing":"unknown until usage returned; timed-out calls may be billed"}));
        ledger(args, &operation, &record.request_sha256, attempts.last().unwrap(), true)?;
        // A screened handle is the only input to the fixed curl transport.
        let p = match screened.send(remaining) {
            Ok(p) => p,
            Err(e) => {
                settle(attempts, json!({"state":"unknown","transport_error":e,"billing":"unknown; no automatic replay"}));
                settle_ledger(args, &operation, &record.request_sha256, attempts);
                return Ok((
                    json!({"schema":"ultragoal/1","operation":operation,"state":"unknown","attempts":attempts}),
                    2,
                ));
            }
        };
        // Without a proxy, curl exits 6 (resolve) and 7 (connect) end before any
        // request byte leaves this host: the attempt is verified not sent.
        if !p.timeout && !p.cancelled && matches!(p.code, Some(6 | 7)) {
            settle(attempts, json!({"state":"verified_not_sent","transport_exit":p.code,"latency_ms":p.ms,"input_token_upper_bound":0,"billing":"none; connection never established"}));
            settle_ledger(args, &operation, &record.request_sha256, attempts);
            if retry_after("not-sent", attempt, attempts_limit, absolute_deadline)? {
                continue;
            }
            break;
        }
        // A timeout, cancellation or other transport failure may have reached the provider.
        if p.timeout || p.cancelled || p.code != Some(0) || p.truncated {
            settle(attempts, json!({"state":"unknown","transport_exit":p.code,"timeout":p.timeout,"cancelled":p.cancelled,"truncated":p.truncated,"latency_ms":p.ms,"billing":"unknown; the request may have been processed"}));
            settle_ledger(args, &operation, &record.request_sha256, attempts);
            break;
        }
        let bytes = match String::from_utf8(p.out) {
            Ok(b) => b,
            Err(_) => {
                settle(attempts, json!({"state":"unknown","transport_exit":p.code,"latency_ms":p.ms,"reason":"invalid UTF-8 response","billing":"unknown"}));
                settle_ledger(args, &operation, &record.request_sha256, attempts);
                return Ok((
                    json!({"schema":"ultragoal/1","operation":operation,"state":"unknown","attempts":attempts}),
                    2,
                ));
            }
        };
        if bytes.contains(&key) {
            settle(attempts, json!({"state":"unknown","reason":"credential echo suppressed","billing":"unknown"}));
            settle_ledger(args, &operation, &record.request_sha256, attempts);
            return Ok((
                json!({"schema":"ultragoal/1","operation":operation,"state":"unknown","attempts":attempts}),
                2,
            ));
        }
        let (body, status) = bytes.rsplit_once('\n').ok_or("curl status line missing")?;
        let decoded = decode_response(body.as_bytes());
        if raw_credential_echo(body, &key)
            || decoded.as_ref().is_ok_and(|v| contains_credential(v, &key))
        {
            settle(attempts, json!({"state":"unknown","http":status,"response_sha256":hash(body.as_bytes()),"reason":"credential echo suppressed","billing":"unknown"}));
            settle_ledger(args, &operation, &record.request_sha256, attempts);
            return Ok((
                json!({"schema":"ultragoal/1","state":"unknown","reason":"credential echo suppressed"}),
                2,
            ));
        }
        record.raw_response = Some(body.to_string());
        record.response_unvalidated = decoded.as_ref().ok().cloned();
        settle(attempts, json!({"state":"responded","http":status,"transport_exit":p.code,"timeout":p.timeout,"cancelled":p.cancelled,"truncated":p.truncated,"latency_ms":p.ms,"billing":"unknown until usage returned; timed-out calls may be billed"}));
        if let Some(attempt) = attempts.last_mut() {
            attempt["raw_response_json"] = json!(body);
            attempt["response_sha256"] = json!(hash(body.as_bytes()));
            attempt["usage_reported_unvalidated"] = record
                .response_unvalidated
                .as_ref()
                .and_then(|v| v.get("usage"))
                .cloned()
                .unwrap_or(Value::Null);
        }
        settle_ledger(args, &operation, &record.request_sha256, attempts);
        if status == "200" {
            let numeric =
                evaluate_until("NUMERIC\n".to_string() + &row(&[body]), absolute_deadline);
            if !matches!(numeric,Ok(ref v) if *v==vec![vec!["NUMERIC".to_string(),"valid".to_string()]])
            {
                return Ok((
                    json!({"schema":"ultragoal/1","operation":operation,"state":"unknown","reason":"raw numeric domain rejected or validation unavailable","response_sha256":hash(body.as_bytes()),"attempts":attempts}),
                    2,
                ));
            }
            match decoded {
                Ok(v) => {
                    if contains_credential(&v, &key) {
                        return Ok((
                            json!({"schema":"ultragoal/1","state":"unknown","reason":"credential echo suppressed","attempts":attempts}),
                            2,
                        ));
                    }
                    last = Some(v);
                    raw_response = body.to_string();
                }
                Err(e) => {
                    return Ok((
                        json!({"schema":"ultragoal/1","operation":operation,"state":"unknown","reason":"provider JSON rejected","detail":e,"attempts":attempts}),
                        2,
                    ));
                }
            }
            break;
        }
        if !retry_after(status, attempt, attempts_limit, absolute_deadline)? {
            break;
        }
    }
    let Some(response) = last else {
        return Ok((
            json!({"schema":"ultragoal/1","operation":operation,"state":"unknown","reason":"provider unavailable","attempts":attempts,"latency_ms":start.elapsed().as_millis()}),
            2,
        ));
    };
    let frame = match response_frame_planned(&request, &response, &layouts) {
        Ok(f) => f,
        Err(e) => {
            return Ok((
                json!({"schema":"ultragoal/1","operation":operation,"state":"unknown","reason":"provider schema rejected","detail":e,"attempts":attempts,"response_sha256":hash(raw_response.as_bytes())}),
                2,
            ));
        }
    };
    let validation = match evaluate_until(frame, absolute_deadline) {
        Ok(v) => v,
        Err(e) => {
            return Ok((
                json!({"schema":"ultragoal/1","operation":operation,"state":"unknown","reason":"local response validation unavailable","detail":e,"attempts":attempts,"response":response}),
                2,
            ));
        }
    };
    let invalid = validation
        .iter()
        .any(|r| r.len() != 3 || r[2] == "malformed");
    if invalid {
        let validated_answers = individually_validated(&response, &validation);
        return Ok((
            json!({"schema":"ultragoal/1","operation":operation,"state":"unknown","validation":validation,"validated_answers":validated_answers,"answer_scope":"individually validated answers only; other answers remain rejected","response_sha256":hash(raw_response.as_bytes()),"attempts":attempts}),
            2,
        ));
    }
    Ok((
        json!({"schema":"ultragoal/1","operation":operation,"request_sha256":hash(&raw),"state":"needs-review","validation":validation,"response":response,"raw_response_json":raw_response,"attempts":attempts,"latency_ms":start.elapsed().as_millis(),"assurance":"uncalibrated advisory; cannot discharge native/formal/authority obligations","disclosure":format!("explicit {}",option(args,"--disclosure")?.unwrap_or_default()),"calibration":"none; original decision-time state and all attempts retained by caller"}),
        2,
    ))
}

#[cfg(test)]
mod tests {
    #[test]
    fn synthetic_endpoint_port_is_bounded_and_loopback_only() {
        for raw in ["", "0", "1023", "65536", "12x3", "-1", " 1024"] {
            assert!(loopback_target(raw).is_err(), "{raw}");
        }
        let target = loopback_target("1024").unwrap();
        assert_eq!(target.url, "http://127.0.0.1:1024/v1/systemone");
        assert_eq!(target.protocol, "=http");
    }
    #[test]
    fn advisory_store_refuses_unvalidated_input_and_symlink_substitution() {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(format!("target/provider-store-test-{}", std::process::id()));
        if dir.exists() { std::fs::remove_dir_all(&dir).unwrap(); }
        let raw = b"synthetic request";
        let invalid = json!({"raw_response_json":"{}","validation":[]});
        store_assessment(&dir, raw, &invalid).unwrap();
        assert!(!dir.exists());
        std::fs::create_dir(&dir).unwrap();
        let tmp = dir.join(format!(".{}.{}.tmp", hash(raw), std::process::id()));
        let outside = dir.parent().unwrap().join(format!("provider-outside-{}", std::process::id()));
        std::fs::write(&outside, b"original").unwrap();
        std::os::unix::fs::symlink(&outside, &tmp).unwrap();
        let advisory = json!({"raw_response_json":"{}","validation":[["ANSWER","x","advisory-supported"]],"attempts":[]});
        assert!(store_assessment(&dir, raw, &advisory).is_err());
        assert_eq!(std::fs::read(&outside).unwrap(), b"original");
        std::fs::remove_file(&outside).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
    }
    #[test]
    fn credential_refusal_code_does_not_depend_on_diagnostic_wording() {
        for error in [CredentialError::Lookup, CredentialError::Invalid,
                      CredentialError::Encoding, CredentialError::MissingService] {
            let (report, code) = credential_refusal("test", error);
            assert_eq!((code, report["state"].as_str(), report["error_code"].as_str()),
                       (2, Some("unknown"), Some("credential_refused")));
            assert_eq!(report["attempts"], json!([]));
        }
    }
    #[test]
    fn credential_boundary_rejects_invalid_representations_without_secret_text() {
        for raw in ["", "  ", "has space", "\n", "é", &"x".repeat(1025)] {
            let error = validate_credential(raw.into()).err().unwrap();
            assert_eq!(error.to_string(), "invalid credential representation");
        }
        assert_eq!(validate_credential(" good-token\n".into()).unwrap().secret, "good-token");
        assert_eq!(CredentialError::Lookup.to_string(), "credential lookup unavailable");
    }
    #[test]
    fn ledger_settlement_must_match_exactly_one_reservation() {
        let path = format!("{}/target/ledger-test-{}.jsonl", env!("CARGO_MANIFEST_DIR"), std::process::id());
        let args = vec!["--usage-ledger".to_string(), path.clone()];
        let reserve = json!({"attempt_id":"ug-test-1","attempt":1,"state":"dispatched"});
        let settle = json!({"attempt_id":"ug-test-1","attempt":1,"state":"responded"});
        ledger(&args, "op", "req-a", &reserve, true).unwrap();
        assert!(ledger(&args, "op", "req-a", &reserve, true).is_err(), "duplicate reservation");
        assert!(ledger(&args, "op", "req-b", &settle, false).is_err(), "settlement for another request");
        ledger(&args, "op", "req-a", &settle, false).unwrap();
        assert!(ledger(&args, "op", "req-a", &settle, false).is_err(), "second settlement");
        assert!(ledger(&args, "op", "req-a", &json!({"attempt_id":"ug-test-2","state":"unknown"}), false).is_err(), "unreserved settlement");
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn usage_counts_every_attempt_including_unknown_outcomes() {
        let mut attempts = vec![json!({"attempt":1,"state":"dispatched","input_token_upper_bound":900})];
        settle(&mut attempts, json!({"state":"unknown","transport_error":"timeout"}));
        attempts.push(json!({"attempt":2,"state":"responded","input_token_upper_bound":900,"usage_reported_unvalidated":{"input_tokens":320,"output_tokens":40}}));
        let u = usage_summary(&attempts);
        assert_eq!(attempts[0]["state"], "unknown");
        assert_eq!(attempts[0]["input_token_upper_bound"], 900);
        assert_eq!(u["attempts"], 2);
        assert_eq!(u["reported_input_tokens"], 320);
        assert_eq!(u["reported_output_tokens"], 40);
        assert_eq!(u["attempts_without_usage"], 1);
        assert_eq!(u["unknown_input_token_upper_bound"], 900);
    }

    use super::*;
    #[test]
    fn one_invalid_independent_answer_does_not_erase_valid_answers() {
        let response = json!({"answers":{"good":{"type":"score","score":3},"bad":{"type":"score","score":99}}});
        let rows = vec![
            vec!["ANSWER".into(), "good".into(), "validated-advisory".into()],
            vec!["ANSWER".into(), "bad".into(), "malformed".into()],
        ];
        let retained = individually_validated(&response, &rows);
        assert_eq!(retained.as_object().unwrap().len(), 1);
        assert_eq!(retained["good"]["score"], 3);
        assert!(retained.get("bad").is_none());
    }
    fn response_frame_no_choice(request: &Value,response: &Value) -> Result<String,String> {
        response_frame_planned(request,response,&ChoiceLayouts::new())
    }
    #[test]
    fn planned_choice_layout_is_mechanical_and_bound_to_fields() {
        let request=json!({"model":"jev-1.13.0","questions":{"q":{"type":"choice","criteria":{"first":"A","second":"B","third":"C"}}}});
        let rows=vec![vec!["CHOICE_LAYOUT","q","wire-tag","first","second","third"].into_iter().map(str::to_string).collect()];
        let layouts=decode_layouts(&request,rows).unwrap();
        let response=json!({"model":"jev-1.13.0","answers":{"q":{"type":"choice","choice":"first","confidence":1,"probabilities":{"first":1,"second":0,"third":0}}},"usage":null});
        assert!(response_frame_planned(&request,&response,&layouts).unwrap().starts_with("JEV\nwire-tag\tq"));
        let wrong=vec![vec!["CHOICE_LAYOUT","q","wire-tag","first","first","third"].into_iter().map(str::to_string).collect()];
        assert!(decode_layouts(&request,wrong).is_err());
        let wrong=vec![vec!["CHOICE_LAYOUT","other","wire-tag","first","second","third"].into_iter().map(str::to_string).collect()];
        assert!(decode_layouts(&request,wrong).is_err());
        assert!(decode_layouts(&request,Vec::new()).is_err());
    }

    #[test]
    fn internal_packet_is_one_obligation_with_unclassified_source_roles() {
        let r = vec![
            "REQUEST",
            "o1",
            "Preserve permission errors",
            "user",
            "consistency",
            "src/lib.rs\tdigest\tfn load() {}\n",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "exact Bend rubric",
        ]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
        let probes = ["conflict", "support"]
            .iter()
            .map(|name| {
                vec![
                    "PROBE",
                    name,
                    "3",
                    "Bend-owned fixture instructions",
                    "a",
                    "A",
                    "b",
                    "B",
                    "c",
                    "C",
                ]
                .into_iter()
                .map(str::to_string)
                .collect()
            })
            .collect::<Vec<Vec<String>>>();
        let (packet, binding) = prepare_obligation(&r, &probes).unwrap();
        assert_eq!(packet["state"]["requirement"], "Preserve permission errors");
        assert_eq!(
            packet["state"]["context"]["evidence"][0]["role"],
            "unclassified-source"
        );
        assert_eq!(packet["questions"].as_object().unwrap().len(), 2);
        assert_eq!(binding["rubric_version"], "3");
        let mut mixed = probes.clone();
        mixed[1][2] = "4".to_string();
        assert!(prepare_obligation(&r, &mixed).is_err());
        assert_eq!(binding["source_bindings"][0]["input_sha256"], "digest");
        assert!(prepare_obligation(&r[..7], &probes).is_err());
        assert!(prepare_obligation(&r, &probes[..1]).is_err());
    }
    #[test]
    fn valid_answer_cannot_rescue_an_invalid_response_envelope() {
        let request = json!({"model":"jev-1.13.0","questions":{"q":{"type":"noul"}}});
        let valid = json!({"model":"jev-1.13.0","answers":{"q":{"type":"noul","noul":0.5}},"usage":null});
        assert!(response_frame_no_choice(&request, &valid).is_ok());
        let mut cases = Vec::new();
        let mut wrong = valid.clone(); wrong["model"] = json!("other-model"); cases.push(wrong);
        let mut wrong = valid.clone(); wrong["answers"]["q"]["type"] = json!("choice"); cases.push(wrong);
        let mut wrong = valid.clone(); wrong.as_object_mut().unwrap().remove("usage"); cases.push(wrong);
        let mut wrong = valid.clone(); wrong["answers"]["extra"] = valid["answers"]["q"].clone(); cases.push(wrong);
        let mut wrong = valid.clone(); wrong["answers"].as_object_mut().unwrap().remove("q"); cases.push(wrong);
        let mut wrong = valid.clone(); wrong["authoritative"] = json!(true); cases.push(wrong);
        let mut wrong = valid.clone(); wrong["answers"]["q"]["noul"] = json!("0.5"); cases.push(wrong);
        for wrong in cases { assert!(response_frame_no_choice(&request, &wrong).is_err(), "{wrong}"); }
        assert!(decode(br#"{"model":"jev-1.13.0","answers":{"q":{"type":"noul","noul":0.5}}"#).is_err());
    }

    #[test]
    fn duplicate_rejected() {
        assert!(decode(br#"{"a":1,"a":2}"#).is_err());
        assert!(decode(br#"{"a":{"b":1,"b":2}}"#).is_err());
    }
    #[test]
    fn nonfinite_rejected() {
        assert!(decode(br#"{"a":NaN}"#).is_err());
        assert!(decode(br#"{"a":1e999}"#).is_err());
    }
    #[test]
    fn incomplete_response_rejected() {
        assert!(
            response_frame_no_choice(
                &json!({"model":"x","questions":{}}),
                &json!({"model":"x","answers":{}})
            )
            .is_err()
        );
    }
    #[test]
    fn usage_and_question_identity_rejected() {
        let request = json!({"model":"jev-1.13.0","questions":{"q":{"type":"noul"}}});
        let mut response = json!({"model":"jev-1.13.0","answers":{"q":{"type":"noul","noul":0.5}},"usage":{"input_tokens":1,"output_tokens":1}});
        assert!(response_frame_no_choice(&request, &response).is_ok());
        response["usage"]["input_tokens"] = json!(-1);
        assert!(response_frame_no_choice(&request, &response).is_err());
        response["usage"] = Value::Null;
        response["answers"]["other"] = json!({"type":"noul","noul":0.1});
        assert!(response_frame_no_choice(&request, &response).is_err());
    }
    #[test]
    fn rejected_and_error_receipts_preserve_identity_raw_and_usage() {
        for outcome in [
            Ok((
                json!({"schema":"ultragoal/1","state":"unknown","validation":[["ANSWER","q0","malformed"]]}),
                2,
            )),
            Err("local validator unavailable".to_string()),
        ] {
            let raw = r#"{"model":"jev-1.13.0","answers":{"q0":{"type":"noul","noul":1.1}},"usage":{"input_tokens":7,"output_tokens":3}}"#;
            let record = ReceiptRecord {
                payload: None,
                operation: "operation".into(),
                request_sha256: "request-digest".into(),
                attempts: vec![json!({"attempt":1,"http":"200","raw_response_json":raw})],
                raw_response: Some(raw.into()),
                raw_request: None,
                response_unvalidated: Some(decode(raw.as_bytes()).unwrap()),
            };
            let (v, code) = finish_receipt(record, outcome, 4).unwrap();
            assert_eq!(code, 2);
            assert_eq!(v["request_sha256"], "request-digest");
            assert_eq!(v["raw_response_json"], raw);
            assert_eq!(v["usage_reported_unvalidated"]["input_tokens"], 7);
            assert!(v.get("response").is_none());
            assert_eq!(v["assurance"], "uncalibrated advisory");
            assert_eq!(v["calibration"], "none");
            assert_eq!(v["attempts"].as_array().unwrap().len(), 1);
        }
    }
    #[test]
    fn numeric_roundtrip_and_escaped_credential_guards() {
        for value in ["0.9299999999999999", "0.41000000000000003", "1e-50"] {
            let parsed = decode(value.as_bytes()).unwrap().as_f64().unwrap();
            assert_eq!(parsed, value.parse::<f64>().unwrap());
        }
        assert!(raw_credential_echo(
            r#"{"x":"key\u002dsecret","x":"safe"}"#,
            "key-secret"
        ));
        assert!(!raw_credential_echo(r#"{"x":"unrelated"}"#, "key-secret"));
    }
}

/// One item for a windowed Jev judgment: its question id, state entry and question.
pub struct WindowItem {
    pub id: String,
    pub state: Value,
    pub question: Value,
}

pub struct WindowOutcome {
    /// Per item id: the accepted answer and its request, or the uninspected reason.
    pub judgments: BTreeMap<String, Result<(Value, usize), String>>,
    pub plan: Vec<Vec<String>>,
    pub assessments: Vec<Value>,
    pub suppressed: bool,
    pub head_request: usize,
}

/// Runs `f` over `items` with at most `concurrency` provider requests in flight;
/// the next starts when one ends.
fn in_flight<T: Sync, R: Send>(items: &[T], concurrency: usize, f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let next = std::sync::atomic::AtomicUsize::new(0);
    let done = std::sync::Mutex::new(Vec::with_capacity(items.len()));
    std::thread::scope(|scope| {
        for _ in 0..items.len().min(concurrency) {
            scope.spawn(|| loop {
                let i = next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let Some(item) = items.get(i) else { break };
                let result = f(item);
                done.lock().unwrap().push((i, result));
            });
        }
    });
    let mut done = done.into_inner().unwrap();
    done.sort_by_key(|(i, _)| *i);
    done.into_iter().map(|(_, r)| r).collect()
}

/// What an answer from a window request was judged beside: the head alone, or
/// the anchor plus one slice. Relative judgments hold only within that scope.
pub fn comparison_scope(head_request: usize, request: usize) -> &'static str {
    if request == head_request { "head" } else { "anchor_and_slice" }
}

fn entry_bytes(id: &str, value: &Value) -> Result<usize, String> {
    // `"id":value,` as it appears inside a serialized JSON object.
    Ok(serde_json::to_vec(&json!(id)).map_err(|e| e.to_string())?.len()
        + 2
        + serde_json::to_vec(value).map_err(|e| e.to_string())?.len())
}

/// Ranked head plus slices (Bend `JEV_WINDOW`): every item is judged exactly once,
/// beside the top-ranked anchor; results are accepted only through Bend's merge,
/// which re-derives the plan. `items` must be in rank order.
pub fn windowed(
    args: &[String],
    fixed_state: &Value,
    items: &[WindowItem],
    budget: &Budget,
) -> Result<WindowOutcome, String> {
    let fixed_total = serde_json::to_vec(&json!({"model":"jev-1.13.0","state":fixed_state,"questions":{}}))
        .map_err(|e| e.to_string())?
        .len();
    let fixed_state_bytes = serde_json::to_vec(fixed_state).map_err(|e| e.to_string())?.len();
    let mut sizes = format!("FIXED\t{fixed_total}\t{fixed_state_bytes}\n");
    for item in items {
        sizes += &row(&[
            "ITEM",
            &item.id,
            &entry_bytes(&item.id, &item.state)?.to_string(),
            &entry_bytes(&item.id, &item.question)?.to_string(),
        ]);
    }
    let plan = evaluate(format!("JEV_WINDOW\n{sizes}"))?;
    if plan.first().is_none_or(|r| r.first().is_none_or(|s| s != "WINDOW")) {
        return Err("Bend window plan missing".into());
    }
    let by_id: BTreeMap<&str, &WindowItem> = items.iter().map(|i| (i.id.as_str(), i)).collect();
    let mut anchor: Vec<&WindowItem> = Vec::new();
    let mut head_request = 0;
    let mut requests: Vec<(usize, bool, Vec<&WindowItem>)> = Vec::new();
    for r in plan.iter().filter(|r| r[0] == "REQUEST") {
        let index: usize = r[1].parse().map_err(|_| "window request index")?;
        let judged = r[3..]
            .iter()
            .map(|id| by_id.get(id.as_str()).copied().ok_or("window item identity"))
            .collect::<Result<Vec<_>, _>>()?;
        let head = r[2] == "head";
        if head {
            anchor = judged.clone();
            head_request = index;
        }
        requests.push((index, head, judged));
    }
    // Each slice carries the anchor as context without its questions; admission in
    // `assess_until` checks both provider limits on these exact bytes.
    let mut bodies: Vec<(usize, Vec<u8>)> = Vec::new();
    for (index, head, judged) in &requests {
        let mut state = fixed_state.clone();
        let candidates = state["candidates"].as_object_mut().ok_or("window state candidates")?;
        let mut questions = serde_json::Map::new();
        if !head {
            for a in &anchor {
                candidates.insert(a.id.clone(), a.state.clone());
            }
        }
        for j in judged {
            candidates.insert(j.id.clone(), j.state.clone());
            questions.insert(j.id.clone(), j.question.clone());
        }
        let body = serde_json::to_vec(&json!({"model":"jev-1.13.0","state":state,"questions":questions}))
            .map_err(|e| e.to_string())?;
        bodies.push((*index, body));
    }
    ledger_plan(args, bodies.len())?;
    let results = in_flight(&bodies, budget.concurrency, |(index, body)| {
        (*index, assess_until(args, body.clone(), Instant::now() + budget.per_request).map(|v| v.0))
    });
    let mut merge = format!("JEV_WINDOW_MERGE\n{sizes}");
    let mut assessments = Vec::new();
    let mut suppressed = false;
    let mut answers: BTreeMap<(usize, String), Value> = BTreeMap::new();
    for (index, result) in &results {
        let Ok(assessment) = result else {
            assessments.push(json!({"request":index,"state":"unavailable","reason":result.as_ref().err()}));
            continue;
        };
        suppressed |= assessment["detail"] == "packet contains the active credential; nothing sent";
        for v in assessment["validation"].as_array().into_iter().flatten() {
            let (Some(id), Some(label)) = (v[1].as_str(), v[2].as_str()) else { continue };
            if !label.starts_with("validated") && !label.starts_with("advisory-") {
                continue;
            }
            merge += &row(&["RESULT", &index.to_string(), id, label]);
            let answer = assessment["validated_answers"]
                .get(id)
                .cloned()
                .unwrap_or_else(|| assessment["response"]["answers"][id].clone());
            answers.insert((*index, id.to_string()), json!({"label":label,"answer":answer}));
        }
        let mut a = assessment.clone();
        a["window_request"] = json!(index);
        a["comparison_scope"] = json!(comparison_scope(head_request, *index));
        a["judged_ids"] = json!(requests.iter().find(|(i, _, _)| i == index).map(|(_, _, j)| j.iter().map(|w| w.id.as_str()).collect::<Vec<_>>()));
        assessments.push(a);
    }
    let merged = evaluate(merge)?;
    let mut judgments = BTreeMap::new();
    for r in merged {
        match r.first().map(String::as_str) {
            Some("JUDGMENT") if r.len() == 4 => {
                let index: usize = r[3].parse().map_err(|_| "merge request index")?;
                let answer = answers.remove(&(index, r[1].clone())).ok_or("merge answer identity")?;
                judgments.insert(r[1].clone(), Ok((answer, index)));
            }
            Some("UNINSPECTED") if r.len() >= 3 => {
                judgments.insert(r[1].clone(), Err(r[2..].join(" ")));
            }
            _ => return Err("Bend window merge shape".into()),
        }
    }
    if judgments.len() != items.len() {
        return Err("Bend window merge omitted an item".into());
    }
    Ok(WindowOutcome { judgments, plan, assessments, suppressed, head_request })
}

/// Bend's retry decision for the outcome of one attempt; a retry waits a short jitter.
fn retry_after(status: &str, attempt: u32, attempts_limit: u32, deadline: Instant) -> Result<bool, String> {
    let retry = evaluate(
        "RETRY\n".to_string()
            + &row(&[
                status,
                &attempts_limit.saturating_sub(attempt + 1).to_string(),
                &deadline.saturating_duration_since(Instant::now()).as_millis().to_string(),
                "running",
            ]),
    )?;
    if retry != vec![vec!["RETRY".to_string()]] {
        return Ok(false);
    }
    let jitter = 10 + (SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().subsec_nanos() % 20) as u64;
    std::thread::sleep(Duration::from_millis(jitter).min(deadline.saturating_duration_since(Instant::now())));
    Ok(true)
}

fn settle(attempts: &mut [Value], fields: Value) {
    let last = attempts.last_mut().expect("settled attempt was recorded before dispatch");
    for (k, v) in fields.as_object().expect("settled fields are an object") {
        last[k] = v.clone();
    }
}

/// Measurement, not a limit: reported input/output tokens where the provider
/// returned usage, and the byte bound for every attempt whose usage is unknown.
fn usage_summary(attempts: &[Value]) -> Value {
    let mut reported_input = 0u64;
    let mut reported_output = 0u64;
    let mut unknown_attempts = 0u64;
    let mut unknown_input_upper_bound = 0u64;
    let mut not_sent = 0u64;
    for a in attempts {
        if a["state"] == "verified_not_sent" {
            not_sent += 1;
            continue;
        }
        let usage = &a["usage_reported_unvalidated"];
        match (usage["input_tokens"].as_u64(), usage["output_tokens"].as_u64()) {
            (Some(i), Some(o)) => {
                reported_input += i;
                reported_output += o;
            }
            _ => {
                unknown_attempts += 1;
                unknown_input_upper_bound += a["input_token_upper_bound"].as_u64().unwrap_or(0);
            }
        }
    }
    json!({"attempts":attempts.len(),"reported_input_tokens":reported_input,"reported_output_tokens":reported_output,"verified_not_sent":not_sent,"attempts_without_usage":unknown_attempts,"unknown_input_token_upper_bound":unknown_input_upper_bound})
}

fn attempt_id() -> Result<String, String> {
    let mut bytes = [0u8; 16];
    std::fs::File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut bytes)).map_err(|e| format!("attempt id: {e}"))?;
    Ok(format!("ug-{}", bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()))
}

/// Reservations made by this process, by attempt id: the request digest and whether
/// the attempt has settled. Attempt ids are fresh 128-bit random values, and only the
/// process that reserves an attempt settles it, so pairing needs no ledger reads.
static RESERVATIONS: std::sync::Mutex<BTreeMap<String, (String, bool)>> = std::sync::Mutex::new(BTreeMap::new());

/// Optional append-only JSONL usage ledger (`--usage-ledger FILE`): a reservation
/// line before each attempt is dispatched and a settlement line when its outcome
/// is known, paired by a unique attempt id. Raw response text stays in the
/// receipt; the ledger holds identities, sizes, status and usage.
fn ledger(args: &[String], operation: &str, request_sha256: &str, attempt: &Value, reserve: bool) -> Result<(), String> {
    let Some(path) = option(args, "--usage-ledger")? else {
        return Ok(());
    };
    let id = attempt["attempt_id"].as_str().expect("attempt carries its id");
    let mut entry = attempt.clone();
    entry.as_object_mut().expect("attempt record is an object").remove("raw_response_json");
    entry["entry"] = json!(if reserve { "reservation" } else { "settlement" });
    entry["operation"] = json!(operation);
    entry["request_sha256"] = json!(request_sha256);
    entry["recorded_unix_ms"] = json!(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e| e.to_string())?.as_millis());
    {
        let mut reservations = RESERVATIONS.lock().unwrap();
        let valid = if reserve {
            reservations.insert(id.to_string(), (request_sha256.to_string(), false)).is_none()
        } else {
            match reservations.get_mut(id) {
                Some((request, settled)) if request == request_sha256 && !*settled => {
                    *settled = true;
                    true
                }
                _ => false,
            }
        };
        if !valid {
            return Err(format!("usage ledger {} does not match attempt {id}", entry["entry"].as_str().unwrap()));
        }
    }
    ledger_append(&mut ledger_file(&path)?, &entry)
}

/// A settlement that cannot be written is recorded on the attempt instead of
/// discarding a response that may already be paid for.
fn settle_ledger(args: &[String], operation: &str, request_sha256: &str, attempts: &mut [Value]) {
    if let Err(error) = ledger(args, operation, request_sha256, attempts.last().unwrap(), false) {
        attempts.last_mut().unwrap()["ledger_error"] = json!(error);
    }
}

/// Opens the ledger and takes its lock, so lines from concurrent writers (window
/// slices, parallel runs) never interleave.
fn ledger_file(path: &str) -> Result<std::fs::File, String> {
    let f = OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
        .map_err(|e| format!("usage ledger: {e}"))?;
    if unsafe { libc::flock(f.as_raw_fd(), libc::LOCK_EX) } != 0 {
        return Err(format!("usage ledger lock: {}", std::io::Error::last_os_error()));
    }
    Ok(f)
}

fn ledger_append(f: &mut std::fs::File, entry: &Value) -> Result<(), String> {
    let mut line = serde_json::to_vec(entry).map_err(|e| e.to_string())?;
    line.push(b'\n');
    f.write_all(&line).map_err(|e| format!("usage ledger: {e}"))?;
    f.sync_data().map_err(|e| format!("usage ledger: {e}"))
}

/// A window's plan, written before its first dispatch: a watcher can compare the
/// reservations that follow with `requests` times each reservation's attempts limit.
fn ledger_plan(args: &[String], requests: usize) -> Result<(), String> {
    let Some(path) = option(args, "--usage-ledger")? else {
        return Ok(());
    };
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e| e.to_string())?.as_millis();
    ledger_append(&mut ledger_file(&path)?, &json!({"entry":"plan","operation":"window","requests":requests,"recorded_unix_ms":now}))
}
