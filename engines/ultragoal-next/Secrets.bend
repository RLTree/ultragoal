import Base
import ./Equality.bend as E

# Credential screening adopted from the jev-codex token-saver (src/secrets.mjs):
# names of files that routinely hold credentials, credential-shaped text, and
# credential content recognised by the file that holds it. Each pattern reads a
# maximal character run and then the character that ends it. No class contains
# its terminator, so this accepts exactly what the source regular expression does.

def word(+c: Char) -> Bool:
  Char.is_alpha(c) || Char.is_digit(c) || Char.is_eq(c,'_')

def class_has(kind: U32,+c: Char) -> Bool:
  match kind:
    case 0: Char.is_alpha(c) || Char.is_digit(c)
    case 1: Char.is_upper(c) || Char.is_digit(c)
    case 2: word(c) || Char.is_eq(c,'-')
    case 3: Char.is_alpha(c) || Char.is_digit(c) || Char.is_eq(c,'-')
    case 4: word(c) || Char.is_eq(c,'+') || Char.is_eq(c,'/') || Char.is_eq(c,'.') || Char.is_eq(c,'=') || Char.is_eq(c,'-')
    case 5: word(c)
    case 6: Char.is_alpha(c) || Char.is_eq(c,' ')
    case 7: Char.is_space(c)
    case 8: word(c) || Char.is_eq(c,'.') || Char.is_eq(c,'-')
    case 9: Char.is_eq(c,' ') || Char.is_eq(c,'\t')
    case _: Bool.not(Char.is_eq(c,'}'))

# Class numbers: 0 [A-Za-z0-9], 1 [A-Z0-9], 2 [\w-], 3 [A-Za-z0-9-],
# 4 [A-Za-z0-9_+/.=-], 5 [\w], 6 [A-Za-z ], 7 \s, 8 [\w.-],
# 9 [ \t], 10 [^}].
type Run is Data:
  Run{count: Nat, rest: String}

def next_in(kind: U32,s: String) -> Bool:
  match s:
    case SNil{}: False{}
    case SCon{+h,_}: class_has(kind,h)

def run_go(+s: String,+kind: U32,inside: Bool,n: Nat) -> Run:
  match s inside:
    case SCon{_,+t} True{}: run_go(t,kind,next_in(kind,t),1n+n)
    case _ _: Run{n,s}

def run(+kind: U32,+s: String) -> Run:
  run_go(s,kind,next_in(kind,s),0n)

def count(r: Run) -> Nat:
  match r:
    case Run{n,_}: n

def rest(r: Run) -> String:
  match r:
    case Run{_,s}: s

def starts_word(s: String) -> Bool:
  match s:
    case SNil{}: False{}
    case SCon{+h,_}: word(h)

# A run of word characters followed by a non-word character ends on \b.
def exact_word(+r: Run,+n: Nat) -> Bool:
  Nat.is_eq(count(r),n) && Bool.not(starts_word(rest(r)))

def long_word(+r: Run,+n: Nat) -> Bool:
  Nat.is_ge(count(r),n) && Bool.not(starts_word(rest(r)))

def at_least(+r: Run,+n: Nat) -> Bool:
  Nat.is_ge(count(r),n)

def seg_dot(+r: Run) -> Bool:
  Nat.is_ge(count(r),8n) && String.starts_with(rest(r),".")

def after_dot(+r: Run) -> String:
  String.drop(rest(r),1n)

# `value && rest(Unit{})`, evaluating `rest` only when `value` holds.
def and_then(value: Bool,rest: Unit -> Bool) -> Bool:
  match value:
    case True{}: rest(Unit{})
    case False{}: False{}

# Case-insensitive prefix test against a lowercase literal, without copying the text.
def starts_ci(o: String,p: String) -> Bool:
  match o p:
    case _ SNil{}: True{}
    case SCon{+h,ot} SCon{+c,pt}: Char.is_eq(Char.to_lower(h),c) && starts_ci(ot,pt)
    case SNil{} _: False{}

# Every pattern tail runs only after its prefix matched (Bend evaluates both
# operands of && and ||), so ordinary text costs one dispatch per character.

# -----BEGIN [A-Z ]*PRIVATE KEY( BLOCK)?----- (any case): the letter/space run
# after "-----begin " must end in the key name and be followed by "-----".
def pem_run(+r: Run,+after: String) -> Bool:
  (String.ends_with(String.to_lower(String.take(after,count(r))),"private key") || String.ends_with(String.to_lower(String.take(after,count(r))),"private key block")) && String.starts_with(rest(r),"-----")

def pem_gate(ok: Bool,+o: String) -> Bool:
  match ok:
    case True{}: pem_run(run(6,String.drop(o,11n)),String.drop(o,11n))
    case False{}: False{}

def pem(+o: String) -> Bool:
  pem_gate(starts_ci(o,"-----begin "),o)

# \b(AKIA (any case)|ASIA)[A-Z0-9]{16}\b
def akia_gate(ok: Bool,+o: String) -> Bool:
  match ok:
    case True{}: exact_word(run(0,String.drop(o,4n)),16n)
    case False{}: False{}

def asia_gate(ok: Bool,+o: String) -> Bool:
  match ok:
    case True{}: exact_word(run(1,String.drop(o,4n)),16n)
    case False{}: False{}

def aws(+o: String) -> Bool:
  akia_gate(starts_ci(o,"akia"),o) || asia_gate(String.starts_with(o,"ASIA"),o)

# \bgh[pousr]_[A-Za-z0-9]{36,}\b and \bgithub_pat_[A-Za-z0-9_]{22,}
def gh_gate(ok: Bool,+o: String) -> Bool:
  match ok:
    case True{}: long_word(run(0,String.drop(o,4n)),36n)
    case False{}: False{}

def pat_gate(ok: Bool,+o: String) -> Bool:
  match ok:
    case True{}: at_least(run(5,String.drop(o,11n)),22n)
    case False{}: False{}

def gh_kind(s: String) -> Bool:
  match s:
    case SCon{+k,SCon{'_',_}}: Char.is_eq(k,'p') || Char.is_eq(k,'o') || Char.is_eq(k,'u') || Char.is_eq(k,'s') || Char.is_eq(k,'r')
    case _: False{}

def github(+o: String) -> Bool:
  gh_gate(String.starts_with(o,"gh") && gh_kind(String.drop(o,2n)),o) || pat_gate(String.starts_with(o,"github_pat_"),o)

# \bxox[abeoprs]-[A-Za-z0-9-]{10,}
def xox_letter(+k: Char) -> Bool:
  Char.is_eq(k,'a') || Char.is_eq(k,'b') || Char.is_eq(k,'e') || Char.is_eq(k,'o') || Char.is_eq(k,'p') || Char.is_eq(k,'r') || Char.is_eq(k,'s')

def xox_kind(s: String) -> Bool:
  match s:
    case SCon{+k,SCon{'-',t}}: xox_letter(k) && at_least(run(3,t),10n)
    case _: False{}

def slack_gate(ok: Bool,+o: String) -> Bool:
  match ok:
    case True{}: xox_kind(String.drop(o,3n))
    case False{}: False{}

def slack(+o: String) -> Bool:
  slack_gate(String.starts_with(o,"xox"),o)

# \b[rs]k_live_[A-Za-z0-9]{16,}
def live_gate(ok: Bool,+o: String) -> Bool:
  match ok:
    case True{}: at_least(run(0,String.drop(o,8n)),16n)
    case False{}: False{}

def live(+o: String) -> Bool:
  live_gate(String.starts_with(o,"rk_live_") || String.starts_with(o,"sk_live_"),o)

# \bsk-[A-Za-z0-9_-]{16,}\b (any case): some prefix of at least 16 run characters
# must end on a word boundary, so each position from the 16th is tried.
def boundary_walk(t: String,left: Nat,+prevw: Bool,+next: String) -> Bool:
  match t left:
    case SNil{} 0n: Bool.xor(prevw,starts_word(next))
    case SNil{} _: False{}
    case SCon{+h,u} 0n: Bool.xor(prevw,word(h)) || boundary_walk(u,0n,word(h),next)
    case SCon{+h,u} 1n+p: boundary_walk(u,p,word(h),next)

def openai_run(+r: Run,+after: String) -> Bool:
  Nat.is_ge(count(r),16n) && boundary_walk(String.take(after,count(r)),16n,False{},rest(r))

def openai_gate(ok: Bool,+o: String) -> Bool:
  match ok:
    case True{}: openai_run(run(2,String.drop(o,3n)),String.drop(o,3n))
    case False{}: False{}

def openai(+o: String) -> Bool:
  openai_gate(starts_ci(o,"sk-"),o)

# \bBearer\s+eyJ[\w-]{8,}\.[\w-]{8,}\.[\w-]{8,}
def jwt_last(+r: Run) -> Bool:
  seg_dot(r) && at_least(run(2,after_dot(r)),8n)

def jwt_tail(+t: String) -> Bool:
  seg_dot(run(2,t)) && jwt_last(run(2,after_dot(run(2,t))))

def eyj_gate(ok: Bool,+s: String) -> Bool:
  match ok:
    case True{}: jwt_tail(String.drop(s,3n))
    case False{}: False{}

def bearer_ws(+r: Run) -> Bool:
  at_least(r,1n) && eyj_gate(String.starts_with(rest(r),"eyJ"),rest(r))

def bearer_gate(ok: Bool,+o: String) -> Bool:
  match ok:
    case True{}: bearer_ws(run(7,String.drop(o,6n)))
    case False{}: False{}

def bearer(+o: String) -> Bool:
  bearer_gate(String.starts_with(o,"Bearer"),o)

# \b(api[_-]?key|access[_-]?token|password|authorization)\s*[:=]\s*["']?(Bearer\s+)?[A-Za-z0-9_+/.=-]{12,} (any case)
def drop_if(ok: Bool,+s: String) -> String:
  match ok:
    case True{}: String.drop(s,1n)
    case False{}: s

def unquote(+s: String) -> String:
  drop_if(String.starts_with(s,"\"") || String.starts_with(s,"'"),s)

def bearer_value(+r: Run) -> Bool:
  at_least(r,1n) && at_least(run(4,rest(r)),12n)

def value_bearer(ok: Bool,+s: String) -> Bool:
  match ok:
    case True{}: bearer_value(run(7,String.drop(s,6n)))
    case False{}: False{}

def value(+s: String) -> Bool:
  at_least(run(4,s),12n) || value_bearer(starts_ci(s,"bearer"),s)

def separator(s: String) -> Bool:
  match s:
    case SCon{':',t}: value(unquote(rest(run(7,t))))
    case SCon{'=',t}: value(unquote(rest(run(7,t))))
    case _: False{}

def assign_gate(ok: Bool,+o: String,+n: Nat) -> Bool:
  match ok:
    case True{}: separator(rest(run(7,String.drop(o,n))))
    case False{}: False{}

def assign(+o: String,+k: String) -> Bool:
  assign_gate(starts_ci(o,k),o,String.length(k))

def keyword_a(+o: String) -> Bool:
  assign(o,"apikey") || assign(o,"api_key") || assign(o,"api-key") || assign(o,"accesstoken") || assign(o,"access_token") || assign(o,"access-token") || assign(o,"authorization")

# "key"\s*: for the service-account rule, and \beyJ...\.eyJ...\.... for JWTs in configuration.
def colon_gate(ok: Bool,+o: String,+n: Nat) -> Bool:
  match ok:
    case True{}: String.starts_with(rest(run(7,String.drop(o,n))),":")
    case False{}: False{}

def second_eyj_gate(ok: Bool,+s: String) -> Bool:
  match ok:
    case True{}: jwt_last(run(2,String.drop(s,3n)))
    case False{}: False{}

def config_jwt_tail(+r: Run) -> Bool:
  seg_dot(r) && second_eyj_gate(String.starts_with(after_dot(r),"eyJ"),after_dot(r))

def config_jwt_gate(ok: Bool,+o: String) -> Bool:
  match ok:
    case True{}: config_jwt_tail(run(2,String.drop(o,3n)))
    case False{}: False{}

# Finding at one position: 1 a credential-shaped text match, 2 the service-account
# "private_key" key, 3 its "client_email" key, 4 a configuration JWT, 0 none.
def code(ok: Bool,+c: U32) -> U32:
  match ok:
    case True{}: c
    case False{}: 0

# The second character, lowered, selects which patterns can start here, so most
# words stop after two comparisons.
def second(o: String) -> Char:
  match o:
    case SCon{_,SCon{+c,_}}: Char.to_lower(c)
    case _: ' '

def after_a(c: Char,+o: String) -> Bool:
  match c:
    case 'k': akia_gate(starts_ci(o,"akia"),o)
    case 's': asia_gate(String.starts_with(o,"ASIA"),o)
    case 'p': assign(o,"apikey") || assign(o,"api_key") || assign(o,"api-key")
    case 'c': assign(o,"accesstoken") || assign(o,"access_token") || assign(o,"access-token")
    case 'u': assign(o,"authorization")
    case _: False{}

def after_s(c: Char,+o: String) -> Bool:
  match c:
    case 'k': live(o) || openai(o)
    case _: False{}

def after_quote(c: Char,+o: String) -> U32:
  match c:
    case 'p': code(colon_gate(String.starts_with(o,"\"private_key\""),o,13n),2)
    case 'c': code(colon_gate(String.starts_with(o,"\"client_email\""),o,14n),3)
    case _: 0

# Patterns begin only at the characters below (both cases where the pattern
# ignores case); \b before a pattern means the previous character is not a word
# character.
def code_at(h: Char,prevw: Bool,+o: String) -> U32:
  match h prevw:
    case '-' _: code(and_then(Char.is_eq(second(o),'-'),u => pem(o)),1)
    case '"' _: after_quote(second(o),o)
    case 'a' False{}: code(after_a(second(o),o),1)
    case 'A' False{}: code(after_a(second(o),o),1)
    case 'p' False{}: code(and_then(Char.is_eq(second(o),'a'),u => assign(o,"password")),1)
    case 'P' False{}: code(and_then(Char.is_eq(second(o),'a'),u => assign(o,"password")),1)
    case 'g' False{}: code(and_then(Char.is_eq(second(o),'h') || Char.is_eq(second(o),'i'),u => github(o)),1)
    case 'x' False{}: code(and_then(Char.is_eq(second(o),'o'),u => slack(o)),1)
    case 'r' False{}: code(and_then(Char.is_eq(second(o),'k'),u => live(o)),1)
    case 's' False{}: code(after_s(second(o),o),1)
    case 'S' False{}: code(and_then(Char.is_eq(second(o),'k'),u => openai(o)),1)
    case 'B' False{}: code(and_then(Char.is_eq(second(o),'e'),u => bearer(o)),1)
    case 'e' False{}: code(and_then(Char.is_eq(second(o),'y'),u => config_jwt_gate(String.starts_with(o,"eyJ"),o)),4)
    case _ _: 0

type Hits is Data:
  Hits{secret: Bool, private_key: Bool, client_email: Bool, jwt: Bool}

# One pass; stops at the first credential-shaped match.
def scan(+o: String,prevw: Bool,secret: Bool,+pk: Bool,+ce: Bool,+jwt: Bool) -> Hits:
  match o secret:
    case _ True{}: Hits{True{},pk,ce,jwt}
    case SCon{+h,t} False{}:
      +c = code_at(h,prevw,o)
      scan(t,word(h),U32.is_eq(c,1),pk || U32.is_eq(c,2),ce || U32.is_eq(c,3),jwt || U32.is_eq(c,4))
    case SNil{} False{}: Hits{False{},pk,ce,jwt}

def found(h: Hits) -> Bool:
  match h:
    case Hits{s,_,_,_}: s

def secret_text(+text: String) -> Bool:
  found(scan(text,False{},False{},False{},False{},False{}))

# ---- file names

def SECRET_FILES() -> List<&2,String>:
  ["credentials.db","id_rsa","id_ed25519","id_ecdsa","id_dsa","known_hosts",".netrc",".npmrc",".pypirc",".pgpass",".git-credentials",".htpasswd",".dockercfg","kubeconfig",".boto",".s3cfg"]

def NAME_STEMS() -> List<&2,String>:
  ["auth","credential","credentials","secret","secrets","password","passwords","token","tokens"]

def NAME_EXTENSIONS() -> List<&2,String>:
  ["json","toml","yaml","yml","txt","ini","cfg"]

def KEY_EXTENSIONS() -> List<&2,String>:
  [".key",".pem",".p12",".pfx",".jks",".sqlite",".db"]

def CONFIG_EXTENSIONS() -> List<&2,String>:
  [".yaml",".yml",".json",".toml",".ini",".cfg",".conf",".properties",".xml",".plist",".tfvars"]

def ENV_TEMPLATES() -> List<&2,String>:
  [".env.example",".env.sample",".env.template",".env.dist"]

def ends_any(+s: String,xs: List<&2,String>) -> Bool:
  match xs:
    case Nil{}: False{}
    case +h <> t: String.ends_with(s,h) || ends_any(s,t)

def last(xs: List<&2,String>) -> String:
  match xs:
    case Nil{}: ""
    case +h <> Nil{}: h
    case _ <> t: last(t)

def stem_ext(parts: List<&2,String>) -> Bool:
  match parts:
    case +s <> +e <> Nil{}: E.text_member(NAME_STEMS(),s) && E.text_member(NAME_EXTENSIONS(),e)
    case _: False{}

# secret.*\.(ya?ml|json)$: "secret" appears before the final extension.
def secret_before(+b: String,+ext: String) -> Bool:
  String.ends_with(b,ext) && String.contains(String.take(b,Nat.sub(String.length(b),String.length(ext))),"secret")

def prefixed_json(+b: String,+p: String) -> Bool:
  String.starts_with(b,p) && String.ends_with(b,".json") && Nat.is_ge(String.length(b),(String.length(p) + 5n : Nat))

def env_file(+b: String) -> Bool:
  String.eq(b,".env") || String.ends_with(b,".env") || (String.starts_with(b,".env.") && Bool.not(E.text_member(ENV_TEMPLATES(),b)))

# `b` is a lowercase base name.
def secret_name(+b: String) -> Bool:
  E.text_member(SECRET_FILES(),b) || env_file(b) || stem_ext(String.split(b,'.')) || String.starts_with(b,"secrets.") || secret_before(b,".yaml") || secret_before(b,".yml") || secret_before(b,".json") || prefixed_json(b,"credentials") || prefixed_json(b,"serviceaccount") || prefixed_json(b,"service-account") || prefixed_json(b,"service_account") || ends_any(b,KEY_EXTENSIONS()) || String.contains(b,".tfstate.") || String.ends_with(b,".tfstate")

def base_name(+path: String) -> String:
  last(String.split(String.to_lower(path),'/'))

# ---- credential content recognised by its file

# Placeholder values in dotfiles: a trailing " # comment" is removed, the value
# trimmed and one leading and one trailing quote stripped.
def comment_at(+s: String,+i: Nat) -> Nat:
  match s:
    case SNil{}: i
    case SCon{+h,t}: Bool.pick(Nat,Char.is_space(h) && String.starts_with(rest(run(7,s)),"#"),i,comment_at(t,1n+i))

def strip_end_quote(+s: String) -> String:
  Bool.pick(String,String.ends_with(s,"\"") || String.ends_with(s,"'"),String.take(s,Nat.sub(String.length(s),1n)),s)

def same_chars(s: String,+c: Char) -> Bool:
  match s:
    case SNil{}: True{}
    case SCon{+h,t}: Char.is_eq(h,c) && same_chars(t,c)

def all_same(s: String) -> Bool:
  match s:
    case SNil{}: False{}
    case SCon{+h,t}: same_chars(t,h)

def PLACEHOLDER_WORDS() -> List<&2,String>:
  ["change","example","placeholder","your","dummy","redacted","replace","sample","todo","fixme","xxx"]

def contains_any(+s: String,xs: List<&2,String>) -> Bool:
  match xs:
    case Nil{}: False{}
    case +h <> t: String.contains(s,h) || contains_any(s,t)

def placeholder_value(+v: String) -> Bool:
  String.is_empty(v) || String.starts_with(v,"$") || (String.starts_with(v,"<") && String.contains(v,">")) || (String.starts_with(v,"{{") && String.starts_with(rest(run(10,String.drop(v,2n))),"}}")) || String.starts_with(v,"var.") || String.starts_with(v,"local.") || String.starts_with(v,"data.") || all_same(v) || contains_any(String.to_lower(v),PLACEHOLDER_WORDS())

def placeholder(+raw: String) -> Bool:
  placeholder_value(strip_end_quote(unquote(String.trim(String.take(raw,comment_at(raw,0n))))))

# Strict files (.tfvars, .envrc) accept only empty values or references;
# ^["']?\s*["']?\s*$ is an optional quote, spaces, an optional quote, spaces.
def empty_value(+v: String) -> Bool:
  String.is_empty(String.trim(unquote(String.trim(unquote(String.trim_start(v))))))

def strict_refused(+v: String) -> Bool:
  Bool.not(empty_value(v)) && Bool.not(String.starts_with(String.trim(v),"var.") || String.starts_with(String.trim(v),"local.") || String.starts_with(String.trim(v),"data.") || String.starts_with(String.trim(v),"${"))

def CREDENTIAL_NAMES() -> List<&2,String>:
  ["password","passwd","secret","token","apikey","api_key","api-key","privatekey","private_key","private-key","accesskey","access_key","access-key","credential"]

def refused_value(strict: Bool,+v: String) -> Bool:
  match strict:
    case True{}: strict_refused(v)
    case False{}: Bool.not(placeholder(v))

# ^\s*(export\s+)?[\w.-]*KEYWORD[\w.-]*\s*[:=][ \t]*(.*)$ on one line; the value
# is taken from the original line at the same offset as in the lowered one.
def assignment_value(+o: String,+s: String,+strict: Bool) -> Bool:
  match s:
    case SCon{':',t}: refused_value(strict,String.drop(o,Nat.sub(String.length(o),String.length(rest(run(9,t))))))
    case SCon{'=',t}: refused_value(strict,String.drop(o,Nat.sub(String.length(o),String.length(rest(run(9,t))))))
    case _: False{}

def named_assignment(+o: String,+r: Run,+after: String,+strict: Bool) -> Bool:
  contains_any(String.take(after,count(r)),CREDENTIAL_NAMES()) && assignment_value(o,rest(run(7,rest(r))),strict)

def export_skip(+s: String) -> String:
  Bool.pick(String,String.starts_with(s,"export") && at_least(run(7,String.drop(s,6n)),1n),rest(run(7,String.drop(s,6n))),s)

def assignment_line(+o: String,+strict: Bool) -> Bool:
  named_assignment(o,run(8,export_skip(rest(run(7,String.to_lower(o))))),export_skip(rest(run(7,String.to_lower(o)))),strict)

def any_assignment(lines: List<&2,String>,+strict: Bool) -> Bool:
  match lines:
    case Nil{}: False{}
    case +h <> t: assignment_line(h,strict) || any_assignment(t,strict)

def assignment_gate(ok: Bool,+b: String,+text: String) -> Bool:
  match ok:
    case True{}: any_assignment(String.lines(text),String.ends_with(b,".tfvars") || String.eq(b,".envrc"))
    case False{}: False{}

def file_verdict(h: Hits,+b: String,+text: String) -> Bool:
  match h:
    case Hits{s,pk,ce,jwt}: s || (pk && ce) || ((ends_any(b,CONFIG_EXTENSIONS()) || String.starts_with(b,".")) && jwt) || assignment_gate(String.ends_with(b,".tfvars") || String.eq(b,".envrc") || String.starts_with(b,"."),b,text)

def secret_file(+b: String,+text: String) -> Bool:
  file_verdict(scan(text,False{},False{},False{},False{},False{}),b,text)
