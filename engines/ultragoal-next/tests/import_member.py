"""Prospective Python member-import rule against real Bend facts and CPython syntax."""
import ast, hashlib, json, os, pathlib, subprocess, tempfile, unittest
from journey import BIN, core, obligation, row

TARGET = 'xml.sax.saxutils|XMLGenerator|XMLGenerator'

def check(source, argument=TARGET, rule='fact-import-member'):
    contract = 'UG\t1\n' + obligation(id='member', rule=rule, scope='a.py', argument=argument)
    frame = contract + row('OP', 'op', 'running', 'contract', 'timely')
    frame += row('F', 'a.py', hashlib.sha256(source.encode()).hexdigest(), source)
    if script := os.environ.get('UG_IMPORT_JS'):
        p = subprocess.run(['bun', script], input=frame, capture_output=True, text=True, timeout=30)
        assert p.returncode == 0, p.stderr
        rows = [line.split('\t') for line in p.stdout.splitlines()]
    else:
        rows = core(frame)
    return next(r[2] for r in rows if r[0] == 'RESULT')

class ImportedMember(unittest.TestCase):
    def test_cli_admits_extra_import_without_erasing_requirement(self):
        with tempfile.TemporaryDirectory() as d:
            root=pathlib.Path(d);repo=root/'repo';repo.mkdir();source=repo/'a.py'
            source.write_text('from xml.sax.saxutils import XMLGenerator, quoteattr\nclass SimplerXMLGenerator(XMLGenerator): pass\n')
            contract=root/'contract.tsv';contract.write_text('UG\t1\n'+obligation(id='xml-generator-base',rule='fact-import-member',scope='a.py',argument=TARGET))
            p=subprocess.run([str(BIN),'check','--root',str(repo),'--contract',str(contract),'--no-cache','--local'],capture_output=True,text=True,timeout=30)
            self.assertEqual(p.returncode,0,p.stdout+p.stderr)
            result=json.loads(p.stdout)
            self.assertEqual(result['obligations'][0]['state'],'verified')
            self.assertEqual(result['coverage']['unresolved'],0)
    def test_extra_reordered_parenthesized_members_preserve_meaning(self):
        for text in (
            'from xml.sax.saxutils import XMLGenerator\n',
            'from xml.sax.saxutils import XMLGenerator, quoteattr\n',
            'from xml.sax.saxutils import quoteattr, XMLGenerator\n',
            'from xml.sax.saxutils import (quoteattr, XMLGenerator)\n',
            'from xml.sax.saxutils import (XMLGenerator as XMLGenerator, quoteattr)\n',
        ):
            with self.subTest(text=text):self.assertEqual(check(text),'verified')
        self.assertEqual(check('from xml.sax.saxutils import XMLGenerator, quoteattr\n',rule='fact-import',argument='from xml.sax.saxutils import XMLGenerator'),'failed')

    def test_wrong_module_binding_and_quoted_text_do_not_pass(self):
        for text in (
            'from other import XMLGenerator\n',
            'from xml.sax.saxutils import XMLGenerator as XGen\n',
            '"from xml.sax.saxutils import XMLGenerator"\n',
            '# from xml.sax.saxutils import XMLGenerator\n',
        ):
            with self.subTest(text=text):self.assertEqual(check(text),'failed')
        self.assertEqual(check('from xml.sax.saxutils import XMLGenerator as XGen\n',argument='xml.sax.saxutils|XMLGenerator|XGen'),'verified')

    def test_unsupported_import_tail_stays_unknown(self):
        self.assertEqual(check('from xml.sax.saxutils import *\n'),'unknown')

    def test_real_import_survives_later_raw_regex_without_admitting_quoted_import(self):
        real = 'from xml.sax.saxutils import XMLGenerator\npattern = r"[\\x00-\\x08\\x0B-\\x1F]"\n'
        quoted = 'pattern = r"from xml.sax.saxutils import XMLGenerator"\n'
        self.assertEqual(check(real),'verified')
        self.assertEqual(check(quoted),'failed')
        imports = [n for n in ast.walk(ast.parse(real)) if isinstance(n, ast.ImportFrom)]
        self.assertEqual([(n.module,[a.name for a in n.names]) for n in imports],[('xml.sax.saxutils',['XMLGenerator'])])
        self.assertFalse(any(isinstance(n,ast.ImportFrom) for n in ast.walk(ast.parse(quoted))))

if __name__ == '__main__': unittest.main()
