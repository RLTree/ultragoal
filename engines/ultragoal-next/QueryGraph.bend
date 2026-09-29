import Base
import ./QueryValue.bend as C
import ./Invalidation.bend as I

type Node is Data:
  Node{id: String, scope: String, config: String, producer: String, op: String, payload: String, deps: List<&2,String>, unknown: Bool}

type State is Data:
  State{values: Map<&2,Maybe<&2,String>>, cache: Map<&2,Maybe<&2,C.Entry>>, work: Map<&2,Bool>}

def lookup(result: Map<&2,Maybe<&2,String>> & Maybe<&2,String>) -> Maybe<&2,String>:
  match result:
    case (_,value): value

def cached(result: Map<&2,Maybe<&2,C.Entry>> & Maybe<&2,C.Entry>) -> Maybe<&2,C.Entry>:
  match result:
    case (_,value): value

def id(node: Node) -> String:
  match node:
    case Node{id,_,_,_,_,_,_,_}: id

def joined(dependency: String,value: Maybe<&2,String>,rest: Maybe<&2,List<&2,String>>) -> Maybe<&2,List<&2,String>>:
  match value rest:
    case Some{v} Some{tail}: Some{dependency <> v <> tail}
    case _ _: None{}

def dependencies(ids: List<&2,String>,+values: Map<&2,Maybe<&2,String>>) -> Maybe<&2,List<&2,String>>:
  match ids:
    case Nil{}: Some{Nil{}}
    case +h <> t: joined(h,lookup(Map.get(Maybe<&2,String>,None{},values,h)),dependencies(t,values))

def input(unknown: Bool,header: List<&2,String>,deps: Maybe<&2,List<&2,String>>) -> Maybe<&2,List<&2,String>>:
  match unknown deps:
    case False{} Some{rows}: Some{List.append(&2,String,header,rows)}
    case _ _: None{}

def key(node: Node,values: Map<&2,Maybe<&2,String>>) -> Maybe<&2,List<&2,String>>:
  match node:
    case Node{id,scope,config,producer,op,payload,deps,unknown}: input(unknown,[id,scope,config,producer,op,payload],dependencies(deps,values))

def stored(+node_id: String,+arg: List<&2,String>,values: Map<&2,Maybe<&2,String>>,cache: Map<&2,Maybe<&2,C.Entry>>,work: Map<&2,Bool>,+result: C.Value<arg>) -> State:
  State{Map.set(&2,Maybe<&2,String>,values,node_id,Some{C.value(arg,result)}),Map.set(&2,Maybe<&2,C.Entry>,cache,node_id,Some{C.Entry{arg,result}}),Map.set(&2,Bool,work,node_id,C.reused(arg,result))}

def step(+node_id: String,key: Maybe<&2,List<&2,String>>,values: Map<&2,Maybe<&2,String>>,+cache: Map<&2,Maybe<&2,C.Entry>>,work: Map<&2,Bool>) -> State:
  match key:
    case None{}: State{Map.set(&2,Maybe<&2,String>,values,node_id,None{}),cache,work}
    case Some{+arg}: stored(node_id,arg,values,cache,work,C.choose(arg,cached(Map.get(Maybe<&2,C.Entry>,None{},cache,node_id))))

def values(state: State) -> Map<&2,Maybe<&2,String>>:
  match state:
    case State{values,_,_}: values

def advance(+node: Node,state: State) -> State:
  match state:
    case State{+before,cache,work}: step(id(node),key(node,before),before,cache,work)

def incremental(nodes: List<&2,Node>,state: State) -> State:
  match nodes:
    case Nil{}: state
    case h <> t: incremental(t,advance(h,state))

type RevisionWork is Data:
  RevisionWork{state: State,plan: I.Plan}

def definition(node: Node) -> I.Definition:
  match node:
    case Node{+id,scope,config,producer,op,payload,deps,unknown}: I.Definition{id,[id,scope,config,producer,op,payload],deps,unknown}

def definitions(nodes: List<&2,Node>) -> List<&2,I.Definition>:
  match nodes:
    case Nil{}: Nil{}
    case h <> t: definition(h) <> definitions(t)

def work_state(work: RevisionWork) -> State:
  match work:
    case RevisionWork{state,_}: state

# Invalidation only removes candidate cache entries. Actual reuse still requires
# a constructive equality check against the complete current evaluator argument.
def rebuild(nodes: List<&2,Node>,defs: List<&2,I.Definition>,cache: Map<&2,Maybe<&2,C.Entry>>,plan: I.Plan) -> RevisionWork:
  match plan:
    case I.Plan{index,local,+dirty,complete}: RevisionWork{incremental(nodes,State{MTip{},I.retained(defs,dirty,cache,MTip{}),MTip{}}),I.Plan{index,local,dirty,complete}}

def prepared(nodes: List<&2,Node>,+defs: List<&2,I.Definition>,+cache: Map<&2,Maybe<&2,C.Entry>>) -> RevisionWork:
  rebuild(nodes,defs,cache,I.plan(defs,cache))

def revision_work(+nodes: List<&2,Node>,old: State) -> RevisionWork:
  match old:
    case State{_,cache,_}: prepared(nodes,definitions(nodes),cache)

def revision(nodes: List<&2,Node>,old: State) -> State:
  work_state(revision_work(nodes,old))

# I.retained constructs the cache from current definitions only, before the
# ordered evaluation. Removed keys therefore need no second trimming pass.
def run_work(nodes: List<&2,Node>,old: State) -> RevisionWork:
  revision_work(nodes,old)

def run(nodes: List<&2,Node>,old: State) -> State:
  work_state(run_work(nodes,old))

def result(value: Maybe<&2,String>) -> String:
  match value:
    case Some{text}: text
    case None{}: "unknown"

def status(node_id: String,state: State) -> String:
  match state:
    case State{values,_,_}: result(lookup(Map.get(Maybe<&2,String>,None{},values,node_id)))

def work_flag(result: Map<&2,Bool> & Bool) -> Bool:
  match result:
    case (_,value): value

def reused(node_id: String,state: State) -> Bool:
  match state:
    case State{_,_,work}: work_flag(Map.get(Bool,False{},work,node_id))
