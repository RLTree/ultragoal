"""The paged advice wire gives the same global order and rank as one frame."""
import os
import pathlib
import subprocess
import unittest
from segment_wire import Session, row

ROOT = pathlib.Path(__file__).resolve().parents[1]
CORE = pathlib.Path(os.environ.get('UG_SEGMENT_CORE', ROOT / 'target/release/ug-core'))


class AdvicePages(unittest.TestCase):
    def compare(self, mode, initial, rows):
        original = ('ADVICE_ORDER\n' + row(initial) if mode == 'order' else 'ADVICE_RANK\n') + ''.join(rows)
        expected = subprocess.run([str(CORE)], input=original, text=True, capture_output=True, check=True).stdout
        for split in range(1, len(rows)):
            session = Session()
            try:
                self.assertEqual(session.ask(row('ADV_BEGIN', 'op', mode, initial, len(rows))), 'ADV_ACK\tbegin\n')
                self.assertEqual(session.ask(row('ADV_PAGE', 'op', 0, ''.join(rows[:split]))), 'ADV_ACK\tpage\n')
                self.assertEqual(session.ask(row('ADV_PAGE', 'op', 1, ''.join(rows[split:]))), 'ADV_ACK\tpage\n')
                self.assertEqual(session.ask(row('ADV_END', 'op', 2)), expected)
            finally:
                session.close()

    def test_order_and_rank_preserve_global_ties(self):
        self.compare('order', 'cache reload after rename', [row('0', 'pip freeze'), row('1', 'send reload signal'), row('2', 'watch rename events for reload'), row('3', 'check disk')])
        self.compare('rank', '', [row('0', 'null'), row('1', '0.8'), row('2', '0.8'), row('3', '0.1')])

    def test_stale_or_incomplete_page_refuses(self):
        session = Session()
        try:
            self.assertEqual(session.ask(row('ADV_BEGIN', 'op', 'rank', '', 1)), 'ADV_ACK\tbegin\n')
            self.assertIn('RANK_ERROR', session.ask(row('ADV_END', 'op', 0)))
            self.assertEqual(session.ask(row('ADV_BEGIN', 'next', 'rank', '', 1)), 'ADV_ACK\tbegin\n')
            self.assertIn('RANK_ERROR', session.ask(row('ADV_PAGE', 'op', 0, row('0', '1'))))
        finally:
            session.close()


if __name__ == '__main__': unittest.main()
