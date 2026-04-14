#!/usr/bin/env python3
"""Show rewrite status of each function in a file.

For each `fn` definition, shows:
  - Last commit that touched it (hash, date, message)
  - Whether that commit is a C2 rewrite (verified against C++)
  - Lines untouched since initial port flagged as UNVERIFIED
  - Function body size (lines until next fn or end of impl block)

Usage:
  python3 scripts/git_audit.py <file>                  # function summary
  python3 scripts/git_audit.py <file> --chunks          # contiguous commit chunks
  python3 scripts/git_audit.py <file> --unverified      # only unverified functions
  python3 scripts/git_audit.py <file> --risk            # sort by risk (size * age * unverified)
  python3 scripts/git_audit.py --batch <file1> ...      # cross-file summary table
"""
import subprocess, sys, re, os, datetime, json
from collections import defaultdict

C2_PATTERNS = re.compile(
    r'C2|match C\+\+|rewrite.*match|match.*C\+\+|align.*C\+\+'
    r'|inline.*C\+\+|remove.*\.max.*clamp|GetTransparented'
    r'|port.*C\+\+.*silhouette|overhaul golden',
    re.IGNORECASE,
)

# Functions that are purely Rust infrastructure (no C++ equivalent)
INFRA_PATTERNS = re.compile(
    r'^(new|default|fmt|clone|drop|from|into|eq|hash|cmp|partial_|'
    r'try_record|require_direct|record_state|read_pixel|GetImage|image_ref|'
    r'set_record_subops|new_recording|push_state|pop_state)$'
)

def classify(msg):
    if C2_PATTERNS.search(msg):
        return 'C2'
    return 'PORT'

def git_blame(filepath):
    """Run git blame -p and parse into [(line_no, hash, content)]."""
    result = subprocess.run(
        ['git', 'blame', '-p', filepath],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        print(f"git blame failed: {result.stderr}", file=sys.stderr)
        sys.exit(1)

    commits = {}
    lines = []

    cur_hash = None
    for raw in result.stdout.splitlines():
        m = re.match(r'^([0-9a-f]{40}) \d+ (\d+)', raw)
        if m:
            cur_hash = m.group(1)
            cur_lineno = int(m.group(2))
            if cur_hash not in commits:
                commits[cur_hash] = {'date': '', 'summary': '', 'epoch': 0}
            continue
        if raw.startswith('committer-time '):
            ts = int(raw.split(' ', 1)[1])
            commits[cur_hash]['epoch'] = ts
            commits[cur_hash]['date'] = datetime.date.fromtimestamp(ts).isoformat()
        elif raw.startswith('summary '):
            commits[cur_hash]['summary'] = raw.split(' ', 1)[1]
        elif raw.startswith('\t'):
            content = raw[1:]
            lines.append((cur_lineno, cur_hash, content))

    return commits, lines

FN_RE = re.compile(r'^\s*(pub\s+)?(pub\(crate\)\s+)?(async\s+)?(fn\s+(\w+))')

def extract_functions(annotated):
    """Find fn definitions and compute body size and per-line C2 coverage."""
    functions = []
    for i, a in enumerate(annotated):
        m = FN_RE.match(a['content'])
        if m:
            functions.append({**a, 'fn_name': m.group(5), 'fn_sig': m.group(0).strip(), 'idx': i})

    # Compute body size: lines from fn definition to next fn or end of file
    for j, f in enumerate(functions):
        start = f['idx']
        end = functions[j + 1]['idx'] if j + 1 < len(functions) else len(annotated)
        body = annotated[start:end]
        f['body_lines'] = len(body)
        f['body_c2'] = sum(1 for b in body if b['status'] == 'C2')
        f['body_port'] = f['body_lines'] - f['body_c2']
        f['c2_pct'] = int(100 * f['body_c2'] / max(f['body_lines'], 1))
        # Oldest line in body
        epochs = [b['epoch'] for b in body if b.get('epoch', 0) > 0]
        if epochs:
            f['oldest'] = datetime.date.fromtimestamp(min(epochs)).isoformat()
            f['newest'] = datetime.date.fromtimestamp(max(epochs)).isoformat()
        else:
            f['oldest'] = f['date']
            f['newest'] = f['date']

    return functions


def risk_score(f):
    """Higher = more likely to contain unverified divergence."""
    if f['status'] == 'C2' and f['c2_pct'] > 80:
        return 0  # mostly verified
    is_infra = bool(INFRA_PATTERNS.match(f['fn_name']))
    if is_infra:
        return 0
    size_factor = f['body_port']  # unverified lines
    # Older = riskier (days since 2026-03-01)
    try:
        age_days = (datetime.date(2026, 4, 15) - datetime.date.fromisoformat(f['oldest'])).days
    except Exception:
        age_days = 30
    return size_factor * max(age_days, 1)


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)

    if sys.argv[1] == '--batch':
        batch_mode(sys.argv[2:])
        return

    filepath = sys.argv[1]
    mode = sys.argv[2] if len(sys.argv) > 2 else '--functions'

    commits, lines = git_blame(filepath)
    for h, info in commits.items():
        info['status'] = classify(info['summary'])

    annotated = []
    for lineno, h, content in lines:
        info = commits[h]
        annotated.append({
            'lineno': lineno, 'hash': h[:8], 'date': info['date'],
            'epoch': info['epoch'],
            'summary': info['summary'][:65], 'status': info['status'],
            'content': content,
        })

    basename = os.path.basename(filepath)
    print(f"=== Git Audit: {basename} ===")
    print()

    if mode == '--chunks':
        chunks = []
        for a in annotated:
            if chunks and chunks[-1]['hash'] == a['hash']:
                chunks[-1]['end'] = a['lineno']
            else:
                chunks.append({
                    'start': a['lineno'], 'end': a['lineno'],
                    'hash': a['hash'], 'date': a['date'],
                    'summary': a['summary'], 'status': a['status'],
                })
        for c in chunks:
            tag = '[C2]  ' if c['status'] == 'C2' else '[PORT] '
            span = c['end'] - c['start'] + 1
            print(f"  {tag} L{c['start']:<5d}–L{c['end']:<5d} ({span:>4d} lines)  "
                  f"{c['date']}  {c['hash']}  {c['summary']}")

    elif mode == '--risk':
        functions = extract_functions(annotated)
        ranked = sorted(functions, key=risk_score, reverse=True)
        print(f"  {'Risk':>6s}  {'Lines':>5s}  {'Unver':>5s}  {'C2%':>3s}  {'Oldest':>10s}  Function")
        print(f"  {'─'*6}  {'─'*5}  {'─'*5}  {'─'*3}  {'─'*10}  {'─'*40}")
        for f in ranked[:40]:
            r = risk_score(f)
            if r == 0:
                continue
            print(f"  {r:>6d}  {f['body_lines']:>5d}  {f['body_port']:>5d}  {f['c2_pct']:>3d}  "
                  f"{f['oldest']:>10s}  {f['fn_sig']}")

    else:
        functions = extract_functions(annotated)
        show_all = mode != '--unverified'

        c2_count = 0
        port_count = 0
        for f in functions:
            tag = '[C2]  ' if f['status'] == 'C2' else '[PORT] '
            if f['status'] == 'C2':
                c2_count += 1
            else:
                port_count += 1
            if show_all or f['status'] != 'C2':
                print(f"  {tag} L{f['lineno']:<5d}  {f['body_lines']:>4d}L  "
                      f"{f['c2_pct']:>3d}%C2  {f['date']}  {f['hash']}  {f['fn_sig']}")

        print()
        print(f"  C2-verified: {c2_count}   Unverified: {port_count}   Total: {c2_count + port_count}")

    c2_lines = sum(1 for a in annotated if a['status'] == 'C2')
    total = len(annotated)
    print(f"  Lines: {c2_lines}/{total} C2-verified ({100*c2_lines//max(total,1)}%)")


def batch_mode(filepaths):
    """Cross-file summary table."""
    print(f"  {'File':<35s}  {'Fns':>4s}  {'C2':>3s}  {'Unv':>4s}  {'Lines':>6s}  {'C2L':>5s}  {'%':>3s}  {'Top risk function':<40s}")
    print(f"  {'─'*35}  {'─'*4}  {'─'*3}  {'─'*4}  {'─'*6}  {'─'*5}  {'─'*3}  {'─'*40}")
    for fp in filepaths:
        if not os.path.isfile(fp):
            continue
        commits, lines = git_blame(fp)
        for h, info in commits.items():
            info['status'] = classify(info['summary'])
        annotated = []
        for lineno, h, content in lines:
            info = commits[h]
            annotated.append({
                'lineno': lineno, 'hash': h[:8], 'date': info['date'],
                'epoch': info['epoch'],
                'summary': info['summary'][:65], 'status': info['status'],
                'content': content,
            })
        functions = extract_functions(annotated)
        c2_fns = sum(1 for f in functions if f['status'] == 'C2')
        port_fns = len(functions) - c2_fns
        c2_lines = sum(1 for a in annotated if a['status'] == 'C2')
        total = len(annotated)
        pct = 100 * c2_lines // max(total, 1)
        ranked = sorted(functions, key=risk_score, reverse=True)
        top = ranked[0]['fn_sig'] if ranked and risk_score(ranked[0]) > 0 else '(all verified)'
        basename = os.path.basename(fp)
        print(f"  {basename:<35s}  {len(functions):>4d}  {c2_fns:>3d}  {port_fns:>4d}  "
              f"{total:>6d}  {c2_lines:>5d}  {pct:>3d}  {top[:40]}")


if __name__ == '__main__':
    main()
