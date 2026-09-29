import Base
import ./Sha256.bend as S
import ./Utf8Encode.bend as U
import ./Wire.bend as W
import ./StreamingSha.bend as Stream

def encoded(value: U.Encoded) -> Maybe<&2,String>:
  match value:
    case U.Encoded{bytes,length,True{}}: S.hash(bytes,length)
    case _: None{}

def hash(text: String) -> Maybe<&2,String>:
  encoded(U.encode(text))

def identity_result(result: Maybe<&2,String>) -> String:
  match result:
    case Some{digest}: digest
    case None{}: "identity-unavailable"

def identity_hash(short: Bool,text: String) -> Maybe<&2,String>:
  match short:
    case True{}: hash(text)
    case False{}: Stream.hash_text(text)

# This invalid sentinel is deliberately not a digest. Receivers must reject it;
# it cannot silently become a shared identity for unsupported inputs.
def identity(+text: String) -> String:
  identity_result(identity_hash(Nat.is_le(String.length(text),4194304n),text))

def emitted(value: Maybe<&2,String>) -> String:
  match value:
    case Some{digest}: W.row(["DIGEST","sha256",digest])
    case None{}: "ERROR\tSHA256 byte/scalar/length bound\n"

def process(row: W.Record) -> String:
  match row:
    case W.Row{text <> Nil{}}: emitted(hash(text))
    case _: "ERROR\tSHA256 input record\n"
