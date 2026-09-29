import Base
import ./Core.bend as C
import ./Wire.bend as W
import ./Equality.bend as E
import ./Secrets.bend as S

def exclusions() -> String:
  W.row(["EXCLUDE",".git"]) ++ W.row(["EXCLUDE","node_modules"]) ++ W.row(["EXCLUDE","target"]) ++ W.row(["EXCLUDE","__pycache__"]) ++ W.row(["EXCLUDE",".pytest_cache"]) ++ W.row(["EXCLUDE",".ruff_cache"]) ++ W.row(["EXCLUDE",".mypy_cache"])

def scope_file(explicit: Bool,selector: String) -> String:
  match explicit:
    case True{}: String.drop(selector,5n)
    case False{}: selector

def scope_prefix(prefix: Bool,+selector: String) -> String:
  match prefix:
    case True{}: String.drop(selector,7n)
    case False{}: scope_file(String.starts_with(selector,"file:"),selector)

def scope_path(+selector: String) -> String:
  scope_prefix(String.starts_with(selector,"prefix:"),selector)

def component(parts: List<&2,String>,+name: String) -> Bool:
  match parts:
    case Nil{}: False{}
    case h <> t: String.eq(h,name) || component(t,name)

def disclosable_path(+path: String) -> Bool:
  Bool.not(String.starts_with(path,".codex/") || String.starts_with(path,".agents/") || String.starts_with(path,".codex-worktree/") || String.contains(path,"/.codex/") || String.contains(path,"/.agents/") || String.contains(path,"/.codex-worktree/") || S.secret_name(S.base_name(path)))

def disclosable(+path: String,+text: String) -> Bool:
  disclosable_path(String.to_lower(path)) && Bool.not(S.secret_file(S.base_name(path),text))

# The whole provider request body is screened before dispatch.
def secret_screen(r: W.Record) -> String:
  match r:
    case W.Row{+text <> Nil{}}: Bool.pick(String,S.secret_text(text),"SCREEN\trefused\n","SCREEN\tclear\n")
    case _: "SCREEN\trefused\n"

def within(value: Maybe<&2,U32>,limit: U32) -> Bool:
  match value:
    case Some{v}: U32.is_le(v,limit)
    case None{}: False{}

def nonzero(value: Maybe<&2,U32>) -> Bool:
  match value:
    case Some{v}: U32.is_gt(v,0)
    case None{}: False{}

def admitted(ok: Bool,remaining: String) -> String:
  match ok:
    case True{}: W.row(["ADMITTED","jev-1.13.0",remaining,"2","64000"])
    case False{}: W.row(["REFUSED","provider policy requires pinned model, explicit permitted disclosure, bounded independent questions, live operation and packet"])

def disclosure_allowed(disclosure: String) -> Bool:
  match disclosure:
    case "synthetic": True{}
    case "public": True{}
    case "project": True{}
    case _: False{}

def reserved(value: Maybe<&2,Nat>) -> Bool:
  match value:
    case Some{_}: True{}
    case None{}: False{}

def provider(r: W.Record) -> String:
  match r:
    # jev-1.13 limits with bytes as the token bound: 64k tokens per request and
    # 32k for the state plus its longest question; 64,000 and 32,000 bytes hold
    # under both the 1,000- and 1,024-token reading of "k".
    case W.Row{model <> disclosure <> bytes <> state_question <> +questions <> state <> +remaining <> Nil{}}:
      admitted(String.eq(model,"jev-1.13.0") && disclosure_allowed(disclosure) && within(U32.read(bytes),64000) && within(U32.read(state_question),32000) && within(U32.read(questions),8) && nonzero(U32.read(questions)) && String.eq(state,"running") && within(U32.read(remaining),15000) && nonzero(U32.read(remaining)) && reserved(C.reserve(2n,2n)),remaining)
    case _: admitted(False{},"0")

def at_least(value: Maybe<&2,U32>,floor: U32) -> Bool:
  match value:
    case Some{v}: U32.is_ge(v,floor)
    case None{}: False{}

# Semantic budget inputs: a per-request deadline of 0.5-15 s (the jev-codex bound)
# and at most four requests in flight.
def budget_result(ok: Bool,ms: String,k: String) -> String:
  match ok:
    case True{}: W.row(["BUDGET",ms,k])
    case False{}: "REFUSED\tsemantic budget out of bounds\n"

def semantic_budget(r: W.Record) -> String:
  match r:
    case W.Row{+ms <> +k <> Nil{}}: budget_result(within(U32.read(ms),15000) && at_least(U32.read(ms),500) && within(U32.read(k),4) && nonzero(U32.read(k)),ms,k)
    case _: "REFUSED\tsemantic budget out of bounds\n"

def native_plan_result(ok: Bool) -> String:
  match ok:
    case True{}: W.row(["ADMITTED","c11-syntax","10000"])
    case False{}: W.row(["REFUSED","captured-input syntax envelope excludes preprocessor directives and external headers"])

def native_plan(r: W.Record) -> String:
  match r:
    case W.Row{+text <> Nil{}}: native_plan_result(Bool.not(String.contains(text,"#") || String.contains(text,"%:") || String.contains(text,"??=") || String.contains(text,"\\\n") || String.contains(text,"\\\r\n") || String.contains(text,"_Pragma") || String.contains(text,"__pragma")))
    case _: native_plan_result(False{})

def native_status(complete: Bool,ok: Bool) -> String:
  match complete ok:
    case True{} True{}: W.row(["STATE","verified","0"])
    case True{} False{}: W.row(["STATE","failed","1"])
    case _ _: W.row(["STATE","unknown","2"])

def native(r: W.Record) -> String:
  match r:
    case W.Row{status <> complete <> Nil{}}: native_status(String.eq(complete,"complete"),String.eq(status,"0"))
    case _: native_status(False{},False{})

def first(lines: List<&2,String>) -> W.Record:
  match lines:
    case h <> Nil{}: W.parse(h)
    case h <> "" <> Nil{}: W.parse(h)
    case _: W.Invalid{"expected one record"}

def retry_result(ok: Bool) -> String:
  match ok:
    case True{}: "RETRY\n"
    case False{}: "STOP\n"

def retry(r: W.Record) -> String:
  match r:
    case W.Row{+status <> remaining <> time <> state <> Nil{}}:
      retry_result((String.eq(status,"not-sent") || String.eq(status,"429") || String.eq(status,"500") || String.eq(status,"502") || String.eq(status,"503") || String.eq(status,"504")) && nonzero(U32.read(remaining)) && Bool.not(within(U32.read(time),49)) && String.eq(state,"running"))
    case _: "STOP\n"

def proof_lines(lines: List<&2,String>) -> Bool:
  match lines:
    case Nil{}: True{}
    case +h <> t: (Bool.not(String.contains(h,"import")) || String.eq(String.trim(h),"import Base")) && proof_lines(t)

def proof_plan(r: W.Record) -> String:
  match r:
    case W.Row{+text <> laws <> Nil{}}: native_plan_result(proof_lines(String.lines(text)) && Bool.not(String.contains(text,"unsafe") || String.contains(text,"?")) && String.contains(text,"law ") && String.contains(text,"def ") && String.eq(laws,"absent"))
    case _: native_plan_result(False{})

def proof_result(r: W.Record) -> String:
  match r:
    case W.Row{status <> complete <> +output <> +errors <> Nil{}}:
      native_status(String.eq(complete,"complete"),String.eq(status,"0") && String.contains(output,"All terms check.") && Bool.not(String.contains(String.to_lower(output ++ errors),"unsafe") || String.contains(output ++ errors,"open claim") || String.contains(output ++ errors,"foreign")))
    case _: native_status(False{},False{})

def proof_words(xs: List<&2,String>) -> List<&2,String>:
  match xs:
    case Nil{}: Nil{}
    case "" <> t: proof_words(t)
    case h <> t: h <> proof_words(t)

def import_path(ok: Bool,path: String) -> String:
  match ok:
    case True{}: W.row(["IMPORT",path])
    case False{}: "REFUSED\tunsupported proof import syntax\n"

def base_import(ok: Bool) -> String:
  match ok:
    case True{}: ""
    case False{}: "REFUSED\tunsupported proof import syntax\n"

# Words are compared with E.text_eq rather than string-literal list patterns,
# which expanded into several MB of character-matching C.
def proof_import(words: List<&2,String>) -> String:
  match words:
    case +a <> +b <> Nil{}: base_import(E.text_eq(a,"import") && E.text_eq(b,"Base"))
    case +a <> +path <> +k <> alias <> Nil{}: import_path(E.text_eq(a,"import") && E.text_eq(k,"as") && (String.starts_with(path,"./") || String.starts_with(path,"../")) && String.ends_with(path,".bend") && Bool.not(String.contains(path,"\"") || String.contains(path,"\\") || String.contains(path,":")),path)
    case _: "REFUSED\tunsupported proof import syntax\n"

def ordinary_line(ok: Bool) -> String:
  match ok:
    case True{}: ""
    case False{}: "REFUSED\tunsupported proof import syntax\n"

def proof_line(is_import: Bool,+line: String) -> String:
  match is_import:
    case True{}: proof_import(proof_words(String.split(line,' ')))
    case False{}: ordinary_line(Bool.not(String.contains(line,"import")))

def proof_scan(lines: List<&2,String>) -> String:
  match lines:
    case Nil{}: ""
    case +h <> t: proof_line(String.starts_with(String.trim(h),"import "),String.trim(h)) ++ proof_scan(t)

def proof_source_allowed(ok: Bool,text: String) -> String:
  match ok:
    case True{}: "SOURCE\tadmitted\n" ++ proof_scan(String.lines(text))
    case False{}: "REFUSED\tunsafe or incomplete source\n"

def proof_source(r: W.Record) -> String:
  match r:
    case W.Row{+text <> Nil{}}: proof_source_allowed(Bool.not(String.contains(text,"unsafe") || String.contains(text,"?")),text)
    case _: "REFUSED\tinvalid source frame\n"
