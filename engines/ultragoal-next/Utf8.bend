import Base

type Decoder is Data:
  Decoder{remaining: U32, code: U32, minimum: U32, valid: Bool, text: String}

def finished(ok: Bool, text: String) -> Maybe<&2,String>:
  match ok:
    case True{}: Some{String.reverse(text)}
    case False{}: None{}

def start_four(ok: Bool,a: U32,valid: Bool,text: String) -> Decoder:
  match ok:
    case True{}: Decoder{3,(a - 240 : U32),65536,valid,text}
    case False{}: Decoder{0,0,0,False{},text}

def start_three(ok: Bool,+a: U32,valid: Bool,text: String) -> Decoder:
  match ok:
    case True{}: Decoder{2,(a - 224 : U32),2048,valid,text}
    case False{}: start_four(U32.is_ge(a,240) && U32.is_le(a,244),a,valid,text)

def start_two(ok: Bool,+a: U32,valid: Bool,text: String) -> Decoder:
  match ok:
    case True{}: Decoder{1,(a - 192 : U32),128,valid,text}
    case False{}: start_three(U32.is_ge(a,224) && U32.is_le(a,239),a,valid,text)

def start_ascii(ok: Bool,+a: U32,valid: Bool,text: String) -> Decoder:
  match ok:
    case True{}: Decoder{0,0,0,valid,SCon{Chr{a},text}}
    case False{}: start_two(U32.is_ge(a,194) && U32.is_le(a,223),a,valid,text)

def next_last(last: Bool,n: U32,+code: U32,+minimum: U32,valid: Bool,text: String) -> Decoder:
  match last:
    case True{}: Decoder{0,0,0,valid && U32.is_ge(code,minimum) && U32.is_le(code,1114111) && (U32.is_lt(code,55296) || U32.is_gt(code,57343)),SCon{Chr{code},text}}
    case False{}: Decoder{n,code,minimum,valid,text}

def step_start(start: Bool,+a: U32,+n: U32,code: U32,minimum: U32,valid: Bool,text: String) -> Decoder:
  match start:
    case True{}: start_ascii(U32.is_le(a,127),a,valid,text)
    case False{}: next_last(U32.is_eq(n,1),(n - 1 : U32),(code * 64 + a - 128 : U32),minimum,valid && U32.is_ge(a,128) && U32.is_le(a,191),text)

def loop(bytes: List<&2,U32>,state: Decoder) -> Maybe<&2,String>:
  match bytes state:
    case Nil{} Decoder{n,code,minimum,valid,text}: finished(valid && U32.is_eq(n,0),text)
    case h <> t Decoder{+n,code,minimum,valid,text}: loop(t,step_start(U32.is_eq(n,0),h,n,code,minimum,valid,text))

def decode(bytes: List<&2,U32>) -> Maybe<&2,String>:
  loop(bytes,Decoder{0,0,0,True{},""})

def feed(bytes: List<&2,U32>,state: Decoder) -> Decoder:
  match bytes state:
    case Nil{} state: state
    case h <> t Decoder{+n,code,minimum,valid,text}: feed(t,step_start(U32.is_eq(n,0),h,n,code,minimum,valid,text))

def finish(state: Decoder) -> Maybe<&2,String>:
  match state:
    case Decoder{n,code,minimum,valid,text}: finished(valid && U32.is_eq(n,0),text)

# Same result as decode on the concatenated chunks, without first copying them.
def decode_chunks(chunks: List<&2,List<&2,U32>>,state: Decoder) -> Maybe<&2,String>:
  match chunks:
    case Nil{}: finish(state)
    case h <> t: decode_chunks(t,feed(h,state))

def decode_all(chunks: List<&2,List<&2,U32>>) -> Maybe<&2,String>:
  decode_chunks(chunks,Decoder{0,0,0,True{},""})
