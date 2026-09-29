import Base
import ./../StreamingSha.bend as S

def value(result: Maybe<&2,String>) -> String:
  match result:
    case Some{digest}: digest
    case None{}: "unavailable"

def main() -> String:
  value(S.hash_pages(Nil{})) ++ "|" ++ value(S.hash_pages(["abc"])) ++ "|" ++ value(S.hash_pages(["a","bc"])) ++ "|" ++ value(S.hash_pages(["π\r","\ntail"])) ++ "|" ++ value(S.hash_text("abc")) ++ "|" ++ value(S.hash_text("π\r\ntail"))
