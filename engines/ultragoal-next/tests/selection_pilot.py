"""Synthetic context selection journey; --live invokes already authorized Jev."""
import argparse,json,pathlib,subprocess,time,hashlib
ROOT=pathlib.Path(__file__).resolve().parents[1]
def main():
 p=argparse.ArgumentParser();p.add_argument('--out',required=True);p.add_argument('--live',action='store_true');a=p.parse_args();out=pathlib.Path(a.out);out.mkdir(exist_ok=False);repo=out/'repo';repo.mkdir();binary=ROOT/'target/release/ultragoal'
 sources={'writer.py':'def save(cancelled, original):\n    if cancelled:\n        return original\n    return b"new"\n','test_writer.py':'def test_cancellation():\n    original = b"existing"\n    assert save(True, original) == original\n','claim.md':'# Summary\nCancellation always deletes the original output.\n','weather.md':'# Weather\nClouds move north in the afternoon.\n','AGENTS.md':'Preserve original user output on cancellation.\n'}
 for name,text in sources.items():(repo/name).write_text(text)
 runs=[]
 def run(name,args):
  start=time.perf_counter();r=subprocess.run([str(binary),*args],capture_output=True,text=True,timeout=30);elapsed=(time.perf_counter()-start)*1000;(out/f'{name}.json').write_text(r.stdout);v=json.loads(r.stdout);runs.append({'name':name,'exit':r.returncode,'wall_ms':elapsed,'output_sha256':hashlib.sha256(r.stdout.encode()).hexdigest()});return v
 index=run('index',['index','--root',str(repo),'--scope','*','--no-cache']);assert index['state']=='indexed'
 common=['select','--root',str(repo),'--report',str(out/'index.json'),'--query','Does cancellation preserve original output?']
 baseline=run('lexical',common)
 selected=run('semantic',common+(['--disclosure','synthetic','--keychain-service','research-run.typesafe']if a.live else []))
 repeated=run('repeated',common+['--prior',str(out/'semantic.json')]+(['--disclosure','synthetic','--keychain-service','research-run.typesafe']if a.live else []));assert repeated['reuse']['same_prior_question_and_prepared_evidence'];assert repeated['assessment']['attempts']==[]
 (repo/'new_writer.py').write_text('def cancel(path): path.write_bytes(b"")\n');delta=run('delta',common);assert 'new_writer.py'in delta['coverage']['added_paths']
 result={'runs':runs,'candidate_paths':[c['path']for c in selected['shortlist']],'decisions':{c['path']:c['relevance']for c in selected['shortlist']},'anchors':[x.get('path')for x in selected['anchors']],'lexical_returned_bytes':baseline['measurements']['returned_excerpt_bytes'],'semantic_returned_bytes':selected['measurements']['returned_excerpt_bytes'],'provider_attempts':selected.get('assessment',{}).get('attempts',[])if selected.get('assessment')else[],'claim':'development journey only; no independently measured critical recall, primary-agent token savings, or overall latency benefit'}
 (out/'report.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))
if __name__=='__main__':main()
