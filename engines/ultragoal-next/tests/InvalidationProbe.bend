import Base
import ../Invalidation.bend as I
import ../QueryValue.bend as C
import ../Wire.bend as W

def node(+id: String,payload: String,deps: List<&2,String>,unknown: Bool) -> I.Definition:
  I.Definition{id,[id,"*","cfg","producer","projection",payload],deps,unknown}

def base(a: String,e: String) -> List<&2,I.Definition>:
  [node("a",a,Nil{},False{}),node("b","b",["a"],False{}),node("c","c",Nil{},False{}),node("d","d",["b"],False{}),node("e",e,["f"],False{}),node("f","f",["e"],False{}),node("g","g",["missing"],False{}),node("h","h",Nil{},True{})]

def args(deps: List<&2,String>) -> List<&2,String>:
  match deps:
    case Nil{}: Nil{}
    case h <> t: h <> "old-value" <> args(t)

def stored(id: String,+arg: List<&2,String>,out: Map<&2,Maybe<&2,C.Entry>>) -> Map<&2,Maybe<&2,C.Entry>>:
  Map.set(&2,Maybe<&2,C.Entry>,out,id,Some{C.Entry{arg,C.fresh(arg)}})

def cache(definitions: List<&2,I.Definition>,out: Map<&2,Maybe<&2,C.Entry>>) -> Map<&2,Maybe<&2,C.Entry>>:
  match definitions:
    case Nil{}: out
    case I.Definition{id,header,deps,_} <> t: cache(t,stored(id,List.append(&2,String,header,args(deps)),out))

def included(yes: Bool,id: String,tail: List<&2,String>) -> List<&2,String>:
  match yes:
    case True{}: id <> tail
    case False{}: tail

def names(ids: List<&2,String>,+dirty: Map<&2,Bool>) -> List<&2,String>:
  match ids:
    case Nil{}: Nil{}
    case +h <> t: included(I.flag(Map.get(Bool,False{},dirty,h)),h,names(t,dirty))

def presence(value: Maybe<&2,C.Entry>) -> Bool:
  match value:
    case Some{_}: True{}
    case None{}: False{}

def retained_names(ids: List<&2,String>,+cache: Map<&2,Maybe<&2,C.Entry>>) -> List<&2,String>:
  match ids:
    case Nil{}: Nil{}
    case +h <> t: included(presence(I.entry(Map.get(Maybe<&2,C.Entry>,None{},cache,h))),h,retained_names(t,cache))

def report_state(+label: String,plan: I.Plan,+defs: List<&2,I.Definition>,old: Map<&2,Maybe<&2,C.Entry>>) -> String:
  match plan:
    case I.Plan{_,local,+dirty,complete}: W.row("LOCAL" <> label <> local) ++ W.row("DIRTY" <> label <> names(["a","b","c","d","e","f","g","h"],dirty)) ++ W.row("RETAINED" <> label <> retained_names(["a","b","c","d","e","f","g","h"],I.retained(defs,dirty,old,MTip{})))

def reported(+label: String,plan: I.Plan) -> String:
  match plan:
    case I.Plan{_,local,dirty,_}: W.row("LOCAL" <> label <> local) ++ W.row("DIRTY" <> label <> names(["a","b","c","d","e","f","g","h"],dirty))

def tail(definitions: List<&2,I.Definition>) -> List<&2,I.Definition>:
  match definitions:
    case Nil{}: Nil{}
    case _ <> t: t

def main() -> IO(Unit):
  IO.print(report_state("same",I.plan(base("old","e"),cache(base("old","e"),MTip{})),base("old","e"),cache(base("old","e"),MTip{})) ++ report_state("changed",I.plan(base("new","e"),cache(base("old","e"),MTip{})),base("new","e"),cache(base("old","e"),MTip{})) ++ report_state("cycle",I.plan(base("old","changed"),cache(base("old","e"),MTip{})),base("old","changed"),cache(base("old","e"),MTip{})) ++ report_state("deleted",I.plan(tail(base("old","e")),cache(base("old","e"),MTip{})),tail(base("old","e")),cache(base("old","e"),MTip{})) ++ reported("fuel-fallback",I.indexed(I.reverse(base("old","e"),MTip{}),["a"],0n,I.present(base("old","e"),MTip{}))))
