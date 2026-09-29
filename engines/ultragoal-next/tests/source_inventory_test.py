"""Source identity admits authored inputs, not caches or source-root artifact aliases."""
import pathlib
import subprocess
import tempfile
import unittest
from source_inventory import GENERATED, hashes


class SourceInventory(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = pathlib.Path(self.tmp.name) / 'engine'
        self.root.mkdir()
        (self.root / 'Rule.bend').write_text('def rule() -> Bool: True{}\n')
        (self.root / 'tests').mkdir()
        (self.root / 'tests' / 'meaning.py').write_text('assert True\n')

    def tearDown(self):
        self.tmp.cleanup()

    def test_authored_changes_bind_and_caches_do_not(self):
        before = hashes(self.root)
        for name in ('target', '.ruff_cache', '__pycache__'):
            directory = self.root / name
            directory.mkdir()
            (directory / 'cache').write_bytes(b'first')
        self.assertEqual(before, hashes(self.root))
        (self.root / '.ruff_cache' / 'cache').write_bytes(b'changed')
        self.assertEqual(before, hashes(self.root))
        (self.root / 'tests' / 'meaning.py').write_text('assert False\n')
        self.assertNotEqual(before, hashes(self.root))
        self.assertIn('tests/meaning.py', hashes(self.root))

    def test_nested_target_directory_and_symlink_are_distinct(self):
        nested = self.root / 'nested'
        nested.mkdir()
        (nested / 'target').mkdir()
        (nested / 'target' / 'generated').write_text('ignored')
        (nested / 'target_file.py').write_text('authored')
        self.assertIn('nested/target_file.py', hashes(self.root))
        self.assertIn('nested/target/generated', hashes(self.root))
        (nested / 'alias.py').symlink_to(self.root / 'Rule.bend')
        with self.assertRaisesRegex(ValueError, 'source symlink'):
            hashes(self.root)

    def test_compiled_nested_target_module_changes_source_identity(self):
        src = self.root / 'src'
        (src / 'target').mkdir(parents=True)
        main = src / 'main.rs'
        main.write_text('#[path = "target/helper.rs"] mod helper; fn main() { println!("{}", helper::answer()); }\n')
        helper = src / 'target/helper.rs'
        output = self.root / 'target/app'
        output.parent.mkdir()
        observed = []
        for value in (1, 2):
            helper.write_text(f'pub fn answer() -> u8 {{ {value} }}\n')
            identity = hashes(self.root)
            subprocess.run(['rustc', str(main), '-o', str(output)], check=True)
            observed.append((identity, subprocess.check_output([str(output)], text=True).strip()))
        self.assertEqual([row[1] for row in observed], ['1', '2'])
        self.assertIn('src/target/helper.rs', observed[0][0])
        self.assertNotEqual(observed[0][0], observed[1][0])
        (self.root / 'target/cache').write_text('new build cache')
        self.assertEqual(hashes(self.root), observed[1][0])

    def test_explicit_module_inside_excluded_build_root_is_rejected(self):
        src = self.root / 'src'
        src.mkdir()
        (src / 'main.rs').write_text('#[path = "../target/helper.rs"] mod helper;\n')
        (self.root / 'target').mkdir()
        (self.root / 'target/helper.rs').write_text('pub fn answer() -> u8 { 1 }\n')
        with self.assertRaisesRegex(ValueError, 'unbound Rust source input'):
            hashes(self.root)

    def test_compiled_source_bearing_syntax_cannot_borrow_excluded_inputs(self):
        cases = {
            'spaced_path': ('#[ path = "../target/helper.rs" ]\nmod helper;\nfn main(){println!("{}",helper::value());}\n', 'target/helper.rs'),
            'raw_path': ('#[path = r"../target/helper.rs"]\nmod helper;\nfn main(){println!("{}",helper::value());}\n', 'target/helper.rs'),
            'comment_path': ('#[path /* source */ = "../target/helper.rs"]\nmod helper;\nfn main(){println!("{}",helper::value());}\n', 'target/helper.rs'),
            'raw_include': ('include!(r"../target/helper.rs");\nfn main(){println!("{}",value());}\n', 'target/helper.rs'),
            'computed_include': ('include!(concat!("../target/", "helper.rs"));\nfn main(){println!("{}",value());}\n', 'target/helper.rs'),
            'pycache_path': ('#[path = "__pycache__/helper.rs"]\nmod helper;\nfn main(){println!("{}",helper::value());}\n', 'src/__pycache__/helper.rs'),
        }
        for name, (program, helper_name) in cases.items():
            with self.subTest(name=name), tempfile.TemporaryDirectory() as directory:
                root = pathlib.Path(directory)
                (root / 'src').mkdir()
                (root / 'target').mkdir()
                main = root / 'src/main.rs'
                main.write_text(program)
                helper = root / helper_name
                helper.parent.mkdir(parents=True, exist_ok=True)
                helper.write_text('pub fn value() -> u32 { 1 }\n')
                with self.assertRaisesRegex(ValueError, 'unbound|unresolved'):
                    hashes(root)
                binary = root / 'target/app'
                subprocess.run(['rustc', str(main), '-o', str(binary)], check=True)
                self.assertEqual(subprocess.check_output([str(binary)], text=True).strip(), '1')

    def test_literal_included_authored_sources_remain_bound(self):
        src = self.root / 'src'
        (src / 'target').mkdir(parents=True)
        main = src / 'main.rs'
        helper = src / 'target/helper.rs'
        helper.write_text('pub fn value() -> u32 { 1 }\n')
        main.write_text('#[ path /* source */ = r"target/helper.rs" ]\nmod helper;\nfn main(){println!("{}",helper::value());}\n')
        first = hashes(self.root)
        self.assertIn('src/target/helper.rs', first)
        helper.write_text('pub fn value() -> u32 { 2 }\n')
        self.assertNotEqual(first, hashes(self.root))
        main.write_text('include!(r"target/helper.rs");\nfn main(){println!("{}",value());}\n')
        second = hashes(self.root)
        helper.write_text('pub fn value() -> u32 { 3 }\n')
        self.assertNotEqual(second, hashes(self.root))
        asset = src / 'asset.txt'
        asset.write_text('one')
        main.write_text('fn main(){println!("{}",include_str!("asset.txt"));}\n')
        third = hashes(self.root)
        asset.write_text('two')
        self.assertNotEqual(third, hashes(self.root))

    def test_nested_attribute_include_is_checked_and_inert_text_is_not(self):
        src = self.root / 'src'
        src.mkdir()
        main = src / 'main.rs'
        (self.root / 'target').mkdir()
        (self.root / 'target/doc.txt').write_text('generated')
        main.write_text('#![doc = include_str!("../target/doc.txt")]\nfn main() {}\n')
        with self.assertRaisesRegex(ValueError, 'unbound Rust source input'):
            hashes(self.root)
        main.write_text('const TEXT: &str = r#"include!(\"../target/doc.txt\")"#; // include!("../target/doc.txt")\nfn main() {}\n')
        self.assertIn('src/main.rs', hashes(self.root))

    def test_character_literals_cannot_conceal_excluded_include(self):
        src = self.root / 'src'
        src.mkdir()
        main = src / 'main.rs'
        (self.root / 'target').mkdir()
        (self.root / 'target/value.txt').write_text('one')
        for literal in ("'\"'", "b'\"'", "'\\\"'", "b'\\\"'"):
            with self.subTest(literal=literal):
                main.write_text(f'fn main() {{ let _quote = {literal}; let value = include_str!("../target/value.txt"); let _other = {literal}; println!("{{value}}"); }}\n')
                with self.assertRaisesRegex(ValueError, 'unbound Rust source input'):
                    hashes(self.root)
                binary = self.root / 'target/app'
                subprocess.run(['rustc', str(main), '-o', str(binary)], check=True)
                self.assertEqual(subprocess.check_output([str(binary)], text=True).strip(), 'one')

    def test_lifetimes_and_escaped_characters_preserve_authored_include(self):
        src = self.root / 'src'
        src.mkdir()
        asset = src / 'value.txt'
        asset.write_text('one')
        main = src / 'main.rs'
        main.write_text("fn borrow<'a>(value: &'a str) -> &'a str { value }\nfn main() { let _a = '\\n'; let _b = b'\\\"'; println!(\"{}\", borrow(include_str!(\"value.txt\"))); }\n")
        first = hashes(self.root)
        self.assertIn('src/value.txt', first)
        asset.write_text('two')
        self.assertNotEqual(first, hashes(self.root))

    def test_raw_c_string_is_inert_but_later_include_is_checked(self):
        src = self.root / 'src'
        src.mkdir()
        main = src / 'main.rs'
        (self.root / 'target').mkdir()
        (self.root / 'target/value.txt').write_text('one')
        prefix = 'fn main(){let _c=cr#"text " include_str!("../target/value.txt")"#;'
        main.write_text(prefix + '}\n')
        self.assertIn('src/main.rs', hashes(self.root))
        main.write_text(prefix + 'let value=include_str!("../target/value.txt");println!("{value}");}\n')
        with self.assertRaisesRegex(ValueError, 'unbound Rust source input'):
            hashes(self.root)
        binary = self.root / 'target/app'
        subprocess.run(['rustc', '--edition=2024', str(main), '-o', str(binary)], check=True)
        self.assertEqual(subprocess.check_output([str(binary)], text=True).strip(), 'one')

    def test_declared_generated_inputs_remain_bound(self):
        for name in GENERATED:
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text('generated one')
        before = hashes(self.root)
        self.assertEqual(set(GENERATED), {'Identity.bend', 'src/native_identity.rs'})
        (self.root / 'Identity.bend').write_text('generated two')
        self.assertNotEqual(before, hashes(self.root))

    def test_relocating_the_root_does_not_change_relative_membership(self):
        before = hashes(self.root)
        moved = self.root.parent / 'relocated-engine'
        self.root.rename(moved)
        self.root = moved
        self.assertEqual(hashes(moved), before)


if __name__ == '__main__':
    unittest.main()
