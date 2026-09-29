import Base
import ./Wire.bend as W
import ./Equality.bend as E

# Ranked head plus slices for optional Jev judgments (select, advise).
# Items arrive in deterministic rank order with the exact byte sizes the bridge
# will serialize: each item's state entry and its question. Request 1 judges the
# anchor (the top-ranked items, at most HEAD, within half of each limit); every
# later request judges a slice of at most HEAD further items beside the same
# anchor, which it repeats as state without questions, so each item is judged
# exactly once and always next to the most relevant items. Every request fits
# both jev-1.13 limits with bytes as the token bound: the whole request within
# TOTAL, and the state plus its longest question within STATE. An item that fits
# no request is named omitted; it is never silently dropped.

def HEAD() -> U32:
  8

def TOTAL() -> U32:
  64000

def STATE() -> U32:
  32000

type Item is Data:
  Item{id: String, state: U32, question: U32}

# Fixed bytes: the request envelope without items, and the state without items.
type Fixed is Data:
  Fixed{total: U32, state: U32}

type Size is Data:
  Size{total: U32, state: U32, longest: U32}

def larger(+a: U32,+b: U32) -> U32:
  Bool.pick(U32,U32.is_gt(a,b),a,b)

# Context items add only state; judged items add state and a question.
def context_size(xs: List<&2,Item>,+s: Size) -> Size:
  match xs s:
    case Nil{} s: s
    case Item{_,+st,_} <> t Size{+total,+state,longest}: context_size(t,Size{(total + st : U32),(state + st : U32),longest})

def judged_size(xs: List<&2,Item>,+s: Size) -> Size:
  match xs s:
    case Nil{} s: s
    case Item{_,+st,+q} <> t Size{+total,+state,+longest}: judged_size(t,Size{(total + st + q : U32),(state + st : U32),larger(longest,q)})

def request_size(+fixed: Fixed,+context: List<&2,Item>,+judged: List<&2,Item>) -> Size:
  match fixed:
    case Fixed{+total,+state}: judged_size(judged,context_size(context,Size{total,state,0}))

def fits_within(+s: Size,+total: U32,+state: U32) -> Bool:
  match s:
    case Size{+t,+st,+q}: U32.is_le(t,total) && U32.is_le((st + q : U32),state)

def count(xs: List<&2,Item>) -> U32:
  U32.from_nat(List.length(&2,Item,xs))

def append(xs: List<&2,Item>,+x: Item) -> List<&2,Item>:
  List.append(&2,Item,xs,[x])

type Split is Data:
  Split{kept: List<&2,Item>, rest: List<&2,Item>}

# The anchor: in rank order, each item joins while the anchor holds fewer than
# HEAD items and fits within half of each limit; others wait for the slices.
def joined(joins: Bool,+item: Item,+kept: List<&2,Item>,+rest: List<&2,Item>) -> Split:
  match joins:
    case True{}: Split{append(kept,item),rest}
    case False{}: Split{kept,item <> rest}

def anchor_go(order: List<&2,Item>,+fixed: Fixed,split: Split) -> Split:
  match order split:
    case Nil{} Split{kept,rest}: Split{kept,List.reverse(&2,Item,rest)}
    case +item <> tail Split{+kept,rest}: anchor_go(tail,fixed,joined(U32.is_lt(count(kept),HEAD()) && fits_within(request_size(fixed,Nil{},append(kept,item)),U32.shr(TOTAL()),U32.shr(STATE())),item,kept,rest))

type Sliced is Data:
  Sliced{done: List<&2,List<&2,Item>>, current: List<&2,Item>, omitted: List<&2,Item>}

def close(+done: List<&2,List<&2,Item>>,+current: List<&2,Item>) -> List<&2,List<&2,Item>>:
  match current:
    case Nil{}: done
    case _: List.append(&2,List<&2,Item>,done,[current])

def fits_full(+fixed: Fixed,+anchor: List<&2,Item>,+judged: List<&2,Item>) -> Bool:
  fits_within(request_size(fixed,anchor,judged),TOTAL(),STATE())

# Each remaining item joins the open slice, starts a new one, or is omitted when
# it cannot fit even alone beside the anchor.
def offered(+state: Sliced,+item: Item,joined: Bool,alone: Bool) -> Sliced:
  match state:
    case Sliced{+done,+current,+omitted}: Bool.pick(Sliced,joined,Sliced{done,append(current,item),omitted},Bool.pick(Sliced,alone,Sliced{close(done,current),[item],omitted},Sliced{done,current,append(omitted,item)}))

def open_slice(state: Sliced) -> List<&2,Item>:
  match state:
    case Sliced{_,current,_}: current

def slice_go(rest: List<&2,Item>,+fixed: Fixed,+anchor: List<&2,Item>,+state: Sliced) -> Sliced:
  match rest:
    case Nil{}: state
    case +item <> tail:
      +current = open_slice(state)
      slice_go(tail,fixed,anchor,offered(state,item,U32.is_lt(count(current),HEAD()) && fits_full(fixed,anchor,append(current,item)),fits_full(fixed,anchor,[item])))

type Plan is Data:
  Plan{anchor: List<&2,Item>, slices: List<&2,List<&2,Item>>, omitted: List<&2,Item>}

def planned(+anchor: List<&2,Item>,state: Sliced) -> Plan:
  match state:
    case Sliced{done,current,omitted}: Plan{anchor,close(done,current),omitted}

def split_kept(s: Split) -> List<&2,Item>:
  match s:
    case Split{k,_}: k

def split_rest(s: Split) -> List<&2,Item>:
  match s:
    case Split{_,r}: r

def plan(+fixed: Fixed,order: List<&2,Item>) -> Plan:
  +split = anchor_go(order,fixed,Split{Nil{},Nil{}})
  planned(split_kept(split),slice_go(split_rest(split),fixed,split_kept(split),Sliced{Nil{},Nil{},Nil{}}))

def ids(xs: List<&2,Item>) -> List<&2,String>:
  match xs:
    case Nil{}: Nil{}
    case Item{id,_,_} <> t: id <> ids(t)

def slice_rows(xs: List<&2,List<&2,Item>>,+index: U32) -> String:
  match xs:
    case Nil{}: ""
    case +h <> t: W.row("REQUEST" <> U32.show(index) <> "slice" <> ids(h)) ++ slice_rows(t,(index + 1 : U32))

def omitted_rows(xs: List<&2,Item>) -> String:
  match xs:
    case Nil{}: ""
    case Item{id,_,_} <> t: W.row(["OMITTED",id]) ++ omitted_rows(t)

# Rows: WINDOW limits, REQUEST index kind judged-ids..., OMITTED id.
def emit(p: Plan) -> String:
  match p:
    case Plan{+anchor,slices,omitted}: W.row(["WINDOW",U32.show(HEAD()),U32.show(TOTAL()),U32.show(STATE())]) ++ W.row("REQUEST" <> "0" <> "head" <> ids(anchor)) ++ slice_rows(slices,1) ++ omitted_rows(omitted)

type Answered is Data:
  Answered{request: String, id: String, label: String}

type Parsed is Data:
  Parsed{fixed: Fixed, items: List<&2,Item>, results: List<&2,Answered>, valid: Bool}

def number(value: Maybe<&2,U32>) -> U32:
  match value:
    case Some{n}: n
    case None{}: 0

def readable(value: Maybe<&2,U32>) -> Bool:
  match value:
    case Some{_}: True{}
    case None{}: False{}

def add_line(record: W.Record,rest: Parsed) -> Parsed:
  match record rest:
    case W.Row{"ITEM" <> +id <> +st <> +q <> Nil{}} Parsed{f,xs,rs,v}: Parsed{f,Item{id,number(U32.read(st)),number(U32.read(q))} <> xs,rs,v && readable(U32.read(st)) && readable(U32.read(q)) && Bool.not(String.is_empty(id))}
    case W.Row{"RESULT" <> request <> id <> label <> Nil{}} Parsed{f,xs,rs,v}: Parsed{f,xs,Answered{request,id,label} <> rs,v}
    case _ Parsed{f,xs,rs,_}: Parsed{f,xs,rs,False{}}

def parse_lines(lines: List<&2,String>) -> Parsed:
  match lines:
    case Nil{}: Parsed{Fixed{0,0},Nil{},Nil{},True{}}
    case "" <> t: parse_lines(t)
    case h <> t: add_line(W.parse(h),parse_lines(t))

def with_fixed(record: W.Record,rest: Parsed) -> Parsed:
  match record rest:
    case W.Row{"FIXED" <> +total <> +state <> Nil{}} Parsed{_,xs,rs,v}: Parsed{Fixed{number(U32.read(total)),number(U32.read(state))},xs,rs,v && readable(U32.read(total)) && readable(U32.read(state))}
    case _ Parsed{f,xs,rs,_}: Parsed{f,xs,rs,False{}}

# Input: FIXED total state, then ITEM id state_bytes question_bytes in rank
# order, then (for a merge) RESULT request-index id label in arrival order.
def parsed(lines: List<&2,String>) -> Parsed:
  match lines:
    case h <> t: with_fixed(W.parse(h),parse_lines(t))
    case Nil{}: Parsed{Fixed{0,0},Nil{},Nil{},False{}}

def distinct(xs: List<&2,Item>,+seen: List<&2,String>) -> Bool:
  match xs:
    case Nil{}: True{}
    case Item{+id,_,_} <> t: Bool.not(E.text_member(seen,id)) && distinct(t,id <> seen)

def choose(ok: Bool,+fixed: Fixed,items: List<&2,Item>) -> String:
  match ok:
    case True{}: emit(plan(fixed,items))
    case False{}: "ERROR\tinvalid window plan input\n"

def planned_text(p: Parsed) -> String:
  match p:
    case Parsed{+fixed,+items,_,valid}: choose(valid && distinct(items,Nil{}),fixed,items)

def process(lines: List<&2,String>) -> String:
  planned_text(parsed(lines))

# The request index that judges each item under the re-derived plan.
type Assigned is Data:
  Assigned{id: String, request: String}

def assign(xs: List<&2,Item>,+request: String) -> List<&2,Assigned>:
  match xs:
    case Nil{}: Nil{}
    case Item{id,_,_} <> t: Assigned{id,request} <> assign(t,request)

def assign_slices(xs: List<&2,List<&2,Item>>,+index: U32) -> List<&2,Assigned>:
  match xs:
    case Nil{}: Nil{}
    case +h <> t: List.append(&2,Assigned,assign(h,U32.show(index)),assign_slices(t,(index + 1 : U32)))

def assignments(p: Plan) -> List<&2,Assigned>:
  match p:
    case Plan{anchor,slices,_}: List.append(&2,Assigned,assign(anchor,"0"),assign_slices(slices,1))

def planned_request(xs: List<&2,Assigned>,+id: String) -> String:
  match xs:
    case Nil{}: ""
    case Assigned{+aid,+r} <> t: Bool.pick(String,E.text_eq(aid,id),r,planned_request(t,id))

# First accepted judgment wins; a result for an item its request was not
# planned to judge is refused.
def first_label(results: List<&2,Answered>,+id: String,+request: String) -> String:
  match results:
    case Nil{}: ""
    case Answered{+rq,+rid,label} <> t: Bool.pick(String,E.text_eq(rid,id) && E.text_eq(rq,request) && Bool.not(String.is_empty(request)),label,first_label(t,id,request))

def judgment_if(unplanned: Bool,unanswered: Bool,+item: String,+request: String,+label: String) -> String:
  match unplanned unanswered:
    case True{} _: W.row(["UNINSPECTED",item,"omitted-by-window"])
    case False{} True{}: W.row(["UNINSPECTED",item,"no-accepted-result",request])
    case False{} False{}: W.row(["JUDGMENT",item,label,request])

def judgment(+item: String,+request: String,+label: String) -> String:
  judgment_if(String.is_empty(request),String.is_empty(label),item,request,label)

def judgments(items: List<&2,Item>,+plan_ids: List<&2,Assigned>,+results: List<&2,Answered>) -> String:
  match items:
    case Nil{}: ""
    case Item{+id,_,_} <> t:
      +request = planned_request(plan_ids,id)
      judgment(id,request,first_label(results,id,request)) ++ judgments(t,plan_ids,results)

def merge_valid(ok: Bool,+fixed: Fixed,+items: List<&2,Item>,+results: List<&2,Answered>) -> String:
  match ok:
    case True{}: judgments(items,assignments(plan(fixed,items)),results)
    case False{}: "ERROR\tinvalid window merge input\n"

def merged_text(p: Parsed) -> String:
  match p:
    case Parsed{+fixed,+items,results,valid}: merge_valid(valid && distinct(items,Nil{}),fixed,items,results)

def merge(lines: List<&2,String>) -> String:
  merged_text(parsed(lines))
