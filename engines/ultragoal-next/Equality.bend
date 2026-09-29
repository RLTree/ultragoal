import Base

# Constructive equality for retained computation arguments.
#
# Characters are compared with the native U32 test and the equality witness is
# obtained from soundness lemmas over Base's Word definition of U32.cmp. The
# lemmas are erased at runtime; matching a Char as its 32-bit Word would instead
# materialize every bit.

def decide_bool(+a: Bool,+b: Bool) -> Maybe<&2,{a == b : Bool}>:
  match a b:
    case True{} True{}: Some{{==}}
    case False{} False{}: Some{{==}}
    case True{} False{}: None{}
    case False{} True{}: None{}

law bool_self:
  for +a: Bool
  {decide_bool(a,a) == Some{{==}} : Maybe<&2,{a == a : Bool}>}

def bool_self(a):
  match a:
    case True{}: {==}
    case False{}: {==}


def cmp_disc(c: Cmp) -> Type:
  match c:
    case EQ{}: Empty
    case _: Unit

def lt_not_eq(e: {LT{} == EQ{} : Cmp}) -> Empty:
  %e : cmp_disc(_)
  Unit{}

def gt_not_eq(e: {GT{} == EQ{} : Cmp}) -> Empty:
  %e : cmp_disc(_)
  Unit{}

def absurd_bool(-a: Bool,-b: Bool,x: Empty) -> {a == b : Bool}:
  match x:

def bool_cmp_sound(a: Bool,b: Bool,e: {Bool.cmp(a,b) == EQ{} : Cmp}) -> {a == b : Bool}:
  match a b:
    case True{} True{}: {==}
    case False{} False{}: {==}
    case True{} False{}: absurd_bool(True{},False{},gt_not_eq(e))
    case False{} True{}: absurd_bool(False{},True{},lt_not_eq(e))

def absurd_word(n: Nat,-a: Word(n),-b: Word(n),x: Empty) -> {a == b : Word(n)}:
  match x:

def word_step(p: Nat,ab: Bool,bb: Bool,at: Word(p),bt: Word(p),t: Cmp,e: {Word.cmp.fin(ab,bb,t) == EQ{} : Cmp},ih: {t == EQ{} : Cmp} -> {at == bt : Word(p)}) -> {WCon{ab,at} == WCon{bb,bt} : Word(1n+p)}:
  match t:
    case LT{}: absurd_word(1n+p,WCon{ab,at},WCon{bb,bt},lt_not_eq(e))
    case GT{}: absurd_word(1n+p,WCon{ab,at},WCon{bb,bt},gt_not_eq(e))
    case EQ{}:
      +h = bool_cmp_sound(ab,bb,e)
      +r = ih({==})
      %r : {WCon{ab,at} == WCon{bb,_} : Word(1n+p)}
      %h : {WCon{ab,at} == WCon{_,at} : Word(1n+p)}
      {==}

def word_sound(+n: Nat,a: Word(n),b: Word(n),e: {Word.cmp(n,a,b) == EQ{} : Cmp}) -> {a == b : Word(n)}:
  match n:
    case 0n:
      match a b:
        case WNil{} WNil{}: {==}
    case 1n+ +p:
      match a b:
        case WCon{+ab,+at} WCon{+bb,+bt}: word_step(p,ab,bb,at,bt,Word.cmp(p,at,bt),e,q => word_sound(p,at,bt,q))

def bool_disc(b: Bool) -> Type:
  match b:
    case True{}: Empty
    case False{}: Unit

def false_not_true(e: {False{} == True{} : Bool}) -> Empty:
  %e : bool_disc(_)
  Unit{}

def absurd_cmp(-a: Cmp,-b: Cmp,x: Empty) -> {a == b : Cmp}:
  match x:

def cmp_is_eq_sound(c: Cmp,e: {Cmp.is_eq(c) == True{} : Bool}) -> {c == EQ{} : Cmp}:
  match c:
    case EQ{}: {==}
    case LT{}: absurd_cmp(LT{},EQ{},false_not_true(e))
    case GT{}: absurd_cmp(GT{},EQ{},false_not_true(e))

def u32_sound(a: U32,b: U32,e: {U32.is_eq(a,b) == True{} : Bool}) -> {a == b : U32}:
  match a b:
    case U32{+x} U32{+y}:
      +w = word_sound(32n,x,y,cmp_is_eq_sound(Word.cmp(32n,x,y),e))
      %w : {U32{x} == U32{_} : U32}
      {==}

def char_sound(a: Char,b: Char,e: {Char.is_eq(a,b) == True{} : Bool}) -> {a == b : Char}:
  match a b:
    case Chr{+x} Chr{+y}:
      +w = u32_sound(x,y,e)
      %w : {Chr{x} == Chr{_} : Char}
      {==}

def same_from(a: String,b: String,ok: Bool) -> Bool:
  match a b:
    case SNil{} SNil{}: ok
    case SCon{ha,ta} SCon{hb,tb}: same_from(ta,tb,Bool.and(ok,Char.is_eq(ha,hb)))
    case _ _: False{}

def absurd_string(-a: String,-b: String,x: Empty) -> {a == b : String}:
  match x:

def same_false(a: String,b: String) -> {same_from(a,b,False{}) == False{} : Bool}:
  match a b:
    case SNil{} SNil{}: {==}
    case SCon{ha,ta} SCon{hb,tb}: same_false(ta,tb)
    case SNil{} SCon{hb,tb}: {==}
    case SCon{ha,ta} SNil{}: {==}

def string_step(ha: Char,hb: Char,+ta: String,+tb: String,c: Bool,ce: {Char.is_eq(ha,hb) == c : Bool},e: {same_from(ta,tb,c) == True{} : Bool},ih: {same_from(ta,tb,True{}) == True{} : Bool} -> {ta == tb : String}) -> {SCon{ha,ta} == SCon{hb,tb} : String}:
  match c:
    case False{}:
      absurd_string(SCon{ha,ta},SCon{hb,tb},false_not_true(Equal.trans(Bool,False{},same_from(ta,tb,False{}),True{},Equal.sym(Bool,same_from(ta,tb,False{}),False{},same_false(ta,tb)),e)))
    case True{}:
      +h = char_sound(ha,hb,ce)
      +t = ih(e)
      %t : {SCon{ha,ta} == SCon{hb,_} : String}
      %h : {SCon{ha,ta} == SCon{_,ta} : String}
      {==}

def string_sound(a: String,b: String,e: {same_from(a,b,True{}) == True{} : Bool}) -> {a == b : String}:
  match a b:
    case SNil{} SNil{}: {==}
    case SNil{} SCon{hb,tb}: absurd_string(SNil{},SCon{hb,tb},false_not_true(e))
    case SCon{ha,ta} SNil{}: absurd_string(SCon{ha,ta},SNil{},false_not_true(e))
    case SCon{+ha,+ta} SCon{+hb,+tb}: string_step(ha,hb,ta,tb,Char.is_eq(ha,hb),{==},e,q => string_sound(ta,tb,q))

def strings_same_from(a: List<&2,String>,b: List<&2,String>,ok: Bool) -> Bool:
  match a b:
    case Nil{} Nil{}: ok
    case ha <> ta hb <> tb: strings_same_from(ta,tb,Bool.and(ok,same_from(ha,hb,True{})))
    case _ _: False{}

def absurd_strings(-a: List<&2,String>,-b: List<&2,String>,x: Empty) -> {a == b : List<&2,String>}:
  match x:

def strings_false(a: List<&2,String>,b: List<&2,String>) -> {strings_same_from(a,b,False{}) == False{} : Bool}:
  match a b:
    case Nil{} Nil{}: {==}
    case ha <> ta hb <> tb: strings_false(ta,tb)
    case Nil{} hb <> tb: {==}
    case ha <> ta Nil{}: {==}

def strings_step(+ha: String,+hb: String,+ta: List<&2,String>,+tb: List<&2,String>,c: Bool,ce: {same_from(ha,hb,True{}) == c : Bool},e: {strings_same_from(ta,tb,c) == True{} : Bool},ih: {strings_same_from(ta,tb,True{}) == True{} : Bool} -> {ta == tb : List<&2,String>}) -> {ha <> ta == hb <> tb : List<&2,String>}:
  match c:
    case False{}:
      absurd_strings(ha <> ta,hb <> tb,false_not_true(Equal.trans(Bool,False{},strings_same_from(ta,tb,False{}),True{},Equal.sym(Bool,strings_same_from(ta,tb,False{}),False{},strings_false(ta,tb)),e)))
    case True{}:
      +h = string_sound(ha,hb,ce)
      +t = ih(e)
      %t : {ha <> ta == hb <> _ : List<&2,String>}
      %h : {ha <> ta == _ <> ta : List<&2,String>}
      {==}

def strings_sound(a: List<&2,String>,b: List<&2,String>,e: {strings_same_from(a,b,True{}) == True{} : Bool}) -> {a == b : List<&2,String>}:
  match a b:
    case Nil{} Nil{}: {==}
    case Nil{} hb <> tb: absurd_strings(Nil{},hb <> tb,false_not_true(e))
    case ha <> ta Nil{}: absurd_strings(ha <> ta,Nil{},false_not_true(e))
    case +ha <> +ta +hb <> +tb: strings_step(ha,hb,ta,tb,same_from(ha,hb,True{}),{==},e,q => strings_sound(ta,tb,q))

def strings_same(+a: List<&2,String>,+b: List<&2,String>) -> Bool:
  strings_same_from(a,b,True{})

def strings_decided(+a: List<&2,String>,+b: List<&2,String>,r: Bool,e: {strings_same_from(a,b,True{}) == r : Bool}) -> Maybe<&2,{a == b : List<&2,String>}>:
  match r:
    case True{}: Some{strings_sound(a,b,e)}
    case False{}: None{}

def decide_strings(+a: List<&2,String>,+b: List<&2,String>) -> Maybe<&2,{a == b : List<&2,String>}>:
  strings_decided(a,b,strings_same_from(a,b,True{}),{==})

# Allocation-free Bool comparisons for hot per-row checks. They agree with
# Base String.eq and String.is_gt, which rebuild both strings while comparing.
def text_eq(+a: String,+b: String) -> Bool:
  same_from(a,b,True{})

def text_member(xs: List<&2,String>,+s: String) -> Bool:
  match xs:
    case Nil{}: False{}
    case +h <> t: text_eq(h,s) || text_member(t,s)

def first_order(c: Cmp,d: Cmp) -> Cmp:
  match c:
    case EQ{}: d
    case _: c

def code(c: Char) -> U32:
  match c:
    case Chr{x}: x

def order_from(a: String,b: String,c: Cmp) -> Cmp:
  match a b:
    case SNil{} SNil{}: c
    case SNil{} _: first_order(c,LT{})
    case _ SNil{}: first_order(c,GT{})
    case SCon{ha,ta} SCon{hb,tb}: order_from(ta,tb,first_order(c,U32.cmp(code(ha),code(hb))))

def text_gt(+a: String,+b: String) -> Bool:
  Cmp.is_gt(order_from(a,b,EQ{}))

# Same result as Base String.contains, which rebuilds the remaining text at
# every position before comparing it with the pattern.
def contains_from(+s: String,+p: String,found: Bool) -> Bool:
  match s:
    case SNil{}: found || String.is_empty(p)
    case SCon{_,t}: contains_from(t,p,found || String.starts_with(s,p))

def text_contains(+s: String,+p: String) -> Bool:
  contains_from(s,p,False{})
