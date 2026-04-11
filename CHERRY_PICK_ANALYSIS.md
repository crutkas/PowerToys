# Cherry-Pick Analysis: rust-awake-module → main

*2026-04-11*

## Honest Assessment

The vtable mismatch, ShortcutGuide startup, FZ shift-drag, and keyboard snap 
direction bugs were ALL Rust implementation issues, NOT C++ bugs. The C++ 
PowerToys codebase does not have these problems.

## What IS cherry-pickable (zero-risk, real value)

### Spelling Fixes (3 commits)
- `1771367` — 25 words added to code.txt allowlist  
- `d8834db` — 17 words added to expect.txt
- `fc2da47` — .rustc_info.json excluded from spell check

### Test Coverage Insights (new C++ test ideas)
The 906 Rust tests document behaviors the C++ codebase doesn't test today.
These could be written as new C++ MSTest tests:

| Area | Rust Tests | C++ Tests Today | Gap |
|------|-----------|----------------|-----|
| FancyZones zone math | 130 | 42 | +88 |
| FancyZones engine | 63 | 0 | +63 |
| Workspaces logic | 83 | 51 | +32 |
| CursorWrap topology | 60 | 0 | +60 |
| Highlighter lifecycle | 46 | 0 | +46 |
| Crosshairs layout | 57 | 0 | +57 |
| MeasureTool edge detection | 53 | 0 | +53 |
| Runner hotkey conflicts | 41 | 0 | +41 |

**Total potential new C++ tests: ~440**

These are test CASES not test CODE — the Rust implementations can't be 
cherry-picked, but the test scenarios can guide writing C++ MSTest equivalents.

### Documentation (if useful independent of Rust)
- Architecture analysis documents (FancyZones comparison, CropAndLock plan)
- Common libs analysis identifying high-consumer shared libraries

## What is NOT cherry-pickable
- All Rust source code
- Vtable fixes (Rust-specific adapter.cpp files, not C++ originals)
- Bug fixes (all were Rust implementation gaps, not C++ bugs)
- Build system changes (Cargo.toml, build.rs, Build-RustModules.ps1)

## Recommendation
The highest-value cherry-pick is:
1. **Spelling fixes** (3 commits, zero risk)
2. **Write C++ tests** based on Rust test scenarios (new PR, not cherry-pick)

The Rust work's biggest contribution to main would be a **new C++ test PR** 
that ports the 440 test cases we identified as gaps back to MSTest format.
That requires writing new C++ code, not cherry-picking.