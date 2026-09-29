import Base
import ./QueryGraph.bend as G
import ./QueryCompute.bend as Q
def fresh_value(arg_key: Maybe<&2,List<&2,String>>) -> Maybe<&2,String>:
  match arg_key:
    case None{}: None{}
    case Some{arg}: Some{Q.compute(arg)}

def fresh_step(node_id: String,arg_key: Maybe<&2,List<&2,String>>,values: Map<&2,Maybe<&2,String>>) -> Map<&2,Maybe<&2,String>>:
  Map.set(&2,Maybe<&2,String>,values,node_id,fresh_value(arg_key))

def full(nodes: List<&2,G.Node>,+before: Map<&2,Maybe<&2,String>>) -> Map<&2,Maybe<&2,String>>:
  match nodes:
    case Nil{}: before
    case +h <> t: full(t,fresh_step(G.id(h),G.key(h,before),before))

