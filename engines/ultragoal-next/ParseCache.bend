import Base
import ./GroupedParse.bend as G
import ./ParsedValue.bend as P
import ./Equality.bend as E
import ./Facts.bend as F
import ./Digest.bend as D

type Entry is Data:
  Entry{args: List<&2,String>,value: P.Result<args>}

type State is Data:
  State{entries: Map<&2,Maybe<&2,Entry>>,bytes: U32,count: U32}

type Chosen<-Args: List<&2,String>> is Data:
  Chosen{value: P.Result<Args>,reused: Bool}

type Work is Data:
  Work{rows: List<&2,G.Bound>,state: State,valid: Bool}

def fresh(+args: List<&2,String>) -> Chosen<args>:
  Chosen{P.fresh_result(args),False{}}

def compared(+args: List<&2,String>,old: List<&2,String>,value: P.Result<old>,same: Maybe<&2,{old == args : List<&2,String>}>) -> Chosen<args>:
  match same:
    case None{}: fresh(args)
    case Some{proof}: Chosen{P.reuse(old,args,value,proof),True{}}

def choose(+args: List<&2,String>,entry: Maybe<&2,Entry>) -> Chosen<args>:
  match entry:
    case None{}: fresh(args)
    case Some{Entry{+old,value}}: compared(args,old,value,E.decide_strings(old,args))

def lookup(pair: Map<&2,Maybe<&2,Entry>> & Maybe<&2,Entry>) -> Maybe<&2,Entry>:
  match pair:
    case (_,value): value

def cache_first(reused: Bool,first: Bool) -> Bool:
  match reused:
    case True{}: False{}
    case False{}: first

def chosen_value(-args: List<&2,String>,chosen: Chosen<args>) -> String:
  match chosen:
    case Chosen{value,_}: P.value(args,value)

def bind(-args: List<&2,String>,value: P.Result<args>,alias: G.Alias<args>,key: String,reused: Bool) -> G.Bound:
  match alias:
    case G.Alias{+req,same,first}: G.bind_request(req,G.alias_value(args,value,G.request_args(req),same),cache_first(reused,first),key)

def aliases(-args: List<&2,String>,+value: P.Result<args>,rows: List<&2,G.Alias<args>>,+key: String,+reused: Bool) -> List<&2,G.Bound>:
  match rows:
    case Nil{}: Nil{}
    case h <> t: bind(args,value,h,key,reused) <> aliases(args,value,t,key,reused)

def size(args: List<&2,String>) -> U32:
  match args:
    case Nil{}: 0
    case h <> t: (F.bytes(h) + size(t) : U32)

# Retention is optional. Every result remains constructed from current arguments.
# This first prototype keeps only the current bounded request's parse groups.
def stored(allowed: Bool,key: String,entry: Entry,bytes: U32,state: State) -> State:
  match allowed state:
    case True{} State{entries,b,n}: State{Map.set(&2,Maybe<&2,Entry>,entries,key,Some{entry}),(b + bytes : U32),(n + 1 : U32)}
    case False{} state: state

def retain(key: String,+entry: Entry,+bytes: U32,state: State) -> State:
  match state:
    case State{entries,+b,+n}: stored(U32.is_lt(n,512) && U32.is_le(bytes,(4194304 - b : U32)),key,entry,bytes,State{entries,b,n})

type Computed is Data:
  Computed{key: String,args: List<&2,String>,value: P.Result<args>,rows: List<&2,G.Bound>,bytes: U32}

def selected(+args: List<&2,String>,result: Chosen<args>,+key: String,rows: List<&2,G.Alias<args>>) -> Computed:
  match result:
    case Chosen{+value,reused}: Computed{key,args,value,aliases(args,value,rows,D.identity(key),reused),(size(args) + F.bytes(P.value(args,value)) + F.bytes(key) + 128 : U32)}

def group(group: G.Group,+old: Map<&2,Maybe<&2,Entry>>) -> Computed:
  match group:
    case G.Group{+key,+args,rows,_}: selected(args,choose(args,lookup(Map.get(Maybe<&2,Entry>,None{},old,key))),key,rows)

def serial(jobs: List<&2,G.Group>,+old: Map<&2,Maybe<&2,Entry>>) -> List<&2,Computed>:
  match jobs:
    case Nil{}: Nil{}
    case h <> t: group(h,old) <> serial(t,old)

def branches(left: List<&2,G.Group> -> List<&2,Computed>,right: List<&2,G.Group> -> List<&2,Computed>,parts: List<&2,G.Group> & List<&2,G.Group>) -> List<&2,Computed>:
  match parts:
    case (xs,ys):
      a b = left(xs) right(ys)
      List.append(&2,Computed,a,b)

def large(ok: Bool,+jobs: List<&2,G.Group>,old: Map<&2,Maybe<&2,Entry>>,left: List<&2,G.Group> -> List<&2,Computed>,right: List<&2,G.Group> -> List<&2,Computed>) -> List<&2,Computed>:
  match ok:
    case False{}: serial(jobs,old)
    case True{}: branches(left,right,G.split(jobs,G.Buckets{Nil{},Nil{},0,0}))

def parallel(fuel: Nat,+jobs: List<&2,G.Group>,+old: Map<&2,Maybe<&2,Entry>>) -> List<&2,Computed>:
  match fuel:
    case 0n: serial(jobs,old)
    case 1n+ +p: large(Nat.is_gt(G.count(jobs),4n) && U32.is_ge(G.total(jobs),65536),jobs,old,xs => parallel(p,xs,old),xs => parallel(p,xs,old))

def joined(rows: List<&2,G.Bound>,tail: Work) -> Work:
  match tail:
    case Work{tail,state,valid}: Work{List.append(&2,G.Bound,rows,tail),state,valid}

def consumed(value: Computed,state: State,next: State -> Work) -> Work:
  match value:
    case Computed{key,args,value,rows,bytes}: joined(rows,next(retain(key,Entry{args,value},bytes,state)))

def fold(values: List<&2,Computed>,state: State) -> Work:
  match values:
    case Nil{}: Work{Nil{},state,True{}}
    case h <> t: consumed(h,state,s => fold(t,s))

def planned(plan: G.Plan,old: State) -> Work:
  match plan old:
    case G.Plan{_,_,False{}} _: Work{Nil{},State{MTip{},0,0},False{}}
    case G.Plan{items,keys,True{}} State{old,_,_}: fold(parallel(16n,G.jobs(keys,items),old),State{MTip{},0,0})

def run(rows: List<&2,List<&2,String>>,+compact: Bool,old: State) -> Work:
  planned(G.collect(rows,compact,G.Plan{MTip{},Nil{},True{}}),old)
