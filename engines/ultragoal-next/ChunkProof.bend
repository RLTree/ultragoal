import Base
import ./Predicate.bend as P
import ./Core.bend as C

def or_else_or(c: Bool,x: Bool) -> {P.or_else(c,u => x) == Bool.or(c,x) : Bool}:
  match c:
    case True{}: {==}
    case False{}: {==}

def or_assoc(a: Bool,b: Bool,c: Bool) -> {Bool.or(Bool.or(a,b),c) == Bool.or(a,Bool.or(b,c)) : Bool}:
  match a:
    case True{}: {==}
    case False{}: {==}

def and_assoc(a: Bool,b: Bool,c: Bool) -> {Bool.and(Bool.and(a,b),c) == Bool.and(a,Bool.and(b,c)) : Bool}:
  match a:
    case True{}: {==}
    case False{}: {==}

def partial_eq(m: Bool,m2: Bool,f: Bool,f2: Bool,s: Bool,s2: Bool,v: Bool,v2: Bool,em: {m == m2 : Bool},ef: {f == f2 : Bool},es: {s == s2 : Bool},ev: {v == v2 : Bool}) -> {P.Partial{m,f,s,v} == P.Partial{m2,f2,s2,v2} : P.Partial}:
  %em : {P.Partial{m,f,s,v} == P.Partial{_,f2,s2,v2} : P.Partial}
  %ef : {P.Partial{m,f,s,v} == P.Partial{m,_,s2,v2} : P.Partial}
  %es : {P.Partial{m,f,s,v} == P.Partial{m,f,_,v2} : P.Partial}
  %ev : {P.Partial{m,f,s,v} == P.Partial{m,f,s,_} : P.Partial}
  {==}

def merge_empty(p: P.Partial) -> {P.merge(P.empty_partial(),p) == p : P.Partial}:
  match p:
    case P.Partial{m,f,s,v}: {==}

def merge_assoc(p: P.Partial,q: P.Partial,r: P.Partial) -> {P.merge(P.merge(p,q),r) == P.merge(p,P.merge(q,r)) : P.Partial}:
  match p q r:
    case P.Partial{+m1,+f1,+s1,+v1} P.Partial{+m2,+f2,+s2,+v2} P.Partial{+m3,+f3,+s3,+v3}: partial_eq(Bool.or(Bool.or(m1,m2),m3),Bool.or(m1,Bool.or(m2,m3)),Bool.or(Bool.or(f1,f2),f3),Bool.or(f1,Bool.or(f2,f3)),Bool.and(Bool.and(s1,s2),s3),Bool.and(s1,Bool.and(s2,s3)),Bool.and(Bool.and(v1,v2),v3),Bool.and(v1,Bool.and(v2,v3)),or_assoc(m1,m2,m3),or_assoc(f1,f2,f3),and_assoc(s1,s2,s3),and_assoc(v1,v2,v3))

# Partials are a monoid homomorphism from row-list concatenation.
def partial_append(+rule: P.Rule,+scope: String,+arg: String,xs: List<&2,List<&2,String>>,+ys: List<&2,List<&2,String>>) -> {P.merge(P.partial(rule,scope,arg,xs),P.partial(rule,scope,arg,ys)) == P.partial(rule,scope,arg,List.append(&2,List<&2,String>,xs,ys)) : P.Partial}:
  match xs:
    case Nil{}: merge_empty(P.partial(rule,scope,arg,ys))
    case +h <> +t:
      %partial_append(rule,scope,arg,t,ys) : {P.merge(P.merge(P.row_partial(rule,scope,arg,h),P.partial(rule,scope,arg,t)),P.partial(rule,scope,arg,ys)) == P.merge(P.row_partial(rule,scope,arg,h),_) : P.Partial}
      merge_assoc(P.row_partial(rule,scope,arg,h),P.partial(rule,scope,arg,t),P.partial(rule,scope,arg,ys))

def partial_concat(+rule: P.Rule,+scope: String,+arg: String,parts: List<&2,List<&2,List<&2,String>>>) -> {P.combine(rule,scope,arg,parts) == P.partial(rule,scope,arg,P.concat(parts)) : P.Partial}:
  match parts:
    case Nil{}: {==}
    case +h <> +t:
      %partial_append(rule,scope,arg,h,P.concat(t)) : {P.merge(P.partial(rule,scope,arg,h),P.combine(rule,scope,arg,t)) == _ : P.Partial}
      %partial_concat(rule,scope,arg,t) : {P.merge(P.partial(rule,scope,arg,h),P.combine(rule,scope,arg,t)) == P.merge(P.partial(rule,scope,arg,h),_) : P.Partial}
      {==}

# Each content rule's unchunked evaluator is the finish of its row partial.
def contains_partial(+scope: String,+arg: String,rs: List<&2,List<&2,String>>) -> {P.Partial{False{},P.any_contains(rs,scope,arg),True{},True{}} == P.partial(P.Contains{},scope,arg,rs) : P.Partial}:
  match rs:
    case Nil{}: {==}
    case +h <> +t:
      %contains_partial(scope,arg,t) : {P.Partial{False{},P.or_else(P.contains_row(h,scope,arg),u => P.any_contains(t,scope,arg)),True{},True{}} == P.merge(P.Partial{False{},P.contains_row(h,scope,arg),True{},True{}},_) : P.Partial}
      %or_else_or(P.contains_row(h,scope,arg),P.any_contains(t,scope,arg)) : {P.Partial{False{},P.or_else(P.contains_row(h,scope,arg),u => P.any_contains(t,scope,arg)),True{},True{}} == P.Partial{False{},_,True{},True{}} : P.Partial}
      {==}

def absent_partial(+scope: String,+arg: String,rs: List<&2,List<&2,String>>) -> {P.Partial{False{},P.any_contains(rs,scope,arg),True{},True{}} == P.partial(P.Absent{},scope,arg,rs) : P.Partial}:
  match rs:
    case Nil{}: {==}
    case +h <> +t:
      %absent_partial(scope,arg,t) : {P.Partial{False{},P.or_else(P.contains_row(h,scope,arg),u => P.any_contains(t,scope,arg)),True{},True{}} == P.merge(P.Partial{False{},P.contains_row(h,scope,arg),True{},True{}},_) : P.Partial}
      %or_else_or(P.contains_row(h,scope,arg),P.any_contains(t,scope,arg)) : {P.Partial{False{},P.or_else(P.contains_row(h,scope,arg),u => P.any_contains(t,scope,arg)),True{},True{}} == P.Partial{False{},_,True{},True{}} : P.Partial}
      {==}

def fact_partial(+kind: String,+value: String,+g: String,+scope: String,+arg: String,rs: List<&2,List<&2,String>>) -> {P.Partial{False{},P.fact_exists(rs,scope,kind,value),P.supported(rs,scope,g),True{}} == P.partial(P.Fact{kind,value,g},scope,arg,rs) : P.Partial}:
  match rs:
    case Nil{}: {==}
    case +h <> +t:
      %fact_partial(kind,value,g,scope,arg,t) : {P.Partial{False{},Bool.or(P.fact_row(h,scope,kind,value),P.fact_exists(t,scope,kind,value)),Bool.and(P.file_supported(h,scope,g),P.supported(t,scope,g)),True{}} == P.merge(P.Partial{False{},P.fact_row(h,scope,kind,value),P.file_supported(h,scope,g),True{}},_) : P.Partial}
      {==}

def import_member_partial(+scope: String,+arg: String,rs: List<&2,List<&2,String>>) -> {P.Partial{False{},P.fact_exists(rs,scope,"structure-python-import-member",arg),P.member_supported(rs,scope),True{}} == P.partial(P.ImportMember{arg},scope,arg,rs) : P.Partial}:
  match rs:
    case Nil{}: {==}
    case +h <> +t:
      %import_member_partial(scope,arg,t) : {P.Partial{False{},Bool.or(P.fact_row(h,scope,"structure-python-import-member",arg),P.fact_exists(t,scope,"structure-python-import-member",arg)),Bool.and(P.member_support(h,scope,"fact-import-member"),P.member_supported(t,scope)),True{}} == P.merge(P.Partial{False{},P.fact_row(h,scope,"structure-python-import-member",arg),P.member_support(h,scope,"fact-import-member"),True{}},_) : P.Partial}
      {==}

def json_partial(+scope: String,+arg: String,rs: List<&2,List<&2,String>>) -> {P.Partial{P.any_member(rs,scope),False{},P.supported(rs,scope,"fact-json-valid"),P.json_all(rs,scope)} == P.partial(P.JsonSyntax{},scope,arg,rs) : P.Partial}:
  match rs:
    case Nil{}: {==}
    case +h <> +t:
      %json_partial(scope,arg,t) : {P.Partial{P.or_else(P.selection(h,scope),u => P.any_member(t,scope)),False{},Bool.and(P.file_supported(h,scope,"fact-json-valid"),P.supported(t,scope,"fact-json-valid")),Bool.and(P.json_row_valid(h,scope),P.json_all(t,scope))} == P.merge(P.Partial{P.selection(h,scope),False{},P.file_supported(h,scope,"fact-json-valid"),P.json_row_valid(h,scope)},_) : P.Partial}
      %or_else_or(P.selection(h,scope),P.any_member(t,scope)) : {P.Partial{P.or_else(P.selection(h,scope),u => P.any_member(t,scope)),False{},Bool.and(P.file_supported(h,scope,"fact-json-valid"),P.supported(t,scope,"fact-json-valid")),Bool.and(P.json_row_valid(h,scope),P.json_all(t,scope))} == P.Partial{_,False{},Bool.and(P.file_supported(h,scope,"fact-json-valid"),P.supported(t,scope,"fact-json-valid")),Bool.and(P.json_row_valid(h,scope),P.json_all(t,scope))} : P.Partial}
      {==}

def bridge(rule: P.ContentRule,+scope: String,+arg: String,+rs: List<&2,List<&2,String>>) -> {P.evaluate(P.content_name(rule),scope,arg,rs) == P.finish(P.resolve(P.content_name(rule),arg),P.partial(P.resolve(P.content_name(rule),arg),scope,arg,rs)) : C.Status}:
  match rule:
    case P.RContains{}:
      %contains_partial(scope,arg,rs) : {P.truth(P.any_contains(rs,scope,arg)) == P.finish(P.Contains{},_) : C.Status}
      {==}
    case P.RAbsent{}:
      %absent_partial(scope,arg,rs) : {P.truth(Bool.not(P.any_contains(rs,scope,arg))) == P.finish(P.Absent{},_) : C.Status}
      {==}
    case P.RHeading{}:
      %fact_partial("heading-candidate",arg,"fact-heading",scope,arg,rs) : {P.fact_result(P.supported(rs,scope,"fact-heading"),P.fact_exists(rs,scope,"heading-candidate",arg)) == P.finish(P.Fact{"heading-candidate",arg,"fact-heading"},_) : C.Status}
      {==}
    case P.RImport{}:
      %fact_partial("import-candidate",arg,"fact-import",scope,arg,rs) : {P.fact_result(P.supported(rs,scope,"fact-import"),P.fact_exists(rs,scope,"import-candidate",arg)) == P.finish(P.Fact{"import-candidate",arg,"fact-import"},_) : C.Status}
      {==}
    case P.RImportMember{}:
      %import_member_partial(scope,arg,rs) : {P.fact_result(P.member_supported(rs,scope),P.fact_exists(rs,scope,"structure-python-import-member",arg)) == P.finish(P.ImportMember{arg},_) : C.Status}
      {==}
    case P.RDefinition{}:
      %fact_partial("definition-candidate",arg,"fact-definition",scope,arg,rs) : {P.fact_result(P.supported(rs,scope,"fact-definition"),P.fact_exists(rs,scope,"definition-candidate",arg)) == P.finish(P.Fact{"definition-candidate",arg,"fact-definition"},_) : C.Status}
      {==}
    case P.RJsonKey{}:
      %fact_partial("json-key",arg,"fact-json-key",scope,arg,rs) : {P.fact_result(P.supported(rs,scope,"fact-json-key"),P.fact_exists(rs,scope,"json-key",arg)) == P.finish(P.Fact{"json-key",arg,"fact-json-key"},_) : C.Status}
      {==}
    case P.RJsonValid{}:
      %fact_partial("json-valid","true","fact-json-valid",scope,arg,rs) : {P.fact_result(P.supported(rs,scope,"fact-json-valid"),P.fact_exists(rs,scope,"json-valid","true")) == P.finish(P.Fact{"json-valid","true","fact-json-valid"},_) : C.Status}
      {==}
    case P.RJsonSyntax{}:
      %json_partial(scope,arg,rs) : {P.fact_result(P.supported(rs,scope,"fact-json-valid"),Bool.and(P.any_member(rs,scope),P.json_all(rs,scope))) == P.finish(P.JsonSyntax{},_) : C.Status}
      {==}

def chunked_exact(+rule: P.ContentRule,+scope: String,+arg: String,+parts: List<&2,List<&2,List<&2,String>>>) -> {P.evaluate(P.content_name(rule),scope,arg,P.concat(parts)) == P.finish(P.resolve(P.content_name(rule),arg),P.combine(P.resolve(P.content_name(rule),arg),scope,arg,parts)) : C.Status}:
  %Equal.sym(P.Partial,P.combine(P.resolve(P.content_name(rule),arg),scope,arg,parts),P.partial(P.resolve(P.content_name(rule),arg),scope,arg,P.concat(parts)),partial_concat(P.resolve(P.content_name(rule),arg),scope,arg,parts)) : {P.evaluate(P.content_name(rule),scope,arg,P.concat(parts)) == P.finish(P.resolve(P.content_name(rule),arg),_) : C.Status}
  bridge(rule,scope,arg,P.concat(parts))

def set_bit_bit(b: Bool) -> {P.set_bit(P.bit(b)) == b : Bool}:
  match b:
    case True{}: {==}
    case False{}: {==}

def bits_roundtrip(p: P.Partial,+chunk: String,+rule: String,+scope: String,+arg: String) -> {P.unbits(List.append(&2,String,["PC",chunk,rule,scope,arg],P.bits(p))) == p : P.Partial}:
  match p:
    case P.Partial{+m,+f,+s,+v}: partial_eq(P.set_bit(P.bit(m)),m,P.set_bit(P.bit(f)),f,P.set_bit(P.bit(s)),s,P.set_bit(P.bit(v)),v,set_bit_bit(m),set_bit_bit(f),set_bit_bit(s),set_bit_bit(v))

# Listing then expansion is emptying: over any rows, followed by any expanded rest.
def listed_step(b: Bool,+h: List<&2,String>,l: List<&2,List<&2,String>>,rest: List<&2,List<&2,String>>,e: List<&2,List<&2,String>>,ih: {P.expand_listing(List.append(&2,List<&2,String>,l,rest)) == List.append(&2,List<&2,String>,e,P.expand_listing(rest)) : List<&2,List<&2,String>>}) -> {P.expand_listing(List.append(&2,List<&2,String>,P.listed(b,h,l),rest)) == List.append(&2,List<&2,String>,P.emptied_row(b,h,e),P.expand_listing(rest)) : List<&2,List<&2,String>>}:
  match b:
    case True{}:
      %ih : {P.expand_listing(List.append(&2,List<&2,String>,P.listed(True{},h,l),rest)) == P.emptied_row(True{},h,_) : List<&2,List<&2,String>>}
      {==}
    case False{}: ih

def listing_expand(xs: List<&2,List<&2,String>>,+rest: List<&2,List<&2,String>>) -> {P.expand_listing(List.append(&2,List<&2,String>,P.listing(xs),rest)) == List.append(&2,List<&2,String>,P.emptied(xs),P.expand_listing(rest)) : List<&2,List<&2,String>>}:
  match xs:
    case Nil{}: {==}
    case +h <> +t: listed_step(P.is_tag(h,"F"),h,P.listing(t),rest,P.emptied(t),listing_expand(t,rest))

def emptied_step(b: Bool,+h: List<&2,String>,x: List<&2,List<&2,String>>,e: List<&2,List<&2,String>>,y: List<&2,List<&2,String>>,ih: {x == List.append(&2,List<&2,String>,e,y) : List<&2,List<&2,String>>}) -> {P.emptied_row(b,h,x) == List.append(&2,List<&2,String>,P.emptied_row(b,h,e),y) : List<&2,List<&2,String>>}:
  match b:
    case True{}:
      %ih : {P.emptied_row(True{},h,x) == P.emptied_row(True{},h,_) : List<&2,List<&2,String>>}
      {==}
    case False{}: ih

def emptied_append(xs: List<&2,List<&2,String>>,+ys: List<&2,List<&2,String>>) -> {P.emptied(List.append(&2,List<&2,String>,xs,ys)) == List.append(&2,List<&2,String>,P.emptied(xs),P.emptied(ys)) : List<&2,List<&2,String>>}:
  match xs:
    case Nil{}: {==}
    case +h <> +t: emptied_step(P.is_tag(h,"F"),h,P.emptied(List.append(&2,List<&2,String>,t,ys)),P.emptied(t),P.emptied(ys),emptied_append(t,ys))

def listing_exact(parts: List<&2,List<&2,List<&2,String>>>) -> {P.expand_listing(P.concat(P.listings(parts))) == P.emptied(P.concat(parts)) : List<&2,List<&2,String>>}:
  match parts:
    case Nil{}: {==}
    case +h <> +t:
      %Equal.sym(List<&2,List<&2,String>>,P.emptied(List.append(&2,List<&2,String>,h,P.concat(t))),List.append(&2,List<&2,String>,P.emptied(h),P.emptied(P.concat(t))),emptied_append(h,P.concat(t))) : {P.expand_listing(P.concat(P.listings(h <> t))) == _ : List<&2,List<&2,String>>}
      %listing_exact(t) : {P.expand_listing(P.concat(P.listings(h <> t))) == List.append(&2,List<&2,String>,P.emptied(h),_) : List<&2,List<&2,String>>}
      listing_expand(h,P.concat(P.listings(t)))
