import Base
import ./Sha256.bend as Sha256
import ./QueryGraphProof.bend as QueryGraphProof
import ./ChunkProof.bend as ChunkProof
import ./RetrievalProof.bend as RP
import ./Retrieval.bend as Rt
import ./Lexicon.bend as Lx
import ./LAWS.bend as L
import ./Core.bend as C
import ./Engine.bend as E
import ./Wire.bend as W
import ./Identity.bend as Identity
import ./ParsedValue.bend as ParsedValue
import ./GroupedParse.bend as Grouped
import ./ParseCache.bend as ParseCache

def L.cancelled_rejects(b,a,g,s):
  {==}

def L.superseded_rejects(b,a,g,s):
  {==}

def L.unknown_preserved(ok):
  match ok:
    case True{}: {==}
    case False{}: {==}

def L.mandatory_preserved():
  {==}

def L.semantic_not_runtime():
  {==}

def L.semantic_not_formal():
  {==}

def L.semantic_never_verified(life,binding):
  L.unknown_preserved(C.permitted(life) && C.binding_ok(binding) && True{})

def L.cancellation_terminal(event):
  {==}

def L.quota_empty(cost):
  {==}

def L.stale_rejects(life,a,g,s):
  match life:
    case C.Planned{}: {==}
    case C.Running{}: {==}
    case C.Completed{}: {==}
    case C.Cancelled{}: {==}
    case C.Superseded{}: {==}

def L.membership_branch_ignores_content(include,path,digest_a,digest_b,text_a,text_b):
  match include:
    case True{}: {==}
    case False{}: {==}

def L.membership_projection_ignores_content(path,digest_a,digest_b,text_a,text_b,scope):
  Equal.cong(String,String,x => x ++ "",E.fingerprint_if(E.selected(path,scope),["F",path,digest_a,text_a],"membership"),E.fingerprint_if(E.selected(path,scope),["F",path,digest_b,text_b],"membership"),L.membership_branch_ignores_content(E.selected(path,scope),path,digest_a,digest_b,text_a,text_b))

def L.producer_operation_independent(native,rows,scope,operation):
  match native:
    case True{}: {==}
    case False{}: {==}

def L.operation_not_computation_key(requirement,rows,operation):
  Equal.cong(String,String,p => W.row([Identity.rules(),W.field(requirement,2n),W.field(requirement,5n),W.field(requirement,6n),W.field(requirement,7n),W.field(requirement,8n)]) ++ E.exclusion_key(rows) ++ E.fingerprint(rows,W.field(requirement,5n),W.field(requirement,6n)) ++ p,E.producer_key(String.eq(W.field(requirement,7n),"native"),["OP",operation,"running","revision","timely"] <> rows,W.field(requirement,5n)),E.producer_key(String.eq(W.field(requirement,7n),"native"),rows,W.field(requirement,5n)),L.producer_operation_independent(String.eq(W.field(requirement,7n),"native"),rows,W.field(requirement,5n),operation))

def L.trusted_parse_reuse(blob,grammar,source,witness):
  witness

def L.trusted_span_parse_reuse(blob,grammar,source,witness):
  witness

# Explicit bit cases avoid relying on negative literal-pattern refinement.
def L.wire_tail(head,tail,witness):
  %witness : {W.prepend(head,W.decode(W.encode(tail))) == W.prepend(head,_) : Maybe<&2,String>}
  {==}

def pack_char(b0: Bool,b1: Bool,b2: Bool,b3: Bool,b4: Bool,b5: Bool,b6: Bool,b7: Bool,b8: Bool,b9: Bool,b10: Bool,b11: Bool,b12: Bool,b13: Bool,b14: Bool,b15: Bool,b16: Bool,b17: Bool,b18: Bool,b19: Bool,b20: Bool,b21: Bool,b22: Bool,b23: Bool,b24: Bool,b25: Bool,b26: Bool,b27: Bool,b28: Bool,b29: Bool,b30: Bool,b31: Bool) -> Char:
  Chr{U32{WCon{b0,WCon{b1,WCon{b2,WCon{b3,WCon{b4,WCon{b5,WCon{b6,WCon{b7,WCon{b8,WCon{b9,WCon{b10,WCon{b11,WCon{b12,WCon{b13,WCon{b14,WCon{b15,WCon{b16,WCon{b17,WCon{b18,WCon{b19,WCon{b20,WCon{b21,WCon{b22,WCon{b23,WCon{b24,WCon{b25,WCon{b26,WCon{b27,WCon{b28,WCon{b29,WCon{b30,WCon{b31,WNil{}}}}}}}}}}}}}}}}}}}}}}}}}}}}}}}}}}}

def wire_bits(+b0: Bool,+b1: Bool,+b2: Bool,+b3: Bool,+b4: Bool,+b5: Bool,+b6: Bool,+b7: Bool,+b8: Bool,+b9: Bool,+b10: Bool,+b11: Bool,+b12: Bool,+b13: Bool,+b14: Bool,+b15: Bool,+b16: Bool,+b17: Bool,+b18: Bool,+b19: Bool,+b20: Bool,+b21: Bool,+b22: Bool,+b23: Bool,+b24: Bool,+b25: Bool,+b26: Bool,+b27: Bool,+b28: Bool,+b29: Bool,+b30: Bool,+b31: Bool,+tail: String,witness: {W.decode(W.encode(tail)) == Some{tail} : Maybe<&2,String>}) -> {W.decode(W.encode(SCon{pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail})) == Some{SCon{pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail}} : Maybe<&2,String>}:
  match b0:
    case False{}:
      match b1:
        case False{}:
          L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
        case True{}:
          match b2:
            case False{}:
              match b3:
                case False{}:
                  L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                case True{}:
                  match b4:
                    case False{}:
                      match b5:
                        case False{}:
                          match b6:
                            case False{}:
                              match b7:
                                case False{}:
                                  match b8:
                                    case False{}:
                                      match b9:
                                        case False{}:
                                          match b10:
                                            case False{}:
                                              match b11:
                                                case False{}:
                                                  match b12:
                                                    case False{}:
                                                      match b13:
                                                        case False{}:
                                                          match b14:
                                                            case False{}:
                                                              match b15:
                                                                case False{}:
                                                                  match b16:
                                                                    case False{}:
                                                                      match b17:
                                                                        case False{}:
                                                                          match b18:
                                                                            case False{}:
                                                                              match b19:
                                                                                case False{}:
                                                                                  match b20:
                                                                                    case False{}:
                                                                                      match b21:
                                                                                        case False{}:
                                                                                          match b22:
                                                                                            case False{}:
                                                                                              match b23:
                                                                                                case False{}:
                                                                                                  match b24:
                                                                                                    case False{}:
                                                                                                      match b25:
                                                                                                        case False{}:
                                                                                                          match b26:
                                                                                                            case False{}:
                                                                                                              match b27:
                                                                                                                case False{}:
                                                                                                                  match b28:
                                                                                                                    case False{}:
                                                                                                                      match b29:
                                                                                                                        case False{}:
                                                                                                                          match b30:
                                                                                                                            case False{}:
                                                                                                                              match b31:
                                                                                                                                case False{}:
                                                                                                                                  L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                                                                case True{}:
                                                                                                                                  L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                                                            case True{}:
                                                                                                                              L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                                                        case True{}:
                                                                                                                          L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                                                    case True{}:
                                                                                                                      L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                                                case True{}:
                                                                                                                  L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                                            case True{}:
                                                                                                              L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                                        case True{}:
                                                                                                          L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                                    case True{}:
                                                                                                      L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                                case True{}:
                                                                                                  L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                            case True{}:
                                                                                              L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                        case True{}:
                                                                                          L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                    case True{}:
                                                                                      L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                case True{}:
                                                                                  L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                            case True{}:
                                                                              L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                        case True{}:
                                                                          L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                    case True{}:
                                                                      L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                case True{}:
                                                                  L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                            case True{}:
                                                              L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                        case True{}:
                                                          L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                    case True{}:
                                                      L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                case True{}:
                                                  L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                            case True{}:
                                              L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                        case True{}:
                                          L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                    case True{}:
                                      L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                case True{}:
                                  L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                            case True{}:
                              L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                        case True{}:
                          L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                    case True{}:
                      L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
            case True{}:
              L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
    case True{}:
      match b1:
        case False{}:
          match b2:
            case False{}:
              match b3:
                case False{}:
                  L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                case True{}:
                  match b4:
                    case False{}:
                      match b5:
                        case False{}:
                          match b6:
                            case False{}:
                              match b7:
                                case False{}:
                                  match b8:
                                    case False{}:
                                      match b9:
                                        case False{}:
                                          match b10:
                                            case False{}:
                                              match b11:
                                                case False{}:
                                                  match b12:
                                                    case False{}:
                                                      match b13:
                                                        case False{}:
                                                          match b14:
                                                            case False{}:
                                                              match b15:
                                                                case False{}:
                                                                  match b16:
                                                                    case False{}:
                                                                      match b17:
                                                                        case False{}:
                                                                          match b18:
                                                                            case False{}:
                                                                              match b19:
                                                                                case False{}:
                                                                                  match b20:
                                                                                    case False{}:
                                                                                      match b21:
                                                                                        case False{}:
                                                                                          match b22:
                                                                                            case False{}:
                                                                                              match b23:
                                                                                                case False{}:
                                                                                                  match b24:
                                                                                                    case False{}:
                                                                                                      match b25:
                                                                                                        case False{}:
                                                                                                          match b26:
                                                                                                            case False{}:
                                                                                                              match b27:
                                                                                                                case False{}:
                                                                                                                  match b28:
                                                                                                                    case False{}:
                                                                                                                      match b29:
                                                                                                                        case False{}:
                                                                                                                          match b30:
                                                                                                                            case False{}:
                                                                                                                              match b31:
                                                                                                                                case False{}:
                                                                                                                                  L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                                                                case True{}:
                                                                                                                                  L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                                                            case True{}:
                                                                                                                              L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                                                        case True{}:
                                                                                                                          L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                                                    case True{}:
                                                                                                                      L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                                                case True{}:
                                                                                                                  L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                                            case True{}:
                                                                                                              L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                                        case True{}:
                                                                                                          L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                                    case True{}:
                                                                                                      L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                                case True{}:
                                                                                                  L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                            case True{}:
                                                                                              L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                        case True{}:
                                                                                          L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                    case True{}:
                                                                                      L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                case True{}:
                                                                                  L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                            case True{}:
                                                                              L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                        case True{}:
                                                                          L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                    case True{}:
                                                                      L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                case True{}:
                                                                  L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                            case True{}:
                                                              L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                        case True{}:
                                                          L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                    case True{}:
                                                      L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                case True{}:
                                                  L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                            case True{}:
                                              L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                        case True{}:
                                          L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                    case True{}:
                                      L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                case True{}:
                                  L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                            case True{}:
                              L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                        case True{}:
                          L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                    case True{}:
                      L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
            case True{}:
              match b3:
                case False{}:
                  match b4:
                    case False{}:
                      match b5:
                        case False{}:
                          L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                        case True{}:
                          match b6:
                            case False{}:
                              match b7:
                                case False{}:
                                  match b8:
                                    case False{}:
                                      match b9:
                                        case False{}:
                                          match b10:
                                            case False{}:
                                              match b11:
                                                case False{}:
                                                  match b12:
                                                    case False{}:
                                                      match b13:
                                                        case False{}:
                                                          match b14:
                                                            case False{}:
                                                              match b15:
                                                                case False{}:
                                                                  match b16:
                                                                    case False{}:
                                                                      match b17:
                                                                        case False{}:
                                                                          match b18:
                                                                            case False{}:
                                                                              match b19:
                                                                                case False{}:
                                                                                  match b20:
                                                                                    case False{}:
                                                                                      match b21:
                                                                                        case False{}:
                                                                                          match b22:
                                                                                            case False{}:
                                                                                              match b23:
                                                                                                case False{}:
                                                                                                  match b24:
                                                                                                    case False{}:
                                                                                                      match b25:
                                                                                                        case False{}:
                                                                                                          match b26:
                                                                                                            case False{}:
                                                                                                              match b27:
                                                                                                                case False{}:
                                                                                                                  match b28:
                                                                                                                    case False{}:
                                                                                                                      match b29:
                                                                                                                        case False{}:
                                                                                                                          match b30:
                                                                                                                            case False{}:
                                                                                                                              match b31:
                                                                                                                                case False{}:
                                                                                                                                  L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                                                                case True{}:
                                                                                                                                  L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                                                            case True{}:
                                                                                                                              L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                                                        case True{}:
                                                                                                                          L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                                                    case True{}:
                                                                                                                      L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                                                case True{}:
                                                                                                                  L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                                            case True{}:
                                                                                                              L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                                        case True{}:
                                                                                                          L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                                    case True{}:
                                                                                                      L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                                case True{}:
                                                                                                  L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                            case True{}:
                                                                                              L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                        case True{}:
                                                                                          L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                    case True{}:
                                                                                      L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                case True{}:
                                                                                  L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                            case True{}:
                                                                              L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                        case True{}:
                                                                          L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                    case True{}:
                                                                      L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                case True{}:
                                                                  L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                            case True{}:
                                                              L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                        case True{}:
                                                          L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                    case True{}:
                                                      L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                case True{}:
                                                  L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                            case True{}:
                                              L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                        case True{}:
                                          L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                    case True{}:
                                      L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                case True{}:
                                  L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                            case True{}:
                              L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                    case True{}:
                      L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                case True{}:
                  match b4:
                    case False{}:
                      match b5:
                        case False{}:
                          match b6:
                            case False{}:
                              match b7:
                                case False{}:
                                  match b8:
                                    case False{}:
                                      match b9:
                                        case False{}:
                                          match b10:
                                            case False{}:
                                              match b11:
                                                case False{}:
                                                  match b12:
                                                    case False{}:
                                                      match b13:
                                                        case False{}:
                                                          match b14:
                                                            case False{}:
                                                              match b15:
                                                                case False{}:
                                                                  match b16:
                                                                    case False{}:
                                                                      match b17:
                                                                        case False{}:
                                                                          match b18:
                                                                            case False{}:
                                                                              match b19:
                                                                                case False{}:
                                                                                  match b20:
                                                                                    case False{}:
                                                                                      match b21:
                                                                                        case False{}:
                                                                                          match b22:
                                                                                            case False{}:
                                                                                              match b23:
                                                                                                case False{}:
                                                                                                  match b24:
                                                                                                    case False{}:
                                                                                                      match b25:
                                                                                                        case False{}:
                                                                                                          match b26:
                                                                                                            case False{}:
                                                                                                              match b27:
                                                                                                                case False{}:
                                                                                                                  match b28:
                                                                                                                    case False{}:
                                                                                                                      match b29:
                                                                                                                        case False{}:
                                                                                                                          match b30:
                                                                                                                            case False{}:
                                                                                                                              match b31:
                                                                                                                                case False{}:
                                                                                                                                  L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                                                                case True{}:
                                                                                                                                  L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                                                            case True{}:
                                                                                                                              L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                                                        case True{}:
                                                                                                                          L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                                                    case True{}:
                                                                                                                      L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                                                case True{}:
                                                                                                                  L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                                            case True{}:
                                                                                                              L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                                        case True{}:
                                                                                                          L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                                    case True{}:
                                                                                                      L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                                case True{}:
                                                                                                  L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                            case True{}:
                                                                                              L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                        case True{}:
                                                                                          L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                    case True{}:
                                                                                      L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                                case True{}:
                                                                                  L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                            case True{}:
                                                                              L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                        case True{}:
                                                                          L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                    case True{}:
                                                                      L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                                case True{}:
                                                                  L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                            case True{}:
                                                              L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                        case True{}:
                                                          L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                    case True{}:
                                                      L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                                case True{}:
                                                  L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                            case True{}:
                                              L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                        case True{}:
                                          L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                    case True{}:
                                      L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                                case True{}:
                                  L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                            case True{}:
                              L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                        case True{}:
                          L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
                    case True{}:
                      L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)
        case True{}:
          L.wire_tail(pack_char(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31),tail,witness)

def wire_char(+head: Char,+tail: String,witness: {W.decode(W.encode(tail)) == Some{tail} : Maybe<&2,String>}) -> {W.decode(W.encode(SCon{head,tail})) == Some{SCon{head,tail}} : Maybe<&2,String>}:
  match head:
    case Chr{U32{WCon{+b0,WCon{+b1,WCon{+b2,WCon{+b3,WCon{+b4,WCon{+b5,WCon{+b6,WCon{+b7,WCon{+b8,WCon{+b9,WCon{+b10,WCon{+b11,WCon{+b12,WCon{+b13,WCon{+b14,WCon{+b15,WCon{+b16,WCon{+b17,WCon{+b18,WCon{+b19,WCon{+b20,WCon{+b21,WCon{+b22,WCon{+b23,WCon{+b24,WCon{+b25,WCon{+b26,WCon{+b27,WCon{+b28,WCon{+b29,WCon{+b30,WCon{+b31,WNil{}}}}}}}}}}}}}}}}}}}}}}}}}}}}}}}}}}}: wire_bits(b0,b1,b2,b3,b4,b5,b6,b7,b8,b9,b10,b11,b12,b13,b14,b15,b16,b17,b18,b19,b20,b21,b22,b23,b24,b25,b26,b27,b28,b29,b30,b31,tail,witness)

def L.wire_roundtrip(source):
  match source:
    case SNil{}: {==}
    case SCon{h,+t}: wire_char(h,t,L.wire_roundtrip(t))

def parsed_result_sound(-args: List<&2,String>,+result: ParsedValue.Result<args>) -> {ParsedValue.value(args,result) == ParsedValue.compute(args) : String}:
  match result:
    case ParsedValue.Result{value,proof}: proof

def request_row_agrees(+request: Grouped.Request,+first: Bool,+key: String,+value: String) -> {Grouped.bound_record(Grouped.bind_request(request,value,first,key)) == L.requested_bound_record(request,first,key,value) : List<&2,String>}:
  match request:
    case Grouped.Request{path,digest,compact,source}: {==}

def grouped_row_proof(-canonical: List<&2,String>,+result: ParsedValue.Result<canonical>,+request: Grouped.Request,+same: {canonical == Grouped.request_args(request) : List<&2,String>},+first: Bool,+key: String) -> {Grouped.bound_record(Grouped.bind_request(request,Grouped.alias_value(canonical,result,Grouped.request_args(request),same),first,key)) == L.requested_bound_record(request,first,key,ParsedValue.compute(Grouped.request_args(request))) : List<&2,String>}:
  Equal.trans(List<&2,String>,Grouped.bound_record(Grouped.bind_request(request,Grouped.alias_value(canonical,result,Grouped.request_args(request),same),first,key)),L.requested_bound_record(request,first,key,Grouped.alias_value(canonical,result,Grouped.request_args(request),same)),L.requested_bound_record(request,first,key,ParsedValue.compute(Grouped.request_args(request))),request_row_agrees(request,first,key,Grouped.alias_value(canonical,result,Grouped.request_args(request),same)),Equal.cong(String,List<&2,String>,v => L.requested_bound_record(request,first,key,v),Grouped.alias_value(canonical,result,Grouped.request_args(request),same),ParsedValue.compute(Grouped.request_args(request)),parsed_result_sound(Grouped.request_args(request),ParsedValue.reuse(canonical,Grouped.request_args(request),result,same))))

def L.grouped_parse_row_correct(canonical,result,alias,key):
  match alias:
    case Grouped.Alias{request,same,first}: grouped_row_proof(canonical,result,request,same,first,key)

def L.query_graph_revision_correct(nodes,old):
  QueryGraphProof.run_correct(nodes,old)

def L.query_record_terminator_excluded():
  {==}

def L.query_argument_newline_preserved():
  {==}

def L.reported_native_cannot_be_current(request,observation):
  {==}

def L.fit_requirement_change_has_no_patch(before,after,exists):
  {==}

def L.digest_hex_length(state):
  match state:
    case Sha256.H{a,b,c,d,e,f,g,h}: {==}

def L.digest_wrong_length_refused():
  {==}

def L.digest_non_byte_refused():
  {==}

def L.digest_oversized_length_refused():
  {==}

def retained_chosen_sound(-args: List<&2,String>,chosen: ParseCache.Chosen<args>) -> {ParseCache.chosen_value(args,chosen) == ParsedValue.compute(args) : String}:
  match chosen:
    case ParseCache.Chosen{value,_}: parsed_result_sound(args,value)

def L.retained_parse_choice_sound(args,entry):
  retained_chosen_sound(args,ParseCache.choose(args,entry))

def L.retained_parse_row_sound(canonical,result,alias,key,reused):
  match alias:
    case Grouped.Alias{request,same,first}: grouped_row_proof(canonical,result,request,same,ParseCache.cache_first(reused,first),key)

def L.chunked_evaluation_exact(rule,scope,arg,parts):
  ChunkProof.chunked_exact(rule,scope,arg,parts)

def L.chunk_partial_wire_roundtrip(p,chunk,rule,scope,arg):
  ChunkProof.bits_roundtrip(p,chunk,rule,scope,arg)

def L.chunk_listing_exact(parts):
  ChunkProof.listing_exact(parts)

def L.select_window_sums_split(xs,ys):
  RP.wsum_split(xs,ys)

def L.select_window_sums_commute(a,b):
  RP.wadd_comm(a,b)

def L.select_file_counts_commute(a,b):
  RP.fadd_comm(a,b)

def L.select_file_counts_associate(a,b,c):
  RP.fadd_assoc(a,b,c)

def L.select_path_counts_once(p,d1,b1,d2,b2):
  RP.max_idem(p)

def same_flip(c: Cmp) -> {RP.flip(c) == L.flip(c) : Cmp}:
  match c:
    case LT{}: {==}
    case EQ{}: {==}
    case GT{}: {==}

def L.select_path_order_flips(a,b):
  +a2 = a
  +b2 = b
  Equal.trans(Cmp,Rt.order(b2,a2),RP.flip(Rt.order(a2,b2)),L.flip(Rt.order(a2,b2)),RP.order_flip(a2,b2),same_flip(Rt.order(a2,b2)))

def L.select_path_order_equal(a,b,e):
  RP.order_eq(a,b,e)

def L.select_path_order_transitive(a,b,c,ab,bc):
  RP.order_trans(a,b,c,ab,bc)

def L.select_grouping_swap(a,b,xs):
  RP.group_swap(xs,a,b)

def rows_permute(xs: List<&2,Rt.FileRow>,+a: Rt.FileRow,+b: Rt.FileRow,+ys: List<&2,Rt.FileRow>,acc: List<&2,Rt.FileRow>) -> {Rt.grouped(List.append(&2,Rt.FileRow,xs,a <> b <> ys),acc) == Rt.grouped(List.append(&2,Rt.FileRow,xs,b <> a <> ys),acc) : List<&2,Rt.FileRow>}:
  match xs:
    case Nil{}: Equal.cong(List<&2,Rt.FileRow>,List<&2,Rt.FileRow>,g => Rt.grouped(ys,g),Rt.group(b,Rt.group(a,acc)),Rt.group(a,Rt.group(b,acc)),RP.group_swap(acc,b,a))
    case h <> t: rows_permute(t,a,b,ys,Rt.group(h,acc))

def L.select_rows_permute(xs,a,b,ys,acc):
  rows_permute(xs,a,b,ys,acc)

def scan_concat(a: String,+b: String,st: Lx.Scan,+node: Lx.Trie) -> {Lx.walk(String.append(a,b),st,node) == Lx.walk(b,Lx.walk(a,st,node),node) : Lx.Scan}:
  match a:
    case SNil{}: {==}
    case SCon{Chr{+x},t}: scan_concat(t,b,Lx.step(st,x,node),node)

def L.select_scan_concat(a,b,st,node):
  scan_concat(a,b,st,node)

def ended_carry(hd: Lx.Head,+ts: List<&2,Lx.Part>,n: Nat,hits: List<&2,U32>,defs: List<&2,U32>,+node: Lx.Trie) -> {L.carry(Lx.ended(hd,ts,10,n,hits,defs,node)) == ("",0) : String & U32}:
  match hd:
    case Lx.Head{h,name}: {==}

def L.select_line_resets(st,node):
  match st:
    case Lx.Scan{run,h,n,hits,defs}:
      match run:
        case SNil{}: {==}
        case SCon{+c,+t}: ended_carry(Lx.head_word(h,SCon{c,t},10),Lx.tokens(String.reverse(SCon{c,t})),n,hits,defs,node)
