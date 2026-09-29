import Base
import ./Sha256.bend as S
import ./Utf8Encode.bend as U

# Independent incremental entry. Sha256.hash and its protected 16 MiB law are
# unchanged; one transport page is encoded and consumed before the next arrives.
type Stream is Data:
  Stream{digest: S.State, pending: List<&2,U32>, length: Nat, valid: Bool}

type Partial is Data:
  Partial{digest: S.State, pending: List<&2,U32>, valid: Bool}

def empty() -> Stream:
  Stream{S.initial(),Nil{},0n,True{}}

def compressed(block: S.Block,digest: S.State) -> Partial:
  match block:
    case S.Block{w0,w1,w2,w3,w4,w5,w6,w7,w8,w9,w10,w11,w12,w13,w14,w15,rest,used,valid}:
      Partial{S.compress(digest,w0,w1,w2,w3,w4,w5,w6,w7,w8,w9,w10,w11,w12,w13,w14,w15),Nil{},valid && U32.is_eq(used,64) && Nat.is_eq(List.length(&2,U32,rest),0n)}

def block(bytes: List<&2,U32>,digest: S.State) -> Partial:
  compressed(S.read(16n,bytes,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,True{}),digest)

def decided_block(value: Partial,rest: List<&2,U32>,valid: Bool,next: List<&2,U32> -> S.State -> Bool -> Partial) -> Partial:
  match value:
    case Partial{updated,_,ok}: next(rest,updated,valid && ok)

def decided(full: Bool,bytes: List<&2,U32>,rest: List<&2,U32>,digest: S.State,valid: Bool,next: List<&2,U32> -> S.State -> Bool -> Partial) -> Partial:
  match full:
    case True{}: decided_block(block(bytes,digest),rest,valid,next)
    case False{}: Partial{digest,bytes,valid}

def loop(fuel: Nat,+bytes: List<&2,U32>,digest: S.State,valid: Bool) -> Partial:
  match fuel:
    case 0n: Partial{digest,bytes,False{}}
    case 1n+p:
      +part = List.take(&2,U32,bytes,64n)
      decided(Nat.is_eq(List.length(&2,U32,part),64n),part,List.drop(&2,U32,bytes,64n),digest,valid,rest => updated => ok => loop(p,rest,updated,ok))

def fed_part(value: Partial,length: Nat,size: U32) -> Stream:
  match value:
    case Partial{updated,remaining,good}: Stream{updated,remaining,Nat.add(length,U32.to_nat(size)),good}

def fed(value: U.Encoded,stream: Stream) -> Stream:
  match value stream:
    case U.Encoded{bytes,+size,+ok} Stream{digest,pending,+length,valid}:
      fed_part(loop(2n+U32.to_nat(U32.shrn(size,6n)),List.append(&2,U32,pending,bytes),digest,valid && ok),length,size)

def feed(stream: Stream,text: String) -> Stream:
  fed(U.encode(text),stream)

def zeros(n: Nat) -> List<&2,U32>:
  match n:
    case 0n: Nil{}
    case 1n+p: 0 <> zeros(p)

def byte(word: U32,shift: Nat) -> U32:
  U32.and(U32.shrn(word,shift),255)

def word_bytes(+word: U32) -> List<&2,U32>:
  [byte(word,24n),byte(word,16n),byte(word,8n),byte(word,0n)]

def two32() -> Nat:
  Nat.mul(65536n,65536n)

def parsed(value: Maybe<&2,U32>) -> U32:
  match value:
    case Some{word}: word
    case None{}: 0

def split(+value: Nat) -> List<&2,U32>:
  List.append(&2,U32,word_bytes(parsed(U32.read(Nat.show(Nat.div(value,two32()))))),word_bytes(parsed(U32.read(Nat.show(Nat.mod(value,two32()))))))

def pad_count(rem: Nat,small: Bool) -> Nat:
  match small:
    case True{}: Nat.sub(55n,rem)
    case False{}: Nat.sub(119n,rem)

def pad(pending: List<&2,U32>,+length: Nat) -> List<&2,U32>:
  +rem = Nat.mod(length,64n)
  List.append(&2,U32,pending,128 <> List.append(&2,U32,zeros(pad_count(rem,Nat.is_le(rem,55n))),split(Nat.mul(length,8n))))

def closed_remaining(remaining: List<&2,U32>,digest: S.State) -> Maybe<&2,String>:
  match remaining:
    case Nil{}: Some{S.hex(digest)}
    case _: None{}

def closed(partial: Partial) -> Maybe<&2,String>:
  match partial:
    case Partial{digest,remaining,True{}}: closed_remaining(remaining,digest)
    case _: None{}

def finish(stream: Stream) -> Maybe<&2,String>:
  match stream:
    case Stream{digest,pending,+length,True{}}: closed(loop(3n,pad(pending,length),digest,True{}))
    case _: None{}

def pages(parts: List<&2,String>,stream: Stream) -> Maybe<&2,String>:
  match parts:
    case Nil{}: finish(stream)
    case h <> t: pages(t,feed(stream,h))

def hash_pages(parts: List<&2,String>) -> Maybe<&2,String>:
  pages(parts,empty())

# A long Bend computation key is already one String. Slice on scalar boundaries;
# each page has at most 262,144 scalars, hence at most 1 MiB of UTF-8 bytes.
def text_loop(fuel: Nat,+remaining: String,stream: Stream) -> Maybe<&2,String>:
  match fuel:
    case 0n: finish(stream)
    case 1n+p: text_loop(p,String.drop(remaining,262144n),feed(stream,String.take(remaining,262144n)))

def hash_text(+text: String) -> Maybe<&2,String>:
  text_loop(1n+Nat.div(String.length(text),262144n),text,empty())
