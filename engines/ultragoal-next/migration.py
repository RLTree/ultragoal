#!/usr/bin/env python3
"""Prepare and rehearse exact rollback custody; no uninstall or source mutation."""
import argparse,hashlib,json,pathlib,shutil,tarfile,tempfile
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def main():
 p=argparse.ArgumentParser();p.add_argument('--legacy-root',required=True);p.add_argument('--out',required=True);a=p.parse_args();root=pathlib.Path(a.legacy_root).resolve();out=pathlib.Path(a.out);out.mkdir(exist_ok=False)
 release=root/'RELEASE_FILES.json';entries=json.loads(release.read_text())['files'];paths=['RELEASE_FILES.json',*entries];observed={};mismatches=[]
 for rel in paths:
  path=pathlib.PurePosixPath(rel)
  if path.is_absolute()or'..'in path.parts:raise SystemExit('unsafe release path')
  f=root/path
  if f.is_symlink()or not f.is_file()or not f.resolve().is_relative_to(root):raise SystemExit(f'unsafe/missing release file: {rel}')
  digest=sha(f);observed[rel]={'sha256':digest,'mode':f.stat().st_mode&0o777,'bytes':f.stat().st_size}
  if rel in entries and digest!=entries[rel]['sha256']:mismatches.append(rel)
 archive=out/'rollback.tar.gz'
 with tarfile.open(archive,'w:gz')as tar:
  for rel in paths:tar.add(root/rel,arcname=rel,recursive=False)
 # Extract to a fresh scratch location only; never over a user's source tree.
 with tempfile.TemporaryDirectory(prefix='ug-rollback-rehearsal-')as d:
  with tarfile.open(archive)as tar:tar.extractall(d,filter='data')
  restored={rel:sha(pathlib.Path(d)/rel)for rel in paths}
  assert all(restored[k]==v['sha256']for k,v in observed.items())
 # Re-read after archive creation to reject changes during custody capture.
 changed=[rel for rel,v in observed.items()if sha(root/rel)!=v['sha256']]
 if changed:raise SystemExit(f'source changed during archive: {changed}')
 plan={'schema':'ultragoal-migration/1','legacy_root':str(root),'files':observed,'release_manifest_mismatches':mismatches,'archive_sha256':sha(archive),'restoration_rehearsal':'exact bytes verified in empty scratch directory','mutations_performed':[],'future_transition':'review consumer-specific patches and supported uninstall only after separate approval','unqualified':['complete source/history outside RELEASE_FILES.json','global/remote consumers','installed discovery and rollback through supported host','new product parity and calibrated acceptance'],'disposition':'HOLD retirement; archive covers exactly the listed release payload, not universal custody'}
 (out/'plan.json').write_text(json.dumps(plan,indent=2)+'\n');print(json.dumps({'plan':str(out/'plan.json'),'files':len(observed),'release_mismatches':mismatches,'disposition':plan['disposition']}))
if __name__=='__main__':main()
