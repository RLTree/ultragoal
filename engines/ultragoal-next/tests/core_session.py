"""Real core session framing and continuation across an index batch."""
import subprocess,unittest
from journey import CORE

def frame(data):
    return str(len(data)).encode()+b'\n'+data

def one(sequence,data):
    return frame(f'S1\t{sequence}\tONE\n'.encode()+data)

def index(sequence,frames):
    return frame(f'S1\t{sequence}\tINDEX\t{len(frames)}\n'.encode())+b''.join(frame(f)for f in frames)

def run(data):
    return subprocess.run([str(CORE),'--threads','4','--gpu','off','--','--session'],input=data,capture_output=True,timeout=20)

def responses(data):
    out=[];offset=0
    while offset<len(data):
        end=data.index(b'\n',offset);header=data[offset:end].decode().split('\t')
        assert len(header)==4 and header[0]=='S1',header
        size=int(header[3]);offset=end+1;body=data[offset:offset+size];assert len(body)==size
        out.append((int(header[1]),header[2],body));offset+=size
    return out

class CoreSession(unittest.TestCase):
    def test_repeated_commands_and_after_multiple_index_batches(self):
        current=b'INDEX_SUMMARY\ncurrent\n';stale=b'INDEX_SUMMARY\nstale\n'
        data=one(0,current)+index(1,[current,stale]*9)+one(2,current)+b'0\n'
        p=run(data);self.assertEqual(p.returncode,0,p.stderr);actual=responses(p.stdout)
        self.assertEqual([(r[0],r[1])for r in actual],[(0,'done'),(1,'more'),(1,'more'),(1,'done'),(2,'done')])
        self.assertEqual(actual[0][2],actual[-1][2])
        expected=subprocess.run([str(CORE)],input=current,capture_output=True,timeout=10).stdout
        other=subprocess.run([str(CORE)],input=stale,capture_output=True,timeout=10).stdout
        self.assertEqual(b''.join(r[2]for r in actual if r[0]==1),(expected+other)*9)

    def test_sequence_trailing_data_and_partial_index_are_rejected(self):
        payload=b'INDEX_SUMMARY\ncurrent\n'
        invalid=[
            one(1,payload)+b'0\n',
            one(0,payload)+one(0,payload)+b'0\n',
            frame(b'S1\t00\tONE\n'+payload)+b'0\n',
            frame(b'S1\t0\tINDEX\t0\n')+b'0\n',
            frame(b'S1\t0\tINDEX\t129\n')+b'0\n',
            frame(b'S1\t0\tINDEX\t2\n')+frame(payload)+b'0\n',
            frame(b'S1\t0\tINDEX\t1\ntrailing')+frame(payload)+b'0\n',
            one(0,payload)+b'0\ntrailing',
        ]
        for data in invalid:
            with self.subTest(data=data[:50]):self.assertNotEqual(run(data).returncode,0)

    def test_utf8_and_index_count_boundary(self):
        p=run(index(0,[b'INDEX_SUMMARY\ncurrent\n']*128)+one(1,b'EXCLUSIONS\n')+b'0\n')
        self.assertEqual(p.returncode,0,p.stderr);r=responses(p.stdout)
        self.assertEqual(sum(x[0]==0 for x in r),16)
        self.assertEqual(r[-1][0],1)
        self.assertNotEqual(run(frame(b'S1\t0\tONE\n\xff')+b'0\n').returncode,0)

def contract(op,files):
    rows=['UG\t1','O\tnames\t1\tNames remain\ttest\t*\tmembership\tmember\t\texact\tmandatory',f'OP\t{op}\trunning\tcontract\ttimely']
    return ('\n'.join(rows+[f'F\t{p}\tunread\t' for p in files])+'\n').encode()

def results(body):
    return [l for l in body.split(b'\n') if l.startswith((b'RESULT\t',b'GRAPH_',b'ERROR\t'))]

UNAVAILABLE=b'ERROR\tdelta base unavailable\n'

class DeltaFrames(unittest.TestCase):
    """Line-delta evaluation frames rebuild the exact lines from retained values."""
    files=[f'src/f{i:03}.py' for i in range(40)]

    def test_delta_equals_full_frame_and_keeps_reuse(self):
        first=contract('op0',self.files);second=contract('op1',self.files[1:]+['src/new.py'])
        lines=second.decode().split('\n')
        # ops: keep UG,O ; skip old OP, literal new OP; skip f000; keep f001..f039; literal new.py and trailing ''
        ops=['K\t2','D\t1','L\t'+lines[2],'D\t1','K\t39','L\t'+lines[-2],'D\t1','L\t']
        delta=('UG_DELTA\t0\n'+'\n'.join(ops)+'\n').encode()
        p=run(one(0,first)+one(1,delta)+b'0\n');self.assertEqual(p.returncode,0,p.stderr)
        full=run(one(0,first)+one(1,second)+b'0\n');self.assertEqual(full.returncode,0,full.stderr)
        a=responses(p.stdout);b=responses(full.stdout)
        self.assertEqual(a[1][2],b[1][2])
        self.assertTrue(results(a[1][2]))

    def test_stale_or_malformed_delta_is_refused_without_losing_state(self):
        first=contract('op0',self.files)
        n=len(first.decode().split('\n'))
        bad=[
            b'UG_DELTA\t5\nK\t'+str(n).encode()+b'\n',      # wrong base sequence
            b'UG_DELTA\t0\nK\t'+str(n+1).encode()+b'\n',    # consumes past the base
            b'UG_DELTA\t0\nK\t'+str(n-1).encode()+b'\n',    # leaves base lines unconsumed
            b'UG_DELTA\t0\nK\t0\n',                          # zero count
            b'UG_DELTA\t0\nK\t01\n',                         # non-canonical count
            b'UG_DELTA\t0\nX\t1\n',                          # unknown op
            b'UG_DELTA\t0\t1\nK\t1\n',                      # malformed header
        ]
        for delta in bad:
            with self.subTest(delta=delta[:40]):
                p=run(one(0,first)+one(1,delta)+one(2,first)+b'0\n');self.assertEqual(p.returncode,0,p.stderr)
                r=responses(p.stdout);self.assertEqual(r[1][2],UNAVAILABLE)
                self.assertTrue(any(l.startswith(b'RESULT\tnames\tverified') for l in results(r[2][2])))
                self.assertIn(b'GRAPH_INVALIDATION\tcomplete',r[2][2])
                self.assertNotIn(b'GRAPH_DIRTY',r[2][2])

    def test_delta_without_base_is_refused(self):
        p=run(one(0,b'UG_DELTA\t0\nK\t1\n')+b'0\n');self.assertEqual(p.returncode,0,p.stderr)
        self.assertEqual(responses(p.stdout)[0][2],UNAVAILABLE)

if __name__=='__main__':unittest.main()
