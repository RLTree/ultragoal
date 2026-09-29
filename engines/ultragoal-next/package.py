#!/usr/bin/env python3
"""Prepare an identity-checked local candidate; never install or publish."""
import hashlib,json,pathlib,shutil,tempfile
from source_inventory import admitted_bytes, hashes as source_hashes, rust_parse_limit, stream_digest
ROOT=pathlib.Path(__file__).resolve().parent
def digest(path):return stream_digest(path)
def sources():
 try:return source_hashes(ROOT)
 except ValueError as error:raise SystemExit(str(error))
identity=json.loads(admitted_bytes(ROOT/'target/release/identity.json',rust_parse_limit()))
if sources()!=identity['source']:raise SystemExit('source identity mismatch; build the exact source before packaging')
for name,expected in identity['binaries'].items():
 path=ROOT/'target/release'/name
 if path.is_symlink()or digest(path)!=expected:raise SystemExit('binary identity mismatch: '+name)
if len((ROOT/'plugin/skills/harness-ultragoal/SKILL.md').read_text().split())>500:raise SystemExit('entry skill exceeds500words')
candidate=hashlib.sha256(json.dumps(identity,sort_keys=True).encode()).hexdigest()[:16]
OUT=ROOT/'target/packages'/candidate/'harness-ultragoal'
if OUT.exists():raise SystemExit('package output exists; preserve the existing candidate')
OUT.parent.mkdir(parents=True,exist_ok=True)
with tempfile.TemporaryDirectory(prefix='.preparing-',dir=OUT.parent)as d:
 stage=pathlib.Path(d)/'harness-ultragoal';shutil.copytree(ROOT/'plugin',stage);(stage/'bin').mkdir()
 # Installed candidates must be distinguishable: stamp the exact candidate as SemVer build metadata.
 manifest=stage/'.codex-plugin/plugin.json';plugin=json.loads(manifest.read_text());plugin['version']=plugin['version'].split('+')[0]+'+'+candidate;manifest.write_text(json.dumps(plugin,separators=(',',':'))+'\n')
 reference=stage/'skills/harness-ultragoal/references/semantic-contract.md';reference.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(ROOT/'SEMANTIC_CONTRACT.md',reference)
 for name in ['ultragoal','ug-core','identity.json']:shutil.copy2(ROOT/'target/release'/name,stage/'bin'/name)
 if sources()!=identity['source']:raise SystemExit('source changed during packaging')
 for name,expected in identity['binaries'].items():
  if digest(stage/'bin'/name)!=expected:raise SystemExit('binary changed during packaging: '+name)
 if json.loads(admitted_bytes(stage/'bin/identity.json',rust_parse_limit()))!=identity:raise SystemExit('identity changed during packaging')
 files={str(p.relative_to(stage)):digest(p)for p in sorted(stage.rglob('*'))if p.is_file()}
 (stage/'MANIFEST.json').write_text(json.dumps({'files':files,'build_identity_sha256':digest(stage/'bin/identity.json'),'claim':'local package candidate only; install/discovery/runtime journey unqualified'},indent=2)+'\n')
 if OUT.exists():raise SystemExit('another candidate appeared; no replacement performed')
 stage.rename(OUT)
print(OUT)
