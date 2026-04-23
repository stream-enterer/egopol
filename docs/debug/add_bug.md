# Add an Issue to ISSUES.json

Read `docs/debug/ISSUES.json` to understand the schema, then add a new entry to the `issues` array.

## Determine kind

- `fix` — something is wrong and an agent can likely find and fix it autonomously
- `design` — requires human architectural or planning decisions before a fix can be written
- `perf` — performance problem, requires measurement

When unsure, use `fix`. The debug harness will reclassify to `design` if investigation reveals it.

## Assign an ID

- `fix` → next available `F###` (e.g. F002 if F001 exists)
- `design` → next available `D###`
- `perf` → next available `P###`

## Fill universal fields

Set `status` to `open`. Set `introduced_date` to today. Leave `introduced_commit`, `fix_note`, `investigation_file`, `root_cause_file`, `blocked_question`, `fixed_in_commit`, `fixed_date` as `null`.

## Add kind-specific fields

**fix:** add `"repro": "<steps or null>"`

**design:** add `"repro": null` and `"details": { "audit_source": null }`

**perf:** add `"repro": "<steps or null>"` and `"details": { "metric": null, "observed": null, "target": null, "benchmark": null }`

## Validate

Run `python3 -c "import json; json.load(open('docs/debug/ISSUES.json')); print('valid')"` to confirm the file still parses.
