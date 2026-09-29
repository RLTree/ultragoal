"""Compiled stream boundary: framing, ordering, rejection and batch rollover."""
import hashlib, pathlib, subprocess, unittest
from journey import BIN

CORE = pathlib.Path(BIN).with_name('ug-core')

def framed(frames):
    return b''.join(str(len(f)).encode()+b'\n'+f for f in frames)+b'0\n'

def core(data, stream=True, threads=4):
    args = [str(CORE), '--threads', str(threads), '--gpu', 'off']
    if stream:
        args += ['--', '--stream']
    return subprocess.run(args, input=data, capture_output=True, timeout=20)

class StreamBoundary(unittest.TestCase):
    def test_empty(self):
        p=core(b'0\n'); self.assertEqual(p.returncode,0,p.stderr); self.assertEqual(p.stdout,b'')

    def test_order_batch_rollover_and_thread_equivalence(self):
        frames=[b'INDEX_SUMMARY\ncurrent\n', b'INDEX_SUMMARY\nstale\n', b'EXCLUSIONS\n']*6
        expected=b''.join(core(f,False).stdout for f in frames)
        for threads in (1,2,4,8):
            with self.subTest(threads=threads):
                p=core(framed(frames),threads=threads)
                self.assertEqual(p.returncode,0,p.stderr); self.assertEqual(p.stdout,expected)

    def test_malformed(self):
        for wire in (b'',b'\n',b'-1\n',b'a\n',b'16777217\n',b'999999999999\n',
                     b'2\na',b'1\nx',b'0\nextra',b'1\n\xff0\n',b'1\nx0\nextra'):
            with self.subTest(wire=wire):
                self.assertNotEqual(core(wire).returncode,0)

    def test_frame_count_boundary(self):
        frames=[b'INDEX_SUMMARY\ncurrent\n']*128
        p=core(framed(frames)); self.assertEqual(p.returncode,0,p.stderr)
        self.assertEqual(len(p.stdout.splitlines()),128)
        self.assertNotEqual(core(framed(frames+frames[:1])).returncode,0)

    def test_exact_identity_distinguishes_normalization_and_late_changes(self):
        def row(values):
            return '\t'.join(s.replace('%','%25').replace('\t','%09').replace('\n','%0A').replace('\r','%0D') for s in values)+'\n'
        header='INDEX\nUG\t1\nQ\t*\nOP\texact\trunning\tcontract\ttimely\n'
        for before,after in (('é','e\u0301'),('x'*100000+'a','x'*100000+'b'),('λ🌲','λ🌱')):
            with self.subTest(before_length=len(before)):
                wire=(header+row(['F','a.py','same',before])+row(['F','b.py','same',after])).encode()
                result=core(framed([wire]))
                self.assertEqual(result.returncode,0,result.stderr)
                self.assertIn(b'ERROR\tconflicting bytes',result.stdout)
                self.assertNotIn(b'P_SPANS\t',result.stdout)

    def test_interned_exact_rows_and_rejection(self):
        def row(values):
            return '\t'.join(s.replace('%','%25').replace('\t','%09').replace('\n','%0A').replace('\r','%0D') for s in values)+'\n'
        source='def café():\n    return "λ%\\n"\n'
        digest=hashlib.sha256(source.encode()).hexdigest()
        header='UG\t1\nQ\t*\nOP\tstream-test\trunning\tcontract\ttimely\n'
        ordinary=('INDEX\n'+header+row(['F','a.py',digest,source])+row(['F','b.py',digest,source])).encode()
        atoms=row(['D','body0',digest,source])
        refs=row(['F_REF','a.py',digest,'body0'])+row(['F_REF','b.py',digest,'body0'])
        compressed=('INDEX_INTERNED\n'+header+atoms+refs).encode()
        expected=core(ordinary,False); actual=core(framed([compressed]))
        self.assertEqual(expected.returncode,0,expected.stderr)
        self.assertEqual(actual.returncode,0,actual.stderr)
        self.assertEqual(actual.stdout,expected.stdout)
        for body in (refs,atoms+atoms+refs,atoms+refs.replace(digest,'wrong'),
                     row(['D','body0',digest])+refs,atoms+row(['F_REF','a.py',digest]),
                     refs+atoms):
            with self.subTest(body=body):
                p=core(framed([('INDEX_INTERNED\n'+header+body).encode()]))
                self.assertIn(b'ERROR\t',p.stdout)
                self.assertNotIn(b'P_SPANS\t',p.stdout)

if __name__=='__main__': unittest.main()
