"""Logical source pages over the real Bend session wire, including invalid transitions."""
import hashlib, os, pathlib, subprocess, unittest
from journey import CORE, obligation, row as wire_row

def row(*fields):
    return wire_row(*map(str, fields))

HERE = pathlib.Path(__file__).resolve().parents[1]
ENGINE = pathlib.Path(os.environ.get('UG_SEGMENT_CORE', CORE))

def command():
    if ENGINE.suffix == '.js':
        return ['bun', str(ENGINE), '--session']
    return [str(ENGINE), '--threads', '4', '--gpu', 'off', '--', '--session']

def single_command():
    if ENGINE.suffix == '.js':
        return ['bun', str(ENGINE)]
    return [str(ENGINE)]

def encoded_lines(value):
    return [x.split('\t') for x in value.splitlines()]

class Session:
    def __init__(self):
        self.process = subprocess.Popen(command(), stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        self.sequence = 0

    def ask(self, body):
        wire = f'S1\t{self.sequence}\tONE\n{body}'.encode()
        try:
            self.process.stdin.write(f'{len(wire)}\n'.encode() + wire)
            self.process.stdin.flush()
        except BrokenPipeError as error:
            raise AssertionError(f'core exited {self.process.poll()}: {self.process.stderr.read(600).decode()}') from error
        head = self.process.stdout.readline().decode().strip().split('\t')
        assert len(head) == 4 and head[:2] == ['S1', str(self.sequence)], (head, self.process.poll(), self.process.stderr.read(600).decode())
        self.sequence += 1
        return self.process.stdout.read(int(head[3])).decode()

    def close(self):
        if self.process.poll() is None:
            self.process.stdin.write(b'0\n')
        self.process.stdin.close()
        self.process.stdout.close()
        code = self.process.wait(timeout=30)
        self.process.stderr.close()
        return code

def stream(session, source, parts, operation='op', path='a.py', version='v1'):
    digest = hashlib.sha256(source.encode()).hexdigest()
    assert session.ask(row('SEG_BEGIN', operation, path, version, digest, len(source.encode()))) == 'SEG_ACK\tbegin\n'
    offset = 0
    for number, part in enumerate(parts):
        assert session.ask(row('SEG_PAGE', operation, path, version, number, offset, part)) == 'SEG_ACK\tpage\n'
        offset += len(part.encode())
    assert offset == len(source.encode())
    assert session.ask(row('SEG_END', operation, path, version, digest, len(parts))) == 'SEG_ACK\tcomplete\n'
    return digest

class SegmentWire(unittest.TestCase):
    def test_complete_disclosure_checks_late_secret_and_clears_state(self):
        session = Session()
        try:
            safe = '{"question":"Which test?","candidates":[{"description":"Run a probe"}]}'
            digest = stream(session, safe, [safe[:20], safe[20:]], operation='safe', path='/work/advice.json')
            self.assertEqual(session.ask(row('SEG_DISCLOSURE', 'safe', '/work/advice.json', 'v1', digest)), 'DISCLOSURE\teligible\n')
            unsafe = safe + 'ghp_' + 'a'*36
            digest = stream(session, unsafe, [unsafe[:len(safe)], unsafe[len(safe):]], operation='late', path='/work/advice.json')
            self.assertEqual(session.ask(row('SEG_DISCLOSURE', 'late', '/work/advice.json', 'v1', digest)), 'DISCLOSURE\trefused\n')
            self.assertIn('SEG_ERROR', session.ask(row('SEG_DISCLOSURE', 'late', '/work/advice.json', 'v1', digest)))
        finally:
            session.close()

    def test_all_scalar_split_positions_match_one_shot_check(self):
        source = 'π\r\ntail'
        contract = 'UG\t1\n' + obligation(id='present', scope='a.py', argument='tail') + obligation(id='absent', rule='absent-text', scope='a.py', argument='forbidden')
        prefix = contract + row('OP', 'op', 'running', 'contract', 'timely')
        digest = hashlib.sha256(source.encode()).hexdigest()
        expected = subprocess.run(single_command(), input=prefix + row('F', 'a.py', digest, source), capture_output=True, text=True, timeout=30)
        self.assertEqual(expected.returncode, 0, expected.stderr)
        want = [x for x in encoded_lines(expected.stdout) if x[0] == 'RESULT']
        for cut in range(len(source) + 1):
            session = Session()
            try:
                stream(session, source, [source[:cut], source[cut:]])
                got = session.ask(row('SEG_EVAL', 'op', 'a.py', 'v1', digest, prefix, ''))
                self.assertEqual([x for x in encoded_lines(got) if x[0] == 'RESULT'], want, cut)
            finally:
                session.close()

    def test_order_digest_and_abort_fail_closed(self):
        source = 'abc'; digest = hashlib.sha256(source.encode()).hexdigest(); session = Session()
        try:
            self.assertEqual(session.ask(row('SEG_BEGIN', 'op', 'a.py', 'v1', digest, 3)), 'SEG_ACK\tbegin\n')
            self.assertIn('page-out-of-order-or-stale', session.ask(row('SEG_PAGE', 'op', 'a.py', 'v1', 1, 0, 'a')))
            self.assertIn('invalid-end', session.ask(row('SEG_END', 'op', 'a.py', 'v1', digest, 1)))
            self.assertEqual(session.ask(row('SEG_BEGIN', 'op2', 'a.py', 'v2', digest, 3)), 'SEG_ACK\tbegin\n')
            self.assertEqual(session.ask(row('SEG_PAGE', 'op2', 'a.py', 'v2', 0, 0, source)), 'SEG_ACK\tpage\n')
            self.assertIn('end-incomplete-or-stale', session.ask(row('SEG_END', 'op2', 'a.py', 'v2', '0'*64, 1)))
            self.assertEqual(session.ask(row('SEG_BEGIN', 'op3', 'a.py', 'v3', digest, 3)), 'SEG_ACK\tbegin\n')
            self.assertEqual(session.ask(row('SEG_ABORT')), 'SEG_ACK\taborted\n')
            self.assertIn('invalid-end', session.ask(row('SEG_END', 'op3', 'a.py', 'v3', digest, 0)))
        finally:
            session.close()

    def test_index_facts_match_one_shot(self):
        source = 'def answer(): return 42\n'; digest = hashlib.sha256(source.encode()).hexdigest()
        prefix = 'UG\t1\n' + row('Q', '*') + row('OP', 'op', 'running', 'contract', 'timely')
        expected = subprocess.run(single_command(), input='INDEX\n' + prefix + row('F', 'a.py', digest, source), capture_output=True, text=True, timeout=30)
        self.assertEqual(expected.returncode, 0, expected.stderr)
        session = Session()
        try:
            stream(session, source, [source[:5], source[5:]])
            got = session.ask(row('SEG_INDEX', 'op', 'a.py', 'v1', digest, prefix, ''))
            self.assertEqual(got, expected.stdout)
        finally:
            session.close()

    def test_segment_chunk_partials_match_ordinary_chunk(self):
        source = 'π\r\ntail'; digest = hashlib.sha256(source.encode()).hexdigest()
        prefix = 'UG\t1\n' + obligation(id='present', scope='a.py', argument='tail') + obligation(id='forbid', scope='a.py', rule='absent-text', argument='tail') + row('CI', '0')
        expected = subprocess.run(single_command(), input='CHUNK\n' + prefix + row('F', 'a.py', digest, source), capture_output=True, text=True, timeout=30)
        self.assertEqual(expected.returncode, 0, expected.stderr)
        session = Session()
        try:
            stream(session, source, ['π\r', '\ntail'])
            got = session.ask(row('SEG_CHUNK', 'op', 'a.py', 'v1', digest, prefix, ''))
            self.assertEqual(got, expected.stdout)
        finally:
            session.close()

    @unittest.skipUnless(os.environ.get('UG_SEGMENT_LARGE') == '1', 'focused former-boundary fixture')
    def test_large_tail_text_predicates(self):
        source = 'x' * int(os.environ.get('UG_SEGMENT_BYTES', 17 * 1024 * 1024)) + '\ntail-needle\n'
        contract = 'UG\t1\n' + obligation(id='present', scope='a.py', argument='tail-needle') + obligation(id='forbid', scope='a.py', rule='absent-text', argument='tail-needle')
        prefix = contract + row('OP', 'op', 'running', 'contract', 'timely')
        session = Session()
        try:
            parts = [source[i:i + 1024 * 1024] for i in range(0, len(source), 1024 * 1024)]
            digest = stream(session, source, parts)
            got = session.ask(row('SEG_EVAL', 'op', 'a.py', 'v1', digest, prefix, ''))
            self.assertEqual([(r[1], r[2]) for r in encoded_lines(got) if r[0] == 'RESULT'], [('present', 'verified'), ('forbid', 'failed')])
        finally:
            session.close()

if __name__ == '__main__':
    unittest.main()
