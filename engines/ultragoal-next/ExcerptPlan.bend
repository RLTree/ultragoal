import Base
import ./Wire.bend as W

type SpanWindow is Data:
  SpanWindow{start: U32,end: U32,count: U32}

def emit(+id: String,window: Maybe<&2,SpanWindow>) -> String:
  match window:
    case None{}: ""
    case Some{SpanWindow{start,end,_}}: W.row(["WINDOW",id,U32.show(start),U32.show(end)])

def extending(split: Bool,+id: String,+start: U32,+end: U32,+a: U32,+b: U32,+count: U32) -> String & Maybe<&2,SpanWindow>:
  match split:
    case True{}: (emit(id,Some{SpanWindow{a,b,count}}),Some{SpanWindow{start,end,1}})
    case False{}: ("",Some{SpanWindow{a,end,(count + 1 : U32)}})

def covered(inside: Bool,+id: String,+start: U32,+end: U32,+a: U32,+b: U32,+count: U32) -> String & Maybe<&2,SpanWindow>:
  match inside:
    case True{}: ("",Some{SpanWindow{a,b,count}})
    case False{}: extending(U32.is_ge(count,8) || U32.is_gt((end - a : U32),4096),id,start,end,a,b,count)

def small(+id: String,+start: U32,+end: U32,current: Maybe<&2,SpanWindow>) -> String & Maybe<&2,SpanWindow>:
  match current:
    case None{}: ("",Some{SpanWindow{start,end,1}})
    case Some{SpanWindow{a,+b,count}}: covered(U32.is_le(end,b),id,start,end,a,b,count)

def add(large: Bool,+id: String,+start: U32,+end: U32,current: Maybe<&2,SpanWindow>) -> String & Maybe<&2,SpanWindow>:
  match large:
    case True{}: (emit(id,current) ++ W.row(["OVERSIZE",id,U32.show(start),U32.show(end)]),None{})
    case False{}: small(id,start,end,current)

def continued(result: String & Maybe<&2,SpanWindow>,+start: U32,next: U32 -> Maybe<&2,SpanWindow> -> String) -> String:
  match result:
    case (output,window): output ++ next(start,window)

# Recursion is passed explicitly so parsing and order failures terminate the row.
def accepted(valid: Bool,+id: String,+start: U32,+end: U32,current: Maybe<&2,SpanWindow>,next: U32 -> Maybe<&2,SpanWindow> -> String) -> String:
  match valid:
    case False{}: W.row(["ERROR",id,"invalid or unordered source span"])
    case True{}: continued(add(U32.is_gt((end - start : U32),4096),id,start,end,current),start,next)

def decoded(start: Maybe<&2,U32>,end: Maybe<&2,U32>,+last: U32,+id: String,current: Maybe<&2,SpanWindow>,next: U32 -> Maybe<&2,SpanWindow> -> String) -> String:
  match start end:
    case Some{+a} Some{+b}: accepted(U32.is_le(a,b) && U32.is_ge(a,last),id,a,b,current,next)
    case _ _: W.row(["ERROR",id,"invalid source span number"])

def spans(fields: List<&2,String>,+id: String,+last: U32,current: Maybe<&2,SpanWindow>) -> String:
  match fields:
    case Nil{}: emit(id,current)
    case a <> b <> rest: decoded(U32.read(a),U32.read(b),last,id,current,n => w => spans(rest,id,n,w))
    case _: W.row(["ERROR",id,"unpaired source span"])

def planned(row: W.Record) -> String:
  match row:
    case W.Row{+id <> fields}: spans(fields,id,0,None{})
    case _: "ERROR\tmalformed source-window row\n"

def process(rows: List<&2,String>) -> String:
  match rows:
    case Nil{}: ""
    case "" <> tail: process(tail)
    case head <> tail: planned(W.parse(head)) ++ process(tail)
