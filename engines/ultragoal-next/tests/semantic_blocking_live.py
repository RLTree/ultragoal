"""Live qualified semantic blocking through `check`: one Jev request, then a stored reuse.
Makes a provider call, so it runs only with UG_LIVE_JEV=1 and the installed
research-run.typesafe Keychain route; attempts use UG_USAGE_LEDGER when set."""
import json, os, pathlib, subprocess, tempfile, unittest
from journey import BIN, row

REQUIREMENT = 'After the failed schema migration, recovery restored the orders database to its exact pre-migration state.'
LOG = ('2026-09-24T10:02:11Z migration 0042 failed at step 3 of 5\n'
       '2026-09-24T10:02:14Z recovery started from snapshot orders-pre-0042\n'
       '2026-09-24T10:03:40Z recovery aborted: snapshot orders-pre-0042 is corrupt; tables orders and order_items remain partially migrated\n'
       '2026-09-24T10:03:41Z database state: NOT restored; manual repair required\n')

@unittest.skipUnless(os.environ.get('UG_LIVE_JEV') == '1', 'live Jev test: set UG_LIVE_JEV=1')
class SemanticBlockingLive(unittest.TestCase):
    def test_qualified_contradiction_blocks_and_the_switch_keeps_it_advisory(self):
        with tempfile.TemporaryDirectory() as d:
            root = pathlib.Path(d, 'repo'); root.mkdir(); (root / 'recovery.log').write_text(LOG)
            contract = pathlib.Path(d, 'contract.tsv')
            contract.write_text('UG\t1\n' + row('O', 'recovered', '1', REQUIREMENT, 'incident review acceptance', 'recovery.log', 'content', 'semantic', 'recovery-state', 'semantic', 'mandatory'))
            ledger = ['--usage-ledger', os.environ['UG_USAGE_LEDGER']] if os.environ.get('UG_USAGE_LEDGER') else []
            def check(*extra):
                p = subprocess.run([str(BIN), 'check', '--root', str(root), '--contract', str(contract), '--cache', str(pathlib.Path(d, 'cache')),
                                    '--disclosure', 'synthetic', *ledger, *extra], capture_output=True, text=True, timeout=120)
                report = json.loads(p.stdout)
                return p.returncode, report['obligations'][0], report['semantic']
            code, o, semantic = check()
            decision = semantic[-1]['groups'][0]['decision'][0]
            self.assertEqual(decision[1], 'contradicted', semantic)
            self.assertEqual((code, o['state']), (1, 'failed'), o)
            self.assertTrue(o['admission_reason'].startswith('qualified semantic block under ug-semantic-fit-v3:sha256:'), o)
            self.assertIn('recovery-state', o['next_action'])
            code, o, semantic = check('--local')
            self.assertEqual((code, o['state']), (2, 'unknown'), o)
            self.assertFalse(any(item.get('attempts') for item in semantic if isinstance(item, dict)), semantic)
            code, o, semantic = check('--semantic-blocking', 'off')
            self.assertEqual(semantic[-1]['groups'][0]['assessment'].get('state'), 'reused-prior', semantic)
            self.assertEqual((code, o['state']), (2, 'unknown'), o)

if __name__ == '__main__':
    unittest.main()
