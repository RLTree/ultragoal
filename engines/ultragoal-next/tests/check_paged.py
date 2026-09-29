"""A complete check over physical pages preserves one Bend decision."""
import os
import pathlib
import subprocess
import unittest
from segment_wire import Session, row
from journey import obligation

ROOT = pathlib.Path(__file__).resolve().parents[1]
CORE = pathlib.Path(os.environ.get('UG_SEGMENT_CORE', ROOT / 'target/release/ug-core'))


class CheckPages(unittest.TestCase):
    def test_all_row_splits_match_one_shot(self):
        frame = ('UG\t1\n' + obligation(id='one', scope='a.py', argument='needle')
            + obligation(id='two', scope='b.py', argument='needle') + '\n'
            + row('OP', 'op', 'running', 'contract', 'timely')
            + row('F', 'a.py', 'd1', 'needle') + row('F', 'b.py', 'd2', 'missing'))
        expected = subprocess.run([str(CORE)], input=frame, text=True, capture_output=True, check=True).stdout
        lines = [x + '\n' for x in frame.splitlines()]
        for split in range(1, len(lines)):
            session = Session()
            try:
                self.assertEqual(session.ask(row('CHECK_BEGIN', 'op', sum(bool(x.strip()) for x in lines))), 'CHECK_ACK\tbegin\n')
                self.assertEqual(session.ask(row('CHECK_PAGE', 'op', 0, ''.join(lines[:split]))), 'CHECK_ACK\tpage\n')
                self.assertEqual(session.ask(row('CHECK_PAGE', 'op', 1, ''.join(lines[split:]))), 'CHECK_ACK\tpage\n')
                self.assertEqual(session.ask(row('CHECK_END', 'op', 2)), expected)
            finally:
                session.close()

    def test_missing_page_then_next_use(self):
        session = Session()
        try:
            self.assertEqual(session.ask(row('CHECK_BEGIN', 'old', 1)), 'CHECK_ACK\tbegin\n')
            self.assertIn('RANK_ERROR', session.ask(row('CHECK_END', 'old', 0)))
            self.assertEqual(session.ask(row('CHECK_BEGIN', 'new', 1)), 'CHECK_ACK\tbegin\n')
            self.assertEqual(session.ask(row('CHECK_PAGE', 'new', 0, 'UG\t1\n')), 'CHECK_ACK\tpage\n')
            self.assertIn('CHECK_ERROR', session.ask(row('CHECK_END', 'new', 1)))
        finally:
            session.close()


if __name__ == '__main__': unittest.main()
