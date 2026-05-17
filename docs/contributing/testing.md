# Testing policy

This is the long form of the test-policy section in
[`CONTRIBUTING.md`](../../CONTRIBUTING.md). The short summary there is enough
for most PRs; come here when you are introducing a new module or wondering
where a test should live.

## Categories

We use four categories. They differ in cost (how long they take and how
flaky they tend to be) and scope (how much of the system they exercise).

### Unit

- Runner: `npm run test` (Vitest, node environment) or `cargo test` for Rust.
- Scope: a single module. No DOM, no IPC, no real filesystem.
- Use for: pure logic, state reducers, parsing, formatters, ring buffer
  semantics, etc.
- Cost target: < 50 ms each, < 5 s for the whole suite.

### Browser

- Runner: `npm run test:browser` (Vitest browser-mode + Playwright Chromium).
- Scope: a Svelte component or small composition of components rendered into
  a real DOM.
- Use for: focus management, keyboard navigation, layout-dependent behaviour,
  CSS-driven state.
- Cost target: < 1 s per test. If a test crosses several components, it is
  probably an integration test and belongs in the host.

### Integration

- Runner: `cargo test` (mostly in `src-tauri/`).
- Scope: multiple modules across a boundary inside `src-tauri/` — for example
  the ring buffer + dispatcher + frontend bridge.
- Use for: verifying that the seams between pure modules behave when wired
  together. No real Tauri runtime, no real Cloudflare Tunnel.

### End-to-end

- Runner: manual or `npm run build:app` + smoke script.
- Scope: the whole bundled app, including the `kiri-cli` binary and the
  Tauri runtime.
- Use for: release verification, things that can only be observed when the
  full UDS round-trip is alive.
- Cost: high. Reach for it sparingly — once per release, or before merging
  changes that touch the CLI socket protocol.

## Coverage

- We aim for ~100% line coverage on pure modules (anything that would qualify
  as a "unit" test target).
- We do **not** chase 100% on modules that wrap I/O — coverage there mostly
  measures how thoroughly you have mocked the world, which is not useful.
- A line that is hard to cover is usually a sign of branchy code; consider
  extracting the branch into a pure helper that you can cover directly.

`npm run test:coverage` and `npm run test:rust:coverage` produce coverage
reports. CI does not fail on coverage thresholds today; treat the reports as
a feedback tool when you are writing tests.

## Dead-code policy

If a function exists only to satisfy a test, delete one of the two. The test
suite must reflect real product behaviour, not historical scaffolding.

If a flag is always off in production, remove the flag and the branches it
gates. The test for the always-off branch goes with them. This keeps coverage
honest.

## Determinism

- No real network calls. Mock the boundary.
- No `setTimeout`/`sleep` to "wait for" something — use Vitest's
  `vi.useFakeTimers()` or the framework's await primitives.
- No clock-dependent assertions. Inject a clock if your code reads `Date.now`.
- Random inputs must be seeded.

A flaky test is a broken test. Quarantine it (`test.skip` with a comment) or
fix it; never re-run CI hoping for green.

## Snapshots

- Useful for a small, stable shape (a configuration object, a tiny DOM
  fragment).
- Harmful as a substitute for an assertion you didn't bother to write.
- If a snapshot exceeds ~30 lines, split it or replace it with targeted
  assertions.

## Picking a category quickly

```
Pure function, no I/O?            → unit
Svelte component, needs DOM?      → browser
Crosses module boundaries in
    src-tauri without runtime?    → integration
Needs the bundled app or CLI?     → end-to-end
```

When in doubt, write a unit test first. You can promote it later if you
discover the bug actually lives at a higher level.
