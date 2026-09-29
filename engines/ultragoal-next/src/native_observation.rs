//! Current-pending-call custody for the fixed captured-input verifier adapters.
//! It is not a protected host attestation channel or an arbitrary command runner.
use super::*;
use std::ffi::OsStr;

pub(crate) struct Pending {
    command: Command,
    input: Vec<u8>,
    input_path: PathBuf,
    tool_path: PathBuf,
    extra_tool: Option<(PathBuf, String)>,
    request: Vec<String>,
    invocation: Value,
}

fn os_text(value: &OsStr) -> Result<String, String> {
    value.to_str().map(str::to_owned).ok_or("native invocation is not UTF-8".into())
}

impl Pending {
    pub(crate) fn issue(kind: &str, mut command: Command, input_path: &Path, input: Vec<u8>, extra_tool: Option<&Path>) -> Result<Self, String> {
        command.env_clear().env("PATH", "/usr/bin:/bin").current_dir("/");
        let tool_path = fs::canonicalize(command.get_program()).map_err(|e| e.to_string())?;
        let tool_digest = hash(&read(&tool_path, 256 * 1024 * 1024)?);
        let extra_tool = extra_tool.map(|p| {
            let path = fs::canonicalize(p).map_err(|e| e.to_string())?;
            Ok::<_,String>((path.clone(), hash(&read(&path, MAX)?)))
        }).transpose()?;
        let directory = fs::metadata("/").map_err(|e| e.to_string())?;
        let argv = command.get_args().map(os_text).collect::<Result<Vec<_>,_>>()?;
        let invocation = json!({"argv":argv,"environment":{"PATH":"/usr/bin:/bin"},"environment_inherited":false,"working_directory":{"path":"/","device":directory.dev(),"inode":directory.ino()},"additional_tool":extra_tool.as_ref().map(|(path,digest)|json!({"path":path,"sha256":digest})),"expected_outputs":["terminal status","bounded diagnostics"],"source_execution":false});
        let invocation_digest = hash(&serde_json::to_vec(&invocation).map_err(|e|e.to_string())?);
        let operation = op_id()?;
        let request_id = op_id()?;
        let rows = evaluate("VERIFIER_PLAN\n".to_string()+&row(&[&operation,&request_id,kind,&hash(&input),&tool_digest,&invocation_digest,&input.len().to_string()]))?;
        if rows.len()!=1 || rows[0].len()!=12 || rows[0][0]!="VERIFICATION_REQUEST" {
            return Err("Bend refused the closed-input verifier request".into());
        }
        let request = rows.into_iter().next().unwrap();
        if request[2]!=operation || request[3]!=request_id || request[4]!=kind || request[5]!=hash(&input) || request[6]!=tool_digest || request[7]!=invocation_digest || request[8]!=input.len().to_string() {
            return Err("native request does not bind the pending invocation".into());
        }
        Ok(Self { command, input, input_path:if input_path.is_absolute(){input_path.to_owned()}else{env::current_dir().map_err(|e|e.to_string())?.join(input_path)}, tool_path, extra_tool, request, invocation })
    }

    /// Consumes the only pending handle. Imported JSON cannot create this value
    /// or replace its command/input between planning and observation.
    pub(crate) fn execute(self) -> Result<(Value, i32), String> {
        let budget = self.request[10].parse::<u64>().map_err(|_|"native deadline")?;
        let request_json = json!({"schema":"ultragoal-verification-request/1","operation":self.request[2],"request_id":self.request[3],"obligation_ids":[],"origin":"explicit standalone verifier request; no adopted project obligation inferred","kind":self.request[4],"input":{"path":self.input_path,"sha256":self.request[5],"bytes":self.input.len(),"binding":self.request[11]},"tool":{"requested_path":self.tool_path,"pre_launch_sha256":self.request[6]},"invocation_sha256":self.request[7],"invocation":self.invocation,"required_surface":self.request[9],"deadline_ms":budget,"authorization":"native host/user invocation; the plan does not grant permission"});
        let process = match run(self.command,self.input,Duration::from_millis(budget)) {
            Ok(p)=>p,
            Err(error)=>return Ok((json!({"schema":"ultragoal/1","state":"unknown","verification_request":request_json,"native_observation":{"schema":"ultragoal-native-observation/1","origin":"current-local-attempt","operation":self.request[2],"request_id":self.request[3],"host_execution_id":null,"local_child_pid":null,"terminal":{"state":"unavailable","exit":null,"signal":null,"cancelled":is_interrupted(),"timeout":null},"reason":error},"host_attestation":"unavailable"}),2)),
        };
        let current_input = read_adaptive(&self.input_path).map(|b|hash(&b));
        let input_state = match &current_input { Ok(value) if value==&self.request[5]=>"current",Ok(_)=>"stale",Err(_)=>"unavailable" };
        let tool_after = read(&self.tool_path,256*1024*1024).map(|b|hash(&b)).unwrap_or_default();
        let extra_current = self.extra_tool.as_ref().is_none_or(|(path,digest)|read(path,MAX).is_ok_and(|bytes|hash(&bytes)==*digest));
        let complete = !process.cancelled && !process.timeout && !process.truncated && process.signal.is_none() && process.input_complete;
        let observed_tool = if extra_current { tool_after.as_str() } else { "additional-tool-changed" };
        let request_wire = row(&self.request.iter().map(String::as_str).collect::<Vec<_>>());
        let observation_wire = row(&["LOCAL_OBSERVATION",&self.request[2],&self.request[3],&self.request[4],&self.request[5],observed_tool,&self.request[7],&process.code.map(|n|n.to_string()).unwrap_or_else(||"unavailable".into()),if complete{"complete"}else{"incomplete"},if process.ms<=u128::from(budget){"timely"}else{"late"},input_state,"current-pending-call"]);
        let disposition = evaluate("VERIFIER_OBSERVE\n".to_string()+&row(&[&request_wire,&observation_wire]));
        let (state,code,admission_error) = match disposition {
            Ok(rows) if rows.len()==1 && rows[0].len()==3 && rows[0][0]=="STATE" => (rows[0][1].clone(),rows[0][2].parse().map_err(|_|"native disposition")?,None),
            Ok(_)=>("unknown".into(),2,Some("malformed Bend observation disposition".to_string())),
            Err(error)=>("unknown".into(),2,Some(error)),
        };
        let report = json!({"schema":"ultragoal/1","state":state,"operation":self.request[2],"surface":self.request[9],"input_sha256":self.request[5],"input_binding":if process.input_complete{"complete captured stdin written to the fixed parser; current source bytes compared separately"}else{"incomplete captured stdin transfer"},"verification_request":request_json,"native_observation":{"schema":"ultragoal-native-observation/1","origin":"current-pending-call","request_id":self.request[3],"operation":self.request[2],"host_execution_id":null,"local_child_pid":process.child_pid,"input_snapshot_sha256":self.request[5],"stdin_complete":process.input_complete,"started_unix_ms":process.started_unix_ms,"ended_unix_ms":process.ended_unix_ms,"monotonic_elapsed_ms":process.ms,"terminal":{"exit":process.code,"signal":process.signal,"cancelled":process.cancelled,"timeout":process.timeout},"stdout":{"complete":process.stdout_complete,"captured_sha256":hash(&process.out),"digest_scope":if process.stdout_complete{"full observed stream"}else{"captured prefix only"},"text":String::from_utf8_lossy(&process.out)},"stderr":{"complete":process.stderr_complete,"captured_sha256":hash(&process.err),"digest_scope":if process.stderr_complete{"full observed stream"}else{"captured prefix only"},"text":String::from_utf8_lossy(&process.err)},"tool_after_sha256":tool_after,"additional_tool_current":extra_current,"current_input":{"state":input_state,"sha256":current_input.ok()},"artifacts":[],"artifact_observation":"not collected; required surface is terminal status and diagnostics"},"terminal":{"exit":process.code,"signal":process.signal,"cancelled":process.cancelled,"timeout":process.timeout,"truncated":process.truncated},"stderr":String::from_utf8_lossy(&process.err),"observation":String::from_utf8_lossy(&process.out),"latency_ms":process.ms,"admission_error":admission_error,"host_attestation":"unavailable; local pending-call provenance is not protected host authentication","trust_assumptions":["current compiled bridge and local OS faithfully observe this child","fixed tool and runtime libraries remain stable; pre/post hashes cannot rule out executable replacement and restoration","captured-input syntax surface only; no original-outcome, project test, permission or adoption claim"]});
        Ok((report,code))
    }
}

pub(crate) fn explain_import(report: Value) -> Value {
    json!({"schema":"ultragoal/1","state":"reported","admitted_as_current":false,"reported_state":report["state"],"verification_request":report["verification_request"],"native_observation":report["native_observation"],"authority":"reported import; a stored report cannot create a current pending call or protected host attestation"})
}
