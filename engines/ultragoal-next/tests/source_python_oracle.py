"""Compare the bounded Python profile to CPython AST without executing inputs."""
import ast,random,unittest
from source_structure import parse

class PythonOracle(unittest.TestCase):
    def test_imported_member_and_local_binding_match_cpython_ast(self):
        for text in (
            'from xml.sax.saxutils import XMLGenerator\n',
            'from xml.sax.saxutils import quoteattr, XMLGenerator\n',
            'from xml.sax.saxutils import (quoteattr, XMLGenerator as XMLGenerator)\n',
            'from xml.sax.saxutils import (XMLGenerator as XGen, quoteattr)\n',
            '"from xml.sax.saxutils import Invented"\n# from xml.sax.saxutils import Hidden\nfrom other import XMLGenerator\n',
        ):
            with self.subTest(text=text):
                native=ast.parse(text)
                expected=[f'{"."*node.level}{node.module or ""}|{alias.name}|{alias.asname or alias.name}'
                          for node in ast.walk(native) if isinstance(node,ast.ImportFrom) for alias in node.names]
                got=[r[1]for r in parse('python',text,'facts')if r[0]=='structure-python-import-member']
                self.assertEqual(got,expected)
    def test_generated_supported_headers_imports_and_name_offsets(self):
        rng=random.Random(20260923)
        for i in range(160):
            n=rng.randrange(1,6)
            imports='import pkg.sub as alias, other\nfrom ..near import (value, item as renamed)\n'
            quote='"""def fabricated():\n    import invented\n}"""\n'
            lines=[imports,quote]
            for j in range(n):
                name=f'function_{i}_{j}'
                prefix='async 'if rng.randrange(2) else''
                lines.append(f'{prefix}def {name}(x=(1, 2), y="🌲"):\n')
                lines.append('    # def nonexistent(): {\n')
                if rng.randrange(2):
                    lines.append(f'    def inner_{j}(value):\n        return value\n')
                lines.append('    return x\n')
            lines.append(f'class Class_{i}(Base):\n    def method(self): pass\n')
            text=''.join(lines)
            if rng.randrange(2):text=text.replace('\n','\r\n')
            tree=ast.parse(text)
            expected_functions=[];expected_classes=[];expected_imports=[]
            class Visit(ast.NodeVisitor):
                def visit_FunctionDef(self,node):expected_functions.append(node);self.generic_visit(node)
                def visit_AsyncFunctionDef(self,node):expected_functions.append(node);self.generic_visit(node)
                def visit_ClassDef(self,node):expected_classes.append(node);self.generic_visit(node)
                def visit_Import(self,node):expected_imports.extend(a.name for a in node.names)
                def visit_ImportFrom(self,node):expected_imports.append('.'*node.level+(node.module or''))
            Visit().visit(tree)
            facts=parse('python',text,'facts')
            with self.subTest(case=i):
                self.assertFalse(any(f[0]=='structure-limitation'for f in facts),facts)
                functions=[f for f in facts if f[0]=='structure-function-name']
                classes=[f for f in facts if f[0]=='structure-class-name']
                self.assertEqual([f[1]for f in functions],[v.name for v in expected_functions])
                self.assertEqual([f[1]for f in classes],[v.name for v in expected_classes])
                self.assertEqual([f[1]for f in facts if f[0]=='structure-python-import-target'],expected_imports)
                for fact,native in zip(functions+classes,expected_functions+expected_classes):
                    self.assertEqual(int(fact[2]),native.lineno)
                    self.assertEqual(text.encode()[int(fact[3]):int(fact[4])].decode(),native.name)

    def test_unsupported_valid_python_does_not_claim_full_coverage(self):
        for text in ('def café(): pass\n','x = f"{1+2}"\n','def typed(x: list[int]) -> str:\n    return str(x)\n','class Generic[T]:\n    pass\n'):
            with self.subTest(text=text):
                try:ast.parse(text)
                except SyntaxError:continue # host version qualification is separate
                facts=parse('python',text,'facts')
                self.assertTrue(any(f[0]=='structure-limitation'for f in facts),facts)

if __name__=='__main__':unittest.main()
