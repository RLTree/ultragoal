import Base
import ./Json.bend as Json
import ./Decimal.bend as D

def text(value: Json.Json) -> String:
  match value:
    case Json.Text{s}: s
    case _: ""

def nonempty(value: Json.Json) -> Bool:
  Bool.not(String.eq(text(value),""))

def role_name(value: String) -> Bool:
  match value:
    case "implementation": True{}
    case "caller-expectation": True{}
    case "asserted-test": True{}
    case "executed-observation": True{}
    case "documentation": True{}
    case "prior-finding": True{}
    case "proposal": True{}
    case "retrieval-metadata": True{}
    case "unclassified-source": True{}
    case _: False{}

def role(value: Json.Json) -> Bool:
  role_name(text(value))

def strings(values: List<&2,Json.Json>,roles: Bool) -> Bool:
  match values roles:
    case Nil{} _: True{}
    case h <> t True{}: role(h) && strings(t,True{})
    case h <> t False{}: nonempty(h) && strings(t,False{})

def array(value: Json.Json,roles: Bool,required: Bool) -> Bool:
  match value:
    case Json.Array{Nil{}}: Bool.not(required)
    case Json.Array{xs}: strings(xs,roles)
    case _: False{}

def clause(+value: Json.Json) -> Bool:
  nonempty(D.get(value,"id")) && nonempty(D.get(value,"statement")) && array(D.get(value,"required_roles"),True{},True{})

def evidence(+value: Json.Json) -> Bool:
  nonempty(D.get(value,"id")) && role(D.get(value,"role")) && nonempty(D.get(value,"content")) && nonempty(D.get(value,"provenance")) && array(D.get(value,"clause_ids"),False{},False{})

def entries(values: List<&2,Json.Json>,clauses: Bool,left: Nat) -> Bool:
  match values clauses left:
    case Nil{} _ _: True{}
    case _ _ 0n: False{}
    case h <> t True{} 1n+p: clause(h) && entries(t,True{},p)
    case h <> t False{} 1n+p: evidence(h) && entries(t,False{},p)

def collection(value: Json.Json,clauses: Bool) -> Bool:
  match value clauses:
    case Json.Array{Nil{}} True{}: False{}
    case Json.Array{xs} c: entries(xs,c,64n)
    case _ _: False{}


def member(names: List<&2,String>,+name: String) -> Bool:
  match names:
    case Nil{}: False{}
    case h <> t: String.eq(h,name) || member(t,name)

def known_fields(fields: List<&2,Sigma<&2,&2,String,_ => Json.Json>>,+allowed: List<&2,String>) -> Bool:
  match fields:
    case Nil{}: True{}
    case (key,_) <> t: member(allowed,key) && known_fields(t,allowed)

def only(value: Json.Json,allowed: List<&2,String>) -> Bool:
  match value:
    case Json.Object{fields}: known_fields(fields,allowed)
    case _: False{}

def list(value: Json.Json) -> List<&2,Json.Json>:
  match value:
    case Json.Array{xs}: xs
    case _: Nil{}

def has_id(entries: List<&2,Json.Json>,+id: String) -> Bool:
  match entries:
    case Nil{}: False{}
    case h <> t: String.eq(text(D.get(h,"id")),id) || has_id(t,id)

def unique_ids(entries: List<&2,Json.Json>) -> Bool:
  match entries:
    case Nil{}: True{}
    case +h <> +t: nonempty(D.get(h,"id")) && Bool.not(has_id(t,text(D.get(h,"id")))) && unique_ids(t)

def linked(ids: List<&2,Json.Json>,+targets: List<&2,Json.Json>) -> Bool:
  match ids:
    case Nil{}: True{}
    case h <> t: has_id(targets,text(h)) && linked(t,targets)

def links(value: Json.Json,targets: List<&2,Json.Json>,required: Bool) -> Bool:
  match value:
    case Json.Array{Nil{}}: Bool.not(required)
    case Json.Array{+xs}: strings(xs,False{}) && linked(xs,targets)
    case _: False{}

def material_kind(role: String,+kind: String) -> Bool:
  match role:
    case "implementation": member(["source-code","patch"],kind)
    case "caller-expectation": member(["source-code","interface-contract"],kind)
    case "asserted-test": member(["test-source","patch"],kind)
    case "executed-observation": String.eq(kind,"execution-report")
    case "documentation": String.eq(kind,"document")
    case "prior-finding": String.eq(kind,"finding")
    case "proposal": String.eq(kind,"action-proposal")
    case "retrieval-metadata": member(["dependency-inventory","selection-report"],kind)
    case "unclassified-source": String.eq(kind,"unclassified")
    case _: False{}

def question_kind(family: String,+kind: String) -> Bool:
  match family:
    case "alignment": member(["behavior-description","reported-outcome"],kind)
    case "verification": member(["verification-design","reported-verification"],kind)
    case "consistency": String.eq(kind,"cross-surface-comparison")
    case "recovery-proposal": String.eq(kind,"proposed-action")
    case "recovery-state": String.eq(kind,"reported-recovery")
    case "investigation-reuse": member(["finding-usefulness","finding-applicability"],kind)
    case _: False{}

def material(+value: Json.Json,clauses: List<&2,Json.Json>) -> Bool:
  only(value,["id","role","material_type","version","content","provenance","clause_ids"]) && evidence(value) && material_kind(text(D.get(value,"role")),text(D.get(value,"material_type"))) && nonempty(D.get(value,"version")) && links(D.get(value,"clause_ids"),clauses,False{})

def materials(entries: List<&2,Json.Json>,+clauses: List<&2,Json.Json>,left: Nat) -> Bool:
  match entries left:
    case Nil{} _: True{}
    case _ 0n: False{}
    case h <> t 1n+p: material(h,clauses) && materials(t,clauses,p)

def condition(+value: Json.Json,evidence: List<&2,Json.Json>) -> Bool:
  only(value,["id","statement","evidence_ids"]) && nonempty(D.get(value,"id")) && nonempty(D.get(value,"statement")) && links(D.get(value,"evidence_ids"),evidence,True{})

def conditions(entries: List<&2,Json.Json>,+evidence: List<&2,Json.Json>,left: Nat) -> Bool:
  match entries left:
    case Nil{} _: True{}
    case _ 0n: False{}
    case h <> t 1n+p: condition(h,evidence) && conditions(t,evidence,p)

def condition_array(value: Json.Json,+evidence: List<&2,Json.Json>) -> Bool:
  match value:
    case Json.Array{+xs}: unique_ids(xs) && conditions(xs,evidence,32n)
    case _: False{}

def inventory_reference(entries: List<&2,Json.Json>,+id: String) -> Bool:
  match entries:
    case Nil{}: False{}
    case +h <> t: (String.eq(text(D.get(h,"id")),id) && String.eq(text(D.get(h,"material_type")),"dependency-inventory")) || inventory_reference(t,id)

def inventory_references(ids: List<&2,Json.Json>,+evidence: List<&2,Json.Json>) -> Bool:
  match ids:
    case Nil{}: False{}
    case h <> t: inventory_reference(evidence,text(h)) || inventory_references(t,evidence)

def inventory_conditions(conditions: List<&2,Json.Json>,+evidence: List<&2,Json.Json>) -> Bool:
  match conditions:
    case Nil{}: False{}
    case h <> t: inventory_references(list(D.get(h,"evidence_ids")),evidence) || inventory_conditions(t,evidence)

def declared_world(world: String,conditions: Json.Json,evidence: List<&2,Json.Json>) -> Bool:
  match world:
    case "provided-materials": True{}
    case "unknown": True{}
    case "explicit-dependencies": inventory_conditions(list(conditions),evidence)
    case _: False{}

def contract(+value: Json.Json,family: String,+evidence: List<&2,Json.Json>) -> Bool:
  only(value,["question","closed_world","conditions"]) && question_kind(family,text(D.get(value,"question"))) && condition_array(D.get(value,"conditions"),evidence) && declared_world(text(D.get(value,"closed_world")),D.get(value,"conditions"),evidence)

def clauses_v3(entries: List<&2,Json.Json>) -> Bool:
  match entries:
    case Nil{}: True{}
    case +h <> t: only(h,["id","statement","required_roles"]) && clause(h) && clauses_v3(t)

def version_three(+value: Json.Json,family: String) -> Bool:
  only(value,["schema","target","scope","material_contract","clauses","evidence"]) && unique_ids(list(D.get(value,"clauses"))) && unique_ids(list(D.get(value,"evidence"))) && clauses_v3(list(D.get(value,"clauses"))) && materials(list(D.get(value,"evidence")),list(D.get(value,"clauses")),64n) && contract(D.get(value,"material_contract"),family,list(D.get(value,"evidence")))

def version(schema: String,value: Json.Json,family: String) -> Bool:
  match schema:
    case "ultragoal-semantic-context/2": True{}
    case "ultragoal-semantic-context/3": version_three(value,family)
    case _: False{}

def context(+value: Json.Json,+family: String,known: Bool) -> Bool:
  known && String.eq(text(D.get(value,"target")),family) && nonempty(D.get(value,"scope")) && collection(D.get(value,"clauses"),True{}) && collection(D.get(value,"evidence"),False{}) && version(text(D.get(value,"schema")),value,family)

def parsed(value: Maybe<&2,Json.Json>,family: String,known: Bool) -> Bool:
  match value:
    case Some{x}: context(x,family,known)
    case None{}: False{}
