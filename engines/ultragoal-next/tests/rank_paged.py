"""Exact global FILE_RANK result across transport pages and failed transitions."""
import os
import pathlib
import subprocess
import unittest
from segment_wire import Session, row

ROOT = pathlib.Path(__file__).resolve().parents[1]
CORE = pathlib.Path(os.environ.get('UG_SEGMENT_CORE', ROOT / 'target/release/ug-core'))

def finish(session, operation, sequence):
    reply = session.ask(row('RANK_END', operation, sequence))
    output = ''
    offset = 0
    while True:
        fields = reply.rstrip('\n').split('\t')
        assert len(fields) == 5 and fields[0] == 'RANK_DATA', fields
        start, next_offset = map(int, fields[1:3])
        chunk = fields[4].replace('%09','\t').replace('%0A','\n').replace('%0D','\r').replace('%25','%')
        assert start == offset and next_offset == start + len(chunk)
        output += chunk
        offset = next_offset
        if fields[3] == 'done': return output
        assert fields[3] == 'more', fields
        reply = session.ask(row('RANK_READ', operation, offset))


class RankPages(unittest.TestCase):
    @unittest.skipUnless(os.environ.get('UG_RANK_OUTPUT') == '1', 'focused output-page boundary')
    def test_global_rank_output_pages_without_dropping_exclusions(self):
        rows = [row('X', f'alpha_{i:05}.py') for i in range(12000)]
        original = 'FILE_RANK\n' + row('QUERY', 'alpha') + row('SHORTLIST', 1) + ''.join(rows)
        expected = subprocess.run([str(CORE)], input=original, text=True, capture_output=True, check=True).stdout
        self.assertGreater(len(expected), 262144)
        session = Session()
        try:
            self.assertEqual(session.ask(row('RANK_BEGIN', 'large-output', 'alpha', 1, len(rows))), 'RANK_ACK\tbegin\n')
            for sequence, start in enumerate(range(0, len(rows), 1000)):
                self.assertEqual(session.ask(row('RANK_PAGE', 'large-output', sequence, ''.join(rows[start:start+1000]))), 'RANK_ACK\tpage\n')
            self.assertEqual(finish(session, 'large-output', 12), expected)
            self.assertIn('RANK_ERROR', session.ask(row('RANK_READ', 'large-output', 0)))
            self.assertEqual(session.ask(row('RANK_BEGIN', 'next', 'alpha', 1, 0)), 'RANK_ACK\tbegin\n')
        finally:
            session.close()

    def test_rare_term_changes_only_global_idf(self):
        query = 'rare common'
        match = subprocess.run([str(CORE)], input='SELECT_MATCH\n' + row('QUERY', query)
            + row('C', 'c0', 'a.py', 'd', 0, 11, 'rare common')
            + row('C', 'c1', 'b.py', 'd', 0, 6, 'common'),
            text=True, capture_output=True, check=True).stdout
        rows = [line + '\n' for line in match.splitlines()]
        expected = subprocess.run([str(CORE)], input='FILE_RANK\n' + row('QUERY', query) + row('SHORTLIST', 1) + match,
            text=True, capture_output=True, check=True).stdout
        session = Session()
        try:
            self.assertEqual(session.ask(row('RANK_BEGIN', 'rare', query, 1, len(rows))), 'RANK_ACK\tbegin\n')
            for sequence, line in enumerate(rows):
                self.assertEqual(session.ask(row('RANK_PAGE', 'rare', sequence, line)), 'RANK_ACK\tpage\n')
            self.assertEqual(finish(session, 'rare', len(rows)), expected)
            self.assertIn('IDF\t0', expected)
            self.assertIn('IDF\t1', expected)
        finally:
            session.close()

    def test_split_rows_preserve_global_idf_and_ties(self):
        rows = [
            row('FILE', 'a.py', 1, '0:0:0:1'),
            row('FILE', 'b.py', 1, '0:0:0:1'),
            row('WINDOWS', 2, 6, '0:2'),
            row('X', 'excluded.py'),
        ]
        original = 'FILE_RANK\n' + row('QUERY', 'alpha') + row('SHORTLIST', 1) + ''.join(rows)
        expected = subprocess.run([str(CORE)], input=original, text=True, capture_output=True, check=True).stdout
        for split in range(1, len(rows)):
            session = Session()
            try:
                self.assertEqual(session.ask(row('RANK_BEGIN', 'op', 'alpha', 1, len(rows))), 'RANK_ACK\tbegin\n')
                self.assertEqual(session.ask(row('RANK_PAGE', 'op', 0, ''.join(rows[:split]))), 'RANK_ACK\tpage\n')
                self.assertEqual(session.ask(row('RANK_PAGE', 'op', 1, ''.join(rows[split:]))), 'RANK_ACK\tpage\n')
                self.assertEqual(finish(session, 'op', 2), expected)
            finally:
                session.close()

    def test_missing_page_refuses_and_later_operation_recovers(self):
        session = Session()
        try:
            self.assertEqual(session.ask(row('RANK_BEGIN', 'old', 'alpha', 1, 1)), 'RANK_ACK\tbegin\n')
            self.assertIn('RANK_ERROR', session.ask(row('RANK_END', 'old', 0)))
            self.assertEqual(session.ask(row('RANK_BEGIN', 'new', 'alpha', 1, 0)), 'RANK_ACK\tbegin\n')
            self.assertIn('RANKED', session.ask(row('RANK_END', 'new', 0)))
        finally:
            session.close()


if __name__ == '__main__': unittest.main()
