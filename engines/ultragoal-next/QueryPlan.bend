import Base
import ./QueryGraph.bend as G
import ./QueryCompute.bend as Q
import ./Predicate.bend as P
import ./Wire.bend as W
import ./Facts.bend as F
import ./Identity.bend as Identity

type Plan is Data:
  Plan{nodes: List<&2,G.Node>, seen: Map<&2,Bool>, count: Nat, bytes: U32, error: String}

def flag(result: Map<&2,Bool> & Bool) -> Bool:
  match result:
    case (_,value): value

def projection_id(+r: List<&2,String>) -> String:
  W.row(["projection",W.field(r,5n),Q.mode(W.field(r,7n))])

def query_id(+r: List<&2,String>) -> String:
  W.row(["predicate",W.field(r,2n),W.field(r,5n),W.field(r,6n),W.field(r,7n),W.field(r,8n)])

def source_node(+r: List<&2,String>,rows: List<&2,List<&2,String>>) -> G.Node:
  G.Node{projection_id(r),W.field(r,5n),"projection/1",Identity.rules(),"projection",Q.project(rows,W.field(r,5n),Q.mode(W.field(r,7n))),Nil{},False{}}

def query_node(+r: List<&2,String>) -> G.Node:
  G.Node{query_id(r),W.field(r,5n),W.row([W.field(r,2n),W.field(r,6n)]),Identity.rules(),"predicate",W.row([W.field(r,7n),W.field(r,8n)]),[projection_id(r)],False{}}

def cost(node: G.Node) -> U32:
  match node:
    case G.Node{id,scope,config,producer,op,payload,deps,_}: (F.bytes(id) + F.bytes(scope) + F.bytes(config) + F.bytes(producer) + F.bytes(op) + F.bytes(payload) + F.bytes(String.join(deps,"")) + 128 : U32)

def append_bounded(ok: Bool,node: G.Node,nodes: List<&2,G.Node>,seen: Map<&2,Bool>,count: Nat,bytes: U32) -> Plan:
  match ok:
    case False{}: Plan{Nil{},MTip{},0n,0,"query graph work/storage bound"}
    case True{}: Plan{node <> nodes,seen,count,bytes,""}

def add_new(exists: Bool,+node: G.Node,+size: U32,nodes: List<&2,G.Node>,seen: Map<&2,Bool>,+count: Nat,+bytes: U32) -> Plan:
  match exists:
    case True{}: Plan{nodes,seen,count,bytes,""}
    case False{}: append_bounded(Nat.is_lt(count,2048n) && U32.is_le(bytes,16777216) && U32.is_le(size,(16777216 - bytes : U32)),node,nodes,Map.set(&2,Bool,seen,G.id(node),True{}),1n+count,(bytes + size : U32))

def add(+node: G.Node,plan: Plan) -> Plan:
  match plan:
    case Plan{nodes,+seen,count,bytes,""}: add_new(flag(Map.get(Bool,False{},seen,G.id(node))),node,cost(node),nodes,seen,count,bytes)
    case plan: plan

# A projection shared by several predicates is built and charged once: each
# predicate's cached argument refers to the same retained projection value.
def fresh_source(exists: Bool,+r: List<&2,String>,+rows: List<&2,List<&2,String>>,plan: Plan) -> Plan:
  match exists:
    case True{}: plan
    case False{}: add(source_node(r,rows),plan)

def source_once(+r: List<&2,String>,+rows: List<&2,List<&2,String>>,plan: Plan) -> Plan:
  match plan:
    case Plan{nodes,+seen,count,bytes,""}: fresh_source(flag(Map.get(Bool,False{},seen,projection_id(r))),r,rows,Plan{nodes,seen,count,bytes,""})
    case plan: plan

def obligation(include: Bool,+r: List<&2,String>,+rows: List<&2,List<&2,String>>,plan: Plan) -> Plan:
  match include:
    case True{}: add(query_node(r),source_once(r,rows,plan))
    case False{}: plan

def collect(queries: List<&2,List<&2,String>>,+rows: List<&2,List<&2,String>>,plan: Plan) -> Plan:
  match queries:
    case Nil{}: plan
    case +r <> t: collect(t,rows,obligation(P.is_tag(r,"O"),r,rows,plan))

def ordered(plan: Plan) -> Plan:
  match plan:
    case Plan{nodes,seen,count,bytes,error}: Plan{List.reverse(&2,G.Node,nodes),seen,count,bytes,error}

def build(queries: List<&2,List<&2,String>>,rows: List<&2,List<&2,String>>) -> Plan:
  ordered(collect(queries,rows,Plan{Nil{},MTip{},0n,0,""}))
