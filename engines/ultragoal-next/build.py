#!/usr/bin/env python3
"""Local reproducible candidate build; no install, update, telemetry or publish."""
import hashlib,json,os,pathlib,subprocess,time,tempfile
from source_inventory import hashes as source_hashes
ROOT=pathlib.Path(__file__).resolve().parent
OUT=ROOT/'target/release'
# Bend runs on Bun, whose heap grows toward free RAM; telling JSC the host has 8 GiB
# keeps the C emission near 14.5 GiB instead of ~28 GiB, with identical output and time.
BUN_RAM_SIZE=str(8*2**30)
# The product posts only to the real provider; a synthetic-provider build is a
# separate test binary (UG_TYPESAFE_PROVIDER is read at compile time).
if 'UG_TYPESAFE_PROVIDER' in os.environ:raise SystemExit('product build refuses UG_TYPESAFE_PROVIDER')
env={**os.environ,'BEND_NO_TELEMETRY':'1','CARGO_TARGET_DIR':str(ROOT/'target'),'BUN_JSC_forceRAMSize':BUN_RAM_SIZE}
def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def source():
 try:return source_hashes(ROOT)
 except ValueError as error:raise SystemExit(str(error))
import shutil
import tomllib
bend=pathlib.Path(shutil.which('bend'))
base=pathlib.Path.home()/'.bend/bend2/base.bend'
def tool_sources():return {'bend':digest(bend),'Base':digest(base)}|{'Base/effs/'+p.name:digest(p)for p in sorted((base.parent/'effs').iterdir())if p.suffix in {'.c','.js'}}
compiler=pathlib.Path(subprocess.check_output(['xcrun','--find','clang'],text=True).strip()) if os.uname().sysname=='Darwin' else pathlib.Path(shutil.which('clang'))
toolchain_sources=tool_sources()
compiler_digest=digest(compiler)
compilation_flags=['-std=c11','-O3']
sdk_path=subprocess.check_output(['xcrun','--show-sdk-path'],text=True).strip() if os.uname().sysname=='Darwin' else None
if sdk_path:compilation_flags+=['-isysroot',sdk_path]
stages=[]
build_started=time.monotonic()
OUT.mkdir(parents=True,exist_ok=True)
def ledger():
 (OUT/'build-stages.json').write_text(json.dumps({'stages':stages,'end_to_end_seconds':time.monotonic()-build_started,'compiler':str(compiler),'compiler_sha256':compiler_digest,'flags':compilation_flags,'maxrss_unit':'bytes' if os.uname().sysname=='Darwin' else 'KiB'},indent=2)+'\n')
def closure_digest(names):return hashlib.sha256(json.dumps({'source':{name:digest(ROOT/name)for name in names},'toolchain':toolchain_sources},sort_keys=True).encode()).hexdigest()
# Rule evaluation consumes parser results: bind every runtime Bend source rather
# than omitting a transitive parser dependency from the rule computation key.
runtime_sources=[p.name for p in sorted(ROOT.glob('*.bend'))if p.name not in {'Identity.bend','LAWS.bend','PROOF.bend'}]
rule_id=closure_digest(runtime_sources)
parser_id=closure_digest(['Facts.bend','Source.bend','SourceLex.bend','SourceTree.bend','SourceIndent.bend','SourceDecl.bend','SourcePyImport.bend','FactRecord.bend','ParseWork.bend','ParseCache.bend','GroupedParse.bend','ParsedValue.bend','Sha256.bend','Utf8Encode.bend','Digest.bend','Equality.bend','Json.bend','Utf8.bend','Wire.bend'])
(ROOT/'Identity.bend').write_text('# Generated source/tool identities; no runtime writes.\nimport Base\ndef rules() -> String:\n  '+json.dumps(rule_id)+'\ndef parsers() -> String:\n  '+json.dumps(parser_id)+'\n')
lock=tomllib.loads((ROOT/'Cargo.lock').read_text());syn=next(p for p in lock['package']if p['name']=='syn')
native_identity=hashlib.sha256(json.dumps({'wrapper':digest(ROOT/'src/native_syntax.rs'),'syn':syn,'rustc':subprocess.check_output(['rustc','-Vv'],text=True),'profile':'release'},sort_keys=True).encode()).hexdigest()
(ROOT/'src/native_identity.rs').write_text('// Generated parser-specific producer identity.\npub const RUST_SYNTAX: &str = '+json.dumps(native_identity)+';\n')
before=source()
def run(args,stage=None):
 started=time.monotonic()
 with tempfile.TemporaryFile() as output:
  process=subprocess.Popen(args,cwd=ROOT,env=env,stdout=output,stderr=subprocess.STDOUT,start_new_session=True)
  _,status,usage=os.wait4(process.pid,0)
  process.returncode=os.waitstatus_to_exitcode(status)
  output.seek(0);text=output.read().decode('utf-8',errors='replace')
 stages.append({'stage':stage or args[0], 'argv':list(map(str,args)), 'seconds':time.monotonic()-started,'wait4_maxrss':usage.ru_maxrss,'user_seconds':usage.ru_utime,'system_seconds':usage.ru_stime,'exit':process.returncode})
 ledger();print(text,end='',flush=True)
 if process.returncode:raise SystemExit(process.returncode)
 return text
version=run(['bend','version']).strip()
proof=run(['bend','PROOF.bend','--check-only'])
if 'All terms check.' not in proof or any(x in proof.lower()for x in ['unsafe','open claim','foreign dependency']):raise SystemExit('proof diagnostic rejected')
run(['cargo','build','--release','--offline','--locked'])
# Generated C is retained, and emission and Clang are timed separately.
cpath=OUT/'ug-core.c'
run(['bend','Engine.bend','-o',str(cpath)],stage='bend-emit-c')
run([str(compiler),*compilation_flags,str(cpath),'-lpthread','-lm','-o',str(OUT/'ug-core')],stage='clang')
# The invalidation and source-structure suites run these probes over production modules,
# so they are rebuilt with every candidate; they are test tools, not packaged binaries.
for probe,name in (('tests/InvalidationProbe.bend','invalidation-probe'),('tests/SourceProbe.bend','source-probe')):
 run(['bend',probe,'-o',str(OUT/(name+'.c'))],stage='probe-emit-'+name)
 run([str(compiler),*compilation_flags,str(OUT/(name+'.c')),'-lpthread','-lm','-o',str(OUT/name)],stage='probe-clang-'+name)
sources=source()
if before!=sources:raise SystemExit('source changed during build; no bound identity emitted')
if toolchain_sources!=tool_sources() or compiler_digest!=digest(compiler):raise SystemExit('compiler/toolchain sources changed during build; no bound identity emitted')
identity={'generated_c_sha256':digest(cpath),'compiler_binary_sha256':compiler_digest,'compiler_flags':compilation_flags+['-lpthread','-lm'],'sdk_path':sdk_path,'build_stages':stages,'version':version,'bend_binary_sha256':digest(bend),'base_sha256':digest(base),'toolchain_sources':toolchain_sources,'rustc':run(['rustc','-Vv']),'clang':run(['clang','--version']),'source':sources,'binaries':{p.name:digest(p)for p in [OUT/'ultragoal',OUT/'ug-core']},'probes':{name:digest(OUT/name)for name in ['invalidation-probe','source-probe']},'bun_jsc_force_ram_size':BUN_RAM_SIZE,'typesafe_provider':'typesafe','built_unix':time.time(),'claim':'local source/build/proof only; no installed/host/product qualification'}
ledger()
(OUT/'identity.json').write_text(json.dumps(identity,indent=2)+'\n')
print(json.dumps({'binary':str(OUT/'ultragoal'),'identity':str(OUT/'identity.json')}))
