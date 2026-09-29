import Base
import ./Wire.bend as W

type Layout is Data:
  Layout{tag: String,names: List<&2,String>}

def layouts() -> List<&2,Layout>:
  [Layout{"CHOICE",["supported","contradicted","insufficient"]},Layout{"RELEVANCE",["relevant","irrelevant","insufficient"]},Layout{"CONFLICT",["explicit-conflict","no-explicit-conflict","uncertain"]},Layout{"SUPPORT",["complete-support","missing-support","uncertain"]}]

def member(names: List<&2,String>,+name: String) -> Bool:
  match names:
    case Nil{}: False{}
    case h <> t: String.eq(h,name) || member(t,name)

def subset(names: List<&2,String>,+allowed: List<&2,String>) -> Bool:
  match names:
    case Nil{}: True{}
    case h <> t: member(allowed,h) && subset(t,allowed)

def distinct(duplicate: Bool,name: String,names: List<&2,String>) -> Maybe<&2,List<&2,String>>:
  match duplicate:
    case True{}: None{}
    case False{}: Some{name <> names}

def criterion(ok: Bool,+name: String,rest: Maybe<&2,List<&2,String>>) -> Maybe<&2,List<&2,String>>:
  match ok rest:
    case True{} Some{+names}: distinct(member(names,name),name,names)
    case _ _: None{}

def criteria(fields: List<&2,String>) -> Maybe<&2,List<&2,String>>:
  match fields:
    case Nil{}: Some{Nil{}}
    case +name <> description <> tail: criterion(Bool.not(String.is_empty(name)) && Bool.not(String.is_empty(description)),name,criteria(tail))
    case _: None{}

def chosen(same: Bool,id: String,tag: String,names: List<&2,String>,otherwise: Unit -> String) -> String:
  match same:
    case True{}: W.row("CHOICE_LAYOUT" <> id <> tag <> names)
    case False{}: otherwise(Unit{})

def select(layouts: List<&2,Layout>,+id: String,+names: List<&2,String>) -> String:
  match layouts:
    case Nil{}: W.row(["ERROR",id,"unsupported Choice option set"])
    case Layout{tag,+allowed} <> tail: chosen(Nat.is_eq(W.length(names),W.length(allowed)) && subset(names,allowed),id,tag,allowed,u => select(tail,id,names))

def named(value: Maybe<&2,List<&2,String>>,id: String) -> String:
  match value:
    case Some{names}: select(layouts(),id,names)
    case None{}: W.row(["ERROR",id,"Choice descriptions and names must be nonempty text"])

def planned(row: W.Record) -> String:
  match row:
    case W.Row{id <> fields}: named(criteria(fields),id)
    case _: "ERROR\tmalformed Choice plan row\n"

def process(rows: List<&2,String>) -> String:
  match rows:
    case Nil{}: ""
    case "" <> tail: process(tail)
    case head <> tail: planned(W.parse(head)) ++ process(tail)
