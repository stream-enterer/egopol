# Round 3: Convergence Ledger

## 1. Compliance Ceiling

| Metric | Value |
|--------|-------|
| Instruction count (N) | 98 |
| Mean CE (P_single) | 0.8041 |
| P(all) = P_single^N | 10^-9.28 = 5.25e-10 |

**Interpretation:** With 98 instructions and an average CE of 0.8041, P(all correct) = 0.8041^98 = 10^-9.28 ≈ 5.25e-10. The probability that an LLM correctly interprets and follows ALL 98 instructions in a single run is essentially zero (less than one in a billion). Even with strong per-instruction clarity (~80%), the multiplicative penalty of 98 independent requirements makes perfect compliance impossible. The prompt compensates through mechanical enforcement (clippy gate, pre-commit hook, contract JSON progressive disclosure), but the ~20 instructions without mechanical backstops (compare ALL methods, test quality, log format, SUSPECT handling) represent the true reliability bottleneck. Expect 5-15 instructions to be partially or wholly violated per run, concentrated in the wounded/contested categories.

## 2. Category Summary

| Category | Count | % | Criteria |
|----------|-------|---|----------|
| Survivor | 76 | 77.6% | Composite >= 0.75, no axis below 0.50 |
| Wounded | 20 | 20.4% | Composite 0.50-0.74, or any axis below 0.50 |
| Contested | 1 | 1.0% | CF below 0.40 |
| Fallen | 1 | 1.0% | Composite below 0.50 or CE below 0.30 |
| **Total** | **98** | **100%** | |

## 3. Full Scoreboard (sorted by composite, worst-first)

| # | ID | CE | SR | CF | ST | Comp | Cat | Crit | Weak | Text |
|---|------|------|------|------|------|------|------|------|------|------|
| 1 | i-28 | 0.70 | 0.45 | 0.25 | 0.35 | 0.44 | F |  | cf | Do NOT modify existing tests (write new tests only). |
| 2 | i-14 | 0.60 | 0.75 | 0.50 | 0.40 | 0.56 | W |  | st | Phase 4 features: Review ALL existing tests (1500+) across every ... |
| 3 | i-61 | 0.70 | 0.83 | 0.25 | 0.50 | 0.57 | C |  | cf | Phase 4 step: Fix any defective test by strengthening its asserti... |
| 4 | i-29 | 0.65 | 0.48 | 0.65 | 0.70 | 0.62 | W |  | sr | Do NOT modify unrelated production code. |
| 5 | i-89 | 0.80 | 0.45 | 0.80 | 0.55 | 0.65 | W |  | sr | Log format: feature entry must include feature_id, description, M... |
| 6 | i-93 | 0.55 | 0.78 | 0.85 | 0.55 | 0.68 | W |  | ce | Phase 1 step: For MISMATCH fixes, apply minimal changes to the Ru... |
| 7 | i-16 | 0.50 | 0.60 | 0.90 | 0.75 | 0.69 | W |  | ce | When writing tests, follow the patterns in the corresponding test... |
| 8 | i-60 | 0.75 | 0.85 | 0.75 | 0.40 | 0.69 | W |  | st | Phase 4 step: For each test function, check 5 anti-patterns: (1) ... |
| 9 | i-97 | 0.60 | 0.83 | 0.90 | 0.45 | 0.69 | W |  | st | Phase 4 anti-pattern check: verify test would FAIL if implementat... |
| 10 | i-34 | 0.80 | 0.52 | 0.90 | 0.60 | 0.70 | W |  | sr | After each feature, append to .workflow/widget-comparison/run-log... |
| 11 | i-32 | 0.75 | 0.50 | 1.00 | 0.60 | 0.71 | W |  | sr | Do NOT treat the gate as proof of completeness. The gate is neces... |
| 12 | i-22 | 0.85 | 0.65 | 0.75 | 0.65 | 0.72 | W |  | sr | If a later feature requires changing code from a previously-compl... |
| 13 | i-96 | 0.65 | 0.83 | 0.90 | 0.50 | 0.72 | W |  | st | Phase 4 anti-pattern check: verify test exercises actual behavior... |
| 14 | i-11 | 0.70 | 0.82 | 0.85 | 0.55 | 0.73 | W |  | st | Phase 1 features: Compare each C++ method listed in the methods f... |
| 15 | i-24 | 0.90 | 0.72 | 0.75 | 0.55 | 0.73 | W | Y | st | Steps are requirements. If a step says 'compare these 20 methods'... |
| 16 | i-62 | 0.85 | 0.83 | 0.60 | 0.65 | 0.73 | W |  | cf | Phase 4 step: If a strengthened test now FAILS, that is a real bu... |
| 17 | i-73 | 0.55 | 0.58 | 1.00 | 0.80 | 0.73 | W |  | ce | CLAUDE.md: Use expect('reason') unless invariant is obvious from ... |
| 18 | i-23 | 0.90 | 0.75 | 0.75 | 0.55 | 0.74 | W | Y | st | The gate (clippy + tests) does NOT verify you compared every meth... |
| 19 | i-49 | 0.55 | 0.85 | 0.95 | 0.60 | 0.74 | W |  | ce | Phase 1 step (RUST-ONLY variant): No C++ equivalent. Verify inter... |
| 20 | i-58 | 0.65 | 0.82 | 0.85 | 0.65 | 0.74 | W |  | ce | Phase 3 step: Verify all public methods from the consolidated C++... |
| 21 | i-71 | 0.60 | 0.55 | 1.00 | 0.80 | 0.74 | W |  | sr | CLAUDE.md: One primary type per file. Private mod + public use re... |
| 22 | i-25 | 0.80 | 0.70 | 0.85 | 0.65 | 0.75 | S |  | st | Tests must assert on specific values and behaviors, not just is_s... |
| 23 | i-77 | 0.60 | 0.70 | 0.95 | 0.75 | 0.75 | S |  | ce | CLAUDE.md (State fidelity): Fully idiomatic Rust for state logic,... |
| 24 | i-78 | 0.70 | 0.65 | 0.95 | 0.70 | 0.75 | S |  | sr | CLAUDE.md: When in doubt, check if the function's output feeds a ... |
| 25 | i-04 | 0.65 | 0.88 | 0.90 | 0.60 | 0.76 | S | Y | st | For each feature, read its steps array and execute each step (rea... |
| 26 | i-12 | 0.70 | 0.78 | 0.90 | 0.65 | 0.76 | S |  | st | Phase 2 features: For each gap method, determine IMPLEMENTED (dif... |
| 27 | i-43 | 0.85 | 0.87 | 0.75 | 0.55 | 0.76 | S | Y | st | Phase 1 step (standard): Compare the N methods listed in the meth... |
| 28 | i-52 | 0.55 | 0.83 | 0.95 | 0.70 | 0.76 | S |  | ce | Phase 2 step: Note that some C++ methods are generated by macros ... |
| 29 | i-70 | 0.60 | 0.58 | 1.00 | 0.85 | 0.76 | S |  | sr | CLAUDE.md: Construction: new() primary, builder with_*(self) -> S... |
| 30 | i-31 | 0.85 | 0.58 | 0.95 | 0.70 | 0.77 | S |  | sr | Do NOT flag idiomatic Rust as MISMATCH. Read the layer field. |
| 31 | i-53 | 0.70 | 0.83 | 0.90 | 0.65 | 0.77 | S |  | st | Phase 2 step: For each listed C++ method, determine: (a) is it ac... |
| 32 | i-55 | 0.80 | 0.83 | 0.85 | 0.60 | 0.77 | S |  | st | Phase 2 step: For NEEDS_IMPLEMENTATION items, implement them and ... |
| 33 | i-98 | 0.85 | 0.82 | 0.75 | 0.65 | 0.77 | S |  | st | Phase 1 step: The report for each method must use one of four cat... |
| 34 | i-13 | 0.65 | 0.75 | 0.95 | 0.75 | 0.78 | S |  | ce | Phase 3 features: Verify all consolidated C++ methods are present... |
| 35 | i-15 | 0.90 | 0.72 | 0.80 | 0.70 | 0.78 | S | Y | st | Phase 1 features have a required_test_layers field. A fix is not ... |
| 36 | i-19 | 0.85 | 0.78 | 0.80 | 0.70 | 0.78 | S |  | st | Features with layer=STATE: idiomatic Rust is correct. Only flag b... |
| 37 | i-45 | 0.75 | 0.85 | 0.90 | 0.60 | 0.78 | S |  | st | Phase 1 step (standard): For each method, verify constructors set... |
| 38 | i-46 | 0.85 | 0.85 | 0.75 | 0.65 | 0.78 | S |  | st | Phase 1 step (standard): For each method, report MATCH, MISMATCH,... |
| 39 | i-94 | 0.70 | 0.75 | 1.00 | 0.65 | 0.78 | S |  | st | Phase 2 step: For NEEDS_IMPLEMENTATION items, include a scope est... |
| 40 | i-95 | 0.70 | 0.83 | 0.90 | 0.70 | 0.78 | S |  | ce | Phase 4 anti-pattern check: verify modifier keys are set correctl... |
| 41 | i-47 | 0.75 | 0.85 | 0.90 | 0.65 | 0.79 | S |  | st | Phase 1 step (standard): For each MISMATCH, fix the Rust code min... |
| 42 | i-76 | 0.80 | 0.72 | 0.90 | 0.75 | 0.79 | S |  | sr | CLAUDE.md (Geometry fidelity): Same algorithm and operation order... |
| 43 | i-30 | 0.90 | 0.55 | 0.90 | 0.85 | 0.80 | S | Y | sr | Do NOT skip a feature. If 0 MISMATCHes found, it still passes -- ... |
| 44 | i-50 | 0.70 | 0.85 | 0.95 | 0.70 | 0.80 | S |  | ce | Phase 1 step (RUST-ONLY variant): Write a unit test for any untes... |
| 45 | i-57 | 0.70 | 0.83 | 0.90 | 0.75 | 0.80 | S |  | ce | Phase 3 step: Verify the listed C++ symbols are covered in the Ru... |
| 46 | i-86 | 0.75 | 0.65 | 1.00 | 0.80 | 0.80 | S |  | sr | CLAUDE.md: Do NOT use assert! for recoverable errors. |
| 47 | i-03 | 0.80 | 0.90 | 0.85 | 0.70 | 0.81 | S | Y | st | Process features in order: read next feature where passes == fals... |
| 48 | i-26 | 0.90 | 0.68 | 0.85 | 0.80 | 0.81 | S |  | sr | If you fix a MISMATCH, write a NEW test. Existing tests didn't ca... |
| 49 | i-39 | 0.85 | 0.80 | 0.85 | 0.75 | 0.81 | S |  | st | Contract rule: Check each feature's layer field. PIXEL = exact fo... |
| 50 | i-68 | 0.75 | 0.68 | 1.00 | 0.80 | 0.81 | S |  | sr | CLAUDE.md: Per-module Result with custom error enums (Display + E... |
| 51 | i-21 | 0.90 | 0.72 | 0.85 | 0.80 | 0.82 | S | Y | sr | Phases execute in order. Before starting a phase, verify its depe... |
| 52 | i-33 | 0.80 | 0.60 | 1.00 | 0.90 | 0.82 | S |  | sr | Run the completion script at the end to report total progress and... |
| 53 | i-54 | 0.80 | 0.83 | 0.90 | 0.75 | 0.82 | S |  | st | Phase 2 step: For each method, mark as IMPLEMENTED (found under d... |
| 54 | i-72 | 0.85 | 0.62 | 0.95 | 0.85 | 0.82 | S |  | sr | CLAUDE.md: pub(crate) default visibility. pub only for library AP... |
| 55 | i-90 | 0.90 | 0.48 | 1.00 | 0.90 | 0.82 | W |  | sr | Log entry must be appended (not overwritten) to .workflow/widget-... |
| 56 | i-59 | 0.85 | 0.83 | 0.90 | 0.75 | 0.83 | S |  | st | Phase 4 step: List all #[test] functions in the specified test di... |
| 57 | i-74 | 0.80 | 0.72 | 0.95 | 0.85 | 0.83 | S |  | sr | CLAUDE.md: Fix warnings (remove dead code, prefix _, apply clippy... |
| 58 | i-18 | 0.90 | 0.80 | 0.80 | 0.85 | 0.84 | S | Y | sr | Features with layer=PIXEL require exact C++ formulas. Integer ari... |
| 59 | i-20 | 0.95 | 0.55 | 0.95 | 0.90 | 0.84 | S |  | sr | Features without a layer field default to STATE fidelity rules. |
| 60 | i-38 | 0.80 | 0.78 | 0.95 | 0.85 | 0.84 | S |  | sr | Contract rule: Commit after each feature. Include the contract JS... |
| 61 | i-44 | 0.80 | 0.87 | 0.95 | 0.75 | 0.84 | S |  | st | Phase 1 step (standard): Also read the C++ .cpp implementation fi... |
| 62 | i-48 | 0.85 | 0.85 | 0.90 | 0.75 | 0.84 | S |  | st | Phase 1 step (standard): For each fix, write a test verifying the... |
| 63 | i-63 | 0.85 | 0.82 | 0.90 | 0.80 | 0.84 | S |  | st | Phase 4 (inline test variant): grep -rn '#[cfg(test)]' src/ to fi... |
| 64 | i-69 | 0.85 | 0.62 | 1.00 | 0.90 | 0.84 | S |  | sr | CLAUDE.md: Import order: std then external then crate::. Explicit... |
| 65 | i-08 | 0.85 | 0.82 | 0.90 | 0.85 | 0.85 | S | Y | sr | After setting passes to true, run git add prompts/master-contract... |
| 66 | i-27 | 0.95 | 0.55 | 1.00 | 0.90 | 0.85 | S | Y | sr | Do NOT modify the contract's steps, description, or methods field... |
| 67 | i-91 | 0.85 | 0.72 | 1.00 | 0.85 | 0.85 | S |  | sr | Commit message format: 'audit: <feature_id> <summary>'. |
| 68 | i-17 | 0.85 | 0.78 | 1.00 | 0.80 | 0.86 | S |  | sr | Phase 1 steps require reading the C++ .cpp file (not just the hea... |
| 69 | i-64 | 0.90 | 0.70 | 0.95 | 0.90 | 0.86 | S |  | sr | CLAUDE.md: Use f64 for logical coordinates, i32 for pixel coordin... |
| 70 | i-75 | 0.90 | 0.78 | 0.90 | 0.85 | 0.86 | S | Y | sr | CLAUDE.md (Pixel fidelity): Reproduce C++ integer formulas exactl... |
| 71 | i-82 | 0.90 | 0.65 | 1.00 | 0.90 | 0.86 | S |  | sr | CLAUDE.md: Do NOT use glob imports (use foo::*) except use super:... |
| 72 | i-84 | 0.90 | 0.75 | 0.95 | 0.85 | 0.86 | S |  | sr | CLAUDE.md: Do NOT use f64 in blend/coverage/interpolation paths -... |
| 73 | i-88 | 0.90 | 0.80 | 0.85 | 0.90 | 0.86 | S | Y | sr | CLAUDE.md: Pre-commit hook runs cargo fmt (auto-applied) then cli... |
| 74 | i-36 | 0.90 | 0.72 | 0.95 | 0.90 | 0.87 | S |  | sr | Contract rule: Agent MAY set passes to true. Agent MAY NOT delete... |
| 75 | i-67 | 0.90 | 0.68 | 1.00 | 0.90 | 0.87 | S |  | sr | CLAUDE.md: Use String owned, &str params. No Cow. |
| 76 | i-79 | 0.90 | 0.72 | 0.95 | 0.90 | 0.87 | S |  | sr | CLAUDE.md: Do NOT use #[allow(...)] or #[expect(...)]. Fix the wa... |
| 77 | i-83 | 0.90 | 0.72 | 1.00 | 0.85 | 0.87 | S |  | sr | CLAUDE.md: Do NOT truncate color math to u8 mid-calculation. |
| 78 | i-85 | 0.90 | 0.62 | 1.00 | 0.95 | 0.87 | S |  | sr | CLAUDE.md: Do NOT use Rayon/parallel iteration on golden-tested c... |
| 79 | i-65 | 0.90 | 0.70 | 1.00 | 0.90 | 0.88 | S |  | sr | CLAUDE.md: Use Color (packed u32 RGBA) for storage. Intermediate ... |
| 80 | i-66 | 0.90 | 0.72 | 1.00 | 0.90 | 0.88 | S |  | sr | CLAUDE.md: Use Rc/RefCell for shared state, Weak for parent refs.... |
| 81 | i-10 | 0.75 | 0.85 | 1.00 | 0.95 | 0.89 | S |  | ce | Run the bootstrap script at the start to see current progress (do... |
| 82 | i-35 | 0.90 | 0.70 | 1.00 | 0.95 | 0.89 | S |  | sr | Start with the first feature in phase-1 where passes is false. |
| 83 | i-37 | 0.90 | 0.82 | 0.95 | 0.90 | 0.89 | S | Y | sr | Contract rule: passes may only be set to true after clippy clean ... |
| 84 | i-92 | 0.90 | 0.75 | 1.00 | 0.90 | 0.89 | S | Y | sr | Include prompts/master-contract.json in the git add for each comm... |
| 85 | i-40 | 0.85 | 0.88 | 1.00 | 0.85 | 0.90 | S |  | ce | Phase 1 step (standard): Read the C++ header file for the feature... |
| 86 | i-41 | 0.85 | 0.88 | 1.00 | 0.85 | 0.90 | S |  | ce | Phase 1 step (standard): Read the C++ .cpp implementation file fo... |
| 87 | i-42 | 0.85 | 0.88 | 1.00 | 0.85 | 0.90 | S |  | ce | Phase 1 step (standard): Read the Rust source file for the featur... |
| 88 | i-51 | 0.90 | 0.88 | 0.95 | 0.85 | 0.90 | S | Y | st | Phase 1 step (CRITICAL, 2 features): Verify every config field's ... |
| 89 | i-56 | 0.85 | 0.83 | 1.00 | 0.90 | 0.90 | S |  | sr | Phase 3 step: Read the Rust source file. |
| 90 | i-81 | 0.95 | 0.68 | 1.00 | 0.95 | 0.90 | S |  | sr | CLAUDE.md: Do NOT use Cow -- use String/&str. |
| 91 | i-06 | 0.95 | 0.90 | 0.95 | 0.85 | 0.91 | S | Y | st | After executing a feature's steps, run cargo nextest run --worksp... |
| 92 | i-07 | 0.90 | 0.85 | 1.00 | 0.90 | 0.91 | S | Y | sr | After the gate passes, edit the contract JSON to set the feature'... |
| 93 | i-09 | 0.95 | 0.80 | 0.95 | 0.95 | 0.91 | S |  | sr | Do not enter plan mode. |
| 94 | i-80 | 0.95 | 0.72 | 1.00 | 0.95 | 0.91 | S |  | sr | CLAUDE.md: Do NOT use Arc/Mutex -- single-threaded UI tree. |
| 95 | i-87 | 0.95 | 0.78 | 1.00 | 0.95 | 0.92 | S | Y | sr | CLAUDE.md: Do NOT use --no-verify on commits. |
| 96 | i-02 | 0.85 | 0.93 | 1.00 | 0.95 | 0.93 | S | Y | ce | Read CLAUDE.md at the workspace root at the start of the run. |
| 97 | i-05 | 0.95 | 0.90 | 0.95 | 0.90 | 0.93 | S | Y | sr | After executing a feature's steps, run cargo clippy --workspace -... |
| 98 | i-01 | 0.85 | 0.95 | 1.00 | 0.95 | 0.94 | S | Y | ce | Read the master contract at prompts/master-contract.json at the s... |

## 4. Critical Path

The critical path is the minimum set of instructions whose failure causes the entire prompt to produce wrong output. 
This includes: the main processing loop, gate conditions, fidelity rules, completeness requirements, and the commit protocol.

| ID | Composite | Category | Weak Axis | Key Role |
|----|-----------|----------|-----------|----------|
| i-24 | 0.73 | wounded | st=0.55 | Steps = requirements |
| i-23 | 0.74 | wounded | st=0.55 | Completeness: ALL methods |
| i-04 | 0.76 | survivor | st=0.60 | Step execution |
| i-43 | 0.76 | survivor | st=0.55 | Compare N methods (step) |
| i-15 | 0.78 | survivor | st=0.70 | Test layer requirement |
| i-30 | 0.80 | survivor | sr=0.55 | No skipping features |
| i-03 | 0.81 | survivor | st=0.70 | Main loop |
| i-21 | 0.82 | survivor | sr=0.72 | Phase ordering |
| i-18 | 0.84 | survivor | sr=0.80 | PIXEL fidelity |
| i-08 | 0.85 | survivor | sr=0.82 | Commit protocol |
| i-27 | 0.85 | survivor | sr=0.55 | Contract immutability |
| i-75 | 0.86 | survivor | sr=0.78 | Pixel fidelity (CLAUDE.md) |
| i-88 | 0.86 | survivor | sr=0.80 | Pre-commit hook |
| i-37 | 0.89 | survivor | sr=0.82 | Gate protocol (contract) |
| i-92 | 0.89 | survivor | sr=0.75 | Contract in commit |
| i-51 | 0.90 | survivor | st=0.85 | CRITICAL config defaults |
| i-06 | 0.91 | survivor | st=0.85 | Gate: tests |
| i-07 | 0.91 | survivor | sr=0.85 | State: mark passes |
| i-87 | 0.92 | survivor | sr=0.78 | No --no-verify |
| i-02 | 0.93 | survivor | ce=0.85 | Entry: load rules |
| i-05 | 0.93 | survivor | sr=0.90 | Gate: clippy |
| i-01 | 0.94 | survivor | ce=0.85 | Entry: load contract |

**Critical path risk:** 2/22 critical-path instructions are non-survivors. 
At-risk critical instructions: i-23(wounded), i-24(wounded)

## 5. Fallen (1)

### i-28 (composite 0.44)
> Do NOT modify existing tests (write new tests only).

CE=0.70 | SR=0.45 | CF=0.25 | ST=0.35 | weak: cf=0.25 | conflicts: t-01, t-02, t-19, t-20, t-21


## 6. Contested (1)

### i-61 (composite 0.57)
> Phase 4 step: Fix any defective test by strengthening its assertions.

CE=0.70 | SR=0.83 | CF=0.25 | ST=0.50 | weak: cf=0.25 | conflicts: t-02


## 7. Wounded (20)

### i-14 (composite 0.56)
> Phase 4 features: Review ALL existing tests (1500+) across every test directory for anti-patterns: test-shaped validation, missing side-effect checks, hardcoded-pass inputs, missing modifier setup. Strengthen defective tests. If a strengthened test reveals a real bug, fix it.

CE=0.60 | SR=0.75 | CF=0.50 | ST=0.40 | weak: st=0.40 | conflicts: t-01

### i-29 (composite 0.62)
> Do NOT modify unrelated production code.

CE=0.65 | SR=0.48 | CF=0.65 | ST=0.70 | weak: sr=0.48 | conflicts: t-03, t-04, t-05, t-24, t-28

### i-89 (composite 0.65)
> Log format: feature entry must include feature_id, description, MATCHes count, MISMATCHes count, MISSINGs count, fixes applied list, and tests added count.

CE=0.80 | SR=0.45 | CF=0.80 | ST=0.55 | weak: sr=0.45 | conflicts: t-18

### i-93 (composite 0.68)
> Phase 1 step: For MISMATCH fixes, apply minimal changes to the Rust code (not rewrite).

CE=0.55 | SR=0.78 | CF=0.85 | ST=0.55 | weak: ce=0.55 | conflicts: t-17

### i-16 (composite 0.69)
> When writing tests, follow the patterns in the corresponding test directory (unit tests in source file, pipeline in tests/pipeline/, golden in tests/golden/, integration in tests/integration/).

CE=0.50 | SR=0.60 | CF=0.90 | ST=0.75 | weak: ce=0.50 | conflicts: t-14

### i-60 (composite 0.69)
> Phase 4 step: For each test function, check 5 anti-patterns: (1) assertions verify SPECIFIC expected values not is_some()/is_ok(), (2) side effects are verified, (3) test would FAIL if implementation were subtly wrong, (4) modifier keys set correctly, (5) test exercises actual behavior not just compilation.

CE=0.75 | SR=0.85 | CF=0.75 | ST=0.40 | weak: st=0.40 | conflicts: t-09, t-26

### i-97 (composite 0.69)
> Phase 4 anti-pattern check: verify test would FAIL if implementation were subtly wrong.

CE=0.60 | SR=0.83 | CF=0.90 | ST=0.45 | weak: st=0.45 | conflicts: t-21

### i-34 (composite 0.70)
> After each feature, append to .workflow/widget-comparison/run-log.md with format: feature_id, description, MATCHes/MISMATCHes/MISSINGs counts, fixes applied, tests added.

CE=0.80 | SR=0.52 | CF=0.90 | ST=0.60 | weak: sr=0.52 | conflicts: t-07

### i-32 (composite 0.71)
> Do NOT treat the gate as proof of completeness. The gate is necessary but not sufficient.

CE=0.75 | SR=0.50 | CF=1.00 | ST=0.60 | weak: sr=0.50 | conflicts: none

### i-22 (composite 0.72)
> If a later feature requires changing code from a previously-completed feature, you MAY modify that code, but you MUST re-run cargo nextest run --workspace to confirm all previously-passing tests still pass. If any break, fix them before proceeding.

CE=0.85 | SR=0.65 | CF=0.75 | ST=0.65 | weak: sr=0.65 | conflicts: t-04, t-10, t-31

### i-96 (composite 0.72)
> Phase 4 anti-pattern check: verify test exercises actual behavior, not just compilation.

CE=0.65 | SR=0.83 | CF=0.90 | ST=0.50 | weak: st=0.50 | conflicts: t-20

### i-11 (composite 0.73)
> Phase 1 features: Compare each C++ method listed in the methods field. Report MATCH/MISMATCH/MISSING for each. Fix MISMATCHes. Write tests.

CE=0.70 | SR=0.82 | CF=0.85 | ST=0.55 | weak: st=0.55 | conflicts: t-06

### i-24 (composite 0.73) **[CRITICAL PATH]**
> Steps are requirements. If a step says 'compare these 20 methods' -- compare all 20, not the ones that seem important.

CE=0.90 | SR=0.72 | CF=0.75 | ST=0.55 | weak: st=0.55 | conflicts: t-09

### i-62 (composite 0.73)
> Phase 4 step: If a strengthened test now FAILS, that is a real bug. Fix the production code too.

CE=0.85 | SR=0.83 | CF=0.60 | ST=0.65 | weak: cf=0.60 | conflicts: t-03

### i-73 (composite 0.73)
> CLAUDE.md: Use expect('reason') unless invariant is obvious from context. Bare unwrap() fine in tests and same-line proofs.

CE=0.55 | SR=0.58 | CF=1.00 | ST=0.80 | weak: ce=0.55 | conflicts: none

### i-23 (composite 0.74) **[CRITICAL PATH]**
> The gate (clippy + tests) does NOT verify you compared every method. If the methods field lists 20 methods and you compared 5, the gate still passes. You must compare ALL listed methods.

CE=0.90 | SR=0.75 | CF=0.75 | ST=0.55 | weak: st=0.55 | conflicts: t-08

### i-49 (composite 0.74)
> Phase 1 step (RUST-ONLY variant): No C++ equivalent. Verify internal correctness: no dead code, no panicking paths, edge cases handled.

CE=0.55 | SR=0.85 | CF=0.95 | ST=0.60 | weak: ce=0.55 | conflicts: t-28

### i-58 (composite 0.74)
> Phase 3 step: Verify all public methods from the consolidated C++ classes have Rust equivalents.

CE=0.65 | SR=0.82 | CF=0.85 | ST=0.65 | weak: ce=0.65 | conflicts: none

### i-71 (composite 0.74)
> CLAUDE.md: One primary type per file. Private mod + public use re-exports in mod.rs.

CE=0.60 | SR=0.55 | CF=1.00 | ST=0.80 | weak: sr=0.55 | conflicts: none

### i-90 (composite 0.82)
> Log entry must be appended (not overwritten) to .workflow/widget-comparison/run-log.md.

CE=0.90 | SR=0.48 | CF=1.00 | ST=0.90 | weak: sr=0.48 | conflicts: none


## 8. Survivor (76)

### i-25 (composite 0.75)
> Tests must assert on specific values and behaviors, not just is_some() or is_ok().

CE=0.80 | SR=0.70 | CF=0.85 | ST=0.65 | weak: st=0.65 | conflicts: t-14

### i-77 (composite 0.75)
> CLAUDE.md (State fidelity): Fully idiomatic Rust for state logic, data structures, ownership, API surface. Preserve behavioral contracts (return values, side effects, ordering). Adapt syntax freely.

CE=0.60 | SR=0.70 | CF=0.95 | ST=0.75 | weak: ce=0.60 | conflicts: none

### i-78 (composite 0.75)
> CLAUDE.md: When in doubt, check if the function's output feeds a golden test. If yes, port the C++ formula exactly. If no, write idiomatic Rust.

CE=0.70 | SR=0.65 | CF=0.95 | ST=0.70 | weak: sr=0.65 | conflicts: none

### i-04 (composite 0.76) **[CRITICAL PATH]**
> For each feature, read its steps array and execute each step (read files, compare, report, fix, test).

CE=0.65 | SR=0.88 | CF=0.90 | ST=0.60 | weak: st=0.60 | conflicts: none

### i-12 (composite 0.76)
> Phase 2 features: For each gap method, determine IMPLEMENTED (different name), NOT_NEEDED (superseded), or NEEDS_IMPLEMENTATION. Implement what's needed, write tests.

CE=0.70 | SR=0.78 | CF=0.90 | ST=0.65 | weak: st=0.65 | conflicts: none

### i-43 (composite 0.76) **[CRITICAL PATH]**
> Phase 1 step (standard): Compare the N methods listed in the methods field.

CE=0.85 | SR=0.87 | CF=0.75 | ST=0.55 | weak: st=0.55 | conflicts: none

### i-52 (composite 0.76)
> Phase 2 step: Note that some C++ methods are generated by macros (EM_IMPL_OBJ_CLASS, EM_IMPLEMENT_*_REC). If a symbol looks like boilerplate (constructor/destructor/copy), check whether it was macro-generated. Macro boilerplate is typically NOT_NEEDED in Rust.

CE=0.55 | SR=0.83 | CF=0.95 | ST=0.70 | weak: ce=0.55 | conflicts: none

### i-70 (composite 0.76)
> CLAUDE.md: Construction: new() primary, builder with_*(self) -> Self for optional config.

CE=0.60 | SR=0.58 | CF=1.00 | ST=0.85 | weak: sr=0.58 | conflicts: none

### i-31 (composite 0.77)
> Do NOT flag idiomatic Rust as MISMATCH. Read the layer field.

CE=0.85 | SR=0.58 | CF=0.95 | ST=0.70 | weak: sr=0.58 | conflicts: none

### i-53 (composite 0.77)
> Phase 2 step: For each listed C++ method, determine: (a) is it actually needed in the Rust port? (b) if yes, is it already implemented under a different name? (c) if truly missing, is it feasible to implement?

CE=0.70 | SR=0.83 | CF=0.90 | ST=0.65 | weak: st=0.65 | conflicts: none

### i-55 (composite 0.77)
> Phase 2 step: For NEEDS_IMPLEMENTATION items, implement them and write a test.

CE=0.80 | SR=0.83 | CF=0.85 | ST=0.60 | weak: st=0.60 | conflicts: t-05

### i-98 (composite 0.77)
> Phase 1 step: The report for each method must use one of four categories: MATCH, MISMATCH, SUSPECT, or MISSING.

CE=0.85 | SR=0.82 | CF=0.75 | ST=0.65 | weak: st=0.65 | conflicts: t-25

### i-13 (composite 0.78)
> Phase 3 features: Verify all consolidated C++ methods are present in the Rust file that consolidates multiple C++ classes.

CE=0.65 | SR=0.75 | CF=0.95 | ST=0.75 | weak: ce=0.65 | conflicts: none

### i-15 (composite 0.78) **[CRITICAL PATH]**
> Phase 1 features have a required_test_layers field. A fix is not complete until ALL required layers have a test. Layers are: unit (#[cfg(test)] module), golden (tests/golden/), pipeline (tests/pipeline/ using PipelineTestHarness).

CE=0.90 | SR=0.72 | CF=0.80 | ST=0.70 | weak: st=0.70 | conflicts: t-13

### i-19 (composite 0.78)
> Features with layer=STATE: idiomatic Rust is correct. Only flag behavioral differences. Do NOT flag Rc vs pointers, enum vs int flags.

CE=0.85 | SR=0.78 | CF=0.80 | ST=0.70 | weak: st=0.70 | conflicts: t-12, t-27

### i-45 (composite 0.78)
> Phase 1 step (standard): For each method, verify constructors set the same default values, and methods produce the same side effects (callback firing, flag invalidation, repaint triggers).

CE=0.75 | SR=0.85 | CF=0.90 | ST=0.60 | weak: st=0.60 | conflicts: t-27

### i-46 (composite 0.78)
> Phase 1 step (standard): For each method, report MATCH, MISMATCH, SUSPECT, or MISSING.

CE=0.85 | SR=0.85 | CF=0.75 | ST=0.65 | weak: st=0.65 | conflicts: t-06, t-07

### i-94 (composite 0.78)
> Phase 2 step: For NEEDS_IMPLEMENTATION items, include a scope estimate.

CE=0.70 | SR=0.75 | CF=1.00 | ST=0.65 | weak: st=0.65 | conflicts: none

### i-95 (composite 0.78)
> Phase 4 anti-pattern check: verify modifier keys are set correctly for modifier-dependent tests.

CE=0.70 | SR=0.83 | CF=0.90 | ST=0.70 | weak: ce=0.70 | conflicts: t-19

### i-47 (composite 0.79)
> Phase 1 step (standard): For each MISMATCH, fix the Rust code minimally.

CE=0.75 | SR=0.85 | CF=0.90 | ST=0.65 | weak: st=0.65 | conflicts: t-25

### i-76 (composite 0.79)
> CLAUDE.md (Geometry fidelity): Same algorithm and operation order on golden-tested paths. Iterator::sum OK. Clamp/min/max must preserve C++ boundary values.

CE=0.80 | SR=0.72 | CF=0.90 | ST=0.75 | weak: sr=0.72 | conflicts: none

### i-30 (composite 0.80) **[CRITICAL PATH]**
> Do NOT skip a feature. If 0 MISMATCHes found, it still passes -- set passes to true.

CE=0.90 | SR=0.55 | CF=0.90 | ST=0.85 | weak: sr=0.55 | conflicts: t-11

### i-50 (composite 0.80)
> Phase 1 step (RUST-ONLY variant): Write a unit test for any untested edge case found.

CE=0.70 | SR=0.85 | CF=0.95 | ST=0.70 | weak: ce=0.70 | conflicts: none

### i-57 (composite 0.80)
> Phase 3 step: Verify the listed C++ symbols are covered in the Rust file.

CE=0.70 | SR=0.83 | CF=0.90 | ST=0.75 | weak: ce=0.70 | conflicts: none

### i-86 (composite 0.80)
> CLAUDE.md: Do NOT use assert! for recoverable errors.

CE=0.75 | SR=0.65 | CF=1.00 | ST=0.80 | weak: sr=0.65 | conflicts: none

### i-03 (composite 0.81) **[CRITICAL PATH]**
> Process features in order: read next feature where passes == false, execute its steps, then move to next.

CE=0.80 | SR=0.90 | CF=0.85 | ST=0.70 | weak: st=0.70 | conflicts: t-26

### i-26 (composite 0.81)
> If you fix a MISMATCH, write a NEW test. Existing tests didn't catch the bug.

CE=0.90 | SR=0.68 | CF=0.85 | ST=0.80 | weak: sr=0.68 | conflicts: t-13

### i-39 (composite 0.81)
> Contract rule: Check each feature's layer field. PIXEL = exact formulas, GEOMETRY = same algorithm, STATE = idiomatic Rust OK.

CE=0.85 | SR=0.80 | CF=0.85 | ST=0.75 | weak: st=0.75 | conflicts: t-22

### i-68 (composite 0.81)
> CLAUDE.md: Per-module Result with custom error enums (Display + Error). assert! only for logic-error invariants.

CE=0.75 | SR=0.68 | CF=1.00 | ST=0.80 | weak: sr=0.68 | conflicts: none

### i-21 (composite 0.82) **[CRITICAL PATH]**
> Phases execute in order. Before starting a phase, verify its dependency phase is fully complete (every feature has passes: true). Phase 2 depends on phase 1. Phase 3 depends on phase 2. Phase 4 depends on phase 3.

CE=0.90 | SR=0.72 | CF=0.85 | ST=0.80 | weak: sr=0.72 | conflicts: t-11, t-31

### i-33 (composite 0.82)
> Run the completion script at the end to report total progress and list any PENDING features.

CE=0.80 | SR=0.60 | CF=1.00 | ST=0.90 | weak: sr=0.60 | conflicts: none

### i-54 (composite 0.82)
> Phase 2 step: For each method, mark as IMPLEMENTED (found under different name), NOT_NEEDED (superseded or not applicable), or NEEDS_IMPLEMENTATION (with scope estimate).

CE=0.80 | SR=0.83 | CF=0.90 | ST=0.75 | weak: st=0.75 | conflicts: t-18

### i-72 (composite 0.82)
> CLAUDE.md: pub(crate) default visibility. pub only for library API consumed by sosumi-7.

CE=0.85 | SR=0.62 | CF=0.95 | ST=0.85 | weak: sr=0.62 | conflicts: none

### i-59 (composite 0.83)
> Phase 4 step: List all #[test] functions in the specified test directory.

CE=0.85 | SR=0.83 | CF=0.90 | ST=0.75 | weak: st=0.75 | conflicts: none

### i-74 (composite 0.83)
> CLAUDE.md: Fix warnings (remove dead code, prefix _, apply clippy fix). Suppress only genuine false positives with a comment.

CE=0.80 | SR=0.72 | CF=0.95 | ST=0.85 | weak: sr=0.72 | conflicts: t-17, t-24

### i-18 (composite 0.84) **[CRITICAL PATH]**
> Features with layer=PIXEL require exact C++ formulas. Integer arithmetic must match. Use (x*257+0x8073)>>16 not f64.

CE=0.90 | SR=0.80 | CF=0.80 | ST=0.85 | weak: sr=0.80 | conflicts: t-12, t-22

### i-20 (composite 0.84)
> Features without a layer field default to STATE fidelity rules.

CE=0.95 | SR=0.55 | CF=0.95 | ST=0.90 | weak: sr=0.55 | conflicts: none

### i-38 (composite 0.84)
> Contract rule: Commit after each feature. Include the contract JSON.

CE=0.80 | SR=0.78 | CF=0.95 | ST=0.85 | weak: sr=0.78 | conflicts: none

### i-44 (composite 0.84)
> Phase 1 step (standard): Also read the C++ .cpp implementation file -- check for static helpers, default values in constructors, and side effects (signals, invalidation, repaint calls) that the header doesn't show.

CE=0.80 | SR=0.87 | CF=0.95 | ST=0.75 | weak: st=0.75 | conflicts: t-29

### i-48 (composite 0.84)
> Phase 1 step (standard): For each fix, write a test verifying the corrected behavior.

CE=0.85 | SR=0.85 | CF=0.90 | ST=0.75 | weak: st=0.75 | conflicts: none

### i-63 (composite 0.84)
> Phase 4 (inline test variant): grep -rn '#[cfg(test)]' src/ to find all inline test modules.

CE=0.85 | SR=0.82 | CF=0.90 | ST=0.80 | weak: st=0.80 | conflicts: none

### i-69 (composite 0.84)
> CLAUDE.md: Import order: std then external then crate::. Explicit names. use super::* only in #[cfg(test)].

CE=0.85 | SR=0.62 | CF=1.00 | ST=0.90 | weak: sr=0.62 | conflicts: none

### i-08 (composite 0.85) **[CRITICAL PATH]**
> After setting passes to true, run git add prompts/master-contract.json <changed files> and commit with message 'audit: <feature_id> <summary>'.

CE=0.85 | SR=0.82 | CF=0.90 | ST=0.85 | weak: sr=0.82 | conflicts: t-10

### i-27 (composite 0.85) **[CRITICAL PATH]**
> Do NOT modify the contract's steps, description, or methods fields.

CE=0.95 | SR=0.55 | CF=1.00 | ST=0.90 | weak: sr=0.55 | conflicts: t-30

### i-91 (composite 0.85)
> Commit message format: 'audit: <feature_id> <summary>'.

CE=0.85 | SR=0.72 | CF=1.00 | ST=0.85 | weak: sr=0.72 | conflicts: none

### i-17 (composite 0.86)
> Phase 1 steps require reading the C++ .cpp file (not just the header) to catch implementation details: static helpers, constructor defaults, and side effects (signals, invalidation, repaint) that headers don't show.

CE=0.85 | SR=0.78 | CF=1.00 | ST=0.80 | weak: sr=0.78 | conflicts: none

### i-64 (composite 0.86)
> CLAUDE.md: Use f64 for logical coordinates, i32 for pixel coordinates, u32 for image dimensions, u8 for color channels.

CE=0.90 | SR=0.70 | CF=0.95 | ST=0.90 | weak: sr=0.70 | conflicts: t-23

### i-75 (composite 0.86) **[CRITICAL PATH]**
> CLAUDE.md (Pixel fidelity): Reproduce C++ integer formulas exactly in blend/coverage/interpolation/sampling. Use (x*257+0x8073)>>16 not f64 division. Wrap in newtypes. Named constants with derivations. No f64 approximations in compositing pipeline.

CE=0.90 | SR=0.78 | CF=0.90 | ST=0.85 | weak: sr=0.78 | conflicts: t-23

### i-82 (composite 0.86)
> CLAUDE.md: Do NOT use glob imports (use foo::*) except use super::* in tests.

CE=0.90 | SR=0.65 | CF=1.00 | ST=0.90 | weak: sr=0.65 | conflicts: none

### i-84 (composite 0.86)
> CLAUDE.md: Do NOT use f64 in blend/coverage/interpolation paths -- use C++ integer arithmetic.

CE=0.90 | SR=0.75 | CF=0.95 | ST=0.85 | weak: sr=0.75 | conflicts: none

### i-88 (composite 0.86) **[CRITICAL PATH]**
> CLAUDE.md: Pre-commit hook runs cargo fmt (auto-applied) then clippy -D warnings then cargo-nextest ntr. Do not skip with --no-verify. If a commit fails, fix the cause and retry.

CE=0.90 | SR=0.80 | CF=0.85 | ST=0.90 | weak: sr=0.80 | conflicts: t-15, t-16

### i-36 (composite 0.87)
> Contract rule: Agent MAY set passes to true. Agent MAY NOT delete features, modify steps, or mark as not applicable.

CE=0.90 | SR=0.72 | CF=0.95 | ST=0.90 | weak: sr=0.72 | conflicts: t-30

### i-67 (composite 0.87)
> CLAUDE.md: Use String owned, &str params. No Cow.

CE=0.90 | SR=0.68 | CF=1.00 | ST=0.90 | weak: sr=0.68 | conflicts: none

### i-79 (composite 0.87)
> CLAUDE.md: Do NOT use #[allow(...)] or #[expect(...)]. Fix the warning instead. UNLESS warning is for too many arguments (which is allowed).

CE=0.90 | SR=0.72 | CF=0.95 | ST=0.90 | weak: sr=0.72 | conflicts: none

### i-83 (composite 0.87)
> CLAUDE.md: Do NOT truncate color math to u8 mid-calculation.

CE=0.90 | SR=0.72 | CF=1.00 | ST=0.85 | weak: sr=0.72 | conflicts: none

### i-85 (composite 0.87)
> CLAUDE.md: Do NOT use Rayon/parallel iteration on golden-tested code paths.

CE=0.90 | SR=0.62 | CF=1.00 | ST=0.95 | weak: sr=0.62 | conflicts: none

### i-65 (composite 0.88)
> CLAUDE.md: Use Color (packed u32 RGBA) for storage. Intermediate blend math in i32 or wider.

CE=0.90 | SR=0.70 | CF=1.00 | ST=0.90 | weak: sr=0.70 | conflicts: none

### i-66 (composite 0.88)
> CLAUDE.md: Use Rc/RefCell for shared state, Weak for parent refs. No Arc/Mutex (single-threaded UI tree).

CE=0.90 | SR=0.72 | CF=1.00 | ST=0.90 | weak: sr=0.72 | conflicts: none

### i-10 (composite 0.89)
> Run the bootstrap script at the start to see current progress (done/total features passing, test results).

CE=0.75 | SR=0.85 | CF=1.00 | ST=0.95 | weak: ce=0.75 | conflicts: none

### i-35 (composite 0.89)
> Start with the first feature in phase-1 where passes is false.

CE=0.90 | SR=0.70 | CF=1.00 | ST=0.95 | weak: sr=0.70 | conflicts: none

### i-37 (composite 0.89) **[CRITICAL PATH]**
> Contract rule: passes may only be set to true after clippy clean + all tests pass.

CE=0.90 | SR=0.82 | CF=0.95 | ST=0.90 | weak: sr=0.82 | conflicts: none

### i-92 (composite 0.89) **[CRITICAL PATH]**
> Include prompts/master-contract.json in the git add for each commit.

CE=0.90 | SR=0.75 | CF=1.00 | ST=0.90 | weak: sr=0.75 | conflicts: none

### i-40 (composite 0.90)
> Phase 1 step (standard): Read the C++ header file for the feature.

CE=0.85 | SR=0.88 | CF=1.00 | ST=0.85 | weak: ce=0.85 | conflicts: none

### i-41 (composite 0.90)
> Phase 1 step (standard): Read the C++ .cpp implementation file for the feature.

CE=0.85 | SR=0.88 | CF=1.00 | ST=0.85 | weak: ce=0.85 | conflicts: t-29

### i-42 (composite 0.90)
> Phase 1 step (standard): Read the Rust source file for the feature.

CE=0.85 | SR=0.88 | CF=1.00 | ST=0.85 | weak: ce=0.85 | conflicts: none

### i-51 (composite 0.90) **[CRITICAL PATH]**
> Phase 1 step (CRITICAL, 2 features): Verify every config field's default value matches C++ emCoreConfig constructor defaults exactly -- these control zoom speed, scroll speed, magnetism radius, thread count, and render quality.

CE=0.90 | SR=0.88 | CF=0.95 | ST=0.85 | weak: st=0.85 | conflicts: none

### i-56 (composite 0.90)
> Phase 3 step: Read the Rust source file.

CE=0.85 | SR=0.83 | CF=1.00 | ST=0.90 | weak: sr=0.83 | conflicts: none

### i-81 (composite 0.90)
> CLAUDE.md: Do NOT use Cow -- use String/&str.

CE=0.95 | SR=0.68 | CF=1.00 | ST=0.95 | weak: sr=0.68 | conflicts: none

### i-06 (composite 0.91) **[CRITICAL PATH]**
> After executing a feature's steps, run cargo nextest run --workspace and ensure zero failures.

CE=0.95 | SR=0.90 | CF=0.95 | ST=0.85 | weak: st=0.85 | conflicts: t-16

### i-07 (composite 0.91) **[CRITICAL PATH]**
> After the gate passes, edit the contract JSON to set the feature's passes field to true.

CE=0.90 | SR=0.85 | CF=1.00 | ST=0.90 | weak: sr=0.85 | conflicts: none

### i-09 (composite 0.91)
> Do not enter plan mode.

CE=0.95 | SR=0.80 | CF=0.95 | ST=0.95 | weak: sr=0.80 | conflicts: t-08

### i-80 (composite 0.91)
> CLAUDE.md: Do NOT use Arc/Mutex -- single-threaded UI tree.

CE=0.95 | SR=0.72 | CF=1.00 | ST=0.95 | weak: sr=0.72 | conflicts: none

### i-87 (composite 0.92) **[CRITICAL PATH]**
> CLAUDE.md: Do NOT use --no-verify on commits.

CE=0.95 | SR=0.78 | CF=1.00 | ST=0.95 | weak: sr=0.78 | conflicts: none

### i-02 (composite 0.93) **[CRITICAL PATH]**
> Read CLAUDE.md at the workspace root at the start of the run.

CE=0.85 | SR=0.93 | CF=1.00 | ST=0.95 | weak: ce=0.85 | conflicts: none

### i-05 (composite 0.93) **[CRITICAL PATH]**
> After executing a feature's steps, run cargo clippy --workspace -- -D warnings and ensure zero warnings.

CE=0.95 | SR=0.90 | CF=0.95 | ST=0.90 | weak: sr=0.90 | conflicts: t-15

### i-01 (composite 0.94) **[CRITICAL PATH]**
> Read the master contract at prompts/master-contract.json at the start of the run.

CE=0.85 | SR=0.95 | CF=1.00 | ST=0.95 | weak: ce=0.85 | conflicts: none


## 9. Conflict Map (sorted by severity)

| Tension | Severity | Type | Instructions | Description |
|---------|----------|------|--------------|-------------|
| t-02 | 0.95 | contradiction | i-28, i-61 | i-28 says 'Do NOT modify existing tests' and i-61 says 'Fix any defective test by strengthening its assertions.' These c... |
| t-01 | 0.90 | contradiction | i-28, i-14 | i-28 says 'Do NOT modify existing tests (write new tests only)' with no qualification. i-14 (Phase 4) says 'Strengthen d... |
| t-19 | 0.80 | mutual_exclusion | i-28, i-95 | i-28 says do NOT modify existing tests. i-95 (Phase 4) requires verifying modifier keys are set correctly and fixing the... |
| t-20 | 0.80 | mutual_exclusion | i-28, i-96 | i-28 says do NOT modify existing tests. i-96 (Phase 4) requires verifying tests exercise actual behavior. Fixing a test ... |
| t-21 | 0.80 | mutual_exclusion | i-28, i-97 | i-28 says do NOT modify existing tests. i-97 (Phase 4) requires verifying tests would fail if implementation were subtly... |
| t-03 | 0.70 | undermining | i-29, i-62 | i-29 says 'Do NOT modify unrelated production code.' i-62 says 'If a strengthened test now FAILS, that is a real bug. Fi... |
| t-04 | 0.55 | undermining | i-29, i-22 | i-29 says 'Do NOT modify unrelated production code.' i-22 permits modifying code from previously-completed features. If ... |
| t-09 | 0.55 | latent_tension | i-24, i-60 | i-24 says steps are requirements (compare ALL methods). i-60 requires checking 5 anti-patterns on every test function in... |
| t-06 | 0.50 | undermining | i-11, i-46 | i-11 (prompt .md Phase 1 summary) uses 3 categories: MATCH/MISMATCH/MISSING. i-46 (contract step) adds a 4th: SUSPECT. T... |
| t-11 | 0.50 | latent_tension | i-21, i-30 | i-21 requires completing all features in a phase before starting the next. i-30 says do NOT skip any feature. If one Pha... |
| t-26 | 0.50 | latent_tension | i-03, i-60 | i-03 requires processing features sequentially. i-60 requires checking 5 anti-patterns on every test function in Phase 4... |
| t-05 | 0.45 | latent_tension | i-29, i-55 | i-29 prohibits modifying unrelated production code. i-55 requires implementing NEEDS_IMPLEMENTATION items in Phase 2, wh... |
| t-07 | 0.45 | undermining | i-46, i-34 | i-46 uses 4 categories (MATCH/MISMATCH/SUSPECT/MISSING) but i-34's log format only has count fields for MATCHes, MISMATC... |
| t-10 | 0.45 | latent_tension | i-08, i-22 | i-08 requires one commit per feature with specific format. i-22 allows modifying previously-completed feature's code. Wh... |
| t-25 | 0.45 | undermining | i-98, i-47 | i-98 introduces SUSPECT as a reporting category, but i-47 only defines an action for MISMATCH (fix the Rust code minimal... |
| t-08 | 0.40 | latent_tension | i-23, i-09 | i-23 requires comparing ALL listed methods (some features have 40+). i-09 forbids plan mode. Systematically comparing 40... |
| t-14 | 0.40 | latent_tension | i-25, i-16 | i-25 requires specific value assertions (not is_some/is_ok). i-16 says follow patterns in existing test directories. If ... |
| t-18 | 0.40 | undermining | i-54, i-89 | Phase 2 classification uses IMPLEMENTED/NOT_NEEDED/NEEDS_IMPLEMENTATION categories (i-54). The log format (i-89) require... |
| t-31 | 0.40 | latent_tension | i-22, i-21 | i-21 requires completing all features in a phase before proceeding. i-22 allows modifying previously-completed feature c... |
| t-12 | 0.35 | latent_tension | i-18, i-19 | PIXEL (exact C++ formulas) and STATE (idiomatic Rust OK) fidelity rules are clear in isolation but create tension at bou... |
| t-13 | 0.35 | latent_tension | i-15, i-26 | i-15 requires tests in all required layers (potentially unit+golden+pipeline). i-26 requires NEW tests for each fix. For... |
| t-17 | 0.35 | latent_tension | i-93, i-74 | i-93 says apply minimal changes for MISMATCH fixes. i-74 says fix warnings (remove dead code, etc.). A minimal fix may i... |
| t-24 | 0.35 | latent_tension | i-29, i-74 | i-29 says do NOT modify unrelated production code. i-74 says fix warnings. If clippy flags a warning in an unrelated fil... |
| t-27 | 0.35 | latent_tension | i-45, i-19 | i-45 requires verifying methods produce 'the same side effects' (callback firing, flag invalidation, repaint triggers). ... |
| t-15 | 0.30 | latent_tension | i-05, i-88 | i-05 requires running clippy as a gate before committing. i-88 says the pre-commit hook also runs clippy. This means cli... |
| t-16 | 0.30 | latent_tension | i-06, i-88 | Same as t-15 but for tests: i-06 runs tests as gate, i-88's pre-commit hook runs tests again. Double test execution per ... |
| t-22 | 0.30 | latent_tension | i-39, i-18 | i-39 (contract) defines three fidelity layers: PIXEL, GEOMETRY, STATE. i-18 (prompt .md) only defines PIXEL and STATE. T... |
| t-23 | 0.30 | latent_tension | i-64, i-75 | i-64 says use f64 for logical coordinates. i-75 says no f64 approximations in the compositing pipeline. At the boundary ... |
| t-28 | 0.30 | latent_tension | i-49, i-29 | i-49 (RUST-ONLY verification) checks for 'no panicking paths, edge cases handled.' Fixing a panicking path or unhandled ... |
| t-29 | 0.15 | latent_tension | i-41, i-44 | i-41 (step 2: read .cpp file) and i-44 (step 5: also read .cpp file, check for hidden details) are redundant. The agent ... |
| t-30 | 0.10 | latent_tension | i-27, i-36 | i-27 and i-36 both express contract immutability rules. Pure redundancy with no conflict, but duplicate instructions con... |

### High-severity conflicts (>= 0.70)

**t-02** (severity 0.95, contradiction): i-28, i-61
> i-28 says 'Do NOT modify existing tests' and i-61 says 'Fix any defective test by strengthening its assertions.' These cannot both be followed simultaneously. This is the sharpest conflict in the prompt.

**t-01** (severity 0.90, contradiction): i-28, i-14
> i-28 says 'Do NOT modify existing tests (write new tests only)' with no qualification. i-14 (Phase 4) says 'Strengthen defective tests' which requires modifying existing tests. The prompt text has no explicit exception clause for Phase 4. An agent must infer that Phase 4 overrides i-28, but the contradiction is textually direct.

**t-19** (severity 0.80, mutual_exclusion): i-28, i-95
> i-28 says do NOT modify existing tests. i-95 (Phase 4) requires verifying modifier keys are set correctly and fixing them if wrong. Fixing incorrect modifier setup in an existing test requires modifying it, which i-28 forbids.

**t-20** (severity 0.80, mutual_exclusion): i-28, i-96
> i-28 says do NOT modify existing tests. i-96 (Phase 4) requires verifying tests exercise actual behavior. Fixing a test that only tests compilation requires modifying it.

**t-21** (severity 0.80, mutual_exclusion): i-28, i-97
> i-28 says do NOT modify existing tests. i-97 (Phase 4) requires verifying tests would fail if implementation were subtly wrong. Strengthening a test that wouldn't fail requires modifying it.

**t-03** (severity 0.70, undermining): i-29, i-62
> i-29 says 'Do NOT modify unrelated production code.' i-62 says 'If a strengthened test now FAILS, that is a real bug. Fix the production code too.' A bug found by strengthening a test in Phase 4 may be in production code that is 'unrelated' to the Phase 4 feature (which is a test directory, not a production file). The definition of 'related' is ambiguous here.
