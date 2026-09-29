import Base
import ./QueryValue.bend as C
import ./Equality.bend as E

type Definition is Data:
  Definition{id: String,header: List<&2,String>,deps: List<&2,String>,unknown: Bool}

type Plan is Data:
  Plan{reverse: Map<&2,List<&2,String>>,local: List<&2,String>,dirty: Map<&2,Bool>,complete: Bool}

type Closure is Data:
  Closure{dirty: Map<&2,Bool>,complete: Bool}

def flag(pair: Map<&2,Bool> & Bool) -> Bool:
  match pair:
    case (_,value): value

def edges(pair: Map<&2,List<&2,String>> & List<&2,String>) -> List<&2,String>:
  match pair:
    case (_,value): value

def entry(pair: Map<&2,Maybe<&2,C.Entry>> & Maybe<&2,C.Entry>) -> Maybe<&2,C.Entry>:
  match pair:
    case (_,value): value

def present(definitions: List<&2,Definition>,out: Map<&2,Bool>) -> Map<&2,Bool>:
  match definitions:
    case Nil{}: out
    case Definition{id,_,_,_} <> t: present(t,Map.set(&2,Bool,out,id,True{}))

def linked(id: String,old: Map<&2,List<&2,String>> & List<&2,String>,dependency: String) -> Map<&2,List<&2,String>>:
  match old:
    case (index,items): Map.set(&2,List<&2,String>,index,dependency,id <> items)

def dependencies(ids: List<&2,String>,+id: String,index: Map<&2,List<&2,String>>) -> Map<&2,List<&2,String>>:
  match ids:
    case Nil{}: index
    case +h <> t: dependencies(t,id,linked(id,Map.get(List<&2,String>,Nil{},index,h),h))

def reverse(definitions: List<&2,Definition>,index: Map<&2,List<&2,String>>) -> Map<&2,List<&2,String>>:
  match definitions:
    case Nil{}: index
    case Definition{id,_,deps,_} <> t: reverse(t,dependencies(deps,id,index))

def all_present(ids: List<&2,String>,+current: Map<&2,Bool>) -> Bool:
  match ids:
    case Nil{}: True{}
    case h <> t: flag(Map.get(Bool,False{},current,h)) && all_present(t,current)

def joined(name: String,rest: Maybe<&2,List<&2,String>>) -> Maybe<&2,List<&2,String>>:
  match rest:
    case Some{tail}: Some{name <> tail}
    case None{}: None{}

def dependency_names(args: List<&2,String>) -> Maybe<&2,List<&2,String>>:
  match args:
    case Nil{}: Some{Nil{}}
    case name <> value <> tail: joined(name,dependency_names(tail))
    case _: None{}

def appended(header: List<&2,String>,rest: Maybe<&2,List<&2,String>>) -> Maybe<&2,List<&2,String>>:
  match rest:
    case Some{tail}: Some{List.append(&2,String,header,tail)}
    case None{}: None{}

def previous(args: List<&2,String>) -> Maybe<&2,List<&2,String>>:
  match args:
    case id <> scope <> config <> producer <> op <> payload <> deps: appended([id,scope,config,producer,op,payload],dependency_names(deps))
    case _: None{}

def matches(current: List<&2,String>,old: Maybe<&2,List<&2,String>>) -> Bool:
  match old:
    case Some{before}: E.strings_same(current,before)
    case None{}: False{}

def unchanged(header: List<&2,String>,deps: List<&2,String>,old: Maybe<&2,C.Entry>) -> Bool:
  match old:
    case Some{C.Entry{args,_}}: matches(List.append(&2,String,header,deps),previous(args))
    case None{}: False{}

def seed(changed: Bool,id: String,rest: List<&2,String>) -> List<&2,String>:
  match changed:
    case True{}: id <> rest
    case False{}: rest

def local(definitions: List<&2,Definition>,+current: Map<&2,Bool>,+cache: Map<&2,Maybe<&2,C.Entry>>) -> List<&2,String>:
  match definitions:
    case Nil{}: Nil{}
    case Definition{+id,header,+deps,unknown} <> tail: seed(unknown || Bool.not(all_present(deps,current)) || Bool.not(unchanged(header,deps,entry(Map.get(Maybe<&2,C.Entry>,None{},cache,id)))),id,local(tail,current,cache))

def visit(seen: Bool,+id: String,tail: List<&2,String>,index: Map<&2,List<&2,String>>,dirty: Map<&2,Bool>,next: List<&2,String> -> Map<&2,Bool> -> Closure) -> Closure:
  match seen:
    case True{}: next(tail,dirty)
    case False{}: next(List.append(&2,String,edges(Map.get(List<&2,String>,Nil{},index,id)),tail),Map.set(&2,Bool,dirty,id,True{}))

def queued(queue: List<&2,String>,+index: Map<&2,List<&2,String>>,+dirty: Map<&2,Bool>,next: List<&2,String> -> Map<&2,Bool> -> Closure) -> Closure:
  match queue:
    case Nil{}: Closure{dirty,True{}}
    case +h <> t: visit(flag(Map.get(Bool,False{},dirty,h)),h,t,index,dirty,next)

def exhausted(queue: List<&2,String>,dirty: Map<&2,Bool>) -> Closure:
  match queue:
    case Nil{}: Closure{dirty,True{}}
    case _: Closure{dirty,False{}}

def closure(fuel: Nat) -> List<&2,String> -> Map<&2,List<&2,String>> -> Map<&2,Bool> -> Closure:
  match fuel:
    case 0n: queue => index => dirty => exhausted(queue,dirty)
    case 1n+p: queue => +index => dirty => queued(queue,index,dirty,q => d => closure(p,q,index,d))

def edge_count(ids: List<&2,String>) -> Nat:
  match ids:
    case Nil{}: 0n
    case _ <> t: 1n+edge_count(t)

def work_bound(definitions: List<&2,Definition>) -> Nat:
  match definitions:
    case Nil{}: 1n
    case Definition{_,_,deps,_} <> t: Nat.add(1n+edge_count(deps),work_bound(t))

def closed(result: Closure,index: Map<&2,List<&2,String>>,seeds: List<&2,String>,current: Map<&2,Bool>) -> Plan:
  match result:
    case Closure{dirty,True{}}: Plan{index,seeds,dirty,True{}}
    case Closure{_,False{}}: Plan{index,seeds,current,False{}}

def indexed(+index: Map<&2,List<&2,String>>,+seeds: List<&2,String>,fuel: Nat,current: Map<&2,Bool>) -> Plan:
  closed(closure(fuel,seeds,index,MTip{}),index,seeds,current)

def plan(+definitions: List<&2,Definition>,cache: Map<&2,Maybe<&2,C.Entry>>) -> Plan:
  indexed(reverse(definitions,MTip{}),local(definitions,present(definitions,MTip{}),cache),work_bound(definitions),present(definitions,MTip{}))

def keep(dirty: Bool,id: String,old: Maybe<&2,C.Entry>,out: Map<&2,Maybe<&2,C.Entry>>) -> Map<&2,Maybe<&2,C.Entry>>:
  match dirty:
    case True{}: out
    case False{}: Map.set(&2,Maybe<&2,C.Entry>,out,id,old)

def retained(definitions: List<&2,Definition>,+dirty: Map<&2,Bool>,+cache: Map<&2,Maybe<&2,C.Entry>>,out: Map<&2,Maybe<&2,C.Entry>>) -> Map<&2,Maybe<&2,C.Entry>>:
  match definitions:
    case Nil{}: out
    case Definition{+id,_,_,_} <> tail: retained(tail,dirty,cache,keep(flag(Map.get(Bool,False{},dirty,id)),id,entry(Map.get(Maybe<&2,C.Entry>,None{},cache,id)),out))
