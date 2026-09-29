import Base

type Record is Data:
  Row{fields: List<&2, String>}
  Invalid{reason: String}

def prepend(c: Char, rest: Maybe<&2, String>) -> Maybe<&2, String>:
  match rest:
    case Some{s}: Some{SCon{c,s}}
    case None{}: None{}

def decode(s: String) -> Maybe<&2, String>:
  match s:
    case SNil{}: Some{""}
    case SCon{'%', SCon{'0', SCon{'A', t}}}: prepend('\n', decode(t))
    case SCon{'%', SCon{'0', SCon{'D', t}}}: prepend('\r', decode(t))
    case SCon{'%', SCon{'0', SCon{'9', t}}}: prepend('\t', decode(t))
    case SCon{'%', SCon{'2', SCon{'5', t}}}: prepend('%', decode(t))
    case SCon{'%', _}: None{}
    case SCon{h,t}: prepend(h, decode(t))

def encode(s: String) -> String:
  match s:
    case SNil{}: ""
    case SCon{'%',t}: "%25" ++ encode(t)
    case SCon{'\n',t}: "%0A" ++ encode(t)
    case SCon{'\r',t}: "%0D" ++ encode(t)
    case SCon{'\t',t}: "%09" ++ encode(t)
    case SCon{h,t}: SCon{h,encode(t)}

# One segment of a line: its text and the remainder after a separator, if any.
type Cut is Data:
  Cut{text: String, rest: Maybe<&2, String>}

type FieldCut is Data:
  Field{text: String, rest: Maybe<&2, String>}
  Malformed{}

def line_cut(s: String, cur: String) -> Cut:
  match s:
    case SNil{}: Cut{String.reverse(cur), None{}}
    case SCon{'\n', t}: Cut{String.reverse(cur), Some{t}}
    case SCon{h, t}: line_cut(t, SCon{h, cur})

def lines_fuel(fuel: Nat, cut: Cut, acc: List<&2, String>) -> List<&2, String>:
  match fuel cut:
    case _ Cut{text, None{}}: List.reverse(&2, String, text <> acc)
    case 1n+p Cut{text, Some{rest}}: lines_fuel(p, line_cut(rest, ""), text <> acc)
    case 0n Cut{text, Some{_}}: List.reverse(&2, String, text <> acc)

# Same segments as String.split(s, '\n'), built without per-character frames.
def lines(+s: String) -> List<&2, String>:
  lines_fuel(String.length(s), line_cut(s, ""), Nil{})

# Splits one tab-separated field and undoes its percent escapes in the same pass.
def field_cut(s: String, cur: String) -> FieldCut:
  match s:
    case SNil{}: Field{String.reverse(cur), None{}}
    case SCon{'\t', t}: Field{String.reverse(cur), Some{t}}
    case SCon{'%', SCon{'0', SCon{'A', t}}}: field_cut(t, SCon{'\n', cur})
    case SCon{'%', SCon{'0', SCon{'D', t}}}: field_cut(t, SCon{'\r', cur})
    case SCon{'%', SCon{'0', SCon{'9', t}}}: field_cut(t, SCon{'\t', cur})
    case SCon{'%', SCon{'2', SCon{'5', t}}}: field_cut(t, SCon{'%', cur})
    case SCon{'%', _}: Malformed{}
    case SCon{h, t}: field_cut(t, SCon{h, cur})

def fields_fuel(fuel: Nat, cut: FieldCut, acc: List<&2, String>) -> Record:
  match fuel cut:
    case _ Malformed{}: Invalid{"malformed percent escape"}
    case _ Field{text, None{}}: Row{List.reverse(&2, String, text <> acc)}
    case 1n+p Field{text, Some{rest}}: fields_fuel(p, field_cut(rest, ""), text <> acc)
    case 0n Field{text, Some{_}}: Row{List.reverse(&2, String, text <> acc)}

# Equal to splitting on tab and applying `decode` to each field: the same fields,
# and Invalid when any field holds a malformed escape.
def parse(+line: String) -> Record:
  fields_fuel(String.length(line), field_cut(line, ""), Nil{})

def field(xs: List<&2, String>, n: Nat) -> String:
  match xs n:
    case Nil{} _: ""
    case h <> t 0n: h
    case h <> t 1n+p: field(t,p)

def length(xs: List<&2, String>) -> Nat:
  match xs:
    case Nil{}: 0n
    case h <> t: 1n+length(t)

def special(+c: Char) -> Bool:
  Char.is_eq(c,'%') || Char.is_eq(c,'\n') || Char.is_eq(c,'\r') || Char.is_eq(c,'\t')

def plain(s: String, found: Bool) -> Bool:
  match s:
    case SNil{}: Bool.not(found)
    case SCon{+h,t}: plain(t, found || special(h))

def encode_if(clean: Bool, +s: String) -> String:
  match clean:
    case True{}: s
    case False{}: encode(s)

def encode_field(+s: String) -> String:
  encode_if(plain(s,False{}), s)

def cells_onto(xs: List<&2, String>, rest: String) -> String:
  match xs:
    case Nil{}: SCon{'\n', rest}
    case h <> t: SCon{'\t', String.append(encode_field(h), cells_onto(t, rest))}

# Writes one encoded row directly in front of rest: row(xs) ++ rest.
def row_onto(xs: List<&2, String>, rest: String) -> String:
  match xs:
    case Nil{}: SCon{'\n', rest}
    case h <> t: String.append(encode_field(h), cells_onto(t, rest))

def row(xs: List<&2, String>) -> String:
  row_onto(xs, "")
