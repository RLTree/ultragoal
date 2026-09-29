//! Read-only fit materialization. The native host owns any patch effect/adoption.
use super::*;
use fs_adapter::{Limits, Problem, Root};

pub(crate) fn propose(args: &[String], root_path: &Path) -> Result<(Value,i32),String> {
    let plan=evaluate("FIT_PATHS\n".into())?;
    let mut paths=Vec::new();let mut target=None;let mut inputs=Vec::new();
    for r in plan {
        if r.len()!=2 || !matches!(r[0].as_str(),"FIT_TARGET"|"FIT_INPUT") {return Err("fit discovery plan envelope".into());}
        relative::components(r[1].as_bytes()).map_err(|_|"fit discovery path")?;
        if r[0]=="FIT_TARGET" {if target.replace(r[1].clone()).is_some(){return Err("duplicate fit target".into());}}
        else {inputs.push(r[1].clone());}
        paths.push(r[1].as_bytes().to_vec());
    }
    let target=target.ok_or("fit target missing")?;
    let root=Root::open(root_path).map_err(|e|e.to_string())?;
    let identity=root.identity().map_err(|e|e.to_string())?;
    let headroom=resources::current().ok_or("fit capture headroom unavailable; retry after host observation recovers")?;
    let per_file=usize::try_from(headroom.work_bytes()/32).unwrap_or(usize::MAX);
    let total=usize::try_from(headroom.work_bytes()/16).unwrap_or(usize::MAX);
    if per_file==0 {return Err("fit capture paused by current memory pressure".into());}
    let limits=Limits{max_entries:32,max_depth:1,max_path_bytes:4096,max_file_bytes:per_file,max_total_bytes:total};
    let captures=root.capture_selected(&paths,&limits);
    let mut texts=BTreeMap::new();let mut observations=Vec::new();let mut unavailable=Vec::new();let mut target_state="unavailable";
    for c in captures {
        let path=String::from_utf8(c.path).map_err(|_|"fit path UTF-8")?;
        let state=if c.bytes.is_some(){"captured"}else if c.problem==Some(Problem::Unavailable(libc::ENOENT)){"absent"}else{"unavailable"};
        if path==target {target_state=if state=="captured"{"existing"}else{state};}
        let digest=c.bytes.as_ref().map(|b|hash(b));
        if let Some(bytes)=c.bytes {
            match String::from_utf8(bytes) {
                Ok(text)=>{texts.insert(path.clone(),text);}
                Err(_)=>{if path==target{target_state="unavailable";}unavailable.push(json!({"path":path,"reason":"input is not UTF-8"}));}
            }
        } else if state=="unavailable" {unavailable.push(json!({"path":path,"reason":format!("{:?}",c.problem)}));}
        observations.push(json!({"path":path,"state":state,"sha256":digest,"identity":c.identity.map(|id|json!({"device":id.device,"inode":id.inode,"size":id.size,"modified":id.modified,"changed":id.changed}))}));
    }
    if !root.binding_current(identity) {return Err("fit root changed during capture; no proposal issued".into());}
    let before=texts.get(&target).cloned().unwrap_or_default();
    let supplied=option(args,"--contract")?.map(|p|read_adaptive(Path::new(&p)).and_then(|b|String::from_utf8(b).map_err(|_|"invalid UTF-8".into()))).transpose()?;
    let selected=if target_state=="unavailable" {evaluate("FIT_CONTRACT\n".to_string()+&row(&[target_state,"",""]))?}
        else if let Some(after)=&supplied {evaluate("FIT_CONTRACT\n".to_string()+&row(&[target_state,&before,after]))?}
        else if target_state=="existing" {evaluate("FIT_CONTRACT\n".to_string()+&row(&[target_state,&before,&before]))?}
        else {let names=inputs.into_iter().filter(|p|texts.contains_key(p)).collect::<Vec<_>>();evaluate(format!("FIT\n{}\n",names.join("\n")))?};
    let result=selected.first().ok_or("fit result missing")?;
    if selected.len()!=1 || result.len()!=6 || result[0]!="FIT" {return Err("fit proposal result envelope".into());}
    let bytes=&result[3];let proposed_hash=(!bytes.is_empty()).then(||hash(bytes.as_bytes()));
    Ok((json!({"schema":"ultragoal-fit/1","state":result[1],"root":root_path,"target":target,"patch":if result[2].is_empty(){None}else{Some(&result[2])},"proposed_contract":if bytes.is_empty(){None}else{Some(bytes)},"proposed_contract_sha256":proposed_hash,"supplied_contract_sha256":supplied.as_ref().map(|s|hash(s.as_bytes())),"before":{"state":target_state,"sha256":if target_state=="existing"{Some(hash(before.as_bytes()))}else{None}},"root_identity":{"device":identity.device,"inode":identity.inode},"observations":observations,"unavailable":unavailable,"rollback_patch":if result[4].is_empty(){None}else{Some(&result[4])},"apply_boundary":"native host/user authorized action only; revalidate root object and target baseline immediately before applying; this report is not authorization","rollback_boundary":"native host must refuse inverse patch after user changes; prior report is not an ownership or permission token","adoption":"unverified; no protected host adoption channel","original_outcome_mapping":if supplied.is_some(){"explicit supplied requirements retained; outcome coverage still requires independent acceptance"}else{"manifest-derived template only; original requirements and verifier suitability are not established"},"action":result[5]}),0))
}
