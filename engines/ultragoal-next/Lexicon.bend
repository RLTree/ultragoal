import Base

# Tokens for `select` retrieval. An identifier run is a maximal run of ASCII
# [A-Za-z0-9_]; it splits into lower-case parts at '_', at letter/digit changes and
# at camelCase boundaries (an upper-case run before a lower-case letter gives it its
# last letter). Each part is stemmed by suffix rules, and a run of two or more parts
# also yields its joined compound. Part and term texts are kept reversed, so the
# suffix rules and the term trie read them from their head.

type Part is Data:
  Part{text: String, len: Nat}

# 0 separator, 1 lower, 2 upper, 3 digit, 4 underscore
def cls(+x: U32) -> U32:
  Bool.pick(U32,U32.is_ge(x,97) && U32.is_le(x,122),1,Bool.pick(U32,U32.is_ge(x,65) && U32.is_le(x,90),2,Bool.pick(U32,U32.is_ge(x,48) && U32.is_le(x,57),3,Bool.pick(U32,U32.is_eq(x,95),4,0))))

# Horizontal white space: space, tab, carriage return, vertical tab, form feed.
def ws(+x: U32) -> Bool:
  U32.is_eq(x,32) || U32.is_eq(x,9) || U32.is_eq(x,13) || U32.is_eq(x,11) || U32.is_eq(x,12)

# ---- parts of one run

type Split is Data:
  Split{part: String, plen: Nat, pc: U32, parts: List<&2,Part>}

def close(part: String,plen: Nat,parts: List<&2,Part>) -> List<&2,Part>:
  match plen:
    case 0n: parts
    case 1n+p: Part{part,1n+p} <> parts

# A lower-case letter after an upper-case run: a single capital starts the word
# ("Parser"); a longer run gives its last capital to the word ("HTMLParser").
def camel_at(p: Nat,part: String,+x: U32,parts: List<&2,Part>) -> Split:
  match p part:
    case 0n _: Split{SCon{Chr{x},part},2n,1,parts}
    case 1n+q SCon{h,rest}: Split{SCon{Chr{x},SCon{h,""}},2n,1,Part{rest,1n+q} <> parts}
    case 1n+q SNil{}: Split{SCon{Chr{x},""},1n,1,parts}

def camel(plen: Nat,part: String,+x: U32,parts: List<&2,Part>) -> Split:
  match plen:
    case 0n: Split{SCon{Chr{x},""},1n,1,parts}
    case 1n+p: camel_at(p,part,x,parts)

def split_cls(c: U32,pc: U32,+x: U32,part: String,plen: Nat,parts: List<&2,Part>) -> Split:
  match c pc:
    case 4 _: Split{"",0n,0,close(part,plen,parts)}
    case 3 3: Split{SCon{Chr{x},part},1n+plen,3,parts}
    case 3 _: Split{SCon{Chr{x},""},1n,3,close(part,plen,parts)}
    case 1 1: Split{SCon{Chr{x},part},1n+plen,1,parts}
    case 1 2: camel(plen,part,x,parts)
    case 1 _: Split{SCon{Chr{x},""},1n,1,close(part,plen,parts)}
    case 2 2: Split{SCon{Chr{(x + 32 : U32)},part},1n+plen,2,parts}
    case _ _: Split{SCon{Chr{(x + 32 : U32)},""},1n,2,close(part,plen,parts)}

def split_step(sp: Split,+x: U32) -> Split:
  match sp:
    case Split{part,plen,pc,parts}: split_cls(cls(x),pc,x,part,plen,parts)

def split_walk(s: String,sp: Split) -> Split:
  match s:
    case SNil{}: sp
    case SCon{Chr{+x},t}: split_walk(t,split_step(sp,x))

def split_done(sp: Split) -> List<&2,Part>:
  match sp:
    case Split{part,plen,_,parts}: close(part,plen,parts)

# Parts of a forward run, most recent first, each reversed and lower-case.
def parts(run: String) -> List<&2,Part>:
  split_done(split_walk(run,Split{"",0n,0,Nil{}}))

def glue(t: String,l: Nat,p: Part) -> Part:
  match p:
    case Part{t2,l2}: Part{String.append(t,t2),Nat.add(l,l2)}

# The reversed compound of reversed parts listed most recent first.
def joined(ps: List<&2,Part>) -> Part:
  match ps:
    case Nil{}: Part{"",0n}
    case Part{t,l} <> rest: glue(t,l,joined(rest))

def many(ps: List<&2,Part>) -> Bool:
  match ps:
    case _ <> _ <> _: True{}
    case _: False{}

# ---- suffix stemmer on reversed text

def ies(long: Bool,r: String,len: Nat) -> Part:
  match long:
    case True{}: Part{SCon{'y',r},Nat.sub(len,2n)}
    case False{}: Part{SCon{'e',SCon{'i',r}},Nat.sub(len,1n)}

def stem1(t: String,+len: Nat) -> Part:
  match t:
    case SCon{'s',SCon{'e',SCon{'i',r}}}: ies(Nat.is_gt(len,4n),r,len)
    case SCon{'s',SCon{'e',SCon{'s',SCon{'s',r}}}}: Part{SCon{'s',SCon{'s',r}},Nat.sub(len,2n)}
    case SCon{'s',SCon{'s',r}}: Part{SCon{'s',SCon{'s',r}},len}
    case SCon{'s',SCon{'u',r}}: Part{SCon{'s',SCon{'u',r}},len}
    case SCon{'s',SCon{'i',r}}: Part{SCon{'s',SCon{'i',r}},len}
    case SCon{'s',r}: Part{r,Nat.sub(len,1n)}
    case other: Part{other,len}

def ing(long: Bool,r: String,len: Nat) -> Part:
  match long:
    case True{}: Part{r,Nat.sub(len,3n)}
    case False{}: Part{SCon{'g',SCon{'n',SCon{'i',r}}},len}

def ed(long: Bool,r: String,len: Nat) -> Part:
  match long:
    case True{}: Part{r,Nat.sub(len,2n)}
    case False{}: Part{SCon{'d',SCon{'e',r}},len}

def stem2(p: Part) -> Part:
  match p:
    case Part{SCon{'g',SCon{'n',SCon{'i',r}}},+len}: ing(Nat.is_gt(len,5n),r,len)
    case Part{SCon{'d',SCon{'e',r}},+len}: ed(Nat.is_gt(len,4n),r,len)
    case other: other

def drop_e(long: Bool,r: String,len: Nat) -> Part:
  match long:
    case True{}: Part{r,Nat.sub(len,1n)}
    case False{}: Part{SCon{'e',r},len}

def stem3(p: Part) -> Part:
  match p:
    case Part{SCon{'e',r},+len}: drop_e(Nat.is_gt(len,3n),r,len)
    case other: other

def y_to_i(long: Bool,r: String,len: Nat) -> Part:
  match long:
    case True{}: Part{SCon{'i',r},len}
    case False{}: Part{SCon{'y',r},len}

def stem4(p: Part) -> Part:
  match p:
    case Part{SCon{'y',r},+len}: y_to_i(Nat.is_gt(len,3n),r,len)
    case other: other

def stem_len(short: Bool,t: String,len: Nat) -> Part:
  match short:
    case True{}: Part{t,len}
    case False{}: stem4(stem3(stem2(stem1(t,len))))

def stem(p: Part) -> Part:
  match p:
    case Part{t,+len}: stem_len(Nat.is_le(len,3n),t,len)

def stems(ps: List<&2,Part>) -> List<&2,Part>:
  match ps:
    case Nil{}: Nil{}
    case h <> t: stem(h) <> stems(t)

def with_compound(more: Bool,+ps: List<&2,Part>) -> List<&2,Part>:
  match more:
    case True{}: joined(ps) <> ps
    case False{}: ps

# Stemmed tokens of one forward run: its parts, then the compound of 2+ parts.
def tokens(run: String) -> List<&2,Part>:
  +ps = parts(run)
  stems(with_compound(many(ps),ps))

# ---- term trie over reversed terms; a node's term is its index + 1, 0 for none

type Trie is Data:
  Trie{term: U32, kids: List<&2,Edge>}

type Edge is Data:
  Edge{c: U32, node: Trie}

def pick_child(hit: Bool,node: Trie,other: Maybe<&2,Trie>) -> Maybe<&2,Trie>:
  match hit:
    case True{}: Some{node}
    case False{}: other

def child(kids: List<&2,Edge>,+x: U32) -> Maybe<&2,Trie>:
  match kids:
    case Nil{}: None{}
    case Edge{c,node} <> rest: pick_child(U32.is_eq(c,x),node,child(rest,x))

def lookup(s: String,node: Maybe<&2,Trie>) -> U32:
  match s node:
    case _ None{}: 0
    case SNil{} Some{Trie{term,_}}: term
    case SCon{Chr{+x},t} Some{Trie{_,kids}}: lookup(t,child(kids,x))

def mark(node: Trie,+k: U32) -> Trie:
  match node:
    case Trie{+term,kids}: Trie{Bool.pick(U32,U32.is_eq(term,0),k,term),kids}

def put_if(eq: Bool,c: U32,node: Trie,rest: List<&2,Edge>,next: (Trie -> Trie) -> List<&2,Edge>,f: Trie -> Trie) -> List<&2,Edge>:
  match eq:
    case True{}: Edge{c,f(node)} <> rest
    case False{}: Edge{c,node} <> next(f)

def put(kids: List<&2,Edge>,+x: U32) -> (Trie -> Trie) -> List<&2,Edge>:
  match kids:
    case Nil{}: f => [Edge{x,f(Trie{0,Nil{}})}]
    case Edge{+c,node} <> +rest: f => put_if(U32.is_eq(c,x),c,node,rest,put(rest,x),f)

def descend(node: Trie,x: U32,f: Trie -> Trie) -> Trie:
  match node:
    case Trie{term,kids}: Trie{term,put(kids,x,f)}

def insert(s: String,+k: U32) -> Trie -> Trie:
  match s:
    case SNil{}: node => mark(node,k)
    case SCon{Chr{+x},t}: node => descend(node,x,insert(t,k))

def trie_from(terms: List<&2,String>,+k: U32,node: Trie) -> Trie:
  match terms:
    case Nil{}: node
    case h <> t: trie_from(t,(k + 1 : U32),insert(h,k,node))

# Term k (from 0) is found as k + 1.
def trie(terms: List<&2,String>) -> Trie:
  trie_from(terms,1,Trie{0,Nil{}})

def matched(k: U32,hits: List<&2,U32>) -> List<&2,U32>:
  match k:
    case 0: hits
    case other: (other - 1 : U32) <> hits

# Term indexes of the tokens found in the trie, in front of hits.
def matches(ts: List<&2,Part>,+node: Trie,hits: List<&2,U32>) -> List<&2,U32>:
  match ts:
    case Nil{}: hits
    case Part{t,_} <> rest: matches(rest,node,matched(lookup(t,Some{node}),hits))

# ---- the scanner: a left fold over characters

# Definition-name recognition on each line, as the pattern
#   ws* (pub[(a-z+)] ws+ | export ws+ [default ws+] | async ws+ | static ws+)*
#   keyword ws+ [( [^)]* ) ws*] name
# with keyword one of def class fn struct enum trait impl type interface func
# function const let var mod. States: 0 line start, 1 after export, 2 after "pub(",
# 3 after "pub(x)", 4 after a keyword, 5 in a receiver, 6 after a receiver, 7 none.
type Scan is Data:
  Scan{run: String, h: U32, n: Nat, hits: List<&2,U32>, defs: List<&2,U32>}

type Head is Data:
  Head{h: U32, name: Bool}

def keyword(w: String) -> Bool:
  match w:
    case "fed": True{}
    case "ssalc": True{}
    case "nf": True{}
    case "tcurts": True{}
    case "mune": True{}
    case "tiart": True{}
    case "lpmi": True{}
    case "epyt": True{}
    case "ecafretni": True{}
    case "cnuf": True{}
    case "noitcnuf": True{}
    case "tsnoc": True{}
    case "tel": True{}
    case "rav": True{}
    case "dom": True{}
    case _: False{}

# 1 a modifier, 2 export, 0 neither (reversed words)
def modifier(w: String) -> U32:
  match w:
    case "bup": 1
    case "cnysa": 1
    case "citats": 1
    case "tropxe": 2
    case _: 0

def all_lower(w: String) -> Bool:
  match w:
    case SNil{}: True{}
    case SCon{Chr{+x},t}: U32.is_ge(x,97) && U32.is_le(x,122) && all_lower(t)

# The forward run starts with a letter or underscore (its reversed last character).
def name_ok(w: String) -> Bool:
  match w:
    case SNil{}: False{}
    case SCon{Chr{+x},SNil{}}: Bool.not(U32.is_eq(cls(x),3))
    case SCon{_,t}: name_ok(t)

def spaced(m: U32,kw: Bool) -> U32:
  match m:
    case 1: 0
    case 2: 1
    case _: Bool.pick(U32,kw,4,7)

def start_state(space: Bool,paren: Bool,+w: String) -> U32:
  match space paren:
    case True{} _: spaced(modifier(w),keyword(w))
    case False{} True{}: Bool.pick(U32,String.eq(w,"bup"),2,7)
    case False{} False{}: 7

def start_word(+w: String,+x: U32) -> Head:
  Head{start_state(ws(x),U32.is_eq(x,40),w),False{}}

def export_word(dflt: Bool,+w: String,+x: U32) -> Head:
  match dflt:
    case True{}: Head{0,False{}}
    case False{}: start_word(w,x)

# A word w (reversed) ended by separator x in state h.
def head_word(h: U32,+w: String,+x: U32) -> Head:
  match h:
    case 0: start_word(w,x)
    case 1: export_word(String.eq(w,"tluafed") && ws(x),w,x)
    case 2: Head{Bool.pick(U32,U32.is_eq(x,41) && all_lower(w),3,7),False{}}
    case 4: Head{7,name_ok(w)}
    case 5: Head{Bool.pick(U32,U32.is_eq(x,41),6,5),False{}}
    case 6: Head{7,name_ok(w)}
    case _: Head{7,False{}}

# A separator x with no word before it in state h.
def head_empty(h: U32,+x: U32) -> U32:
  match h:
    case 0: Bool.pick(U32,ws(x),0,7)
    case 1: Bool.pick(U32,ws(x),1,7)
    case 3: Bool.pick(U32,ws(x),0,7)
    case 4: Bool.pick(U32,ws(x),4,Bool.pick(U32,U32.is_eq(x,40),5,7))
    case 5: Bool.pick(U32,U32.is_eq(x,41),6,5)
    case 6: Bool.pick(U32,ws(x),6,7)
    case _: 7

# A line break resets the recognizer.
def line_reset(+x: U32,h: U32) -> U32:
  Bool.pick(U32,U32.is_eq(x,10),0,h)

def name_defs(name: Bool,+ts: List<&2,Part>,+node: Trie,defs: List<&2,U32>) -> List<&2,U32>:
  match name:
    case True{}: matches(ts,node,defs)
    case False{}: defs

def ended(hd: Head,+ts: List<&2,Part>,+x: U32,n: Nat,hits: List<&2,U32>,defs: List<&2,U32>,+node: Trie) -> Scan:
  match hd:
    case Head{h,name}: Scan{"",line_reset(x,h),Nat.add(n,List.length(&2,Part,ts)),matches(ts,node,hits),name_defs(name,ts,node,defs)}

def boundary_run(run: String,h: U32,+x: U32,n: Nat,hits: List<&2,U32>,defs: List<&2,U32>,+node: Trie) -> Scan:
  match run:
    case SNil{}: Scan{"",line_reset(x,head_empty(h,x)),n,hits,defs}
    case +w: ended(head_word(h,w,x),tokens(String.reverse(w)),x,n,hits,defs,node)

def boundary(st: Scan,+x: U32,+node: Trie) -> Scan:
  match st:
    case Scan{run,h,n,hits,defs}: boundary_run(run,h,x,n,hits,defs,node)

# An identifier character right after "pub(x)" breaks that modifier.
def ident_h(h: U32) -> U32:
  match h:
    case 3: 7
    case other: other

def push(st: Scan,x: U32) -> Scan:
  match st:
    case Scan{run,h,n,hits,defs}: Scan{SCon{Chr{x},run},ident_h(h),n,hits,defs}

def step_cls(c: U32,st: Scan,+x: U32,+node: Trie) -> Scan:
  match c:
    case 0: boundary(st,x,node)
    case _: push(st,x)

def step(st: Scan,+x: U32,+node: Trie) -> Scan:
  step_cls(cls(x),st,x,node)

def walk(s: String,st: Scan,+node: Trie) -> Scan:
  match s:
    case SNil{}: st
    case SCon{Chr{+x},t}: walk(t,step(st,x,node),node)

def fresh() -> Scan:
  Scan{"",0,0n,Nil{},Nil{}}

# Token count, matched term indexes, and matched definition-name term indexes of a
# text; the end of the text ends its last line.
def scan(s: String,+node: Trie) -> Scan:
  step(walk(s,fresh(),node),10,node)

# ---- question terms

type Runs is Data:
  Runs{cur: String, out: List<&2,String>}

def run_end(cur: String,out: List<&2,String>) -> List<&2,String>:
  match cur:
    case SNil{}: out
    case w: String.reverse(w) <> out

def runs_pick(ident: Bool,x: U32,cur: String,out: List<&2,String>) -> Runs:
  match ident:
    case True{}: Runs{SCon{Chr{x},cur},out}
    case False{}: Runs{"",run_end(cur,out)}

def runs_cls(ident: Bool,x: U32,r: Runs) -> Runs:
  match r:
    case Runs{cur,out}: runs_pick(ident,x,cur,out)

def runs_walk(s: String,r: Runs) -> Runs:
  match s:
    case SNil{}: r
    case SCon{Chr{+x},t}: runs_walk(t,runs_cls(Bool.not(U32.is_eq(cls(x),0)),x,r))

def runs_done(r: Runs) -> List<&2,String>:
  match r:
    case Runs{cur,out}: List.reverse(&2,String,run_end(cur,out))

# Forward identifier runs of a text, in order.
def runs(s: String) -> List<&2,String>:
  runs_done(runs_walk(s,Runs{"",Nil{}}))

def STOP() -> List<&2,String>:
  String.split("a about above after again against all also am an and any are aren as at be because been before being below between both but by can cannot could did didn do does doesn doing don down during each either else even ever every few for from further get gets got had has hasn have having he her here hers him his how i if in into is isn it its itself just let like ll me might more most much must my myself no nor not now of off on once only or other our ours out over own per re s same shall she should so some such t than that the their theirs them then there these they this those through to too under until up us use used uses using ve very via want wants was wasn way we were weren what whatever when where whether which while who whom whose why will with within without would yes yet you your yours exactly actually happen happens happening instead need needs say says tell work works",' ')

def member(xs: List<&2,String>,+w: String) -> Bool:
  match xs:
    case Nil{}: False{}
    case h <> t: String.eq(h,w) || member(t,w)

def keep_if(skip: Bool,p: Part,out: List<&2,Part>) -> List<&2,Part>:
  match skip:
    case True{}: out
    case False{}: p <> out

def keep_part(p: Part,+stops: List<&2,String>,out: List<&2,Part>) -> List<&2,Part>:
  match p:
    case Part{+t,l}: keep_if(member(stops,String.reverse(t)),Part{t,l},out)

# Parts that are not function words, forward order (ps is most recent first).
def kept(ps: List<&2,Part>,+stops: List<&2,String>,out: List<&2,Part>) -> List<&2,Part>:
  match ps:
    case Nil{}: out
    case h <> t: kept(t,stops,keep_part(h,stops,out))

def run_words(+ps: List<&2,Part>,+stops: List<&2,String>) -> List<&2,Part>:
  List.append(&2,Part,kept(ps,stops,Nil{}),Bool.pick(List<&2,Part>,many(ps),[joined(ps)],Nil{}))

def words(rs: List<&2,String>,+stops: List<&2,String>) -> List<&2,Part>:
  match rs:
    case Nil{}: Nil{}
    case h <> t: List.append(&2,Part,run_words(parts(h),stops),words(t,stops))

def has_underscore(s: String) -> Bool:
  match s:
    case SNil{}: False{}
    case SCon{Chr{+x},t}: U32.is_eq(x,95) || has_underscore(t)

def lower_char(c: Char) -> Char:
  match c:
    case Chr{+x}: Chr{Bool.pick(U32,U32.is_eq(cls(x),2),(x + 32 : U32),x)}

def lower(s: String) -> String:
  match s:
    case SNil{}: SNil{}
    case SCon{c,t}: SCon{lower_char(c),lower(t)}

def plain_if(skip: Bool,w: String,out: List<&2,String>) -> List<&2,String>:
  match skip:
    case True{}: out
    case False{}: w <> out

def plain_one(+w: String,underscore: Bool,+stops: List<&2,String>,out: List<&2,String>) -> List<&2,String>:
  plain_if(underscore || member(stops,w),w,out)

# Lower-case runs that are not function words and hold no underscore.
def plain(rs: List<&2,String>,+stops: List<&2,String>) -> List<&2,String>:
  match rs:
    case Nil{}: Nil{}
    case +h <> t: plain_one(lower(h),has_underscore(h),stops,plain(t,stops))

def pair_with(+a: String,rest: List<&2,String>,tail: List<&2,Part>) -> List<&2,Part>:
  match rest:
    case Nil{}: tail
    case +b <> _: Part{String.reverse(String.append(a,b)),Nat.add(String.length(a),String.length(b))} <> tail

# Each adjacent pair of words joined, in order.
def pairs(ws: List<&2,String>) -> List<&2,Part>:
  match ws:
    case Nil{}: Nil{}
    case +a <> +rest: pair_with(a,rest,pairs(rest))

def seen(xs: List<&2,String>,+t: String) -> Bool:
  match xs:
    case Nil{}: False{}
    case h <> rest: String.eq(h,t) || seen(rest,t)

def keep_term(p: Part,+out: List<&2,String>) -> List<&2,String>:
  match p:
    case Part{+t,l}: Bool.pick(List<&2,String>,Nat.is_lt(l,2n) || seen(out,t),out,t <> out)

def distinct(ps: List<&2,Part>,out: List<&2,String>) -> List<&2,String>:
  match ps:
    case Nil{}: List.reverse(&2,String,out)
    case h <> t: distinct(t,keep_term(h,out))

# The question's terms, reversed and stemmed, in first-occurrence order: the parts
# of each run that are not function words, the compound of each run of 2+ parts,
# then each adjacent pair of plain words joined. Terms shorter than 2 are dropped.
def terms(q: String) -> List<&2,String>:
  +rs = runs(q)
  +stops = STOP()
  distinct(stems(List.append(&2,Part,words(rs,stops),pairs(plain(rs,stops)))),Nil{})
