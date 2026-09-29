import Base

# Bounded byte-message SHA-256. Byte values and actual consumed length are
# checked; mismatched length, non-byte values, or input beyond16MiB returns None.
type State is Data:
  H{a: U32,b: U32,c: U32,d: U32,e: U32,f: U32,g: U32,h: U32}

def rotate(+x: U32,+n: Nat) -> U32:
  U32.or(U32.shrn(x,n),U32.shln(x,Nat.sub(32n,n)))

def small0(+x: U32) -> U32:
  U32.xor(U32.xor(rotate(x,7n),rotate(x,18n)),U32.shrn(x,3n))

def small1(+x: U32) -> U32:
  U32.xor(U32.xor(rotate(x,17n),rotate(x,19n)),U32.shrn(x,10n))

def big0(+x: U32) -> U32:
  U32.xor(U32.xor(rotate(x,2n),rotate(x,13n)),rotate(x,22n))

def big1(+x: U32) -> U32:
  U32.xor(U32.xor(rotate(x,6n),rotate(x,11n)),rotate(x,25n))

def choose(+x: U32,y: U32,z: U32) -> U32:
  U32.xor(U32.and(x,y),U32.and(U32.not(x),z))

def majority(+x: U32,+y: U32,+z: U32) -> U32:
  U32.xor(U32.xor(U32.and(x,y),U32.and(x,z)),U32.and(y,z))

# Rounds keep the eight working words and a sliding window of the next sixteen
# schedule words in U32 parameters, so a block allocates no per-round values.
def rounds(ks: List<&2,U32>,+a: U32,+b: U32,+c: U32,+d: U32,+e: U32,+f: U32,+g: U32,+h: U32,+w0: U32,+w1: U32,+w2: U32,+w3: U32,+w4: U32,+w5: U32,+w6: U32,+w7: U32,+w8: U32,+w9: U32,+w10: U32,+w11: U32,+w12: U32,+w13: U32,+w14: U32,+w15: U32) -> State:
  match ks:
    case Nil{}: H{a,b,c,d,e,f,g,h}
    case k <> tail:
      +t1 = (h + big1(e) + choose(e,f,g) + k + w0 : U32)
      t2 = (big0(a) + majority(a,b,c) : U32)
      rounds(tail,(t1 + t2 : U32),a,b,c,(d + t1 : U32),e,f,g,w1,w2,w3,w4,w5,w6,w7,w8,w9,w10,w11,w12,w13,w14,w15,(small1(w14) + w9 + small0(w1) + w0 : U32))

def add(a: State,b: State) -> State:
  match a b:
    case H{a,b,c,d,e,f,g,h} H{i,j,k,l,m,n,o,p}: H{(a + i : U32),(b + j : U32),(c + k : U32),(d + l : U32),(e + m : U32),(f + n : U32),(g + o : U32),(h + p : U32)}

# Constants are data, avoiding a 64 - branch literal decision tree.
def constants() -> List<&2,U32>:
  [1116352408,1899447441,3049323471,3921009573,961987163,1508970993,2453635748,2870763221,3624381080,310598401,607225278,1426881987,1925078388,2162078206,2614888103,3248222580,3835390401,4022224774,264347078,604807628,770255983,1249150122,1555081692,1996064986,2554220882,2821834349,2952996808,3210313671,3336571891,3584528711,113926993,338241895,666307205,773529912,1294757372,1396182291,1695183700,1986661051,2177026350,2456956037,2730485921,2820302411,3259730800,3345764771,3516065817,3600352804,4094571909,275423344,430227734,506948616,659060556,883997877,958139571,1322822218,1537002063,1747873779,1955562222,2024104815,2227730452,2361852424,2428436474,2756734187,3204031479,3329325298]

def initial() -> State:
  H{1779033703,3144134277,1013904242,2773480762,1359893119,2600822924,528734635,1541459225}

def compress(state: State,w0: U32,w1: U32,w2: U32,w3: U32,w4: U32,w5: U32,w6: U32,w7: U32,w8: U32,w9: U32,w10: U32,w11: U32,w12: U32,w13: U32,w14: U32,w15: U32) -> State:
  match state:
    case H{+a,+b,+c,+d,+e,+f,+g,+h}: add(H{a,b,c,d,e,f,g,h},rounds(constants(),a,b,c,d,e,f,g,h,w0,w1,w2,w3,w4,w5,w6,w7,w8,w9,w10,w11,w12,w13,w14,w15))

def word(a: U32,b: U32,c: U32,d: U32) -> U32:
  U32.or(U32.or(U32.shln(a,24n),U32.shln(b,16n)),U32.or(U32.shln(c,8n),d))

type Block is Data:
  Block{w0: U32, w1: U32, w2: U32, w3: U32, w4: U32, w5: U32, w6: U32, w7: U32, w8: U32, w9: U32, w10: U32, w11: U32, w12: U32, w13: U32, w14: U32, w15: U32, rest: List<&2,U32>, used: U32, valid: Bool}

# Shifts up to sixteen big-endian words into the window; missing bytes are zero.
def read(n: Nat,input: List<&2,U32>,+w0: U32,+w1: U32,+w2: U32,+w3: U32,+w4: U32,+w5: U32,+w6: U32,+w7: U32,+w8: U32,+w9: U32,+w10: U32,+w11: U32,+w12: U32,+w13: U32,+w14: U32,+w15: U32,+used: U32,valid: Bool) -> Block:
  match n:
    case 0n: Block{w0,w1,w2,w3,w4,w5,w6,w7,w8,w9,w10,w11,w12,w13,w14,w15,input,used,valid}
    case 1n+p:
      match input:
        case Nil{}: read(p,Nil{},w1,w2,w3,w4,w5,w6,w7,w8,w9,w10,w11,w12,w13,w14,w15,0,used,valid)
        case +a <> Nil{}: read(p,Nil{},w1,w2,w3,w4,w5,w6,w7,w8,w9,w10,w11,w12,w13,w14,w15,word(a,0,0,0),(used + 1 : U32),valid && U32.is_le(a,255))
        case +a <> +b <> Nil{}: read(p,Nil{},w1,w2,w3,w4,w5,w6,w7,w8,w9,w10,w11,w12,w13,w14,w15,word(a,b,0,0),(used + 2 : U32),valid && U32.is_le(a,255) && U32.is_le(b,255))
        case +a <> +b <> +c <> Nil{}: read(p,Nil{},w1,w2,w3,w4,w5,w6,w7,w8,w9,w10,w11,w12,w13,w14,w15,word(a,b,c,0),(used + 3 : U32),valid && U32.is_le(a,255) && U32.is_le(b,255) && U32.is_le(c,255))
        case +a <> +b <> +c <> +d <> tail: read(p,tail,w1,w2,w3,w4,w5,w6,w7,w8,w9,w10,w11,w12,w13,w14,w15,word(a,b,c,d),(used + 4 : U32),valid && U32.is_le(a,255) && U32.is_le(b,255) && U32.is_le(c,255) && U32.is_le(d,255))

def mark(+index: U32,+slot: U32,+bit: U32,word: U32) -> U32:
  U32.or(word,Bool.pick(U32,U32.is_eq(index,slot),bit,0))

def length_block(state: State,+length: U32) -> State:
  compress(state,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,(length * 8 : U32))

def final_block(fits: Bool,state: State,+length: U32,w0: U32,w1: U32,w2: U32,w3: U32,w4: U32,w5: U32,w6: U32,w7: U32,w8: U32,w9: U32,w10: U32,w11: U32,w12: U32,w13: U32,w14: U32,w15: U32) -> State:
  match fits:
    case True{}: compress(state,w0,w1,w2,w3,w4,w5,w6,w7,w8,w9,w10,w11,w12,w13,0,(length * 8 : U32))
    case False{}: length_block(compress(state,w0,w1,w2,w3,w4,w5,w6,w7,w8,w9,w10,w11,w12,w13,w14,w15),length)

def padded(+used: U32,state: State,+length: U32,w0: U32,w1: U32,w2: U32,w3: U32,w4: U32,w5: U32,w6: U32,w7: U32,w8: U32,w9: U32,w10: U32,w11: U32,w12: U32,w13: U32,w14: U32,w15: U32) -> State:
  +index = U32.shrn(used,2n)
  +bit = U32.shln(128,U32.to_nat(((3 - U32.and(used,3)) * 8 : U32)))
  final_block(U32.is_le(used,55),state,length,mark(index,0,bit,w0),mark(index,1,bit,w1),mark(index,2,bit,w2),mark(index,3,bit,w3),mark(index,4,bit,w4),mark(index,5,bit,w5),mark(index,6,bit,w6),mark(index,7,bit,w7),mark(index,8,bit,w8),mark(index,9,bit,w9),mark(index,10,bit,w10),mark(index,11,bit,w11),mark(index,12,bit,w12),mark(index,13,bit,w13),mark(index,14,bit,w14),mark(index,15,bit,w15))

def final_checked(equal: Bool,+used: U32,state: State,+length: U32,w0: U32,w1: U32,w2: U32,w3: U32,w4: U32,w5: U32,w6: U32,w7: U32,w8: U32,w9: U32,w10: U32,w11: U32,w12: U32,w13: U32,w14: U32,w15: U32) -> Maybe<&2,State>:
  match equal:
    case True{}: Some{padded(used,state,length,w0,w1,w2,w3,w4,w5,w6,w7,w8,w9,w10,w11,w12,w13,w14,w15)}
    case False{}: None{}

def block_ready(full: Bool,next: List<&2,U32> -> State -> U32 -> Maybe<&2,State>,rest: List<&2,U32>,+used: U32,state: State,+expected: U32,+consumed: U32,w0: U32,w1: U32,w2: U32,w3: U32,w4: U32,w5: U32,w6: U32,w7: U32,w8: U32,w9: U32,w10: U32,w11: U32,w12: U32,w13: U32,w14: U32,w15: U32) -> Maybe<&2,State>:
  match full:
    case True{}: next(rest,compress(state,w0,w1,w2,w3,w4,w5,w6,w7,w8,w9,w10,w11,w12,w13,w14,w15),(consumed + 64 : U32))
    case False{}: final_checked(U32.is_eq((consumed + used : U32),expected),used,state,(consumed + used : U32),w0,w1,w2,w3,w4,w5,w6,w7,w8,w9,w10,w11,w12,w13,w14,w15)

def block_read(next: List<&2,U32> -> State -> U32 -> Maybe<&2,State>,block: Block,state: State,expected: U32,consumed: U32) -> Maybe<&2,State>:
  match block:
    case Block{w0,w1,w2,w3,w4,w5,w6,w7,w8,w9,w10,w11,w12,w13,w14,w15,_,_,False{}}: None{}
    case Block{w0,w1,w2,w3,w4,w5,w6,w7,w8,w9,w10,w11,w12,w13,w14,w15,rest,+used,True{}}: block_ready(U32.is_eq(used,64),next,rest,used,state,expected,consumed,w0,w1,w2,w3,w4,w5,w6,w7,w8,w9,w10,w11,w12,w13,w14,w15)

def blocks(fuel: Nat,+expected: U32) -> List<&2,U32> -> State -> U32 -> Maybe<&2,State>:
  match fuel:
    case 0n: input => state => consumed => None{}
    case 1n+p: input => state => consumed => block_read(blocks(p,expected),read(16n,input,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,True{}),state,expected,consumed)

def digit(small: Bool,value: U32) -> Char:
  match small:
    case True{}: Chr{(value + 48 : U32)}
    case False{}: Chr{(value + 87 : U32)}

def hex_loop(fuel: Nat,+value: U32,tail: String) -> String:
  match fuel:
    case 0n: tail
    case 1n+p: hex_loop(p,U32.shrn(value,4n),SCon{digit(U32.is_lt(U32.and(value,15),10),U32.and(value,15)),tail})

def hex(state: State) -> String:
  match state:
    case H{a,b,c,d,e,f,g,h}: hex_loop(8n,a,"") ++ hex_loop(8n,b,"") ++ hex_loop(8n,c,"") ++ hex_loop(8n,d,"") ++ hex_loop(8n,e,"") ++ hex_loop(8n,f,"") ++ hex_loop(8n,g,"") ++ hex_loop(8n,h,"")

def rendered(result: Maybe<&2,State>) -> Maybe<&2,String>:
  match result:
    case Some{state}: Some{hex(state)}
    case None{}: None{}

def bounded(allowed: Bool,input: List<&2,U32>,+length: U32) -> Maybe<&2,String>:
  match allowed:
    case True{}: rendered(blocks((2n + U32.to_nat(U32.shrn(length,6n)) : Nat),length,input,initial(),0))
    case False{}: None{}

def hash(input: List<&2,U32>,+length: U32) -> Maybe<&2,String>:
  bounded(U32.is_le(length,16777216),input,length)
