import Base
import ./Retrieval.bend as Rt
import ./Lexicon.bend as Lx

# Lemmas for the select-retrieval laws of LAWS.bend.

# ---- natural numbers

def add_zero(+a: Nat) -> {Nat.add(a,0n) == a : Nat}:
  match a:
    case 0n: {==}
    case 1n+p:
      %add_zero(p) : {1n+Nat.add(p,0n) == 1n+_ : Nat}
      {==}

def add_succ(+a: Nat,+b: Nat) -> {Nat.add(a,1n+b) == 1n+Nat.add(a,b) : Nat}:
  match a:
    case 0n: {==}
    case 1n+p:
      %add_succ(p,b) : {1n+Nat.add(p,1n+b) == 1n+_ : Nat}
      {==}

def add_comm(+a: Nat,+b: Nat) -> {Nat.add(a,b) == Nat.add(b,a) : Nat}:
  match a:
    case 0n: Equal.sym(Nat,Nat.add(b,0n),b,add_zero(b))
    case 1n+p:
      %Equal.sym(Nat,Nat.add(b,1n+p),1n+Nat.add(b,p),add_succ(b,p)) : {1n+Nat.add(p,b) == _ : Nat}
      %add_comm(p,b) : {1n+Nat.add(p,b) == 1n+_ : Nat}
      {==}

def add_assoc(+a: Nat,+b: Nat,+c: Nat) -> {Nat.add(Nat.add(a,b),c) == Nat.add(a,Nat.add(b,c)) : Nat}:
  match a:
    case 0n: {==}
    case 1n+p:
      %add_assoc(p,b,c) : {1n+Nat.add(Nat.add(p,b),c) == 1n+_ : Nat}
      {==}

def max_comm(+a: Nat,+b: Nat) -> {Nat.max(a,b) == Nat.max(b,a) : Nat}:
  match a b:
    case 0n 0n: {==}
    case 0n 1n+q: {==}
    case 1n+p 0n: {==}
    case 1n+p 1n+q:
      %max_comm(p,q) : {1n+Nat.max(p,q) == 1n+_ : Nat}
      {==}

def max_assoc(+a: Nat,+b: Nat,+c: Nat) -> {Nat.max(Nat.max(a,b),c) == Nat.max(a,Nat.max(b,c)) : Nat}:
  match a b c:
    case 0n _ _: {==}
    case 1n+p 0n _: {==}
    case 1n+p 1n+q 0n: {==}
    case 1n+p 1n+q 1n+r:
      %max_assoc(p,q,r) : {1n+Nat.max(Nat.max(p,q),r) == 1n+_ : Nat}
      {==}

def max_idem(+a: Nat) -> {Nat.max(a,a) == a : Nat}:
  match a:
    case 0n: {==}
    case 1n+p:
      %max_idem(p) : {1n+Nat.max(p,p) == 1n+_ : Nat}
      {==}

# ---- term counts

def tri_eq(p: Nat,p2: Nat,d: Nat,d2: Nat,b: Nat,b2: Nat,ep: {p == p2 : Nat},ed: {d == d2 : Nat},eb: {b == b2 : Nat}) -> {Rt.Tri{p,d,b} == Rt.Tri{p2,d2,b2} : Rt.Tri}:
  %ep : {Rt.Tri{p,d,b} == Rt.Tri{_,d2,b2} : Rt.Tri}
  %ed : {Rt.Tri{p,d,b} == Rt.Tri{p,_,b2} : Rt.Tri}
  %eb : {Rt.Tri{p,d,b} == Rt.Tri{p,d,_} : Rt.Tri}
  {==}

def tri_comm(x: Rt.Tri,y: Rt.Tri) -> {Rt.tri_add(x,y) == Rt.tri_add(y,x) : Rt.Tri}:
  match x y:
    case Rt.Tri{+p1,+d1,+b1} Rt.Tri{+p2,+d2,+b2}: tri_eq(Nat.max(p1,p2),Nat.max(p2,p1),Nat.add(d1,d2),Nat.add(d2,d1),Nat.add(b1,b2),Nat.add(b2,b1),max_comm(p1,p2),add_comm(d1,d2),add_comm(b1,b2))

def tri_assoc(x: Rt.Tri,y: Rt.Tri,z: Rt.Tri) -> {Rt.tri_add(Rt.tri_add(x,y),z) == Rt.tri_add(x,Rt.tri_add(y,z)) : Rt.Tri}:
  match x y z:
    case Rt.Tri{+p1,+d1,+b1} Rt.Tri{+p2,+d2,+b2} Rt.Tri{+p3,+d3,+b3}: tri_eq(Nat.max(Nat.max(p1,p2),p3),Nat.max(p1,Nat.max(p2,p3)),Nat.add(Nat.add(d1,d2),d3),Nat.add(d1,Nat.add(d2,d3)),Nat.add(Nat.add(b1,b2),b3),Nat.add(b1,Nat.add(b2,b3)),max_assoc(p1,p2,p3),add_assoc(d1,d2,d3),add_assoc(b1,b2,b3))

def cons_eq(-A: Data,h: A,h2: A,t: List<&2,A>,t2: List<&2,A>,eh: {h == h2 : A},et: {t == t2 : List<&2,A>}) -> {h <> t == h2 <> t2 : List<&2,A>}:
  %eh : {h <> t == _ <> t2 : List<&2,A>}
  %et : {h <> t == h <> _ : List<&2,A>}
  {==}

def vadd_comm(xs: List<&2,Rt.Tri>,ys: List<&2,Rt.Tri>) -> {Rt.vadd(xs,ys) == Rt.vadd(ys,xs) : List<&2,Rt.Tri>}:
  match xs ys:
    case Nil{} Nil{}: {==}
    case Nil{} h <> t: {==}
    case h <> t Nil{}: {==}
    case +h1 <> +t1 +h2 <> +t2: cons_eq(Rt.Tri,Rt.tri_add(h1,h2),Rt.tri_add(h2,h1),Rt.vadd(t1,t2),Rt.vadd(t2,t1),tri_comm(h1,h2),vadd_comm(t1,t2))

def vadd_assoc(xs: List<&2,Rt.Tri>,ys: List<&2,Rt.Tri>,zs: List<&2,Rt.Tri>) -> {Rt.vadd(Rt.vadd(xs,ys),zs) == Rt.vadd(xs,Rt.vadd(ys,zs)) : List<&2,Rt.Tri>}:
  match xs ys zs:
    case Nil{} _ _: {==}
    case h <> t Nil{} _: {==}
    case h <> t h2 <> t2 Nil{}: {==}
    case +h1 <> +t1 +h2 <> +t2 +h3 <> +t3: cons_eq(Rt.Tri,Rt.tri_add(Rt.tri_add(h1,h2),h3),Rt.tri_add(h1,Rt.tri_add(h2,h3)),Rt.vadd(Rt.vadd(t1,t2),t3),Rt.vadd(t1,Rt.vadd(t2,t3)),tri_assoc(h1,h2,h3),vadd_assoc(t1,t2,t3))

def nadd_comm(xs: List<&2,Nat>,ys: List<&2,Nat>) -> {Rt.nadd(xs,ys) == Rt.nadd(ys,xs) : List<&2,Nat>}:
  match xs ys:
    case Nil{} Nil{}: {==}
    case Nil{} h <> t: {==}
    case h <> t Nil{}: {==}
    case +h1 <> +t1 +h2 <> +t2: cons_eq(Nat,Nat.add(h1,h2),Nat.add(h2,h1),Rt.nadd(t1,t2),Rt.nadd(t2,t1),add_comm(h1,h2),nadd_comm(t1,t2))

def nadd_assoc(xs: List<&2,Nat>,ys: List<&2,Nat>,zs: List<&2,Nat>) -> {Rt.nadd(Rt.nadd(xs,ys),zs) == Rt.nadd(xs,Rt.nadd(ys,zs)) : List<&2,Nat>}:
  match xs ys zs:
    case Nil{} _ _: {==}
    case h <> t Nil{} _: {==}
    case h <> t h2 <> t2 Nil{}: {==}
    case +h1 <> +t1 +h2 <> +t2 +h3 <> +t3: cons_eq(Nat,Nat.add(Nat.add(h1,h2),h3),Nat.add(h1,Nat.add(h2,h3)),Rt.nadd(Rt.nadd(t1,t2),t3),Rt.nadd(t1,Rt.nadd(t2,t3)),add_assoc(h1,h2,h3),nadd_assoc(t1,t2,t3))

def fstat_eq(l: Nat,l2: Nat,v: List<&2,Rt.Tri>,v2: List<&2,Rt.Tri>,el: {l == l2 : Nat},ev: {v == v2 : List<&2,Rt.Tri>}) -> {Rt.FStat{l,v} == Rt.FStat{l2,v2} : Rt.FStat}:
  %el : {Rt.FStat{l,v} == Rt.FStat{_,v2} : Rt.FStat}
  %ev : {Rt.FStat{l,v} == Rt.FStat{l,_} : Rt.FStat}
  {==}

def fadd_comm(x: Rt.FStat,y: Rt.FStat) -> {Rt.fadd(x,y) == Rt.fadd(y,x) : Rt.FStat}:
  match x y:
    case Rt.FStat{+l1,+v1} Rt.FStat{+l2,+v2}: fstat_eq(Nat.add(l1,l2),Nat.add(l2,l1),Rt.vadd(v1,v2),Rt.vadd(v2,v1),add_comm(l1,l2),vadd_comm(v1,v2))

def fadd_assoc(x: Rt.FStat,y: Rt.FStat,z: Rt.FStat) -> {Rt.fadd(Rt.fadd(x,y),z) == Rt.fadd(x,Rt.fadd(y,z)) : Rt.FStat}:
  match x y z:
    case Rt.FStat{+l1,+v1} Rt.FStat{+l2,+v2} Rt.FStat{+l3,+v3}: fstat_eq(Nat.add(Nat.add(l1,l2),l3),Nat.add(l1,Nat.add(l2,l3)),Rt.vadd(Rt.vadd(v1,v2),v3),Rt.vadd(v1,Rt.vadd(v2,v3)),add_assoc(l1,l2,l3),vadd_assoc(v1,v2,v3))

def wstat_eq(n: Nat,n2: Nat,t: Nat,t2: Nat,c: List<&2,Nat>,c2: List<&2,Nat>,en: {n == n2 : Nat},et: {t == t2 : Nat},ec: {c == c2 : List<&2,Nat>}) -> {Rt.WStat{n,t,c} == Rt.WStat{n2,t2,c2} : Rt.WStat}:
  %en : {Rt.WStat{n,t,c} == Rt.WStat{_,t2,c2} : Rt.WStat}
  %et : {Rt.WStat{n,t,c} == Rt.WStat{n,_,c2} : Rt.WStat}
  %ec : {Rt.WStat{n,t,c} == Rt.WStat{n,t,_} : Rt.WStat}
  {==}

def wadd_comm(x: Rt.WStat,y: Rt.WStat) -> {Rt.wadd(x,y) == Rt.wadd(y,x) : Rt.WStat}:
  match x y:
    case Rt.WStat{+n1,+t1,+c1} Rt.WStat{+n2,+t2,+c2}: wstat_eq(Nat.add(n1,n2),Nat.add(n2,n1),Nat.add(t1,t2),Nat.add(t2,t1),Rt.nadd(c1,c2),Rt.nadd(c2,c1),add_comm(n1,n2),add_comm(t1,t2),nadd_comm(c1,c2))

def wadd_assoc(x: Rt.WStat,y: Rt.WStat,z: Rt.WStat) -> {Rt.wadd(Rt.wadd(x,y),z) == Rt.wadd(x,Rt.wadd(y,z)) : Rt.WStat}:
  match x y z:
    case Rt.WStat{+n1,+t1,+c1} Rt.WStat{+n2,+t2,+c2} Rt.WStat{+n3,+t3,+c3}: wstat_eq(Nat.add(Nat.add(n1,n2),n3),Nat.add(n1,Nat.add(n2,n3)),Nat.add(Nat.add(t1,t2),t3),Nat.add(t1,Nat.add(t2,t3)),Rt.nadd(Rt.nadd(c1,c2),c3),Rt.nadd(c1,Rt.nadd(c2,c3)),add_assoc(n1,n2,n3),add_assoc(t1,t2,t3),nadd_assoc(c1,c2,c3))

def wadd_zero(y: Rt.WStat) -> {Rt.wadd(Rt.WStat{0n,0n,Nil{}},y) == y : Rt.WStat}:
  match y:
    case Rt.WStat{n,t,c}: {==}

def wsum_split(xs: List<&2,Rt.WStat>,+ys: List<&2,Rt.WStat>) -> {Rt.wsum(List.append(&2,Rt.WStat,xs,ys)) == Rt.wadd(Rt.wsum(xs),Rt.wsum(ys)) : Rt.WStat}:
  match xs:
    case Nil{}: Equal.sym(Rt.WStat,Rt.wadd(Rt.WStat{0n,0n,Nil{}},Rt.wsum(ys)),Rt.wsum(ys),wadd_zero(Rt.wsum(ys)))
    case +h <> +t:
      %Equal.sym(Rt.WStat,Rt.wsum(List.append(&2,Rt.WStat,t,ys)),Rt.wadd(Rt.wsum(t),Rt.wsum(ys)),wsum_split(t,ys)) : {Rt.wadd(h,_) == Rt.wadd(Rt.wadd(h,Rt.wsum(t)),Rt.wsum(ys)) : Rt.WStat}
      Equal.sym(Rt.WStat,Rt.wadd(Rt.wadd(h,Rt.wsum(t)),Rt.wsum(ys)),Rt.wadd(h,Rt.wadd(Rt.wsum(t),Rt.wsum(ys))),wadd_assoc(h,Rt.wsum(t),Rt.wsum(ys)))

# ---- the code-point order of paths is a total order

def flip(c: Cmp) -> Cmp:
  match c:
    case LT{}: GT{}
    case EQ{}: EQ{}
    case GT{}: LT{}

def disc_eq(c: Cmp) -> Type:
  match c:
    case LT{}: Unit
    case EQ{}: Empty
    case GT{}: Unit

def disc_lt(c: Cmp) -> Type:
  match c:
    case LT{}: Empty
    case EQ{}: Unit
    case GT{}: Unit

def elim(-G: Type,x: Empty) -> G:
  match x:

def lt_is_not_eq(e: {LT{} == EQ{} : Cmp}) -> Empty:
  %e : disc_eq(_)
  Unit{}

def gt_is_not_eq(e: {GT{} == EQ{} : Cmp}) -> Empty:
  %e : disc_eq(_)
  Unit{}

def eq_is_not_lt(e: {EQ{} == LT{} : Cmp}) -> Empty:
  %e : disc_lt(_)
  Unit{}

def gt_is_not_lt(e: {GT{} == LT{} : Cmp}) -> Empty:
  %e : disc_lt(_)
  Unit{}

def fin_flip(ab: Bool,bb: Bool,t: Cmp) -> {Word.cmp.fin(bb,ab,flip(t)) == flip(Word.cmp.fin(ab,bb,t)) : Cmp}:
  match ab bb t:
    case _ _ LT{}: {==}
    case _ _ GT{}: {==}
    case True{} True{} EQ{}: {==}
    case True{} False{} EQ{}: {==}
    case False{} True{} EQ{}: {==}
    case False{} False{} EQ{}: {==}

def word_flip(+n: Nat,a: Word(n),b: Word(n)) -> {Word.cmp(n,b,a) == flip(Word.cmp(n,a,b)) : Cmp}:
  match n:
    case 0n: {==}
    case 1n+p:
      match a b:
        case WCon{+ab,+at} WCon{+bb,+bt}: Equal.trans(Cmp,Word.cmp.fin(bb,ab,Word.cmp(p,bt,at)),Word.cmp.fin(bb,ab,flip(Word.cmp(p,at,bt))),flip(Word.cmp.fin(ab,bb,Word.cmp(p,at,bt))),Equal.cong(Cmp,Cmp,x => Word.cmp.fin(bb,ab,x),Word.cmp(p,bt,at),flip(Word.cmp(p,at,bt)),word_flip(p,at,bt)),fin_flip(ab,bb,Word.cmp(p,at,bt)))

def bool_refl(b: Bool) -> {Bool.cmp(b,b) == EQ{} : Cmp}:
  match b:
    case True{}: {==}
    case False{}: {==}

def word_refl(+n: Nat,a: Word(n)) -> {Word.cmp(n,a,a) == EQ{} : Cmp}:
  match n:
    case 0n: {==}
    case 1n+p:
      match a:
        case WCon{+ab,+at}:
          %Equal.sym(Cmp,Word.cmp(p,at,at),EQ{},word_refl(p,at)) : {Word.cmp.fin(ab,ab,_) == EQ{} : Cmp}
          bool_refl(ab)

def fin_tail(ab: Bool,bb: Bool,t: Cmp,e: {Word.cmp.fin(ab,bb,t) == EQ{} : Cmp}) -> {t == EQ{} : Cmp}:
  match t:
    case LT{}: e
    case EQ{}: {==}
    case GT{}: e

def fin_head(ab: Bool,bb: Bool,t: Cmp,e: {Word.cmp.fin(ab,bb,t) == EQ{} : Cmp}) -> {ab == bb : Bool}:
  match ab bb t:
    case _ _ LT{}: elim({ab == bb : Bool},lt_is_not_eq(e))
    case _ _ GT{}: elim({ab == bb : Bool},gt_is_not_eq(e))
    case True{} True{} EQ{}: {==}
    case True{} False{} EQ{}: elim({True{} == False{} : Bool},gt_is_not_eq(e))
    case False{} True{} EQ{}: elim({False{} == True{} : Bool},lt_is_not_eq(e))
    case False{} False{} EQ{}: {==}

def wcon_eq(-p: Nat,ab: Bool,bb: Bool,at: Word(p),bt: Word(p),eh: {ab == bb : Bool},et: {at == bt : Word(p)}) -> {WCon{ab,at} == WCon{bb,bt} : Word(1n+p)}:
  %eh : {WCon{ab,at} == WCon{_,bt} : Word(1n+p)}
  %et : {WCon{ab,at} == WCon{ab,_} : Word(1n+p)}
  {==}

def word_eq(+n: Nat,a: Word(n),b: Word(n),+e: {Word.cmp(n,a,b) == EQ{} : Cmp}) -> {a == b : Word(n)}:
  match n:
    case 0n:
      match a b:
        case WNil{} WNil{}: {==}
    case 1n+p:
      match a b:
        case WCon{+ab,+at} WCon{+bb,+bt}: wcon_eq(p,ab,bb,at,bt,fin_head(ab,bb,Word.cmp(p,at,bt),e),word_eq(p,at,bt,fin_tail(ab,bb,Word.cmp(p,at,bt),e)))

def bits_trans(ab: Bool,bb: Bool,cb: Bool,e1: {Bool.cmp(ab,bb) == LT{} : Cmp},e2: {Bool.cmp(bb,cb) == LT{} : Cmp}) -> {Bool.cmp(ab,cb) == LT{} : Cmp}:
  match ab bb cb:
    case False{} False{} False{}: elim({Bool.cmp(False{},False{}) == LT{} : Cmp},eq_is_not_lt(e1))
    case False{} False{} True{}: elim({Bool.cmp(False{},True{}) == LT{} : Cmp},eq_is_not_lt(e1))
    case True{} True{} False{}: elim({Bool.cmp(True{},False{}) == LT{} : Cmp},eq_is_not_lt(e1))
    case True{} True{} True{}: elim({Bool.cmp(True{},True{}) == LT{} : Cmp},eq_is_not_lt(e1))
    case True{} False{} False{}: elim({Bool.cmp(True{},False{}) == LT{} : Cmp},gt_is_not_lt(e1))
    case True{} False{} True{}: elim({Bool.cmp(True{},True{}) == LT{} : Cmp},gt_is_not_lt(e1))
    case False{} True{} False{}: elim({Bool.cmp(False{},False{}) == LT{} : Cmp},gt_is_not_lt(e2))
    case False{} True{} True{}: elim({Bool.cmp(False{},True{}) == LT{} : Cmp},eq_is_not_lt(e2))

def trans_at(+p: Nat,ab: Bool,bb: Bool,cb: Bool,+at: Word(p),+bt: Word(p),+ct: Word(p),t1: Cmp,t2: Cmp,q1: {Word.cmp(p,at,bt) == t1 : Cmp},q2: {Word.cmp(p,bt,ct) == t2 : Cmp},e1: {Word.cmp.fin(ab,bb,t1) == LT{} : Cmp},e2: {Word.cmp.fin(bb,cb,t2) == LT{} : Cmp},ih: {Word.cmp(p,at,bt) == LT{} : Cmp} -> {Word.cmp(p,bt,ct) == LT{} : Cmp} -> {Word.cmp(p,at,ct) == LT{} : Cmp}) -> {Word.cmp.fin(ab,cb,Word.cmp(p,at,ct)) == LT{} : Cmp}:
  match t1 t2:
    case GT{} _: elim({Word.cmp.fin(ab,cb,Word.cmp(p,at,ct)) == LT{} : Cmp},gt_is_not_lt(e1))
    case _ GT{}: elim({Word.cmp.fin(ab,cb,Word.cmp(p,at,ct)) == LT{} : Cmp},gt_is_not_lt(e2))
    case LT{} LT{}:
      %Equal.sym(Cmp,Word.cmp(p,at,ct),LT{},ih(q1,q2)) : {Word.cmp.fin(ab,cb,_) == LT{} : Cmp}
      {==}
    case LT{} EQ{}:
      %word_eq(p,bt,ct,q2) : {Word.cmp.fin(ab,cb,Word.cmp(p,at,_)) == LT{} : Cmp}
      %Equal.sym(Cmp,Word.cmp(p,at,bt),LT{},q1) : {Word.cmp.fin(ab,cb,_) == LT{} : Cmp}
      {==}
    case EQ{} LT{}:
      %Equal.sym(Word(p),at,bt,word_eq(p,at,bt,q1)) : {Word.cmp.fin(ab,cb,Word.cmp(p,_,ct)) == LT{} : Cmp}
      %Equal.sym(Cmp,Word.cmp(p,bt,ct),LT{},q2) : {Word.cmp.fin(ab,cb,_) == LT{} : Cmp}
      {==}
    case EQ{} EQ{}:
      %Equal.sym(Word(p),at,bt,word_eq(p,at,bt,q1)) : {Word.cmp.fin(ab,cb,Word.cmp(p,_,ct)) == LT{} : Cmp}
      %word_eq(p,bt,ct,q2) : {Word.cmp.fin(ab,cb,Word.cmp(p,bt,_)) == LT{} : Cmp}
      %Equal.sym(Cmp,Word.cmp(p,bt,bt),EQ{},word_refl(p,bt)) : {Word.cmp.fin(ab,cb,_) == LT{} : Cmp}
      bits_trans(ab,bb,cb,e1,e2)

def word_trans(+n: Nat,a: Word(n),b: Word(n),c: Word(n),e1: {Word.cmp(n,a,b) == LT{} : Cmp},e2: {Word.cmp(n,b,c) == LT{} : Cmp}) -> {Word.cmp(n,a,c) == LT{} : Cmp}:
  match n:
    case 0n: elim({Word.cmp(0n,a,c) == LT{} : Cmp},eq_is_not_lt(e1))
    case 1n+p:
      match a b c:
        case WCon{+ab,+at} WCon{+bb,+bt} WCon{+cb,+ct}: trans_at(p,ab,bb,cb,at,bt,ct,Word.cmp(p,at,bt),Word.cmp(p,bt,ct),{==},{==},e1,e2,word_trans(p,at,bt,ct))

def char_flip(a: Char,b: Char) -> {Rt.ccmp(b,a) == flip(Rt.ccmp(a,b)) : Cmp}:
  match a b:
    case Chr{U32{wx}} Chr{U32{wy}}: word_flip(32n,wx,wy)

def char_refl(a: Char) -> {Rt.ccmp(a,a) == EQ{} : Cmp}:
  match a:
    case Chr{U32{w}}: word_refl(32n,w)

def char_eq(a: Char,b: Char,e: {Rt.ccmp(a,b) == EQ{} : Cmp}) -> {a == b : Char}:
  match a b:
    case Chr{U32{+wx}} Chr{U32{+wy}}: Equal.cong(Word(32n),Char,w => Chr{U32{w}},wx,wy,word_eq(32n,wx,wy,e))

def char_trans(a: Char,b: Char,c: Char,e1: {Rt.ccmp(a,b) == LT{} : Cmp},e2: {Rt.ccmp(b,c) == LT{} : Cmp}) -> {Rt.ccmp(a,c) == LT{} : Cmp}:
  match a b c:
    case Chr{U32{wx}} Chr{U32{wy}} Chr{U32{wz}}: word_trans(32n,wx,wy,wz,e1,e2)

def then_flip(c: Cmp,r: Cmp) -> {Rt.then(flip(c),flip(r)) == flip(Rt.then(c,r)) : Cmp}:
  match c:
    case LT{}: {==}
    case EQ{}: {==}
    case GT{}: {==}

def then_head(c: Cmp,r: Cmp,e: {Rt.then(c,r) == EQ{} : Cmp}) -> {c == EQ{} : Cmp}:
  match c:
    case LT{}: e
    case EQ{}: {==}
    case GT{}: e

def then_tail(c: Cmp,r: Cmp,e: {Rt.then(c,r) == EQ{} : Cmp}) -> {r == EQ{} : Cmp}:
  match c:
    case LT{}: elim({r == EQ{} : Cmp},lt_is_not_eq(e))
    case EQ{}: e
    case GT{}: elim({r == EQ{} : Cmp},gt_is_not_eq(e))

def order_flip(a: String,b: String) -> {Rt.order(b,a) == flip(Rt.order(a,b)) : Cmp}:
  match a b:
    case SNil{} SNil{}: {==}
    case SNil{} SCon{_,_}: {==}
    case SCon{_,_} SNil{}: {==}
    case SCon{+x,+xs} SCon{+y,+ys}:
      %Equal.sym(Cmp,Rt.ccmp(y,x),flip(Rt.ccmp(x,y)),char_flip(x,y)) : {Rt.then(_,Rt.order(ys,xs)) == flip(Rt.then(Rt.ccmp(x,y),Rt.order(xs,ys))) : Cmp}
      %Equal.sym(Cmp,Rt.order(ys,xs),flip(Rt.order(xs,ys)),order_flip(xs,ys)) : {Rt.then(flip(Rt.ccmp(x,y)),_) == flip(Rt.then(Rt.ccmp(x,y),Rt.order(xs,ys))) : Cmp}
      then_flip(Rt.ccmp(x,y),Rt.order(xs,ys))

def scon_eq(x: Char,y: Char,xs: String,ys: String,eh: {x == y : Char},et: {xs == ys : String}) -> {SCon{x,xs} == SCon{y,ys} : String}:
  %eh : {SCon{x,xs} == SCon{_,ys} : String}
  %et : {SCon{x,xs} == SCon{x,_} : String}
  {==}

def order_eq(a: String,b: String,+e: {Rt.order(a,b) == EQ{} : Cmp}) -> {a == b : String}:
  match a b:
    case SNil{} SNil{}: {==}
    case SNil{} SCon{y,ys}: elim({SNil{} == SCon{y,ys} : String},lt_is_not_eq(e))
    case SCon{x,xs} SNil{}: elim({SCon{x,xs} == SNil{} : String},gt_is_not_eq(e))
    case SCon{+x,+xs} SCon{+y,+ys}: scon_eq(x,y,xs,ys,char_eq(x,y,then_head(Rt.ccmp(x,y),Rt.order(xs,ys),e)),order_eq(xs,ys,then_tail(Rt.ccmp(x,y),Rt.order(xs,ys),e)))

def order_trans_at(+x: Char,+y: Char,+z: Char,xs: String,ys: String,zs: String,c1: Cmp,c2: Cmp,q1: {Rt.ccmp(x,y) == c1 : Cmp},q2: {Rt.ccmp(y,z) == c2 : Cmp},e1: {Rt.then(c1,Rt.order(xs,ys)) == LT{} : Cmp},e2: {Rt.then(c2,Rt.order(ys,zs)) == LT{} : Cmp},ih: {Rt.order(xs,ys) == LT{} : Cmp} -> {Rt.order(ys,zs) == LT{} : Cmp} -> {Rt.order(xs,zs) == LT{} : Cmp}) -> {Rt.then(Rt.ccmp(x,z),Rt.order(xs,zs)) == LT{} : Cmp}:
  match c1 c2:
    case GT{} _: elim({Rt.then(Rt.ccmp(x,z),Rt.order(xs,zs)) == LT{} : Cmp},gt_is_not_lt(e1))
    case _ GT{}: elim({Rt.then(Rt.ccmp(x,z),Rt.order(xs,zs)) == LT{} : Cmp},gt_is_not_lt(e2))
    case LT{} LT{}:
      %Equal.sym(Cmp,Rt.ccmp(x,z),LT{},char_trans(x,y,z,q1,q2)) : {Rt.then(_,Rt.order(xs,zs)) == LT{} : Cmp}
      {==}
    case LT{} EQ{}:
      %char_eq(y,z,q2) : {Rt.then(Rt.ccmp(x,_),Rt.order(xs,zs)) == LT{} : Cmp}
      %Equal.sym(Cmp,Rt.ccmp(x,y),LT{},q1) : {Rt.then(_,Rt.order(xs,zs)) == LT{} : Cmp}
      {==}
    case EQ{} LT{}:
      %Equal.sym(Char,x,y,char_eq(x,y,q1)) : {Rt.then(Rt.ccmp(_,z),Rt.order(xs,zs)) == LT{} : Cmp}
      %Equal.sym(Cmp,Rt.ccmp(y,z),LT{},q2) : {Rt.then(_,Rt.order(xs,zs)) == LT{} : Cmp}
      {==}
    case EQ{} EQ{}:
      %Equal.sym(Char,x,y,char_eq(x,y,q1)) : {Rt.then(Rt.ccmp(_,z),Rt.order(xs,zs)) == LT{} : Cmp}
      %char_eq(y,z,q2) : {Rt.then(Rt.ccmp(y,_),Rt.order(xs,zs)) == LT{} : Cmp}
      %Equal.sym(Cmp,Rt.ccmp(y,y),EQ{},char_refl(y)) : {Rt.then(_,Rt.order(xs,zs)) == LT{} : Cmp}
      ih(e1,e2)

def order_trans(a: String,b: String,c: String,e1: {Rt.order(a,b) == LT{} : Cmp},e2: {Rt.order(b,c) == LT{} : Cmp}) -> {Rt.order(a,c) == LT{} : Cmp}:
  match a b c:
    case SNil{} SNil{} z: elim({Rt.order(SNil{},z) == LT{} : Cmp},eq_is_not_lt(e1))
    case SNil{} SCon{_,_} SNil{}: elim({Rt.order(SNil{},SNil{}) == LT{} : Cmp},gt_is_not_lt(e2))
    case SNil{} SCon{_,_} SCon{_,_}: {==}
    case SCon{x,xs} SNil{} z: elim({Rt.order(SCon{x,xs},z) == LT{} : Cmp},gt_is_not_lt(e1))
    case SCon{x,xs} SCon{_,_} SNil{}: elim({Rt.order(SCon{x,xs},SNil{}) == LT{} : Cmp},gt_is_not_lt(e2))
    case SCon{+x,+xs} SCon{+y,+ys} SCon{+z,+zs}: order_trans_at(x,y,z,xs,ys,zs,Rt.ccmp(x,y),Rt.ccmp(y,z),{==},{==},e1,e2,order_trans(xs,ys,zs))

# ---- grouping rows by path does not depend on their order

def red_lt(pr: String,sr: Rt.FStat,ph: String,sh: Rt.FStat,t: List<&2,Rt.FileRow>,e: {Rt.order(pr,ph) == LT{} : Cmp}) -> {Rt.group(Rt.FileRow{pr,sr},Rt.FileRow{ph,sh} <> t) == Rt.FileRow{ph,sh} <> Rt.group(Rt.FileRow{pr,sr},t) : List<&2,Rt.FileRow>}:
  %Equal.sym(Cmp,Rt.order(pr,ph),LT{},e) : {Rt.group_at(_,Rt.FileRow{pr,sr},Rt.FileRow{ph,sh},t,u => Rt.group(Rt.FileRow{pr,sr},t)) == Rt.FileRow{ph,sh} <> Rt.group(Rt.FileRow{pr,sr},t) : List<&2,Rt.FileRow>}
  {==}

def red_gt(pr: String,sr: Rt.FStat,ph: String,sh: Rt.FStat,t: List<&2,Rt.FileRow>,e: {Rt.order(pr,ph) == GT{} : Cmp}) -> {Rt.group(Rt.FileRow{pr,sr},Rt.FileRow{ph,sh} <> t) == Rt.FileRow{pr,sr} <> Rt.FileRow{ph,sh} <> t : List<&2,Rt.FileRow>}:
  %Equal.sym(Cmp,Rt.order(pr,ph),GT{},e) : {Rt.group_at(_,Rt.FileRow{pr,sr},Rt.FileRow{ph,sh},t,u => Rt.group(Rt.FileRow{pr,sr},t)) == Rt.FileRow{pr,sr} <> Rt.FileRow{ph,sh} <> t : List<&2,Rt.FileRow>}
  {==}

def red_eq(pr: String,sr: Rt.FStat,ph: String,sh: Rt.FStat,t: List<&2,Rt.FileRow>,e: {Rt.order(pr,ph) == EQ{} : Cmp}) -> {Rt.group(Rt.FileRow{pr,sr},Rt.FileRow{ph,sh} <> t) == Rt.FileRow{ph,Rt.fadd(sr,sh)} <> t : List<&2,Rt.FileRow>}:
  %Equal.sym(Cmp,Rt.order(pr,ph),EQ{},e) : {Rt.group_at(_,Rt.FileRow{pr,sr},Rt.FileRow{ph,sh},t,u => Rt.group(Rt.FileRow{pr,sr},t)) == Rt.FileRow{ph,Rt.fadd(sr,sh)} <> t : List<&2,Rt.FileRow>}
  {==}

# x = m and y = n and m = n give x = y.
def meet(-A: Type,x: A,m: A,y: A,n: A,xm: {x == m : A},yn: {y == n : A},mn: {m == n : A}) -> {x == y : A}:
  Equal.trans(A,x,m,y,xm,Equal.trans(A,m,n,y,mn,Equal.sym(A,y,n,yn)))

def flipped(a: String,b: String,c: Cmp,e: {Rt.order(a,b) == c : Cmp}) -> {Rt.order(b,a) == flip(c) : Cmp}:
  %e : {Rt.order(b,a) == flip(_) : Cmp}
  order_flip(a,b)

def fadd_swap(+a: Rt.FStat,+b: Rt.FStat,+c: Rt.FStat) -> {Rt.fadd(a,Rt.fadd(b,c)) == Rt.fadd(b,Rt.fadd(a,c)) : Rt.FStat}:
  Equal.trans(Rt.FStat,Rt.fadd(a,Rt.fadd(b,c)),Rt.fadd(Rt.fadd(a,b),c),Rt.fadd(b,Rt.fadd(a,c)),Equal.sym(Rt.FStat,Rt.fadd(Rt.fadd(a,b),c),Rt.fadd(a,Rt.fadd(b,c)),fadd_assoc(a,b,c)),Equal.trans(Rt.FStat,Rt.fadd(Rt.fadd(a,b),c),Rt.fadd(Rt.fadd(b,a),c),Rt.fadd(b,Rt.fadd(a,c)),Equal.cong(Rt.FStat,Rt.FStat,x => Rt.fadd(x,c),Rt.fadd(a,b),Rt.fadd(b,a),fadd_comm(a,b)),fadd_assoc(b,a,c)))

def eq_then_lt(+a: String,+b: String,c: String,e: {Rt.order(a,b) == EQ{} : Cmp},l: {Rt.order(b,c) == LT{} : Cmp}) -> {Rt.order(a,c) == LT{} : Cmp}:
  %Equal.sym(String,a,b,order_eq(a,b,e)) : {Rt.order(_,c) == LT{} : Cmp}
  l

# Rows of equal paths merge alike in either order.
def pair_eq(+pa: String,+sa: Rt.FStat,+pb: String,+sb: Rt.FStat,rest: List<&2,Rt.FileRow>,e: {Rt.order(pa,pb) == EQ{} : Cmp}) -> {Rt.FileRow{pb,Rt.fadd(sa,sb)} <> rest == Rt.FileRow{pa,Rt.fadd(sb,sa)} <> rest : List<&2,Rt.FileRow>}:
  %Equal.sym(String,pa,pb,order_eq(pa,pb,e)) : {Rt.FileRow{pb,Rt.fadd(sa,sb)} <> rest == Rt.FileRow{_,Rt.fadd(sb,sa)} <> rest : List<&2,Rt.FileRow>}
  %fadd_comm(sa,sb) : {Rt.FileRow{pb,Rt.fadd(sa,sb)} <> rest == Rt.FileRow{pb,_} <> rest : List<&2,Rt.FileRow>}
  {==}

# Two rows that both precede everything in rest (ra, rb) are grouped alike in either order.
def pair_swap(+pa: String,+sa: Rt.FStat,+pb: String,+sb: Rt.FStat,+rest: List<&2,Rt.FileRow>,cab: Cmp,+qab: {Rt.order(pa,pb) == cab : Cmp},ra: {Rt.group(Rt.FileRow{pa,sa},rest) == Rt.FileRow{pa,sa} <> rest : List<&2,Rt.FileRow>},rb: {Rt.group(Rt.FileRow{pb,sb},rest) == Rt.FileRow{pb,sb} <> rest : List<&2,Rt.FileRow>}) -> {Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{pb,sb} <> rest) == Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{pa,sa} <> rest) : List<&2,Rt.FileRow>}:
  match cab:
    case GT{}: meet(List<&2,Rt.FileRow>,Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{pb,sb} <> rest),Rt.FileRow{pa,sa} <> Rt.FileRow{pb,sb} <> rest,Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{pa,sa} <> rest),Rt.FileRow{pa,sa} <> Rt.FileRow{pb,sb} <> rest,red_gt(pa,sa,pb,sb,rest,qab),Equal.trans(List<&2,Rt.FileRow>,Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{pa,sa} <> rest),Rt.FileRow{pa,sa} <> Rt.group(Rt.FileRow{pb,sb},rest),Rt.FileRow{pa,sa} <> Rt.FileRow{pb,sb} <> rest,red_lt(pb,sb,pa,sa,rest,flipped(pa,pb,GT{},qab)),Equal.cong(List<&2,Rt.FileRow>,List<&2,Rt.FileRow>,x => Rt.FileRow{pa,sa} <> x,Rt.group(Rt.FileRow{pb,sb},rest),Rt.FileRow{pb,sb} <> rest,rb)),{==})
    case EQ{}: meet(List<&2,Rt.FileRow>,Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{pb,sb} <> rest),Rt.FileRow{pb,Rt.fadd(sa,sb)} <> rest,Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{pa,sa} <> rest),Rt.FileRow{pa,Rt.fadd(sb,sa)} <> rest,red_eq(pa,sa,pb,sb,rest,qab),red_eq(pb,sb,pa,sa,rest,flipped(pa,pb,EQ{},qab)),pair_eq(pa,sa,pb,sb,rest,qab))
    case LT{}: meet(List<&2,Rt.FileRow>,Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{pb,sb} <> rest),Rt.FileRow{pb,sb} <> Rt.FileRow{pa,sa} <> rest,Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{pa,sa} <> rest),Rt.FileRow{pb,sb} <> Rt.FileRow{pa,sa} <> rest,Equal.trans(List<&2,Rt.FileRow>,Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{pb,sb} <> rest),Rt.FileRow{pb,sb} <> Rt.group(Rt.FileRow{pa,sa},rest),Rt.FileRow{pb,sb} <> Rt.FileRow{pa,sa} <> rest,red_lt(pa,sa,pb,sb,rest,qab),Equal.cong(List<&2,Rt.FileRow>,List<&2,Rt.FileRow>,x => Rt.FileRow{pb,sb} <> x,Rt.group(Rt.FileRow{pa,sa},rest),Rt.FileRow{pa,sa} <> rest,ra)),red_gt(pb,sb,pa,sa,rest,flipped(pa,pb,LT{},qab)),{==})

def swap_cons(+pa: String,+sa: Rt.FStat,+pb: String,+sb: Rt.FStat,+ph: String,+sh: Rt.FStat,+t: List<&2,Rt.FileRow>,ca: Cmp,cb: Cmp,+qa: {Rt.order(pa,ph) == ca : Cmp},+qb: {Rt.order(pb,ph) == cb : Cmp},ih: {Rt.group(Rt.FileRow{pa,sa},Rt.group(Rt.FileRow{pb,sb},t)) == Rt.group(Rt.FileRow{pb,sb},Rt.group(Rt.FileRow{pa,sa},t)) : List<&2,Rt.FileRow>}) -> {Rt.group(Rt.FileRow{pa,sa},Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{ph,sh} <> t)) == Rt.group(Rt.FileRow{pb,sb},Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{ph,sh} <> t)) : List<&2,Rt.FileRow>}:
  match ca cb:
    case LT{} LT{}: meet(List<&2,Rt.FileRow>,Rt.group(Rt.FileRow{pa,sa},Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{ph,sh} <> t)),Rt.FileRow{ph,sh} <> Rt.group(Rt.FileRow{pa,sa},Rt.group(Rt.FileRow{pb,sb},t)),Rt.group(Rt.FileRow{pb,sb},Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{ph,sh} <> t)),Rt.FileRow{ph,sh} <> Rt.group(Rt.FileRow{pb,sb},Rt.group(Rt.FileRow{pa,sa},t)),Equal.trans(List<&2,Rt.FileRow>,Rt.group(Rt.FileRow{pa,sa},Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{ph,sh} <> t)),Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{ph,sh} <> Rt.group(Rt.FileRow{pb,sb},t)),Rt.FileRow{ph,sh} <> Rt.group(Rt.FileRow{pa,sa},Rt.group(Rt.FileRow{pb,sb},t)),Equal.cong(List<&2,Rt.FileRow>,List<&2,Rt.FileRow>,x => Rt.group(Rt.FileRow{pa,sa},x),Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{ph,sh} <> t),Rt.FileRow{ph,sh} <> Rt.group(Rt.FileRow{pb,sb},t),red_lt(pb,sb,ph,sh,t,qb)),red_lt(pa,sa,ph,sh,Rt.group(Rt.FileRow{pb,sb},t),qa)),Equal.trans(List<&2,Rt.FileRow>,Rt.group(Rt.FileRow{pb,sb},Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{ph,sh} <> t)),Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{ph,sh} <> Rt.group(Rt.FileRow{pa,sa},t)),Rt.FileRow{ph,sh} <> Rt.group(Rt.FileRow{pb,sb},Rt.group(Rt.FileRow{pa,sa},t)),Equal.cong(List<&2,Rt.FileRow>,List<&2,Rt.FileRow>,x => Rt.group(Rt.FileRow{pb,sb},x),Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{ph,sh} <> t),Rt.FileRow{ph,sh} <> Rt.group(Rt.FileRow{pa,sa},t),red_lt(pa,sa,ph,sh,t,qa)),red_lt(pb,sb,ph,sh,Rt.group(Rt.FileRow{pa,sa},t),qb)),Equal.cong(List<&2,Rt.FileRow>,List<&2,Rt.FileRow>,x => Rt.FileRow{ph,sh} <> x,Rt.group(Rt.FileRow{pa,sa},Rt.group(Rt.FileRow{pb,sb},t)),Rt.group(Rt.FileRow{pb,sb},Rt.group(Rt.FileRow{pa,sa},t)),ih))
    case LT{} EQ{}: meet(List<&2,Rt.FileRow>,Rt.group(Rt.FileRow{pa,sa},Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{ph,sh} <> t)),Rt.FileRow{ph,Rt.fadd(sb,sh)} <> Rt.group(Rt.FileRow{pa,sa},t),Rt.group(Rt.FileRow{pb,sb},Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{ph,sh} <> t)),Rt.FileRow{ph,Rt.fadd(sb,sh)} <> Rt.group(Rt.FileRow{pa,sa},t),Equal.trans(List<&2,Rt.FileRow>,Rt.group(Rt.FileRow{pa,sa},Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{ph,sh} <> t)),Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{ph,Rt.fadd(sb,sh)} <> t),Rt.FileRow{ph,Rt.fadd(sb,sh)} <> Rt.group(Rt.FileRow{pa,sa},t),Equal.cong(List<&2,Rt.FileRow>,List<&2,Rt.FileRow>,x => Rt.group(Rt.FileRow{pa,sa},x),Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{ph,sh} <> t),Rt.FileRow{ph,Rt.fadd(sb,sh)} <> t,red_eq(pb,sb,ph,sh,t,qb)),red_lt(pa,sa,ph,Rt.fadd(sb,sh),t,qa)),Equal.trans(List<&2,Rt.FileRow>,Rt.group(Rt.FileRow{pb,sb},Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{ph,sh} <> t)),Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{ph,sh} <> Rt.group(Rt.FileRow{pa,sa},t)),Rt.FileRow{ph,Rt.fadd(sb,sh)} <> Rt.group(Rt.FileRow{pa,sa},t),Equal.cong(List<&2,Rt.FileRow>,List<&2,Rt.FileRow>,x => Rt.group(Rt.FileRow{pb,sb},x),Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{ph,sh} <> t),Rt.FileRow{ph,sh} <> Rt.group(Rt.FileRow{pa,sa},t),red_lt(pa,sa,ph,sh,t,qa)),red_eq(pb,sb,ph,sh,Rt.group(Rt.FileRow{pa,sa},t),qb)),{==})
    case LT{} GT{}: meet(List<&2,Rt.FileRow>,Rt.group(Rt.FileRow{pa,sa},Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{ph,sh} <> t)),Rt.FileRow{pb,sb} <> Rt.FileRow{ph,sh} <> Rt.group(Rt.FileRow{pa,sa},t),Rt.group(Rt.FileRow{pb,sb},Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{ph,sh} <> t)),Rt.FileRow{pb,sb} <> Rt.FileRow{ph,sh} <> Rt.group(Rt.FileRow{pa,sa},t),Equal.trans(List<&2,Rt.FileRow>,Rt.group(Rt.FileRow{pa,sa},Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{ph,sh} <> t)),Rt.FileRow{pb,sb} <> Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{ph,sh} <> t),Rt.FileRow{pb,sb} <> Rt.FileRow{ph,sh} <> Rt.group(Rt.FileRow{pa,sa},t),Equal.trans(List<&2,Rt.FileRow>,Rt.group(Rt.FileRow{pa,sa},Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{ph,sh} <> t)),Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{pb,sb} <> Rt.FileRow{ph,sh} <> t),Rt.FileRow{pb,sb} <> Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{ph,sh} <> t),Equal.cong(List<&2,Rt.FileRow>,List<&2,Rt.FileRow>,x => Rt.group(Rt.FileRow{pa,sa},x),Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{ph,sh} <> t),Rt.FileRow{pb,sb} <> Rt.FileRow{ph,sh} <> t,red_gt(pb,sb,ph,sh,t,qb)),red_lt(pa,sa,pb,sb,Rt.FileRow{ph,sh} <> t,order_trans(pa,ph,pb,qa,flipped(pb,ph,GT{},qb)))),Equal.cong(List<&2,Rt.FileRow>,List<&2,Rt.FileRow>,x => Rt.FileRow{pb,sb} <> x,Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{ph,sh} <> t),Rt.FileRow{ph,sh} <> Rt.group(Rt.FileRow{pa,sa},t),red_lt(pa,sa,ph,sh,t,qa))),Equal.trans(List<&2,Rt.FileRow>,Rt.group(Rt.FileRow{pb,sb},Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{ph,sh} <> t)),Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{ph,sh} <> Rt.group(Rt.FileRow{pa,sa},t)),Rt.FileRow{pb,sb} <> Rt.FileRow{ph,sh} <> Rt.group(Rt.FileRow{pa,sa},t),Equal.cong(List<&2,Rt.FileRow>,List<&2,Rt.FileRow>,x => Rt.group(Rt.FileRow{pb,sb},x),Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{ph,sh} <> t),Rt.FileRow{ph,sh} <> Rt.group(Rt.FileRow{pa,sa},t),red_lt(pa,sa,ph,sh,t,qa)),red_gt(pb,sb,ph,sh,Rt.group(Rt.FileRow{pa,sa},t),qb)),{==})
    case EQ{} LT{}: meet(List<&2,Rt.FileRow>,Rt.group(Rt.FileRow{pa,sa},Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{ph,sh} <> t)),Rt.FileRow{ph,Rt.fadd(sa,sh)} <> Rt.group(Rt.FileRow{pb,sb},t),Rt.group(Rt.FileRow{pb,sb},Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{ph,sh} <> t)),Rt.FileRow{ph,Rt.fadd(sa,sh)} <> Rt.group(Rt.FileRow{pb,sb},t),Equal.trans(List<&2,Rt.FileRow>,Rt.group(Rt.FileRow{pa,sa},Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{ph,sh} <> t)),Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{ph,sh} <> Rt.group(Rt.FileRow{pb,sb},t)),Rt.FileRow{ph,Rt.fadd(sa,sh)} <> Rt.group(Rt.FileRow{pb,sb},t),Equal.cong(List<&2,Rt.FileRow>,List<&2,Rt.FileRow>,x => Rt.group(Rt.FileRow{pa,sa},x),Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{ph,sh} <> t),Rt.FileRow{ph,sh} <> Rt.group(Rt.FileRow{pb,sb},t),red_lt(pb,sb,ph,sh,t,qb)),red_eq(pa,sa,ph,sh,Rt.group(Rt.FileRow{pb,sb},t),qa)),Equal.trans(List<&2,Rt.FileRow>,Rt.group(Rt.FileRow{pb,sb},Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{ph,sh} <> t)),Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{ph,Rt.fadd(sa,sh)} <> t),Rt.FileRow{ph,Rt.fadd(sa,sh)} <> Rt.group(Rt.FileRow{pb,sb},t),Equal.cong(List<&2,Rt.FileRow>,List<&2,Rt.FileRow>,x => Rt.group(Rt.FileRow{pb,sb},x),Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{ph,sh} <> t),Rt.FileRow{ph,Rt.fadd(sa,sh)} <> t,red_eq(pa,sa,ph,sh,t,qa)),red_lt(pb,sb,ph,Rt.fadd(sa,sh),t,qb)),{==})
    case EQ{} EQ{}: meet(List<&2,Rt.FileRow>,Rt.group(Rt.FileRow{pa,sa},Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{ph,sh} <> t)),Rt.FileRow{ph,Rt.fadd(sa,Rt.fadd(sb,sh))} <> t,Rt.group(Rt.FileRow{pb,sb},Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{ph,sh} <> t)),Rt.FileRow{ph,Rt.fadd(sb,Rt.fadd(sa,sh))} <> t,Equal.trans(List<&2,Rt.FileRow>,Rt.group(Rt.FileRow{pa,sa},Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{ph,sh} <> t)),Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{ph,Rt.fadd(sb,sh)} <> t),Rt.FileRow{ph,Rt.fadd(sa,Rt.fadd(sb,sh))} <> t,Equal.cong(List<&2,Rt.FileRow>,List<&2,Rt.FileRow>,x => Rt.group(Rt.FileRow{pa,sa},x),Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{ph,sh} <> t),Rt.FileRow{ph,Rt.fadd(sb,sh)} <> t,red_eq(pb,sb,ph,sh,t,qb)),red_eq(pa,sa,ph,Rt.fadd(sb,sh),t,qa)),Equal.trans(List<&2,Rt.FileRow>,Rt.group(Rt.FileRow{pb,sb},Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{ph,sh} <> t)),Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{ph,Rt.fadd(sa,sh)} <> t),Rt.FileRow{ph,Rt.fadd(sb,Rt.fadd(sa,sh))} <> t,Equal.cong(List<&2,Rt.FileRow>,List<&2,Rt.FileRow>,x => Rt.group(Rt.FileRow{pb,sb},x),Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{ph,sh} <> t),Rt.FileRow{ph,Rt.fadd(sa,sh)} <> t,red_eq(pa,sa,ph,sh,t,qa)),red_eq(pb,sb,ph,Rt.fadd(sa,sh),t,qb)),Equal.cong(Rt.FStat,List<&2,Rt.FileRow>,x => Rt.FileRow{ph,x} <> t,Rt.fadd(sa,Rt.fadd(sb,sh)),Rt.fadd(sb,Rt.fadd(sa,sh)),fadd_swap(sa,sb,sh)))
    case EQ{} GT{}: meet(List<&2,Rt.FileRow>,Rt.group(Rt.FileRow{pa,sa},Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{ph,sh} <> t)),Rt.FileRow{pb,sb} <> Rt.FileRow{ph,Rt.fadd(sa,sh)} <> t,Rt.group(Rt.FileRow{pb,sb},Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{ph,sh} <> t)),Rt.FileRow{pb,sb} <> Rt.FileRow{ph,Rt.fadd(sa,sh)} <> t,Equal.trans(List<&2,Rt.FileRow>,Rt.group(Rt.FileRow{pa,sa},Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{ph,sh} <> t)),Rt.FileRow{pb,sb} <> Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{ph,sh} <> t),Rt.FileRow{pb,sb} <> Rt.FileRow{ph,Rt.fadd(sa,sh)} <> t,Equal.trans(List<&2,Rt.FileRow>,Rt.group(Rt.FileRow{pa,sa},Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{ph,sh} <> t)),Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{pb,sb} <> Rt.FileRow{ph,sh} <> t),Rt.FileRow{pb,sb} <> Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{ph,sh} <> t),Equal.cong(List<&2,Rt.FileRow>,List<&2,Rt.FileRow>,x => Rt.group(Rt.FileRow{pa,sa},x),Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{ph,sh} <> t),Rt.FileRow{pb,sb} <> Rt.FileRow{ph,sh} <> t,red_gt(pb,sb,ph,sh,t,qb)),red_lt(pa,sa,pb,sb,Rt.FileRow{ph,sh} <> t,eq_then_lt(pa,ph,pb,qa,flipped(pb,ph,GT{},qb)))),Equal.cong(List<&2,Rt.FileRow>,List<&2,Rt.FileRow>,x => Rt.FileRow{pb,sb} <> x,Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{ph,sh} <> t),Rt.FileRow{ph,Rt.fadd(sa,sh)} <> t,red_eq(pa,sa,ph,sh,t,qa))),Equal.trans(List<&2,Rt.FileRow>,Rt.group(Rt.FileRow{pb,sb},Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{ph,sh} <> t)),Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{ph,Rt.fadd(sa,sh)} <> t),Rt.FileRow{pb,sb} <> Rt.FileRow{ph,Rt.fadd(sa,sh)} <> t,Equal.cong(List<&2,Rt.FileRow>,List<&2,Rt.FileRow>,x => Rt.group(Rt.FileRow{pb,sb},x),Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{ph,sh} <> t),Rt.FileRow{ph,Rt.fadd(sa,sh)} <> t,red_eq(pa,sa,ph,sh,t,qa)),red_gt(pb,sb,ph,Rt.fadd(sa,sh),t,qb)),{==})
    case GT{} LT{}: meet(List<&2,Rt.FileRow>,Rt.group(Rt.FileRow{pa,sa},Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{ph,sh} <> t)),Rt.FileRow{pa,sa} <> Rt.FileRow{ph,sh} <> Rt.group(Rt.FileRow{pb,sb},t),Rt.group(Rt.FileRow{pb,sb},Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{ph,sh} <> t)),Rt.FileRow{pa,sa} <> Rt.FileRow{ph,sh} <> Rt.group(Rt.FileRow{pb,sb},t),Equal.trans(List<&2,Rt.FileRow>,Rt.group(Rt.FileRow{pa,sa},Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{ph,sh} <> t)),Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{ph,sh} <> Rt.group(Rt.FileRow{pb,sb},t)),Rt.FileRow{pa,sa} <> Rt.FileRow{ph,sh} <> Rt.group(Rt.FileRow{pb,sb},t),Equal.cong(List<&2,Rt.FileRow>,List<&2,Rt.FileRow>,x => Rt.group(Rt.FileRow{pa,sa},x),Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{ph,sh} <> t),Rt.FileRow{ph,sh} <> Rt.group(Rt.FileRow{pb,sb},t),red_lt(pb,sb,ph,sh,t,qb)),red_gt(pa,sa,ph,sh,Rt.group(Rt.FileRow{pb,sb},t),qa)),Equal.trans(List<&2,Rt.FileRow>,Rt.group(Rt.FileRow{pb,sb},Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{ph,sh} <> t)),Rt.FileRow{pa,sa} <> Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{ph,sh} <> t),Rt.FileRow{pa,sa} <> Rt.FileRow{ph,sh} <> Rt.group(Rt.FileRow{pb,sb},t),Equal.trans(List<&2,Rt.FileRow>,Rt.group(Rt.FileRow{pb,sb},Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{ph,sh} <> t)),Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{pa,sa} <> Rt.FileRow{ph,sh} <> t),Rt.FileRow{pa,sa} <> Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{ph,sh} <> t),Equal.cong(List<&2,Rt.FileRow>,List<&2,Rt.FileRow>,x => Rt.group(Rt.FileRow{pb,sb},x),Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{ph,sh} <> t),Rt.FileRow{pa,sa} <> Rt.FileRow{ph,sh} <> t,red_gt(pa,sa,ph,sh,t,qa)),red_lt(pb,sb,pa,sa,Rt.FileRow{ph,sh} <> t,order_trans(pb,ph,pa,qb,flipped(pa,ph,GT{},qa)))),Equal.cong(List<&2,Rt.FileRow>,List<&2,Rt.FileRow>,x => Rt.FileRow{pa,sa} <> x,Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{ph,sh} <> t),Rt.FileRow{ph,sh} <> Rt.group(Rt.FileRow{pb,sb},t),red_lt(pb,sb,ph,sh,t,qb))),{==})
    case GT{} EQ{}: meet(List<&2,Rt.FileRow>,Rt.group(Rt.FileRow{pa,sa},Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{ph,sh} <> t)),Rt.FileRow{pa,sa} <> Rt.FileRow{ph,Rt.fadd(sb,sh)} <> t,Rt.group(Rt.FileRow{pb,sb},Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{ph,sh} <> t)),Rt.FileRow{pa,sa} <> Rt.FileRow{ph,Rt.fadd(sb,sh)} <> t,Equal.trans(List<&2,Rt.FileRow>,Rt.group(Rt.FileRow{pa,sa},Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{ph,sh} <> t)),Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{ph,Rt.fadd(sb,sh)} <> t),Rt.FileRow{pa,sa} <> Rt.FileRow{ph,Rt.fadd(sb,sh)} <> t,Equal.cong(List<&2,Rt.FileRow>,List<&2,Rt.FileRow>,x => Rt.group(Rt.FileRow{pa,sa},x),Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{ph,sh} <> t),Rt.FileRow{ph,Rt.fadd(sb,sh)} <> t,red_eq(pb,sb,ph,sh,t,qb)),red_gt(pa,sa,ph,Rt.fadd(sb,sh),t,qa)),Equal.trans(List<&2,Rt.FileRow>,Rt.group(Rt.FileRow{pb,sb},Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{ph,sh} <> t)),Rt.FileRow{pa,sa} <> Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{ph,sh} <> t),Rt.FileRow{pa,sa} <> Rt.FileRow{ph,Rt.fadd(sb,sh)} <> t,Equal.trans(List<&2,Rt.FileRow>,Rt.group(Rt.FileRow{pb,sb},Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{ph,sh} <> t)),Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{pa,sa} <> Rt.FileRow{ph,sh} <> t),Rt.FileRow{pa,sa} <> Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{ph,sh} <> t),Equal.cong(List<&2,Rt.FileRow>,List<&2,Rt.FileRow>,x => Rt.group(Rt.FileRow{pb,sb},x),Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{ph,sh} <> t),Rt.FileRow{pa,sa} <> Rt.FileRow{ph,sh} <> t,red_gt(pa,sa,ph,sh,t,qa)),red_lt(pb,sb,pa,sa,Rt.FileRow{ph,sh} <> t,eq_then_lt(pb,ph,pa,qb,flipped(pa,ph,GT{},qa)))),Equal.cong(List<&2,Rt.FileRow>,List<&2,Rt.FileRow>,x => Rt.FileRow{pa,sa} <> x,Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{ph,sh} <> t),Rt.FileRow{ph,Rt.fadd(sb,sh)} <> t,red_eq(pb,sb,ph,sh,t,qb))),{==})
    case GT{} GT{}: meet(List<&2,Rt.FileRow>,Rt.group(Rt.FileRow{pa,sa},Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{ph,sh} <> t)),Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{pb,sb} <> Rt.FileRow{ph,sh} <> t),Rt.group(Rt.FileRow{pb,sb},Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{ph,sh} <> t)),Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{pa,sa} <> Rt.FileRow{ph,sh} <> t),Equal.cong(List<&2,Rt.FileRow>,List<&2,Rt.FileRow>,x => Rt.group(Rt.FileRow{pa,sa},x),Rt.group(Rt.FileRow{pb,sb},Rt.FileRow{ph,sh} <> t),Rt.FileRow{pb,sb} <> Rt.FileRow{ph,sh} <> t,red_gt(pb,sb,ph,sh,t,qb)),Equal.cong(List<&2,Rt.FileRow>,List<&2,Rt.FileRow>,x => Rt.group(Rt.FileRow{pb,sb},x),Rt.group(Rt.FileRow{pa,sa},Rt.FileRow{ph,sh} <> t),Rt.FileRow{pa,sa} <> Rt.FileRow{ph,sh} <> t,red_gt(pa,sa,ph,sh,t,qa)),pair_swap(pa,sa,pb,sb,Rt.FileRow{ph,sh} <> t,Rt.order(pa,pb),{==},red_gt(pa,sa,ph,sh,t,qa),red_gt(pb,sb,ph,sh,t,qb)))

# Swapping two insertions gives the same grouped rows; xs first so the induction is structural.
def group_swap(xs: List<&2,Rt.FileRow>,+a: Rt.FileRow,+b: Rt.FileRow) -> {Rt.group(a,Rt.group(b,xs)) == Rt.group(b,Rt.group(a,xs)) : List<&2,Rt.FileRow>}:
  match xs a b:
    case Nil{} Rt.FileRow{+pa,+sa} Rt.FileRow{+pb,+sb}: pair_swap(pa,sa,pb,sb,Nil{},Rt.order(pa,pb),{==},{==},{==})
    case Rt.FileRow{+ph,+sh} <> +t Rt.FileRow{+pa,+sa} Rt.FileRow{+pb,+sb}: swap_cons(pa,sa,pb,sb,ph,sh,t,Rt.order(pa,ph),Rt.order(pb,ph),{==},{==},group_swap(t,Rt.FileRow{pa,sa},Rt.FileRow{pb,sb}))

