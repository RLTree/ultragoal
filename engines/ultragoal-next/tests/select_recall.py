"""Development recall of `ultragoal select` (a regression check, not evidence of quality: the
design was chosen on these questions). A critical file counts as held when a shortlist group's
representative path is that file. At G = 128 the real binary must hold at least 0.85 of the
115 critical files; G = 64 is reported beside it. Needs the select-design development set
(UG_SELECT_DEV, as select_differential.py); UG_SELECT_REPOS narrows the repositories."""
import json, unittest
from select_differential import DEV, REPOS, select


class SelectRecall(unittest.TestCase):
    def test_development_recall_at_128(self):
        repos = [r for r in REPOS if (DEV / 'questions' / f'{r}.json').is_file() and (DEV / 'idx' / f'{r}.json').is_file()]
        if not repos:
            self.skipTest(f'development set absent at {DEV}')
        held = {64: 0, 128: 0}
        total, per = 0, {}
        for repo in repos:
            for q in json.loads((DEV / 'questions' / f'{repo}.json').read_text())['questions']:
                critical = {c['path'] for c in q['critical']}
                total += len(critical)
                for g in held:
                    paths = {c['path'] for c in select(repo, q['question'], g)['shortlist']}
                    held[g] += len(critical & paths)
                    per.setdefault(repo, {}).setdefault(g, [0, 0])
                    per[repo][g][0] += len(critical & paths)
                    per[repo][g][1] += len(critical)
                print(q['id'], {g: per[repo][g] for g in held}, flush=True)
        recall = {g: held[g] / total for g in held}
        print('SELECT_RECALL', json.dumps({'critical': total, 'held': held, 'recall': recall, 'per_repository': per}), flush=True)
        self.assertGreaterEqual(recall[128], 0.85, recall)


if __name__ == '__main__':
    unittest.main()
