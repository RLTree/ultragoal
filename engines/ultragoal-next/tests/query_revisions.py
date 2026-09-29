"""Actual private-session revisions versus the independent uncached core route."""
import hashlib,random,subprocess,unittest
from journey import CORE,obligation,row
from core_session import one,run,responses

def snapshot(files,queries,operation='op',life='running',timely='timely',extra=''):
    return ('UG\t1\n'+queries+row('OP',operation,life,'contract',timely)+''.join(row('F',p,hashlib.sha256(t.encode()).hexdigest(),t) for p,t in sorted(files.items()))+extra).encode()

def records(body):
    return [r.split('\t')for r in body.decode().splitlines()]

def results(body):
    return {r[1]:r for r in records(body) if r[0]=='RESULT'}

def normal(body):
    # Computation work attribution is deliberately different; all evidence,
    # results, keys, semantic packets and fact output must otherwise agree.
    return [r[:7]+['WORK']+r[8:] if r[0]=='RESULT' else r[:5]+['PARSE_WORK']+r[6:] if r[0]in ['P','P_SPANS','P_MIXED'] else r for r in records(body)if not r[0].startswith('GRAPH_')]

class QueryRevisions(unittest.TestCase):
    def revisions(self,frames):
        p=run(b''.join(one(i,f)for i,f in enumerate(frames))+b'0\n')
        self.assertEqual(p.returncode,0,p.stderr.decode());out=responses(p.stdout)
        self.assertEqual([(i,d)for i,d,_ in out],[(i,'done')for i in range(len(frames))])
        return [b for _,_,b in out]

    def test_membership_content_negative_and_unrelated_revision(self):
        queries=(obligation('members','member','prefix:src/','membership')+
                 obligation('text','contains','src/a.py','content','hello')+
                 obligation('negative','absent-path','src/missing.py','membership'))
        a={'src/a.py':'hello','other.py':'one'}
        b=a|{'other.py':'two'};c=b|{'src/a.py':'changed'}
        d=c|{'src/missing.py':'new'};e={'src/renamed.py':'changed','other.py':'two'}
        frames=[snapshot(f,queries,operation=str(i))for i,f in enumerate([a,a,b,c,d,e,a])]
        out=self.revisions(frames)
        for frame,body in zip(frames,out):
            fresh=subprocess.run([str(CORE)],input=frame,capture_output=True,timeout=20)
            self.assertEqual(fresh.returncode,0,fresh.stderr);self.assertEqual(normal(body),normal(fresh.stdout))
        r=list(map(results,out))
        self.assertEqual(r[0]['members'][7],'computed')
        self.assertTrue(all(x[7]=='reused'for x in r[1].values()));self.assertFalse(any(x[0]=='GRAPH_DIRTY'for x in records(out[1])))
        self.assertTrue(all(x[7]=='reused'for x in r[2].values()))
        self.assertEqual(r[3]['members'][7],'reused');self.assertEqual(sum(x[0]=='GRAPH_DIRTY'for x in records(out[3])),2)
        self.assertEqual(r[3]['text'][7],'computed')
        self.assertEqual(r[4]['negative'][2],'failed');self.assertEqual(r[4]['negative'][7],'computed')
        self.assertEqual(r[5]['negative'][2],'verified')
        self.assertEqual(r[6]['text'][2],'verified')

    def test_retained_fact_values_match_cold_and_work_labels_do_not_invalidate(self):
        query=obligation('key','fact-json-key','prefix:src/','content','answer')
        states=[{'src/a.json':'{"answer":1}'},{'src/a.json':'{"answer":1}'},{'src/a.json':'{"other":1}'},{'src/renamed.json':'{"other":1}'},{'src/renamed.json':'{"answer":1}'}]
        frames=[snapshot(files,query,operation=str(i))for i,files in enumerate(states)]
        out=self.revisions(frames)
        for frame,body in zip(frames,out):
            fresh=subprocess.run([str(CORE)],input=frame,capture_output=True,timeout=20)
            self.assertEqual(fresh.returncode,0,fresh.stderr);self.assertEqual(normal(body),normal(fresh.stdout))
        self.assertEqual(results(out[1])['key'][7],'reused')
        self.assertTrue(all(r[5]=='reused'for r in records(out[1])if r[0]=='P'))
        self.assertTrue(all(r[5]=='reused'for r in records(out[3])if r[0]=='P'))

    def test_live_admission_never_reuses_old_success(self):
        queries=obligation('negative','absent-text','prefix:src/','content','bad')
        files={'src/a.py':'good'}
        frames=[snapshot(files,queries),snapshot(files,queries,life='cancelled'),
                snapshot(files,queries,life='superseded'),snapshot(files,queries,timely='late'),
                snapshot(files,queries,extra=row('U','src/unread.py','unavailable','unreadable')),
                snapshot(files,queries)]
        r=list(map(results,self.revisions(frames)))
        self.assertEqual([x['negative'][2]for x in r],['verified','unknown','unknown','unknown','unknown','verified'])
        self.assertTrue(all(x['negative'][7]=='reused'for x in r[1:]))

    def test_changed_query_contract_and_error_reset(self):
        files={'a.py':'hello'};a=snapshot(files,obligation());b=snapshot(files,obligation(argument='goodbye'))
        c=snapshot(files,obligation(revision='2'));bad=b'UG\t1\nO\tinvalid\n'
        out=self.revisions([a,b,c,bad,a])
        self.assertEqual(results(out[1])['text'][2],'failed')
        self.assertEqual(results(out[2])['text'][7],'computed')
        self.assertTrue(out[3].startswith(b'ERROR\t'))
        self.assertEqual(results(out[4])['text'][7],'computed')

    def test_facts_and_native_observations_follow_current_inputs(self):
        queries=(obligation('definition','fact-definition','a.py','content','hello')+
                 obligation('native','native','a.rs','content','rust-syntax'))
        files={'a.py':'def hello():\n    pass\n','a.rs':'fn hello() {}'}
        digest=hashlib.sha256(files['a.rs'].encode()).hexdigest()
        def frame(op,status,extra=''):
            return snapshot(files,queries,operation=op,extra=row('N','a.rs',digest,status,op,'syn-2.0.117','producer')+extra)
        frames=[frame('one','verified'),frame('two','failed'),frame('two','verified'),
                frame('two','verified',row('N','a.rs',digest,'failed','two','syn-2.0.117','producer'))]
        for f,b in zip(frames,self.revisions(frames)):
            fresh=subprocess.run([str(CORE)],input=f,capture_output=True,timeout=20)
            self.assertEqual(normal(b),normal(fresh.stdout))

    def test_scoped_chunk_partials_match_fresh_under_malformed_unselected_rows(self):
        queries=(obligation('has','contains','a.txt','content','needle')+
                 obligation('lacks','absent-text','a.txt','content','needle'))
        def frame(op,bits=('0','0','1','1'),omit_a=False,extra='',cf_count='1'):
            rows=(row('CH','0','complete','','5','1','0')+row('CF','a.txt','digest-a')+
                  row('PC','0','contains','a.txt','needle','1','0','1','1')+
                  row('PC','0','absent-text','a.txt','needle','1','0','1','1')+
                  row('CH','1','omitted','pressure','0',cf_count,'0')+row('CF','b.txt','unavailable'))
            if not omit_a: rows+=row('PC','1','contains','a.txt','needle',*bits)
            rows+=row('PC','1','absent-text','a.txt','needle','0','0','1','1')+extra
            return ('UG\t1\n'+queries+row('OP',op,'running','contract','timely')+row('FC')+rows).encode()
        cases=[frame('clean'),frame('forged-found',('0','1','1','1')),
               frame('forged-supported',('0','0','0','1')),frame('forged-valid',('0','0','1','0')),
               frame('noncanonical',('x','0','1','1')),frame('missing',omit_a=True),
               frame('duplicate',extra=row('PC','1','contains','a.txt','needle','0','0','1','1')),
               frame('wrong-id',extra=row('PC','7','contains','a.txt','needle','0','0','1','1')),
               frame('bad-cf-count',cf_count='2'),frame('clean-again')]
        for request,retained in zip(cases,self.revisions(cases)):
            fresh=subprocess.run([str(CORE)],input=request,capture_output=True,timeout=20)
            self.assertEqual(fresh.returncode,0,fresh.stderr)
            self.assertEqual(normal(retained),normal(fresh.stdout),request[:200])
            current=results(retained)
            if current:
                self.assertNotEqual(current['has'][2],'verified',request[:200])

    def test_selected_chunk_recovery_and_scope_controls_match_fresh(self):
        queries=(obligation('exact','contains','a.txt','content','needle')+
                 obligation('prefix','contains','prefix:','content','needle')+
                 obligation('all','contains','*','content','needle'))
        def frame(op,state,matched):
            if state=='omitted':
                a=(row('CH','0','omitted','pressure','0','1','0')+row('CF','a.txt','unavailable'))
                bits=('1','0','1','1')
            else:
                a=(row('CH','0','complete','','6','1','0')+row('CF','a.txt','digest-a'))
                bits=('1','1' if matched else '0','1','1')
            a+=''.join(row('PC','0','contains',scope,'needle',*bits) for scope in ('a.txt','prefix:','*'))
            return ('UG\t1\n'+queries+row('OP',op,'running','contract','timely')+row('FC')+a).encode()
        states=[('omitted',False),('complete',True),('complete',False),('omitted',False),('complete',True)]
        frames=[frame(str(i),state,matched) for i,(state,matched) in enumerate(states)]
        retained=self.revisions(frames)
        for request,body in zip(frames,retained):
            fresh=subprocess.run([str(CORE)],input=request,capture_output=True,timeout=20)
            self.assertEqual(normal(body),normal(fresh.stdout))
        expected=['unknown','verified','failed','unknown','verified']
        for body,state in zip(retained,expected):
            self.assertEqual({name:result[2] for name,result in results(body).items()},
                             {name:state for name in ('exact','prefix','all')})

    def test_seeded_replacements_match_uncached_evaluation(self):
        rng=random.Random(23921);frames=[]
        for i in range(40):
            files={p:rng.choice(['hello','goodbye','π\t%\nhello'])for p in ['src/a.py','src/b.py','other.py']if rng.randrange(3)}
            queries=''.join(obligation(str(j),rule,scope,projection,arg,revision=str(i//10)) for j,(rule,scope,projection,arg) in enumerate([
                ('member','prefix:src/','membership',''),('path-member','src/a.py','membership',''),
                ('absent-path','src/b.py','membership',''),('contains','src/a.py','content','hello'),
                ('absent-text','prefix:src/','content','goodbye')]))
            frames.append(snapshot(files,queries,operation=str(i)))
        for f,b in zip(frames,self.revisions(frames)):
            fresh=subprocess.run([str(CORE)],input=f,capture_output=True,timeout=20)
            self.assertEqual(normal(b),normal(fresh.stdout))

if __name__=='__main__':unittest.main()
