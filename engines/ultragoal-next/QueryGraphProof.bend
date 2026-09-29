import Base
import ./QueryGraphSpec.bend as Spec
import ./QueryGraph.bend as G
import ./QueryValue.bend as C
import ./QueryCompute.bend as Q
import ./Invalidation.bend as I
def sound(-arg: List<&2,String>,+result: C.Value<arg>) -> {C.value(arg,result) == Q.compute(arg) : String}:
  match result:
    case C.Value{v,proof,_}: proof

def correct(+arg: List<&2,String>,+old: Maybe<&2,C.Entry>) -> {C.value(arg,C.choose(arg,old)) == Q.compute(arg) : String}:
  sound(arg,C.choose(arg,old))
def step_correct(+node_id: String,+arg_key: Maybe<&2,List<&2,String>>,+before: Map<&2,Maybe<&2,String>>,+cache: Map<&2,Maybe<&2,C.Entry>>,+work: Map<&2,Bool>) -> {G.values(G.step(node_id,arg_key,before,cache,work)) == Spec.fresh_step(node_id,arg_key,before) : Map<&2,Maybe<&2,String>>}:
  match arg_key:
    case None{}: {==}
    case Some{+arg}: Equal.cong(String,Map<&2,Maybe<&2,String>>,v => Map.set(&2,Maybe<&2,String>,before,node_id,Some{v}),C.value(arg,C.choose(arg,G.cached(Map.get(Maybe<&2,C.Entry>,None{},cache,node_id)))),Q.compute(arg),correct(arg,G.cached(Map.get(Maybe<&2,C.Entry>,None{},cache,node_id))))

def advance_correct(+node: G.Node,+state: G.State) -> {G.values(G.advance(node,state)) == Spec.fresh_step(G.id(node),G.key(node,G.values(state)),G.values(state)) : Map<&2,Maybe<&2,String>>}:
  match state:
    case G.State{+before,cache,work}: step_correct(G.id(node),G.key(node,before),before,cache,work)

def equivalent(+nodes: List<&2,G.Node>,+state: G.State) -> {G.values(G.incremental(nodes,state)) == Spec.full(nodes,G.values(state)) : Map<&2,Maybe<&2,String>>}:
  match nodes:
    case Nil{}: {==}
    case +h <> +t:
      Equal.trans(Map<&2,Maybe<&2,String>>,G.values(G.incremental(t,G.advance(h,state))),Spec.full(t,G.values(G.advance(h,state))),Spec.full(t,Spec.fresh_step(G.id(h),G.key(h,G.values(state)),G.values(state))),equivalent(t,G.advance(h,state)),Equal.cong(Map<&2,Maybe<&2,String>>,Map<&2,Maybe<&2,String>>,v => Spec.full(t,v),G.values(G.advance(h,state)),Spec.fresh_step(G.id(h),G.key(h,G.values(state)),G.values(state)),advance_correct(h,state)))

def rebuild_correct(+nodes: List<&2,G.Node>,+defs: List<&2,I.Definition>,+cache: Map<&2,Maybe<&2,C.Entry>>,+plan: I.Plan) -> {G.values(G.work_state(G.rebuild(nodes,defs,cache,plan))) == Spec.full(nodes,MTip{}) : Map<&2,Maybe<&2,String>>}:
  match plan:
    case I.Plan{_,_,dirty,_}: equivalent(nodes,G.State{MTip{},I.retained(defs,dirty,cache,MTip{}),MTip{}})

def revision_correct(+nodes: List<&2,G.Node>,+old: G.State) -> {G.values(G.revision(nodes,old)) == Spec.full(nodes,MTip{}) : Map<&2,Maybe<&2,String>>}:
  match old:
    case G.State{_,+cache,_}: rebuild_correct(nodes,G.definitions(nodes),cache,I.plan(G.definitions(nodes),cache))

def transition_correct(previous: List<&2,G.Node>,+next: List<&2,G.Node>,old: G.State) -> {G.values(G.revision(next,G.revision(previous,old))) == Spec.full(next,MTip{}) : Map<&2,Maybe<&2,String>>}:
  revision_correct(next,G.revision(previous,old))

def run_correct(+nodes: List<&2,G.Node>,+old: G.State) -> {G.values(G.run(nodes,old)) == Spec.full(nodes,MTip{}) : Map<&2,Maybe<&2,String>>}:
  revision_correct(nodes,old)
