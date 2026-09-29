"""Python reference of `select` retrieval (Lexicon.bend and Retrieval.bend), in the same
integer fixed-point arithmetic, for the differential test. It ranks a list of admitted
candidate windows exactly as the four Bend stages do:

  1. MATCH: every file's path, definition-name and body term counts, window counts.
  2. FILE_RANK: BM25F over files (path x3, definition names x2, body x1); a pool of M = 4G.
  3. PASSAGE: each pooled file's best window by BM25 with window-level IDF.
  4. FUSE: reciprocal-rank fusion (k = 60) of the file rank and the best-window rank;
     the first G distinct excerpt groups, each shown as its best window.

Candidates are dicts with id, path, excerpt (and grammar for grouping); refused rows must
already be removed, as Bend's disclosure screen removes them before counting.
"""
import collections

# ---- tokens

STOP = set('''a about above after again against all also am an and any are aren as at be because been before being
below between both but by can cannot could did didn do does doesn doing don down during each either else even ever
every few for from further get gets got had has hasn have having he her here hers him his how i if in into is isn it
its itself just let like ll me might more most much must my myself no nor not now of off on once only or other our
ours out over own per re s same shall she should so some such t than that the their theirs them then there these they
this those through to too under until up us use used uses using ve very via want wants was wasn way we were weren what
whatever when where whether which while who whom whose why will with within without would yes yet you your yours
exactly actually happen happens happening instead need needs say says tell work works'''.split())
KEYWORDS = {'def', 'class', 'fn', 'struct', 'enum', 'trait', 'impl', 'type', 'interface', 'func', 'function', 'const',
            'let', 'var', 'mod'}
WS = {' ', '\t', '\r', '\x0b', '\x0c'}


def cls(c):
    if 'a' <= c <= 'z':
        return 1
    if 'A' <= c <= 'Z':
        return 2
    if '0' <= c <= '9':
        return 3
    if c == '_':
        return 4
    return 0


def parts(run):
    """Lower-case parts of one identifier run, in order."""
    out, part, pc = [], '', 0
    for c in run:
        k = cls(c)
        if k == 4:
            out.append(part); part, pc = '', 0
        elif k == 3:
            if pc == 3:
                part += c
            else:
                out.append(part); part, pc = c, 3
        elif k == 1:
            if pc == 1:
                part += c
            elif pc == 2 and len(part) >= 2:
                out.append(part[:-1]); part, pc = part[-1] + c, 1
            elif pc == 2:
                part, pc = part + c, 1
            else:
                out.append(part); part, pc = c, 1
        else:
            if pc == 2:
                part += c.lower()
            else:
                out.append(part); part, pc = c.lower(), 2
    out.append(part)
    return [p for p in out if p]


def stem(w):
    if len(w) <= 3:
        return w
    if w.endswith('ies') and len(w) > 4:
        w = w[:-3] + 'y'
    elif w.endswith('sses'):
        w = w[:-2]
    elif w.endswith('s') and not w.endswith(('ss', 'us', 'is')):
        w = w[:-1]
    if w.endswith('ing') and len(w) > 5:
        w = w[:-3]
    elif w.endswith('ed') and len(w) > 4:
        w = w[:-2]
    if w.endswith('e') and len(w) > 3:
        w = w[:-1]
    if w.endswith('y') and len(w) > 3:
        w = w[:-1] + 'i'
    return w


def tokens(run):
    ps = parts(run)
    out = [stem(p) for p in ps]
    if len(ps) > 1:
        out.append(stem(''.join(ps)))
    return out


def runs(text):
    out, cur = [], ''
    for c in text:
        if cls(c):
            cur += c
        else:
            if cur:
                out.append(cur)
            cur = ''
    if cur:
        out.append(cur)
    return out


def head_word(h, w, x):
    """State after word w (forward, non-empty) ended by separator x; and whether w is a name."""
    if h == 1 and w == 'default' and x in WS:
        return 0, False
    if h in (0, 1):
        if x in WS:
            if w in ('pub', 'async', 'static'):
                return 0, False
            if w == 'export':
                return 1, False
            return (4 if w in KEYWORDS else 7), False
        if x == '(':
            return (2 if w == 'pub' else 7), False
        return 7, False
    if h == 2:
        return (3 if x == ')' and all('a' <= c <= 'z' for c in w) else 7), False
    if h in (4, 6):
        return 7, cls(w[0]) != 3
    if h == 5:
        return (6 if x == ')' else 5), False
    return 7, False


def head_empty(h, x):
    if h in (0, 1):
        return h if x in WS else 7
    if h == 3:
        return 0 if x in WS else 7
    if h == 4:
        return 4 if x in WS else (5 if x == '(' else 7)
    if h == 5:
        return 6 if x == ')' else 5
    if h == 6:
        return 6 if x in WS else 7
    return 7


def scan(text):
    """(token count, body tokens, definition-name tokens) of one text, as Lexicon.scan."""
    body, defs, run, h = [], [], '', 0
    for x in text + '\n':
        if cls(x):
            run += x
            if h == 3:
                h = 7
            continue
        if run:
            h, name = head_word(h, run, x)
            ts = tokens(run)
            body.extend(ts)
            if name:
                defs.extend(ts)
        else:
            h = head_empty(h, x)
        run = ''
        if x == '\n':
            h = 0
    return len(body), body, defs


def terms(q):
    rs = runs(q)
    words = []
    for r in rs:
        ps = parts(r)
        words.extend(p for p in ps if p not in STOP)
        if len(ps) > 1:
            words.append(''.join(ps))
    out = [stem(w) for w in words]
    plain = [r.lower() for r in rs if r.lower() not in STOP and '_' not in r]
    out.extend(stem(a + b) for a, b in zip(plain, plain[1:]))
    return list(dict.fromkeys(t for t in out if len(t) >= 2))


# ---- fixed point

def log2fx(x):
    """log2(x) in 1/1024 units for x >= 1: integer part, then 10 bits by squaring in Q15."""
    e = x.bit_length() - 1
    m = x << (15 - e) if e <= 15 else x >> (e - 15)
    frac = 0
    for _ in range(10):
        m = (m * m) >> 15
        if m >= 1 << 16:
            m >>= 1
            frac = frac * 2 + 1
        else:
            frac = frac * 2
    return 1024 * e + frac


def idf(count, df):
    """log2((count + 1) / (df + 0.5)), in 1/1000 units."""
    return (log2fx(2 * count + 2) - log2fx(2 * df + 1)) * 1000 // 1024


def bm25(weight, f, length, total, count):
    """One BM25 term (k1 = 1.2, b = 0.75) in 1/1000 units."""
    avg = total * 1000 // count if count else 0
    ratio = length * 1000000 // avg if avg else 0
    norm = (3000 + 9 * ratio) // 10
    tf = f * 2200000 // (f * 1000 + norm)
    return weight * tf // 1000


def fused(file_rank, best_rank):
    return 1000000000 // (60 + file_rank) + 1000000000 // (60 + best_rank)


# ---- the four stages over one list of admitted candidates

def rank(query, cands, shortlist=128):
    """Pool, best windows and the fused shortlist, as the Bend stages compute them."""
    ts = terms(query)
    index = {t: k for k, t in enumerate(ts)}
    T = len(ts)
    files = collections.OrderedDict()
    N = clen = 0
    cdf = [0] * T
    path_cache = {}
    windows = collections.defaultdict(list)
    for c in cands:
        p = c['path']
        if p not in path_cache:
            n, b, _ = scan(p)
            path_cache[p] = (n, collections.Counter(index[t] for t in b if t in index))
        pn, pc = path_cache[p]
        n, b, d = scan(c['excerpt'])
        f = files.setdefault(p, {'flen': 0, 'path': pc, 'def': collections.Counter(), 'body': collections.Counter()})
        f['flen'] += n
        bc = collections.Counter(index[t] for t in b if t in index)
        f['body'].update(bc)
        f['def'].update(index[t] for t in d if t in index)
        N += 1
        clen += n + pn
        for k in set(bc) | set(pc):
            cdf[k] += 1
        windows[p].append((c, n + pn, bc + pc))
    F = len(files)
    total_flen = sum(f['flen'] for f in files.values())
    fdf = [0] * T
    for f in files.values():
        for k in set(f['path']) | set(f['def']) | set(f['body']):
            fdf[k] += 1
    fw = [idf(F, fdf[k]) for k in range(T)]
    ww = [idf(N, cdf[k]) for k in range(T)]
    scored = []
    for p, f in files.items():
        s = 0
        for k in range(T):
            x = 3 * f['path'][k] + 2 * f['def'][k] + f['body'][k]
            if x:
                s += bm25(fw[k], x, f['flen'], total_flen, F)
        if s > 0:
            scored.append((s, p))
    scored.sort(key=lambda v: (-v[0], v[1]))
    M = 4 * shortlist
    pool = scored[:M]
    best = {}
    for _, p in pool:
        top = None
        for c, length, tf in windows[p]:
            s = sum(bm25(ww[k], x, length, clen, N) for k, x in tf.items())
            if top is None or s > top[0]:
                top = (s, c)
        best[p] = top
    by_best = sorted((p for _, p in pool), key=lambda p: (-best[p][0], p))
    best_rank = {p: i + 1 for i, p in enumerate(by_best)}
    order = sorted(((fused(i + 1, best_rank[p]), p) for i, (_, p) in enumerate(pool)), key=lambda v: (-v[0], v[1]))
    groups, chosen = set(), []
    for score, p in order:
        c = best[p][1]
        key = (c['grammar'], c['excerpt'])
        if key in groups:
            continue
        groups.add(key)
        chosen.append((c['id'], score, p))
        if len(chosen) >= shortlist:
            break
    return {'terms': ts, 'files': F, 'matched': len(scored), 'pool': [(p, s) for s, p in pool],
            'best': {p: (best[p][1]['id'], best[p][0]) for _, p in pool}, 'shortlist': chosen,
            'stop': chosen[-1][1] if chosen else 0}
