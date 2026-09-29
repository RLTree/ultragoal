import Base
import ./Wire.bend as W

def default_contract(manifest: String,command: String) -> String:
  "UG\t1\n" ++ W.row(["O","manifest","1","Project manifest remains available","fit template; original-outcome mapping and adoption remain unverified",manifest,"membership","member","","exact","mandatory"]) ++ W.row(["O","project-tests","1","Project test suite succeeds on current relevant inputs","fit template; original-outcome mapping and adoption remain unverified","*","unknown","native",command,"runtime","mandatory"])

# Fit remains a proposal. Native host tools own any subsequent filesystem effect.
def paths() -> String:
  W.row(["FIT_TARGET",".ultragoal.tsv"]) ++ W.row(["FIT_INPUT","Cargo.toml"]) ++ W.row(["FIT_INPUT","package.json"]) ++ W.row(["FIT_INPUT","pyproject.toml"])

def prefixed(xs: List<&2,String>,+prefix: String) -> String:
  match xs:
    case Nil{}: ""
    case "" <> Nil{}: ""
    case h <> t: prefix ++ h ++ "\n" ++ prefixed(t,prefix)

def eof_marker(needed: Bool) -> String:
  match needed:
    case True{}: "\\ No newline at end of file\n"
    case False{}: ""

def patch_part(+text: String,prefix: String) -> String:
  prefixed(String.lines(text),prefix) ++ eof_marker(Bool.not(String.is_empty(text)) && Bool.not(String.ends_with(text,"\n")))

def end_count(end: Bool) -> Nat:
  match end:
    case True{}: 1n
    case False{}: 0n

def line_count(+text: String) -> Nat:
  Nat.sub(W.length(String.lines(text)),end_count(String.is_empty(text) || String.ends_with(text,"\n")))

def range(empty: Bool,count: Nat) -> String:
  match empty:
    case True{}: "0,0"
    case False{}: "1," ++ Nat.show(count)

def path(exists: Bool,prefix: String) -> String:
  match exists:
    case True{}: prefix ++ ".ultragoal.tsv"
    case False{}: "/dev/null"

def patch(+before: String,+after: String,before_exists: Bool,after_exists: Bool) -> String:
  "--- " ++ path(before_exists,"a/") ++ "\n+++ " ++ path(after_exists,"b/") ++ "\n@@ -" ++ range(String.is_empty(before),line_count(before)) ++ " +" ++ range(String.is_empty(after),line_count(after)) ++ " @@\n" ++ patch_part(before,"-") ++ patch_part(after,"+")

def contract_proposal(same: Bool,+before: String,+after: String,+exists: Bool) -> String:
  match same:
    case True{}: W.row(["FIT","unchanged","",after,"","Contract bytes are unchanged; no effect or adoption granted."])
    case False{}: W.row(["FIT","proposal",patch(before,after,exists,True{}),after,patch(after,before,True{},exists),"Review original-outcome coverage and these exact bytes. The native host must revalidate/apply; this plan does not grant permission or adoption."])

def process(names: List<&2,String>) -> String:
  match names:
    case Nil{}: W.row(["FIT","unknown","","","","No supported manifest found; supply original requirements with --contract."])
    case "Cargo.toml" <> _: contract_proposal(False{},"",default_contract("Cargo.toml","cargo test --offline --locked"),False{})
    case "package.json" <> _: contract_proposal(False{},"",default_contract("package.json","npm test"),False{})
    case "pyproject.toml" <> _: contract_proposal(False{},"",default_contract("pyproject.toml","python -m pytest"),False{})
    case _ <> t: process(t)
