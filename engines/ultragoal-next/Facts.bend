import Base
import ./Wire.bend as W
import ./Json.bend as J
import ./Source.bend as Source
import ./FactRecord.bend as R
import ./Equality.bend as E

def choose(ok: Bool,a: String,b: String) -> String:
  match ok:
    case True{}: a
    case False{}: b

def grammar(+path: String) -> String:
  choose(String.ends_with(path,".json"),"json/1",choose(String.ends_with(path,".rs"),"rust-structure/1",choose(String.ends_with(path,".py"),"python-structure/1",choose(String.ends_with(path,".ts") || String.ends_with(path,".tsx") || String.ends_with(path,".js") || String.ends_with(path,".jsx"),"ecmascript-structure/1",choose(String.ends_with(path,".md"),"markdown-lines/1","text-lines/1")))))

def width_four(ok: Bool) -> U32:
  match ok:
    case True{}: 3
    case False{}: 4

def width_three(ok: Bool,c: U32) -> U32:
  match ok:
    case True{}: 2
    case False{}: width_four(U32.is_le(c,65535))

def width_two(ok: Bool,+c: U32) -> U32:
  match ok:
    case True{}: 1
    case False{}: width_three(U32.is_le(c,2047),c)

def bytes_from(s: String,+n: U32) -> U32:
  match s:
    case SNil{}: n
    case SCon{Chr{+c},t}: bytes_from(t,(n + width_two(U32.is_le(c,127),c) : U32))

# UTF-8 byte length, accumulated so long lines need no continuation per character.
def bytes(s: String) -> U32:
  bytes_from(s,0)

def markdown_link(+line: String) -> String:
  choose(E.text_contains(line,"]("),"link-candidate","line")

def markdown_fence(fence: Bool,+line: String) -> String:
  match fence:
    case True{}: "fence"
    case False{}: markdown_link(line)

def markdown_heading(heading: Bool,+line: String) -> String:
  match heading:
    case True{}: "heading-candidate"
    case False{}: markdown_fence(String.starts_with(line,"```") || String.starts_with(line,"~~~"),line)

def code_definition(+line: String) -> String:
  choose(String.starts_with(line,"def ") || String.starts_with(line,"class ") || String.starts_with(line,"fn ") || String.starts_with(line,"pub fn ") || String.starts_with(line,"function ") || String.starts_with(line,"export function "),"definition-candidate","line")

def code_call_import(call: Bool,+line: String) -> String:
  match call:
    case True{}: "import-candidate"
    case False{}: code_definition(line)

def code_import(imported: Bool,+line: String) -> String:
  match imported:
    case True{}: "import-candidate"
    case False{}: code_call_import(E.text_contains(line,"require(") || E.text_contains(line,"import("),line)

def classify_kind(markdown: Bool,+line: String) -> String:
  match markdown:
    case True{}: markdown_heading(String.starts_with(line,"#"),line)
    case False{}: code_import(String.starts_with(line,"import ") || String.starts_with(line,"from ") || String.starts_with(line,"use ") || String.starts_with(line,"mod "),line)

# Each branch is evaluated only when selected.
def classify(+line: String,+g: String) -> String:
  classify_kind(E.text_eq(g,"markdown-lines/1"),line)

def facts(lines: List<&2,String>,+g: String,+line: U32,+start: U32) -> List<&2,R.Fact>:
  match lines:
    case Nil{}: Nil{}
    case +h <> t:
      +width = bytes(h)
      R.Fact{classify(String.trim_start(h),g),h,line,start,(start + width : U32)} <> facts(t,g,(line + 1 : U32),(start + width + 1 : U32))

def encoded(fs: List<&2,R.Fact>) -> String:
  R.encoded(False{},fs)

def encoded_spans(fs: List<&2,R.Fact>) -> String:
  R.encoded(True{},fs)

def json_fields(fields: List<&2,Sigma<&2,&2,String,_ => J.Json>>) -> String:
  match fields:
    case Nil{}: ""
    case (k,v) <> t: W.row(["json-key",k,"0","0","0"]) ++ json_fields(t)

def json_value(value: J.Json) -> String:
  match value:
    case J.Object{fields}: json_fields(fields)
    case _: ""

def json_parsed(value: Maybe<&2,J.Json>) -> String:
  match value:
    case Some{v}: W.row(["json-valid","true","0","0","0"]) ++ json_value(v)
    case None{}: W.row(["json-valid","false","0","0","0"])

def parse(g: String,+text: String) -> String:
  match g:
    case "json/1": json_parsed(J.parse(text)) ++ encoded(facts(W.lines(text),"text-lines/1",1,0))
    case +name: encoded(facts(W.lines(text),name,1,0)) ++ Source.parse(False{},name,text)

def parse_spans(g: String,+text: String) -> String:
  match g:
    case "json/1": json_parsed(J.parse(text)) ++ encoded_spans(facts(W.lines(text),"text-lines/1",1,0))
    case +name: encoded_spans(facts(W.lines(text),name,1,0)) ++ Source.parse(True{},name,text)

def matches_record(r: W.Record,+kind: String,+value: String) -> Bool:
  match r:
    case W.Row{+xs}: String.eq(W.field(xs,0n),kind) && String.eq(W.field(xs,1n),value)
    case _: False{}

def contains(lines: List<&2,String>,+kind: String,+value: String) -> Bool:
  match lines:
    case Nil{}: False{}
    case h <> t: matches_record(W.parse(h),kind,value) || contains(t,kind,value)

def kind_record(r: W.Record,+kind: String) -> Bool:
  match r:
    case W.Row{xs}: E.text_eq(W.field(xs,0n),kind)
    case _: False{}

def has_kind(lines: List<&2,String>,+kind: String) -> Bool:
  match lines:
    case Nil{}: False{}
    case h <> t: kind_record(W.parse(h),kind) || has_kind(t,kind)
