#!/usr/bin/env python3
"""Local measurements and all-attempt semantic metrics; no primary-model dispatch."""
import argparse,hashlib,json,pathlib,platform,statistics,subprocess,tempfile,time
ROOT=pathlib.Path(__file__).resolve().parent
def fixture(repo,files,bytes_per_file):
 bodies=[('rs','pub fn answer() -> u32 { 42 }\n'),('ts','export const answer = 42;\n'),('py','def answer(): return 42\n'),('md','# Reference\nA measured document.\n'),('json','{"answer":42}\n')];manifest={}
 for i in range(files):
  ext,body=bodies[i%len(bodies)];folder=repo/f'group{i//100:04}';folder.mkdir(exist_ok=True);text=body
  if ext!='json':text+=(('// 'if ext in ['rs','ts']else '# ')+'x'*max(0,bytes_per_file-len(body)-4)+'\n')
  else:text=json.dumps({'answer':42,'padding':'x'*max(0,bytes_per_file-30)})
  path=folder/f'file{i:06}.{ext}';path.write_text(text);actual=path.read_bytes();manifest[str(path.relative_to(repo))]={'bytes':len(actual),'sha256':hashlib.sha256(actual).hexdigest(),'language':ext}
 return manifest
def metrics(rows):
 automatic=[r for r in rows if r.get('automatic')is True]
 wrong=[r for r in automatic if r.get('correct')is False]
 critical=[r for r in rows if r.get('critical_positive')is True]
 missed=[r for r in critical if r.get('decision')=='supported']
 known=[r['cost_usd']for r in rows if isinstance(r.get('cost_usd'),(float,int))]
 accepted=[r for r in rows if r.get('accepted')is True]
 return {'attempts':len(rows),'automatic':len(automatic),'automatic_coverage':len(automatic)/len(rows)if rows else None,'selective_error':len(wrong)/len(automatic)if automatic else None,'critical_misses':len(missed),'critical_denominator':len(critical),'critical_miss_rate':len(missed)/len(critical)if critical else None,'review_burden':sum(r.get('decision')in ['review','insufficient','unavailable','contradicted']for r in rows),'accepted':len(accepted),'known_cost_subtotal':sum(known),'unknown_cost_attempts':len(rows)-len(known),'all_attempt_cost_per_accepted':sum(known)/len(accepted)if accepted and len(known)==len(rows)else None,'claim':'descriptive denominators; no independence or calibration inferred'}
def benchmark(args):
 binary=pathlib.Path(args.binary).resolve();results=[];failed_streak=0;stopped=False
 ledger=pathlib.Path(args.output+'.attempts.jsonl')
 if ledger.exists():raise SystemExit('attempt ledger already exists; choose a new output identity')
 def event(v):
  with ledger.open('a')as f:f.write(json.dumps(v)+'\n');f.flush()
 with tempfile.TemporaryDirectory(prefix='ug-next-benchmark-')as d:
  p=pathlib.Path(d);repo=p/'repo';repo.mkdir();cache=p/'cache';contract=p/'contract.tsv'
  manifest=fixture(repo,args.files,args.bytes)
  target='group0000/file000000.rs'
  contract.write_text('UG\t1\nO\tmember\t1\tSources remain present\tbenchmark\t*\tmembership\tmember\t\texact\tmandatory\nO\tlocal\t1\tSelected function retained\tbenchmark\t'+target+'\tcontent\tcontains\tanswer\texact\tmandatory\n')
  for case in ['cold','warm','one-file','unrelated-document','membership-add']:
   if stopped:break
   for rep in range(args.repeats):
    if case=='one-file':(repo/target).write_text(f'pub fn answer() -> u32 {{ {rep} }}\n')
    if case=='unrelated-document':(repo/'note.md').write_text(f'# Note {rep}\n')
    if case=='membership-add':(repo/f'added{rep}.py').write_text('pass\n')
    command=[str(binary),'check','--root',str(repo),'--contract',str(contract),'--cache',str(cache)]
    if case=='cold':command[-2:]=['--no-cache']
    event({'case':case,'repeat':rep,'event':'started'});start=time.perf_counter()
    try:
     r=subprocess.run(command,capture_output=True,text=True,timeout=90);code=r.returncode
     try:report=json.loads(r.stdout)
     except Exception:report={'error':'non-JSON output','stdout':r.stdout[-1000:]}
    except subprocess.TimeoutExpired:code=None;report={'state':'unknown','error':'90-second external benchmark timeout'}
    except KeyboardInterrupt:event({'case':case,'repeat':rep,'event':'cancelled','wall_ms':(time.perf_counter()-start)*1000});raise
    ms=(time.perf_counter()-start)*1000
    observation={'case':case,'repeat':rep,'wall_ms':ms,'exit':code,'state':report.get('state'),'stages':report.get('timings_ms'),'error':report.get('error')};results.append(observation);event({'event':'finished',**observation})
    failed_streak=0 if code==0 else failed_streak+1
    if failed_streak>=3:stopped=True;break
   print(json.dumps({'case':case,'attempts':sum(r['case']==case for r in results),'stopped':stopped}),flush=True)
  groups={}
  for case in sorted({r['case']for r in results}):
   rows=[r for r in results if r['case']==case];xs=sorted(r['wall_ms']for r in rows);groups[case]={'attempts':len(rows),'successful':sum(r['exit']==0 for r in rows),'median_ms':statistics.median(xs),'p95_ms':xs[min(len(xs)-1,int(.95*len(xs)))]}
  return {'binary':str(binary),'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'core_sha256':hashlib.sha256(binary.with_name('ug-core').read_bytes()).hexdigest(),'host':platform.platform(),'expected_attempts':args.repeats*5,'stopped_after_repeated_failures':stopped,'fixture':{'files':args.files,'nominal_bytes_per_file':args.bytes,'actual_total_source_bytes':sum(f['bytes']for f in manifest.values()),'manifest_sha256':hashlib.sha256(json.dumps(manifest,sort_keys=True).encode()).hexdigest(),'languages':['rs','ts','py','md','json'],'selected_work':'all-name membership plus one Rust text predicate; not full-repository syntax/type/runtime verification'},'os_cache':'not flushed; cold means no UG cache, fresh process','groups':groups,'attempts':results,'claim':'scoped local timing only; no Bend-vs-Rust or end-to-end product benefit claim'}
def main():
 p=argparse.ArgumentParser();s=p.add_subparsers(dest='command',required=True);m=s.add_parser('metrics');m.add_argument('file');b=s.add_parser('benchmark');b.add_argument('--binary',default=str(ROOT/'target/release/ultragoal'));b.add_argument('--files',type=int,default=1000);b.add_argument('--bytes',type=int,default=10000);b.add_argument('--repeats',type=int,default=30);b.add_argument('--output',required=True);a=p.parse_args()
 if a.command=='metrics':print(json.dumps(metrics(json.loads(pathlib.Path(a.file).read_text())),indent=2))
 else:pathlib.Path(a.output).write_text(json.dumps(benchmark(a),indent=2)+'\n');print(a.output)
if __name__=='__main__':main()
