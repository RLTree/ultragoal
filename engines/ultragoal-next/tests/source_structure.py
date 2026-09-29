import pathlib, subprocess, unittest, os

CORE=pathlib.Path(os.environ.get('UG_SOURCE_PROBE',pathlib.Path(__file__).resolve().parents[1]/'target/release/source-probe'))
def decode(text):
    for old,new in (('%09','\t'),('%0A','\n'),('%0D','\r'),('%25','%')):
        text=text.replace(old,new)
    return text

def parse(language,text,mode=None):
    data=text.encode()
    args=[str(CORE),'--',language]
    if mode:args.append(mode)
    proc=subprocess.run(args,input=str(len(data)).encode()+b'\n'+data+b'0\n',capture_output=True,timeout=10)
    assert proc.returncode==0,(proc.returncode,proc.stderr)
    return [[decode(value)for value in line.split('\t')]for line in proc.stdout.decode().split('\n')if line]

class Structure(unittest.TestCase):
    def checked(self,language,text):
        rows=parse(language,text);self.assertEqual(rows[0],['STATE','balanced-token-tree'])
        data=text.encode();parents={}
        for kind,value,line,start,end,depth in rows[1:]:
            line,start,end,depth=map(int,(line,start,end,depth))
            self.assertLess(start,end);self.assertLessEqual(end,len(data));self.assertLessEqual(depth,64)
            actual=data[start:end].decode();self.assertEqual(line,data[:start].decode().count('\n')+1)
            if kind=='group':
                self.assertEqual(actual[0],value);self.assertEqual(actual[-1],{'(':')','[':']','{':'}'}[value])
            elif kind!='suite':self.assertEqual(actual,value)
            if depth:
                self.assertIn(depth-1,parents)
                if parents[depth-1][2]=='group':
                    self.assertGreater(start,parents[depth-1][0]);self.assertLess(end,parents[depth-1][1])
                else:
                    self.assertGreaterEqual(start,parents[depth-1][0]);self.assertLessEqual(end,parents[depth-1][1])
            if kind in('group','suite'):parents[depth]=(start,end,kind)
        return rows

    def test_comments_and_literals_do_not_create_structure(self):
        fixtures=[('python','"""fake def bad(): {\n import invented\n}"""\ndef real(x=(1,2)):\n    return x // 2\n'),
                  ('rust','pub fn real(x: i32) { /* fake { /* import invented */ } */ let s = "}"; }'),
                  ('ecmascript','export function real(x) { // fake } import invented\n return "{"; }')]
        for language,text in fixtures:
            with self.subTest(language=language):
                rows=self.checked(language,text);words=[r[1]for r in rows[1:]if r[0]=='word']
                self.assertIn('real',words);self.assertNotIn('fake',words);self.assertNotIn('invented',words)

    def test_python_raw_regex_keeps_exact_spans_and_real_import(self):
        text='from xml.sax.saxutils import XMLGenerator\npattern = r"[\\x00-\\x08\\x0B-\\x1F]"\n'
        rows=self.checked('python',text)
        self.assertTrue(any(r[0]=='literal'and r[1].startswith('r"')for r in rows))
        facts=parse('python',text,'facts')
        self.assertEqual([r[1]for r in facts if r[0]=='structure-python-import-member'],['xml.sax.saxutils|XMLGenerator|XMLGenerator'])
        self.assertFalse(any(r[0]=='structure-limitation'for r in facts),facts)

    def test_native_comment_semantics_differ(self):
        source='/* outer /* nested */ function real() {}'
        self.checked('ecmascript',source)
        self.assertEqual(parse('rust',source)[0][1],'unsupported-or-malformed')

    def test_unicode_spans_crlf_and_continuation(self):
        self.checked('python','# 🌲 λ\r\ndef real(x=\n    "λ🌲"):\r\n    return x\r\n')
        rows=self.checked('python','from \\\n pkg import value\n')
        self.assertEqual([r[1]for r in rows[1:]if r[0]=='word'],['from','pkg','import','value'])
        self.assertEqual(parse('python','def café(): pass\n')[0][1],'unsupported-or-malformed')

    def test_malformed_and_explicit_unsupported(self):
        cases=[('rust','fn bad(x] {}'),('python','x = "unterminated'),('python','x = """unterminated\n'),
               ('ecmascript','const x = `fake ${x}`;'),('ecmascript','const r = /fake/;'),
               ('rust','let x = r#" fake } "#;'),('python','x = f"{dynamic}"'),
               ('python','x = r"""fake import hidden"""'),
               ('python','x = 1\ry = 2'),('rust','/* missing'),('python','x = (1'),('python','x = 1)')]
        for language,text in cases:
            with self.subTest(language=language,text=text):
                rows=parse(language,text);self.assertEqual(rows[0][1],'unsupported-or-malformed');self.assertEqual(len(rows),1)

    def test_depth_and_work_limits(self):
        self.checked('python','('*64+'1'+')'*64)
        self.assertEqual(parse('python','('*65+'1'+')'*65)[0][1],'unsupported-or-malformed')
        rows=parse('python',' '*200001+'pass');self.assertIn('work limit',rows[0][2])

    def test_escaped_quotes_and_brackets(self):
        self.checked('rust','fn f() { let x = "\\\" } ["; let c = \'\\\'\'; }')
        self.checked('python','x = """line1\n\\\" not closed\nline3"""\ny = [1, (2,3)]\n')

    def test_python_suites_and_dedents(self):
        text='def outer(x):\n    def inner(y):\n        return y\n    return inner(x)\nvalue=1\n'
        rows=self.checked('python',text)
        self.assertEqual(sum(r[0]=='suite'for r in rows[1:]),2)
        self.assertEqual(next(r[-1]for r in rows[1:]if r[0]=='word'and r[1]=='value'),'0')
        self.checked('python','if x:\n    # blank body prefix\n\n    y=(\n        1,2\n    )\nelse:\n    y=3\n')
        for text in('if x:\npass\n','if x:\n','if x:\n    if y:\npass\n','if x:\n    y=1\n  z=2\n','    unexpected=1\n','if x:\n\tpass\n'):
            with self.subTest(text=text):
                self.assertEqual(parse('python',text)[0][1],'unsupported-or-malformed')

    def test_suite_depth_limit(self):
        supported=''.join(' '*i+'if x:\n'for i in range(64))+' '*64+'pass\n'
        self.checked('python',supported)
        unsupported=''.join(' '*i+'if x:\n'for i in range(65))+' '*65+'pass\n'
        self.assertEqual(parse('python',unsupported)[0][1],'unsupported-or-malformed')

    def test_python_function_class_and_import_facts(self):
        text='from pkg.sub import (thing, other as alias)\nimport a.b as ab, c\nfrom ..sibling import value\nasync def real(x=(1,2)):\n    return x\nclass C(Base):\n    def method(self): pass\n'
        facts=parse('python',text,'facts')
        self.assertEqual([r[1]for r in facts if r[0]=='structure-function-name'],['real','method'])
        self.assertEqual([r[1]for r in facts if r[0]=='structure-class-name'],['C'])
        self.assertEqual([r[1]for r in facts if r[0]=='structure-python-import-target'],['pkg.sub','a.b','c','..sibling'])
        self.assertFalse(any(r[0]=='structure-limitation'for r in facts),facts)
        spans=parse('python',text,'spans')
        self.assertTrue(all(r[1]==''for r in spans if r[2]!='0'))
        self.assertEqual([r[1]for r in spans if r[0]=='structure-python-import-target'],['pkg.sub','a.b','c','..sibling'])
        self.assertEqual([r[1]for r in facts if r[0]=='structure-python-import-member'],['pkg.sub|thing|thing','pkg.sub|other|alias','..sibling|value|value'])
        for kind,value,line,start,end in facts:
            if kind.endswith('-name'):
                self.assertEqual(text.encode()[int(start):int(end)].decode(),value)

    def test_rust_macros_and_attributes_are_opaque(self):
        text='macro_rules! outer { () => { fn fake() {} } }\n#[custom(fn hidden() {})]\nfn real(x: i32) -> i32 { assert_eq!(x,1); x }\ntrait T { fn signature(&self); }\n'
        facts=parse('rust',text,'facts')
        self.assertEqual([r[1]for r in facts if r[0]=='structure-function-name'],['real'])
        self.assertEqual([r[1]for r in facts if r[0]=='structure-function-signature-name'],['signature'])
        self.assertEqual([r[1]for r in facts if r[0]=='structure-macro-definition'],['outer'])
        self.assertEqual([r[1]for r in facts if r[0]=='structure-macro-invocation'],['assert_eq'])
        self.assertTrue(any(r[0]=='structure-limitation'for r in facts))

    def test_ecmascript_named_headers_and_static_specifiers(self):
        text='import {a, b as c} from "./m.js"; export {x} from "./n.js"; function* run(x) { return x; } class C extends Base { method(x) { return x; } }'
        facts=parse('ecmascript',text,'facts')
        self.assertEqual([r[1]for r in facts if r[0]=='structure-function-name'],['run'])
        self.assertEqual([r[1]for r in facts if r[0]=='structure-class-name'],['C'])
        self.assertEqual([r[1]for r in facts if r[0]=='structure-import-specifier'],['"./m.js"'])
        self.assertEqual([r[1]for r in facts if r[0]=='structure-reexport-specifier'],['"./n.js"'])

    def test_non_identifier_declaration_names(self):
        for language,text in(('python','class 123:\n    pass\n'),('rust','struct 123;'),('ecmascript','class 123 {}')):
            with self.subTest(language=language):
                facts=parse(language,text,'facts')
                self.assertFalse(any(r[0].endswith('-name')for r in facts),facts)
                self.assertTrue(any(r[0]=='structure-limitation'for r in facts),facts)

    def test_body_forms_and_jsx_remain_explicit(self):
        for language,text in(('ecmascript','class Missing;'),('ecmascript','interface Missing;'),('rust','enum Missing;'),('rust','trait Missing;'),('python','import .sibling\n'),('ecmascript','const x = <div>function fake() {{}}</div>;')):
            with self.subTest(language=language,text=text):
                facts=parse(language,text,'facts')
                self.assertTrue(any(r[0]=='structure-limitation'for r in facts),facts)
                self.assertFalse(any(r[0].endswith('-name')for r in facts),facts)

if __name__=='__main__':unittest.main()
