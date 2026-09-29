import Base

type Encoded is Data:
  Encoded{bytes: List<&2,U32>,length: U32,valid: Bool}

def byte(+value: U32) -> U32:
  (128 + U32.and(value,63) : U32)

# Bytes are prepended in reverse order and the list is reversed once at the end.
def push_sized(one: Bool,two: Bool,three: Bool,+value: U32,bytes: List<&2,U32>) -> List<&2,U32>:
  match one two three:
    case True{} _ _: value <> bytes
    case False{} True{} _: byte(value) <> (192 + U32.shrn(value,6n) : U32) <> bytes
    case False{} False{} True{}: byte(value) <> byte(U32.shrn(value,6n)) <> (224 + U32.shrn(value,12n) : U32) <> bytes
    case False{} False{} False{}: byte(value) <> byte(U32.shrn(value,6n)) <> byte(U32.shrn(value,12n)) <> (240 + U32.shrn(value,18n) : U32) <> bytes

def push(+value: U32,bytes: List<&2,U32>) -> List<&2,U32>:
  push_sized(U32.is_le(value,127),U32.is_le(value,2047),U32.is_le(value,65535),value,bytes)

def size(+value: U32) -> U32:
  Bool.pick(U32,U32.is_le(value,127),1,Bool.pick(U32,U32.is_le(value,2047),2,Bool.pick(U32,U32.is_le(value,65535),3,4)))

def scalar(+value: U32) -> Bool:
  U32.is_le(value,1114111) && (U32.is_lt(value,55296) || U32.is_gt(value,57343))

def ended(valid: Bool,bytes: List<&2,U32>,+length: U32) -> Encoded:
  match valid:
    case True{}: Encoded{List.reverse(&2,U32,bytes),length,U32.is_le(length,16777216)}
    case False{}: Encoded{Nil{},0,False{}}

# Invalid scalars and inputs beyond the bound yield Encoded{Nil,0,False}.
def loop(text: String,bytes: List<&2,U32>,+length: U32,valid: Bool) -> Encoded:
  match text:
    case SNil{}: ended(valid,bytes,length)
    case SCon{Chr{+value},tail}: loop(tail,push(value,bytes),(length + size(value) : U32),valid && scalar(value) && U32.is_lt(length,16777216))

def encode(text: String) -> Encoded:
  loop(text,Nil{},0,True{})
