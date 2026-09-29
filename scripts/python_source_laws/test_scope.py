"""Discriminating interpreter-token controls for the embedded-program law."""

import unittest

from scripts.python_source_laws.scope import embedded_python


class EmbeddedPythonScope(unittest.TestCase):
    def test_stdin_programs_use_exact_interpreter_tokens(self) -> None:
        for command in (
            "python - <<END",
            "python3 - <<END",
            "python3.12 - <<END",
            "/usr/bin/python3 - <<END",
            "/opt/bin/python3.12 - | cat",
            "'./python3' - <input.py",
            "python3 -m unittest; python3 - <<END",
            "python3 -m unittest && /usr/bin/python3 - <<END",
            "if python3 - <<END",
            "if true; then /usr/bin/python3 - <<END",
            "/usr/bin/env python3 - <<END",
            "if true; then /usr/bin/env python3 - <<END",
        ):
            with self.subTest(command=command):
                self.assertEqual(len(embedded_python("fixture.sh", (command + "\n").encode())), 1)

    def test_normal_invocations_and_unrelated_names_stay_allowed(self) -> None:
        for command in (
            "python3 -m unittest discover",
            "/usr/bin/python3 -c 'print(1)'",
            "python3.12 script.py",
            "my-python3 - <<END",
            "/opt/bin/notpython3 - <<END",
            "echo 'python3 - <<END'",
            "python3 -m unittest; echo '/usr/bin/python3 - <<END'",
            "if true; then python3 -m unittest",
        ):
            with self.subTest(command=command):
                self.assertEqual(embedded_python("fixture.sh", (command + "\n").encode()), [])


if __name__ == "__main__":
    unittest.main()
