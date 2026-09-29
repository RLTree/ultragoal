#!/usr/bin/env python3
"""Replayable cold mixed-language index benchmark with immutable build custody."""
import argparse,hashlib,json,math,pathlib,platform,shutil,statistics,subprocess,tempfile,time,sys
sys.dont_write_bytecode=True
from evaluate import fixture,ROOT
def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def main():
 p=argparse.ArgumentParser();p.add_argument('--binary',default=str(ROOT/'target/release/ultragoal'));p.add_argument('--files',type=int,default=10000);p.add_argument('--bytes-per-file',type=int,default=10000);p.add_argument('--repeats',type=int,default=30);p.add_argument('--out',required=True);a=p.parse_args()
 if min(a.files,a.bytes_per_file,a.repeats)<=0:raise SystemExit('positive fixture/repeat bounds required')
 binary=pathlib.Path(a.binary).resolve();out=pathlib.Path(a.out);out.mkdir(exist_ok=False);identity=json.loads(binary.with_name('identity.json').read_text())
 for name,sha in identity['binaries'].items():
  original=binary.with_name(name)
  if digest(original)!=sha:raise SystemExit('binary identity mismatch: '+name)
  target=out/'executor'/name;target.parent.mkdir(exist_ok=True);shutil.copy2(original,target)
 for name,sha in identity['source'].items():
  original=ROOT/name
  if digest(original)!=sha:raise SystemExit('source identity mismatch: '+name)
  target=out/'source'/name;target.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(original,target)
 (out/'executor/identity.json').write_text(json.dumps(identity,indent=2)+'\n');rows=[];failures=0
 def event(value):
  with (out/'attempts.jsonl').open('a')as f:f.write(json.dumps(value)+'\n');f.flush()
 with tempfile.TemporaryDirectory(prefix='ug-index-reference-')as d:
  repo=pathlib.Path(d);manifest=fixture(repo,a.files,a.bytes_per_file);(out/'fixture.json').write_text(json.dumps(manifest,indent=2)+'\n');expected_bytes=sum(f['bytes']for f in manifest.values())
  for repeat in range(a.repeats):
   event({'event':'started','repeat':repeat});start=time.perf_counter()
   try:
    proc=subprocess.run([str(out/'executor/ultragoal'),'index','--root',str(repo),'--scope','*','--no-cache','--events'],capture_output=True,text=True,timeout=45);process_ms=(time.perf_counter()-start)*1000;decode_start=time.perf_counter();value=json.loads(proc.stdout);decode_ms=(time.perf_counter()-decode_start)*1000
    observed={v['path']:v['content_sha256']for v in value.get('fact_sets',[])};expected={path:v['sha256']for path,v in manifest.items()};complete=proc.returncode==0 and value.get('state')=='indexed' and observed==expected and value.get('inputs',{}).get('bytes')==expected_bytes
    events=[json.loads(line)for line in proc.stderr.splitlines()if line.startswith('{')];(out/f'events-{repeat:02}.json').write_text(json.dumps(events,indent=2)+'\n')
    entry={'repeat':repeat,'exit':proc.returncode,'complete':complete,'state':value.get('state'),'wall_process_ms':process_ms,'consumer_json_decode_ms':decode_ms,'stages':value.get('timings_ms'),'work':value.get('work'),'actual_captured_source_bytes':value.get('inputs',{}).get('bytes'),'fact_sets':len(observed),'output_bytes':len(proc.stdout.encode()),'output_events':events,'error':value.get('error')}
   except subprocess.TimeoutExpired:entry={'repeat':repeat,'complete':False,'error':'45second external timeout','wall_process_ms':(time.perf_counter()-start)*1000}
   except KeyboardInterrupt:event({'event':'cancelled','repeat':repeat,'wall_ms':(time.perf_counter()-start)*1000});raise
   except Exception as error:entry={'repeat':repeat,'complete':False,'error':f'{type(error).__name__}: {error}','wall_process_ms':(time.perf_counter()-start)*1000}
   rows.append(entry);event({'event':'finished',**entry});print(json.dumps({'repeat':repeat,'complete':entry['complete'],'wall_process_ms':entry['wall_process_ms']}),flush=True);failures=0 if entry['complete']else failures+1
   if failures>=3:break
  unchanged=all(hashlib.sha256((repo/path).read_bytes()).hexdigest()==v['sha256']for path,v in manifest.items())
 xs=sorted(r['wall_process_ms']for r in rows);p95=xs[max(0,math.ceil(.95*len(xs))-1)]
 report={'schema':'ultragoal-index-benchmark/1','host':platform.platform(),'source_build_identity':identity,'fixture':{'files':a.files,'nominal_bytes_per_file':a.bytes_per_file,'actual_total_source_bytes':expected_bytes,'manifest_sha256':digest(out/'fixture.json'),'source_unchanged_after_attempts':unchanged,'languages':['rs','ts','py','md','json'],'coverage':'Bend source-line lexical/Markdown facts and JSON value syntax/key metadata; no native language syntax, type checking, runtime or whole-program semantics'},'os_cache':'not flushed; cold means fresh process and no UG disk cache','attempts':rows,'requested_repeats':a.repeats,'successful':sum(r['complete']for r in rows),'median_ms':statistics.median(xs),'p95_ms':p95,'target_p95_ms':5000,'target_met':len(rows)==a.repeats and all(r['complete']for r in rows) and unchanged and p95<=5000,'serialization':'report-output events separately measure JSON serialization and stdout write; wall includes process startup/transport; consumer JSON decode reported separately','replay_command':f'python3 source/benchmark_index.py --binary executor/ultragoal --files {a.files} --bytes-per-file {a.bytes_per_file} --repeats {a.repeats} --out /tmp/NEW-UNUSED-OUTPUT','claim':'owner-run reference-fixture timing only; not independent product acceptance or general language performance'}
 if any(digest(out/'executor'/name)!=sha for name,sha in identity['binaries'].items()):raise SystemExit('measured executable changed')
 (out/'report.json').write_text(json.dumps(report,indent=2)+'\n');payload={str(f.relative_to(out)):digest(f)for f in sorted(out.rglob('*'))if f.is_file()};(out/'MANIFEST.json').write_text(json.dumps(payload,indent=2)+'\n');print(json.dumps({'report':str(out/'report.json'),'p95_ms':p95,'target_met':report['target_met'],'manifest_sha256':digest(out/'MANIFEST.json')}))
if __name__=='__main__':main()
