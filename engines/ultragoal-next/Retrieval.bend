import Base
import ./Wire.bend as W
import ./Policy.bend as P
import ./Lexicon.bend as L
import ./Facts.bend as F
import ./Select.bend as S

# `select` retrieval in four stages. Every statistic a later stage needs is a sum of
# stage-1 rows, so chunks of candidates in any grouping give the exact answer.
#   SELECT_MATCH   (per chunk)  QUERY, C rows -> FILE and WINDOWS rows
#   FILE_RANK      (one frame)  QUERY, SHORTLIST, FILE/WINDOWS/X rows -> RANKED, POOL, IDF, NORM, EXCLUDED
#   SELECT_PASSAGE (per chunk)  QUERY, NORM, IDF, C rows of pooled files -> BEST rows
#   SELECT_FUSE    SHORTLIST, POOL, BEST, C rows of best windows -> representatives and STOP
#   SELECT_ALIASES (paged) PRIMARY rows, then remaining C rows -> complete exact alias membership
# Term k is the question's k-th term (Lexicon.terms), from 0. Refused rows (Policy.disclosable)
# are skipped before anything is counted.

# ---- small sorted lists

def choose(ok: Bool,a: String,b: String) -> String:
  match ok:
    case True{}: a
    case False{}: b

def le(a: U32,b: U32) -> Bool:
  U32.is_le(a,b)

def sorted(+xs: List<&2,U32>) -> List<&2,U32>:
  List.sort(~U32,~(a => b => le(a,b)),xs)

def dedupe_pick(same: Bool,prev: U32,rest: List<&2,U32>) -> List<&2,U32>:
  match same:
    case True{}: rest
    case False{}: prev <> rest

def dedupe_from(xs: List<&2,U32>,+prev: U32) -> List<&2,U32>:
  match xs:
    case Nil{}: [prev]
    case +h <> t: dedupe_pick(U32.is_eq(h,prev),prev,dedupe_from(t,h))

# Distinct values of a sorted list.
def dedupe(xs: List<&2,U32>) -> List<&2,U32>:
  match xs:
    case Nil{}: Nil{}
    case h <> t: dedupe_from(t,h)

type Tally is Data:
  Tally{k: U32, c: Nat}

def tally_at(same: Bool,+x: U32,t: Tally,rest: List<&2,Tally>) -> List<&2,Tally>:
  match same t:
    case True{} Tally{k,c}: Tally{k,1n+c} <> rest
    case False{} Tally{k,c}: Tally{x,1n} <> Tally{k,c} <> rest

def tally_one(+x: U32,acc: List<&2,Tally>) -> List<&2,Tally>:
  match acc:
    case Nil{}: [Tally{x,1n}]
    case Tally{+k,c} <> rest: tally_at(U32.is_eq(k,x),x,Tally{k,c},rest)

# Counts of each value of a sorted list, largest value first.
def tallies(xs: List<&2,U32>,acc: List<&2,Tally>) -> List<&2,Tally>:
  match xs:
    case Nil{}: acc
    case h <> t: tallies(t,tally_one(h,acc))

def show_tallies(ts: List<&2,Tally>,out: List<&2,String>) -> List<&2,String>:
  match ts:
    case Nil{}: out
    case Tally{k,c} <> rest: show_tallies(rest,(U32.show(k) ++ ":" ++ Nat.show(c)) <> out)

# ---- stage 1: SELECT_MATCH

# A file's term events: 4k path hit, 4k+1 definition-name hit, 4k+2 body hit.
def tag(xs: List<&2,U32>,+t: U32,out: List<&2,U32>) -> List<&2,U32>:
  match xs:
    case Nil{}: out
    case h <> rest: tag(rest,t,((h * 4) + t : U32) <> out)

type Entry is Data:
  Entry{k: U32, p: Nat, d: Nat, b: Nat}

def ev(+t: U32,which: U32) -> Nat:
  Bool.pick(Nat,U32.is_eq(t,which),1n,0n)

def entry_at(same: Bool,+t: U32,+e: U32,top: Entry,rest: List<&2,Entry>) -> List<&2,Entry>:
  match same top:
    case True{} Entry{k,p,d,b}: Entry{k,Nat.add(p,ev(t,0)),Nat.add(d,ev(t,1)),Nat.add(b,ev(t,2))} <> rest
    case False{} Entry{k,p,d,b}: Entry{U32.shrn(e,2n),ev(t,0),ev(t,1),ev(t,2)} <> Entry{k,p,d,b} <> rest

def entry_one(+e: U32,acc: List<&2,Entry>) -> List<&2,Entry>:
  match acc:
    case Nil{}: [Entry{U32.shrn(e,2n),ev(U32.and(e,3),0),ev(U32.and(e,3),1),ev(U32.and(e,3),2)}]
    case Entry{+k,p,d,b} <> rest: entry_at(U32.is_eq(k,U32.shrn(e,2n)),U32.and(e,3),e,Entry{k,p,d,b},rest)

def entries(xs: List<&2,U32>,acc: List<&2,Entry>) -> List<&2,Entry>:
  match xs:
    case Nil{}: acc
    case h <> t: entries(t,entry_one(h,acc))

def show_entries(es: List<&2,Entry>,out: List<&2,String>) -> List<&2,String>:
  match es:
    case Nil{}: out
    case Entry{k,p,d,b} <> rest: show_entries(rest,(U32.show(k) ++ ":" ++ Nat.show(p) ++ ":" ++ Nat.show(d) ++ ":" ++ Nat.show(b)) <> out)

# The file open in a chunk: path token count and path hits, body tokens, events.
type Open is Data:
  Open{path: String, pn: Nat, phits: List<&2,U32>, flen: Nat, events: List<&2,U32>}

type Chunk is Data:
  Chunk{open: Open, windows: Nat, tokens: Nat, cdf: List<&2,U32>, out: List<&2,String>}

def file_row(path: String,flen: Nat,events: List<&2,U32>) -> String:
  W.row(List.append(&2,String,["FILE",path,Nat.show(flen)],show_entries(entries(sorted(events),Nil{}),Nil{})))

def closed(o: Open,out: List<&2,String>) -> List<&2,String>:
  match o:
    case Open{SNil{},_,_,_,_}: out
    case Open{path,_,_,flen,events}: file_row(path,flen,events) <> out

def opened(path: String,sc: L.Scan) -> Open:
  match sc:
    case L.Scan{_,_,n,+hits,_}: Open{path,n,hits,0n,tag(hits,0,Nil{})}

def is_open(o: Open,+path: String) -> Bool:
  match o:
    case Open{p,_,_,_,_}: String.eq(p,path)

type Cur is Data:
  Cur{open: Open, out: List<&2,String>}

def current(same: Bool,open: Open,+path: String,+node: L.Trie,out: List<&2,String>) -> Cur:
  match same:
    case True{}: Cur{open,out}
    case False{}: Cur{opened(path,L.scan(path,node)),closed(open,out)}

# The distinct terms of a window: its body hits and its path's hits.
def window_terms(hits: List<&2,U32>,phits: List<&2,U32>,cdf: List<&2,U32>) -> List<&2,U32>:
  List.append(&2,U32,dedupe(sorted(List.append(&2,U32,hits,phits))),cdf)

def add_window(sc: L.Scan,cur: Cur,windows: Nat,tokens: Nat,cdf: List<&2,U32>) -> Chunk:
  match sc cur:
    case L.Scan{_,_,+n,+hits,defs} Cur{Open{path,+pn,+phits,flen,events},out}: Chunk{Open{path,pn,phits,Nat.add(flen,n),tag(hits,2,tag(defs,1,events))},1n+windows,Nat.add(tokens,Nat.add(n,pn)),window_terms(hits,phits,cdf),out}

def window(+path: String,excerpt: String,+node: L.Trie,ch: Chunk) -> Chunk:
  match ch:
    case Chunk{+open,windows,tokens,cdf,out}: add_window(L.scan(excerpt,node),current(is_open(open,path),open,path,node,out),windows,tokens,cdf)

def admitted(ok: Bool,+path: String,excerpt: String,+node: L.Trie,ch: Chunk) -> Chunk:
  match ok:
    case True{}: window(path,excerpt,node,ch)
    case False{}: ch

def match_row(r: W.Record,+node: L.Trie,ch: Chunk) -> Chunk:
  match r:
    case W.Row{"C" <> _ <> +path <> _ <> _ <> _ <> +excerpt <> Nil{}}: admitted(P.disclosable(path,excerpt),path,excerpt,node,ch)
    case _: ch

def match_rows(rows: List<&2,String>,+node: L.Trie,ch: Chunk) -> Chunk:
  match rows:
    case Nil{}: ch
    case h <> t: match_rows(t,node,match_row(W.parse(h),node,ch))

def match_done(ch: Chunk) -> String:
  match ch:
    case Chunk{open,windows,tokens,cdf,out}: String.concat(List.reverse(&2,String,closed(open,out))) ++ W.row(List.append(&2,String,["WINDOWS",Nat.show(windows),Nat.show(tokens)],show_tallies(tallies(sorted(cdf),Nil{}),Nil{})))

def no_file() -> Open:
  Open{"",0n,Nil{},0n,Nil{}}

def match_query(r: W.Record,rows: List<&2,String>) -> String:
  match r:
    case W.Row{"QUERY" <> q <> Nil{}}: match_done(match_rows(rows,L.trie(L.terms(q)),Chunk{no_file(),0n,0n,Nil{},Nil{}}))
    case _: "ERROR\tquery required\n"

# FILE <path> <body tokens> <k>:<path>:<definition>:<body> ... for every path with an
# admitted window (terms with a count only), then WINDOWS <windows> <tokens> <k>:<windows with k> ...
def match_process(rows: List<&2,String>) -> String:
  match rows:
    case h <> t: match_query(W.parse(h),t)
    case Nil{}: "ERROR\tquery required\n"

# ---- stage-1 statistics and their merges (the monoid of LAWS.bend)

# One term's counts in one file: path, definition-name and body occurrences. A file
# split across chunks sends each chunk's partial row; the path is tokenised in each,
# so its count is merged by max and the other counts by sum.
type Tri is Data:
  Tri{p: Nat, d: Nat, b: Nat}

def tri_add(x: Tri,y: Tri) -> Tri:
  match x y:
    case Tri{p1,d1,b1} Tri{p2,d2,b2}: Tri{Nat.max(p1,p2),Nat.add(d1,d2),Nat.add(b1,b2)}

# Term vectors indexed by term, zeros past their end.
def vadd(xs: List<&2,Tri>,ys: List<&2,Tri>) -> List<&2,Tri>:
  match xs ys:
    case Nil{} r: r
    case h <> t Nil{}: h <> t
    case h1 <> t1 h2 <> t2: tri_add(h1,h2) <> vadd(t1,t2)

type FStat is Data:
  FStat{flen: Nat, vec: List<&2,Tri>}

def fadd(x: FStat,y: FStat) -> FStat:
  match x y:
    case FStat{l1,v1} FStat{l2,v2}: FStat{Nat.add(l1,l2),vadd(v1,v2)}

type FileRow is Data:
  FileRow{path: String, st: FStat}

def nadd(xs: List<&2,Nat>,ys: List<&2,Nat>) -> List<&2,Nat>:
  match xs ys:
    case Nil{} r: r
    case h <> t Nil{}: h <> t
    case h1 <> t1 h2 <> t2: Nat.add(h1,h2) <> nadd(t1,t2)

# Window statistics of a chunk: admitted windows, window tokens (excerpt and path),
# windows containing each term.
type WStat is Data:
  WStat{n: Nat, tok: Nat, cdf: List<&2,Nat>}

def wadd(x: WStat,y: WStat) -> WStat:
  match x y:
    case WStat{n1,t1,c1} WStat{n2,t2,c2}: WStat{Nat.add(n1,n2),Nat.add(t1,t2),nadd(c1,c2)}

def wsum(xs: List<&2,WStat>) -> WStat:
  match xs:
    case Nil{}: WStat{0n,0n,Nil{}}
    case h <> t: wadd(h,wsum(t))

# ---- path order and grouping (the permutation law of LAWS.bend)

def ccmp(a: Char,b: Char) -> Cmp:
  match a b:
    case Chr{x} Chr{y}: U32.cmp(x,y)

def then(c: Cmp,rest: Cmp) -> Cmp:
  match c:
    case LT{}: LT{}
    case EQ{}: rest
    case GT{}: GT{}

# Code-point order of paths (UTF-8 byte order).
def order(a: String,b: String) -> Cmp:
  match a b:
    case SNil{} SNil{}: EQ{}
    case SNil{} SCon{_,_}: LT{}
    case SCon{_,_} SNil{}: GT{}
    case SCon{x,xs} SCon{y,ys}: then(ccmp(x,y),order(xs,ys))

def row_path(r: FileRow) -> String:
  match r:
    case FileRow{path,_}: path

def group_at(c: Cmp,r: FileRow,h: FileRow,t: List<&2,FileRow>,rest: Unit -> List<&2,FileRow>) -> List<&2,FileRow>:
  match c r h:
    case GT{} x y: x <> y <> t
    case EQ{} FileRow{_,rs} FileRow{hp,hs}: FileRow{hp,fadd(rs,hs)} <> t
    case LT{} _ y: y <> rest(Unit{})

# Inserts a row into a list kept in descending path order, merged into the row of the
# same path; rows arriving in ascending path order go straight to the head.
def group(+r: FileRow,xs: List<&2,FileRow>) -> List<&2,FileRow>:
  match xs:
    case Nil{}: [r]
    case +h <> +t: group_at(order(row_path(r),row_path(h)),r,h,t,u => group(r,t))

def grouped(rows: List<&2,FileRow>,acc: List<&2,FileRow>) -> List<&2,FileRow>:
  match rows:
    case Nil{}: acc
    case h <> t: grouped(t,group(h,acc))

# ---- fixed point: weights and scores in 1/1000 units, integer arithmetic

def frac_next(big: Bool,m2: Nat) -> Nat:
  match big:
    case True{}: Nat.div(m2,2n)
    case False{}: m2

def squared(+m: Nat) -> Nat:
  Nat.div(Nat.mul(m,m),32768n)

def frac(fuel: Nat,+m: Nat,acc: Nat) -> Nat:
  match fuel:
    case 0n: acc
    case 1n+f: frac(f,frac_next(Nat.is_ge(squared(m),65536n),squared(m)),Nat.add(Nat.mul(acc,2n),Bool.pick(Nat,Nat.is_ge(squared(m),65536n),1n,0n)))

def mantissa(small: Bool,x: Nat,+e: Nat) -> Nat:
  match small:
    case True{}: Nat.mul(x,Nat.pow(2n,Nat.sub(15n,e)))
    case False{}: Nat.div(x,Nat.pow(2n,Nat.sub(e,15n)))

# log2(x) in 1/1024 units for 1 <= x < 2^32: the integer part, then ten fraction bits by
# repeated squaring of the mantissa in Q15.
def log2fx(+x: Nat) -> Nat:
  +e = U32.log2(U32.from_nat(x))
  Nat.add(Nat.mul(1024n,e),frac(10n,mantissa(Nat.is_le(e,15n),x,e),0n))

# log2((count + 1) / (df + 1/2)), the BM25 IDF up to the factor ln 2, in 1/1000 units.
def idf(+count: Nat,+df: Nat) -> Nat:
  Nat.div(Nat.mul(Nat.sub(log2fx(Nat.add(Nat.mul(2n,count),2n)),log2fx(Nat.add(Nat.mul(2n,df),1n))),1000n),1024n)

def average(count: Nat,total: Nat) -> Nat:
  match count:
    case 0n: 0n
    case 1n+p: Nat.div(Nat.mul(total,1000n),1n+p)

def ratio(avg: Nat,len: Nat) -> Nat:
  match avg:
    case 0n: 0n
    case 1n+p: Nat.div(Nat.mul(len,1000000n),1n+p)

# One BM25 term, k1 = 1.2 and b = 0.75: weight * f * 2.2 / (f + 1.2 * (0.25 + 0.75 * len / avg)).
def bm25(w: Nat,+f: Nat,len: Nat,total: Nat,count: Nat) -> Nat:
  Nat.div(Nat.mul(w,Nat.div(Nat.mul(f,2200000n),Nat.add(Nat.mul(f,1000n),Nat.div(Nat.add(3000n,Nat.mul(9n,ratio(average(count,total),len))),10n)))),1000n)

def rrf(+rank: Nat) -> Nat:
  Nat.div(1000000000n,Nat.add(60n,rank))

# ---- row parsing helpers

def num(m: Maybe<&2,Nat>) -> Nat:
  match m:
    case Some{n}: n
    case None{}: 0n

def valid(m: Maybe<&2,Nat>) -> Bool:
  match m:
    case Some{_}: True{}
    case None{}: False{}

def field_nat(+s: String) -> Nat:
  num(Nat.read(s))

def field_ok(+s: String) -> Bool:
  valid(Nat.read(s))

type Ent is Data:
  Ent{k: Nat, p: Nat, d: Nat, b: Nat, ok: Bool}

def ent_of(fields: List<&2,String>) -> Ent:
  match fields:
    case +k <> +p <> +d <> +b <> Nil{}: Ent{field_nat(k),field_nat(p),field_nat(d),field_nat(b),field_ok(k) && field_ok(p) && field_ok(d) && field_ok(b)}
    case +k <> +c <> Nil{}: Ent{field_nat(k),0n,0n,field_nat(c),field_ok(k) && field_ok(c)}
    case _: Ent{0n,0n,0n,0n,False{}}

def ents(xs: List<&2,String>) -> List<&2,Ent>:
  match xs:
    case Nil{}: Nil{}
    case h <> t: ent_of(String.split(h,':')) <> ents(t)

# Entries are well formed, strictly ascending and below the term count.
def ents_ok(es: List<&2,Ent>,+next: Nat,+count: Nat) -> Bool:
  match es:
    case Nil{}: True{}
    case Ent{+k,_,_,_,ok} <> t: ok && Nat.is_ge(k,next) && Nat.is_lt(k,count) && ents_ok(t,1n+k,count)

def ent_here(es: List<&2,Ent>,+i: Nat) -> Tri:
  match es:
    case Ent{+k,p,d,b,_} <> _: Bool.pick(Tri,Nat.is_eq(k,i),Tri{p,d,b},Tri{0n,0n,0n})
    case Nil{}: Tri{0n,0n,0n}

def ent_after(+es: List<&2,Ent>,+i: Nat) -> List<&2,Ent>:
  match es:
    case Ent{+k,_,_,_,_} <> t: Bool.pick(List<&2,Ent>,Nat.is_eq(k,i),t,es)
    case Nil{}: Nil{}

# The dense vector of ascending entries, up to the last entry.
def dense(fuel: Nat,+i: Nat,+es: List<&2,Ent>) -> List<&2,Tri>:
  match fuel es:
    case _ Nil{}: Nil{}
    case 0n _: Nil{}
    case 1n+f _: ent_here(es,i) <> dense(f,1n+i,ent_after(es,i))

def cdf_of(ts: List<&2,Tri>) -> List<&2,Nat>:
  match ts:
    case Nil{}: Nil{}
    case Tri{_,_,b} <> t: b <> cdf_of(t)

# ---- stage 2: FILE_RANK

type Parsed is Data:
  Parsed{files: List<&2,FileRow>, wins: List<&2,WStat>, excl: List<&2,String>, ok: Bool}

def with_file(path: String,+flen: String,+es: List<&2,Ent>,+count: Nat,p: Parsed) -> Parsed:
  match p:
    case Parsed{fs,ws,xs,ok}: Parsed{FileRow{path,FStat{field_nat(flen),dense(count,0n,es)}} <> fs,ws,xs,ok && field_ok(flen) && ents_ok(es,0n,count)}

def with_windows(+n: String,+tok: String,+es: List<&2,Ent>,+count: Nat,p: Parsed) -> Parsed:
  match p:
    case Parsed{fs,ws,xs,ok}: Parsed{fs,WStat{field_nat(n),field_nat(tok),cdf_of(dense(count,0n,es))} <> ws,xs,ok && field_ok(n) && field_ok(tok) && ents_ok(es,0n,count)}

def with_excluded(path: String,p: Parsed) -> Parsed:
  match p:
    case Parsed{fs,ws,xs,ok}: Parsed{fs,ws,path <> xs,ok}

def invalid(p: Parsed) -> Parsed:
  match p:
    case Parsed{fs,ws,xs,_}: Parsed{fs,ws,xs,False{}}

def parse_row(r: W.Record,+count: Nat,p: Parsed) -> Parsed:
  match r:
    case W.Row{"FILE" <> path <> flen <> es}: with_file(path,flen,ents(es),count,p)
    case W.Row{"WINDOWS" <> n <> tok <> es}: with_windows(n,tok,ents(es),count,p)
    case W.Row{"X" <> path <> Nil{}}: with_excluded(path,p)
    case W.Row{"" <> Nil{}}: p
    case _: invalid(p)

# Rows in arrival order (files and windows are reversed back).
def parse_rows(rows: List<&2,String>,+count: Nat,p: Parsed) -> Parsed:
  match rows:
    case Nil{}: p
    case h <> t: parse_rows(t,count,parse_row(W.parse(h),count,p))

def present(t: Tri) -> Nat:
  match t:
    case Tri{p,d,b}: Bool.pick(Nat,Nat.is_eq(Nat.add(p,Nat.add(d,b)),0n),0n,1n)

def presence(ts: List<&2,Tri>) -> List<&2,Nat>:
  match ts:
    case Nil{}: Nil{}
    case h <> t: present(h) <> presence(t)

# Files containing each term, total body tokens.
type Corpus is Data:
  Corpus{files: Nat, tokens: Nat, fdf: List<&2,Nat>}

def corpus(rows: List<&2,FileRow>,acc: Corpus) -> Corpus:
  match rows acc:
    case Nil{} a: a
    case FileRow{_,FStat{l,v}} <> t Corpus{f,tl,df}: corpus(t,Corpus{1n+f,Nat.add(tl,l),nadd(df,presence(v))})

def at(xs: List<&2,Nat>,n: Nat) -> Nat:
  match xs n:
    case Nil{} _: 0n
    case h <> _ 0n: h
    case _ <> t 1n+p: at(t,p)

def weights(fuel: Nat,+i: Nat,+count: Nat,+df: List<&2,Nat>) -> List<&2,Nat>:
  match fuel:
    case 0n: Nil{}
    case 1n+f: idf(count,at(df,i)) <> weights(f,1n+i,count,df)

def file_score(v: List<&2,Tri>,ws: List<&2,Nat>,+len: Nat,+total: Nat,+count: Nat) -> Nat:
  match v ws:
    case Tri{p,d,b} <> vt w <> wt: Nat.add(bm25(w,Nat.add(Nat.mul(3n,p),Nat.add(Nat.mul(2n,d),b)),len,total,count),file_score(vt,wt,len,total,count))
    case _ _: 0n

type Scored is Data:
  Scored{score: Nat, path: String}

# Score descending, then path ascending.
def not_after(c: Cmp) -> Bool:
  match c:
    case GT{}: False{}
    case _: True{}

def before(a: Scored,b: Scored) -> Bool:
  match a b:
    case Scored{+sa,pa} Scored{+sb,pb}: Nat.is_gt(sa,sb) || (Nat.is_eq(sa,sb) && not_after(order(pa,pb)))

def keep_scored(s: Nat,path: String,rest: List<&2,Scored>) -> List<&2,Scored>:
  match s:
    case 0n: rest
    case 1n+p: Scored{1n+p,path} <> rest

def scored(rows: List<&2,FileRow>,+ws: List<&2,Nat>,+total: Nat,+count: Nat) -> List<&2,Scored>:
  match rows:
    case Nil{}: Nil{}
    case FileRow{path,FStat{l,v}} <> t: keep_scored(file_score(v,ws,l,total,count),path,scored(t,ws,total,count))

def take(xs: List<&2,Scored>,n: Nat) -> List<&2,Scored>:
  match xs n:
    case h <> t 1n+p: h <> take(t,p)
    case _ _: Nil{}

def pool_rows(xs: List<&2,Scored>,+rank: Nat) -> String:
  match xs:
    case Nil{}: ""
    case Scored{s,p} <> t: W.row(["POOL",p,Nat.show(rank),Nat.show(s)]) ++ pool_rows(t,1n+rank)

def idf_rows(fw: List<&2,Nat>,ww: List<&2,Nat>,+k: Nat) -> String:
  match fw ww:
    case f <> ft w <> wt: W.row(["IDF",Nat.show(k),Nat.show(f),Nat.show(w)]) ++ idf_rows(ft,wt,1n+k)
    case _ _: ""

def excluded_one(sc: L.Scan,path: String) -> String:
  match sc:
    case L.Scan{_,_,_,Nil{},_}: ""
    case _: W.row(["EXCLUDED",path])

def excluded_rows(xs: List<&2,String>,+node: L.Trie) -> String:
  match xs:
    case Nil{}: ""
    case +h <> t: excluded_one(L.scan(h,node),h) ++ excluded_rows(t,node)

def ranked_rows(+ranked: List<&2,Scored>,+files: Nat,+g: Nat,+m: Nat) -> String:
  W.row(["RANKED",Nat.show(files),Nat.show(List.length(&2,Scored,ranked)),Nat.show(g),Nat.show(m)]) ++ pool_rows(take(ranked,m),1n)

def rank_out(c: Corpus,w: WStat,grouped_rows: List<&2,FileRow>,+count: Nat,+g: Nat,+m: Nat,excl: List<&2,String>,+node: L.Trie) -> String:
  match c w:
    case Corpus{+f,+tl,+df} WStat{+n,+tok,+cdf}: ranked_rows(List.sort(~Scored,~(a => b => before(a,b)),scored(grouped_rows,weights(count,0n,f,df),tl,f)),f,g,m) ++ idf_rows(weights(count,0n,f,df),weights(count,0n,n,cdf),0n) ++ W.row(["NORM",Nat.show(n),Nat.show(tok)]) ++ excluded_rows(List.reverse(&2,String,excl),node)

def rank_grouped(+rows: List<&2,FileRow>,w: WStat,+count: Nat,+g: Nat,+m: Nat,xs: List<&2,String>,+node: L.Trie) -> String:
  rank_out(corpus(rows,Corpus{0n,0n,Nil{}}),w,rows,count,g,m,xs,node)

def rank_parsed(p: Parsed,+count: Nat,+g: Nat,+m: Nat,+node: L.Trie) -> String:
  match p:
    case Parsed{fs,ws,xs,True{}}: rank_grouped(grouped(List.reverse(&2,FileRow,fs),Nil{}),wsum(ws),count,g,m,xs,node)
    case Parsed{_,_,_,False{}}: "ERROR\tmalformed ranking row\n"

def rank_terms(+ts: List<&2,String>,+g: Nat,rows: List<&2,String>) -> String:
  rank_parsed(parse_rows(rows,List.length(&2,String,ts),Parsed{Nil{},Nil{},Nil{},True{}}),List.length(&2,String,ts),g,Nat.mul(4n,g),L.trie(ts))

# The shortlist size G a caller may ask for; the pool is 4G.
def shortlist(m: Maybe<&2,Nat>) -> Nat:
  match m:
    case Some{+g}: Bool.pick(Nat,Nat.is_ge(g,1n) && Nat.is_le(g,512n),g,0n)
    case None{}: 0n

def rank_sized(g: Nat,q: String,rows: List<&2,String>) -> String:
  match g:
    case 0n: "ERROR\tshortlist must be 1..512\n"
    case 1n+p: rank_terms(L.terms(q),1n+p,rows)

def rank_head(q: W.Record,g: W.Record,rows: List<&2,String>) -> String:
  match q g:
    case W.Row{"QUERY" <> text <> Nil{}} W.Row{"SHORTLIST" <> n <> Nil{}}: rank_sized(shortlist(Nat.read(n)),text,rows)
    case _ _: "ERROR\tquery and shortlist required\n"

# RANKED <files> <files matched> <G> <M>, POOL <path> <rank> <score> for the first M = 4G files by
# BM25F (path x3, definition names x2, body), IDF <k> <file weight> <window weight>,
# NORM <windows> <window tokens>, EXCLUDED <path> for X paths whose tokens match a term.
def rank_process(rows: List<&2,String>) -> String:
  match rows:
    case q <> g <> t: rank_head(W.parse(q),W.parse(g),t)
    case _: "ERROR\tquery and shortlist required\n"

# ---- stage 3: SELECT_PASSAGE

# The pooled file open in a chunk and its best window so far (first of equal scores).
type Best is Data:
  Best{path: String, pn: Nat, phits: List<&2,U32>, id: String, score: Nat, has: Bool}

type Pass is Data:
  Pass{best: Best, out: List<&2,String>}

def best_row(b: Best,out: List<&2,String>) -> List<&2,String>:
  match b:
    case Best{_,_,_,_,_,False{}}: out
    case Best{path,_,_,id,score,True{}}: W.row(["BEST",path,id,Nat.show(score)]) <> out

def window_score(ts: List<&2,Tally>,+ww: List<&2,Nat>,+len: Nat,+tok: Nat,+n: Nat) -> Nat:
  match ts:
    case Nil{}: 0n
    case Tally{k,c} <> t: Nat.add(bm25(at(ww,U32.to_nat(k)),c,len,tok,n),window_score(t,ww,len,tok,n))

def better(win: Bool,id: String,score: Nat,old: Best) -> Best:
  match win old:
    case True{} Best{path,pn,phits,_,_,_}: Best{path,pn,phits,id,score,True{}}
    case False{} b: b

def scored_window(+score: Nat,id: String,old: Best) -> Best:
  match old:
    case Best{path,pn,phits,oid,+os,+has}: better(Bool.not(has) || Nat.is_gt(score,os),id,score,Best{path,pn,phits,oid,os,has})

def pass_window(sc: L.Scan,id: String,b: Best,+ww: List<&2,Nat>,+tok: Nat,+n: Nat) -> Best:
  match sc b:
    case L.Scan{_,_,wn,hits,_} Best{path,+pn,+phits,oid,os,has}: scored_window(window_score(tallies(sorted(List.append(&2,U32,hits,phits)),Nil{}),ww,Nat.add(wn,pn),tok,n),id,Best{path,pn,phits,oid,os,has})

def pass_open(path: String,sc: L.Scan) -> Best:
  match sc:
    case L.Scan{_,_,n,hits,_}: Best{path,n,hits,"",0n,False{}}

def pass_file(same: Bool,+path: String,+node: L.Trie,p: Pass) -> Pass:
  match same p:
    case True{} q: q
    case False{} Pass{b,out}: Pass{pass_open(path,L.scan(path,node)),best_row(b,out)}

def pass_best_path(b: Best) -> String:
  match b:
    case Best{path,_,_,_,_,_}: path

def pass_same(p: Pass,+path: String) -> Bool:
  match p:
    case Pass{b,_}: String.eq(pass_best_path(b),path)

def pass_add(p: Pass,id: String,excerpt: String,+node: L.Trie,+ww: List<&2,Nat>,+tok: Nat,+n: Nat) -> Pass:
  match p:
    case Pass{b,out}: Pass{pass_window(L.scan(excerpt,node),id,b,ww,tok,n),out}

def pass_admitted(ok: Bool,+path: String,id: String,excerpt: String,+node: L.Trie,+ww: List<&2,Nat>,+tok: Nat,+n: Nat,+p: Pass) -> Pass:
  match ok:
    case True{}: pass_add(pass_file(pass_same(p,path),path,node,p),id,excerpt,node,ww,tok,n)
    case False{}: p

def pass_row(r: W.Record,+node: L.Trie,+ww: List<&2,Nat>,+tok: Nat,+n: Nat,p: Pass) -> Pass:
  match r:
    case W.Row{"C" <> id <> +path <> _ <> _ <> _ <> +excerpt <> Nil{}}: pass_admitted(P.disclosable(path,excerpt),path,id,excerpt,node,ww,tok,n,p)
    case _: p

def pass_rows(rows: List<&2,String>,+node: L.Trie,+ww: List<&2,Nat>,+tok: Nat,+n: Nat,p: Pass) -> Pass:
  match rows:
    case Nil{}: p
    case h <> t: pass_rows(t,node,ww,tok,n,pass_row(W.parse(h),node,ww,tok,n,p))

def pass_done(p: Pass) -> String:
  match p:
    case Pass{b,out}: String.concat(List.reverse(&2,String,best_row(b,out)))
def idf_take(ok: Bool,w: Nat,h: String,t: List<&2,String>,acc: List<&2,Nat>,next: Nat -> Weights) -> Weights:
  match ok:
    case True{}: next(w)
    case False{}: Weights{h <> t,List.reverse(&2,Nat,acc)}

type Weights is Data:
  Weights{rows: List<&2,String>, ws: List<&2,Nat>}

def idf_at(r: W.Record,+k: Nat,h: String,t: List<&2,String>,acc: List<&2,Nat>,next: Nat -> Weights) -> Weights:
  match r:
    case W.Row{"IDF" <> +i <> _ <> +w <> Nil{}}: idf_take(field_ok(i) && field_ok(w) && Nat.is_eq(field_nat(i),k),field_nat(w),h,t,acc,next)
    case _: Weights{h <> t,List.reverse(&2,Nat,acc)}

# The leading IDF rows k = 0, 1, ... give the window weights; the rows after them remain.
def idf_weights(rows: List<&2,String>,+k: Nat,+acc: List<&2,Nat>) -> Weights:
  match rows:
    case Nil{}: Weights{Nil{},List.reverse(&2,Nat,acc)}
    case +h <> +t: idf_at(W.parse(h),k,h,t,acc,w => idf_weights(t,1n+k,w <> acc))

def passage_run(ok: Bool,rows: List<&2,String>,ws: List<&2,Nat>,+node: L.Trie,+n: Nat,+tok: Nat) -> String:
  match ok:
    case True{}: pass_done(pass_rows(rows,node,ws,tok,n,Pass{Best{"",0n,Nil{},"",0n,False{}},Nil{}}))
    case False{}: "ERROR\tweights required\n"

def passage_weights(w: Weights,+count: Nat,+node: L.Trie,+n: Nat,+tok: Nat) -> String:
  match w:
    case Weights{rows,+ws}: passage_run(Nat.is_eq(List.length(&2,Nat,ws),count),rows,ws,node,n,tok)

def passage_ready(ok: Bool,+ts: List<&2,String>,+n: String,+tok: String,rows: List<&2,String>) -> String:
  match ok:
    case True{}: passage_weights(idf_weights(rows,0n,Nil{}),List.length(&2,String,ts),L.trie(ts),field_nat(n),field_nat(tok))
    case False{}: "ERROR\tnorm required\n"

def passage_norm(r: W.Record,+ts: List<&2,String>,rows: List<&2,String>) -> String:
  match r:
    case W.Row{"NORM" <> +n <> +tok <> Nil{}}: passage_ready(field_ok(n) && field_ok(tok),ts,n,tok,rows)
    case _: "ERROR\tnorm required\n"

def passage_head(q: W.Record,rows: List<&2,String>) -> String:
  match q rows:
    case W.Row{"QUERY" <> text <> Nil{}} h <> t: passage_norm(W.parse(h),L.terms(text),t)
    case _ _: "ERROR\tquery required\n"

# BEST <path> <candidate id> <score> for each file with an admitted window: its best
# window by BM25 with the window weights; ties go to the earlier window.
def passage_process(rows: List<&2,String>) -> String:
  match rows:
    case h <> t: passage_head(W.parse(h),t)
    case Nil{}: "ERROR\tquery required\n"

# ---- stage 4: SELECT_FUSE

type CRow is Data:
  CRow{id: String, path: String, digest: String, start: String, end: String, excerpt: String, grammar: String}

type Won is Data:
  Won{id: String, score: Nat, has: Bool}

type Pooled is Data:
  Pooled{path: String, rank: Nat}

type Fuse is Data:
  Fuse{pool: List<&2,Pooled>, bests: Map<&2,Won>, rows: Map<&2,CRow>, order: List<&2,CRow>, ok: Bool}

def won_pick(win: Bool,id: String,score: Nat,old: Won) -> Won:
  match win:
    case True{}: Won{id,score,True{}}
    case False{}: old

# A file split across passage chunks sends several BEST rows: the highest score wins,
# the first row on ties (chunks keep candidate order).
def put_best(pair: Map<&2,Won> & Won,path: String,id: String,+score: Nat) -> Map<&2,Won>:
  match pair:
    case (m,Won{oid,+os,+has}): Map.set(&2,Won,m,path,won_pick(Bool.not(has) || Nat.is_gt(score,os),id,score,Won{oid,os,has}))

def no_won() -> Won:
  Won{"",0n,False{}}

def fuse_best(+path: String,id: String,+score: String,f: Fuse) -> Fuse:
  match f:
    case Fuse{pool,bests,rows,order,ok}: Fuse{pool,put_best(Map.get(Won,no_won(),bests,path),path,id,field_nat(score)),rows,order,ok && field_ok(score)}
def crow_id(c: CRow) -> String:
  match c:
    case CRow{id,_,_,_,_,_,_}: id

def fuse_pool(path: String,+rank: String,f: Fuse) -> Fuse:
  match f:
    case Fuse{pool,bests,rows,order,ok}: Fuse{Pooled{path,field_nat(rank)} <> pool,bests,rows,order,ok && field_ok(rank)}

def fuse_row(ok: Bool,+c: CRow,f: Fuse) -> Fuse:
  match ok f:
    case True{} Fuse{pool,bests,rows,order,good}: Fuse{pool,bests,Map.set(&2,CRow,rows,crow_id(c),c),c <> order,good}
    case False{} g: g

def fuse_invalid(f: Fuse) -> Fuse:
  match f:
    case Fuse{pool,bests,rows,order,_}: Fuse{pool,bests,rows,order,False{}}

def fuse_parse(r: W.Record,f: Fuse) -> Fuse:
  match r:
    case W.Row{"POOL" <> path <> rank <> _ <> Nil{}}: fuse_pool(path,rank,f)
    case W.Row{"BEST" <> path <> id <> score <> Nil{}}: fuse_best(path,id,score,f)
    case W.Row{"C" <> id <> +path <> digest <> start <> end <> +excerpt <> Nil{}}: fuse_row(P.disclosable(path,excerpt),CRow{id,path,digest,start,end,excerpt,F.grammar(path)},f)
    case W.Row{"" <> Nil{}}: f
    case _: fuse_invalid(f)

def fuse_rows(rows: List<&2,String>,f: Fuse) -> Fuse:
  match rows:
    case Nil{}: f
    case h <> t: fuse_rows(t,fuse_parse(W.parse(h),f))

type Entrant is Data:
  Entrant{path: String, rank: Nat, best: Nat, id: String, has: Bool}

type Gather is Data:
  Gather{bests: Map<&2,Won>, out: List<&2,Entrant>}

def gather_got(pair: Map<&2,Won> & Won,path: String,rank: Nat,out: List<&2,Entrant>) -> Gather:
  match pair:
    case (m,Won{id,score,has}): Gather{m,Entrant{path,rank,score,id,has} <> out}

def gather_one(+path: String,rank: Nat,g: Gather) -> Gather:
  match g:
    case Gather{m,out}: gather_got(Map.get(Won,no_won(),m,path),path,rank,out)

def gather(ps: List<&2,Pooled>,g: Gather) -> Gather:
  match ps:
    case Nil{}: g
    case Pooled{path,rank} <> t: gather(t,gather_one(path,rank,g))

def all_have(es: List<&2,Entrant>) -> Bool:
  match es:
    case Nil{}: True{}
    case Entrant{_,_,_,_,has} <> t: has && all_have(t)

# Best-window score descending, then path ascending.
def best_first(a: Entrant,b: Entrant) -> Bool:
  match a b:
    case Entrant{pa,_,+sa,_,_} Entrant{pb,_,+sb,_,_}: Nat.is_gt(sa,sb) || (Nat.is_eq(sa,sb) && not_after(order(pa,pb)))

type Fused is Data:
  Fused{score: Nat, path: String, id: String}

def fused_first(a: Fused,b: Fused) -> Bool:
  match a b:
    case Fused{+sa,pa,_} Fused{+sb,pb,_}: Nat.is_gt(sa,sb) || (Nat.is_eq(sa,sb) && not_after(order(pa,pb)))

# Reciprocal-rank fusion (k = 60) of the file rank and the best-window rank (both from 1).
def fuse_ranks(es: List<&2,Entrant>,+brank: Nat) -> List<&2,Fused>:
  match es:
    case Nil{}: Nil{}
    case Entrant{path,rank,_,id,_} <> t: Fused{Nat.add(rrf(rank),rrf(brank)),path,id} <> fuse_ranks(t,1n+brank)

type Chosen is Data:
  Chosen{row: CRow, score: Nat}

type Pick is Data:
  Pick{rows: Map<&2,CRow>, chosen: List<&2,Chosen>, left: Nat, ok: Bool}

def no_row() -> CRow:
  CRow{"","","","","","",""}

def same_group(chosen: List<&2,Chosen>,+excerpt: String,+grammar: String) -> Bool:
  match chosen:
    case Nil{}: False{}
    case Chosen{CRow{_,_,_,_,_,e,g},_} <> t: (String.eq(grammar,g) && String.eq(excerpt,e)) || same_group(t,excerpt,grammar)

def row_group(c: CRow,chosen: List<&2,Chosen>) -> Bool:
  match c:
    case CRow{_,_,_,_,_,e,g}: same_group(chosen,e,g)

def pick_new(dup: Bool,c: CRow,+score: Nat,m: Map<&2,CRow>,chosen: List<&2,Chosen>,left: Nat,ok: Bool) -> Pick:
  match dup left:
    case True{} _: Pick{m,chosen,left,ok}
    case False{} 0n: Pick{m,chosen,0n,ok}
    case False{} 1n+p: Pick{m,Chosen{c,score} <> chosen,p,ok}

def pick_got(pair: Map<&2,CRow> & CRow,+score: Nat,+chosen: List<&2,Chosen>,left: Nat,ok: Bool) -> Pick:
  match pair:
    case (m,+c): pick_new(row_group(c,chosen),c,score,m,chosen,left,ok && Bool.not(String.eq(crow_id(c),"")))

def pick_one(f: Fused,p: Pick) -> Pick:
  match f p:
    case Fused{_,_,_} Pick{m,chosen,0n,ok}: Pick{m,chosen,0n,ok}
    case Fused{score,_,id} Pick{m,chosen,1n+l,ok}: pick_got(Map.get(CRow,no_row(),m,id),score,chosen,1n+l,ok)

def pick_all(fs: List<&2,Fused>,p: Pick) -> Pick:
  match fs:
    case Nil{}: p
    case h <> t: pick_all(t,pick_one(h,p))

def alias_row(c: CRow,+primary: String,+grammar: String,+excerpt: String) -> String:
  match c:
    case CRow{+id,path,digest,start,end,e,g}: choose(Bool.not(String.eq(id,primary)) && String.eq(grammar,g) && String.eq(excerpt,e),W.row(["ALIAS",primary,id,path,digest,start,end]),"")

def alias_rows(rows: List<&2,CRow>,+primary: String,+grammar: String,+excerpt: String) -> String:
  match rows:
    case Nil{}: ""
    case h <> t: alias_row(h,primary,grammar,excerpt) ++ alias_rows(t,primary,grammar,excerpt)

def candidate_rows(cs: List<&2,Chosen>,+order: List<&2,CRow>) -> String:
  match cs:
    case Nil{}: ""
    case Chosen{CRow{+id,path,digest,start,end,+excerpt,+grammar},score} <> t: W.row(["CANDIDATE",id,Nat.show(score),path,digest,start,end,excerpt]) ++ alias_rows(order,id,grammar,excerpt) ++ candidate_rows(t,order)

def stop_row(cs: List<&2,Chosen>) -> String:
  match cs:
    case Nil{}: W.row(["STOP","0"])
    case Chosen{_,score} <> _: W.row(["STOP",Nat.show(score)])

def fuse_emit(p: Pick,order: List<&2,CRow>) -> String:
  match p:
    case Pick{_,+chosen,_,True{}}: W.row(["PROVENANCE_LIMIT","16"]) ++ S.rubric() ++ candidate_rows(List.reverse(&2,Chosen,chosen),List.reverse(&2,CRow,order)) ++ stop_row(chosen)
    case Pick{_,_,_,False{}}: "ERROR\tbest window missing\n"

def fuse_ranked(have: Bool,+es: List<&2,Entrant>,rows: Map<&2,CRow>,order: List<&2,CRow>,+g: Nat) -> String:
  match have:
    case True{}: fuse_emit(pick_all(List.sort(~Fused,~(a => b => fused_first(a,b)),fuse_ranks(List.sort(~Entrant,~(a => b => best_first(a,b)),es),1n)),Pick{rows,Nil{},g,True{}}),order)
    case False{}: "ERROR\tbest window missing\n"

def fuse_gathered(ga: Gather,rows: Map<&2,CRow>,order: List<&2,CRow>,+g: Nat) -> String:
  match ga:
    case Gather{_,+es}: fuse_ranked(all_have(es),es,rows,order,g)

def fuse_parsed(f: Fuse,+g: Nat) -> String:
  match f:
    case Fuse{pool,bests,rows,order,True{}}: fuse_gathered(gather(pool,Gather{bests,Nil{}}),rows,order,g)
    case Fuse{_,_,_,_,False{}}: "ERROR\tmalformed fusion row\n"

def fuse_sized(g: Nat,rows: List<&2,String>) -> String:
  match g:
    case 0n: "ERROR\tshortlist must be 1..512\n"
    case 1n+p: fuse_parsed(fuse_rows(rows,Fuse{Nil{},Map.new(&2,Won),Map.new(&2,CRow),Nil{},True{}}),1n+p)

def fuse_head(r: W.Record,rows: List<&2,String>) -> String:
  match r:
    case W.Row{"SHORTLIST" <> n <> Nil{}}: fuse_sized(shortlist(Nat.read(n)),rows)
    case _: "ERROR\tshortlist required\n"

# The first G distinct excerpt groups of the pool in fused order, as today's SELECT rows:
# PROVENANCE_LIMIT, RUBRIC, then CANDIDATE <id> <fused score> <path> <digest> <start> <end>
# <excerpt> with every admitted byte-identical row of the same grammar as ALIAS; then STOP
# <fused score of the last group>.
def fuse_process(rows: List<&2,String>) -> String:
  match rows:
    case h <> t: fuse_head(W.parse(h),t)
    case Nil{}: "ERROR\tshortlist required\n"

# A separate bounded page of non-best candidates is compared against every
# selected representative. Grammar and exact excerpt equality remain Bend
# decisions, while the host only transports and binds returned references.
type AliasSource is Data:
  AliasSource{primary: List<&2,CRow>, candidates: List<&2,CRow>, ok: Bool}

def alias_parse(lines: List<&2,String>) -> List<&2,W.Record>:
  match lines:
    case Nil{}: Nil{}
    case h <> t: W.parse(h) <> alias_parse(t)

def alias_add_primary(source: AliasSource,item: CRow) -> AliasSource:
  match source:
    case AliasSource{primary,candidates,ok}: AliasSource{item <> primary,candidates,ok}

def alias_add_candidate(source: AliasSource,item: CRow) -> AliasSource:
  match source:
    case AliasSource{primary,candidates,ok}: AliasSource{primary,item <> candidates,ok}

def alias_invalid(source: AliasSource) -> AliasSource:
  match source:
    case AliasSource{primary,candidates,_}: AliasSource{primary,candidates,False{}}

def alias_collect(rows: List<&2,W.Record>) -> AliasSource:
  match rows:
    case Nil{}: AliasSource{Nil{},Nil{},True{}}
    case W.Row{"PRIMARY" <> id <> +path <> digest <> start <> end <> +excerpt <> Nil{}} <> t:
      alias_add_primary(alias_collect(t),CRow{id,path,digest,start,end,excerpt,F.grammar(path)})
    case W.Row{"C" <> id <> +path <> digest <> start <> end <> +excerpt <> Nil{}} <> t:
      alias_add_candidate(alias_collect(t),CRow{id,path,digest,start,end,excerpt,F.grammar(path)})
    case W.Row{"" <> Nil{}} <> t: alias_collect(t)
    case _ <> t: alias_invalid(alias_collect(t))

def alias_against(primary: List<&2,CRow>,+candidate: CRow) -> String:
  match primary:
    case Nil{}: ""
    case CRow{id,_,_,_,_,+excerpt,+grammar} <> t:
      alias_row(candidate,id,grammar,excerpt) ++ alias_against(t,candidate)

def alias_candidate_row(candidate: CRow,primary: List<&2,CRow>) -> String:
  match candidate:
    case CRow{id,+path,digest,start,end,+excerpt,grammar}:
      choose(P.disclosable(path,excerpt),alias_against(primary,CRow{id,path,digest,start,end,excerpt,grammar}),"")

def alias_candidates(rows: List<&2,CRow>,+primary: List<&2,CRow>) -> String:
  match rows:
    case Nil{}: ""
    case h <> t: alias_candidate_row(h,primary) ++ alias_candidates(t,primary)

def alias_ready(source: AliasSource) -> String:
  match source:
    case AliasSource{primary,candidates,True{}}: alias_candidates(candidates,primary)
    case AliasSource{_,_,False{}}: "ERROR\tmalformed alias page\n"

def alias_process(rows: List<&2,String>) -> String:
  alias_ready(alias_collect(alias_parse(rows)))
