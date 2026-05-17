# Testing

This document describes how kiri's automated tests are organised and how to
run them locally. (The `docs/` directory is gitignored as a scratch space,
so we keep contributor-facing docs at the repo root.)

## Test layout

| Project          | Where it lives                                          | Runner                              | When it runs              |
| ---------------- | ------------------------------------------------------- | ----------------------------------- | ------------------------- |
| `unit`           | `src/**/*.test.{js,ts}` (excluding `*.browser.test.*`)  | Vitest in `jsdom`                   | `npm test`                |
| `browser`        | `src/**/*.browser.test.{js,ts}`                         | Vitest in real Chromium (Playwright) | `npm run test:browser`    |
| Rust integration | `src-tauri/tests/*.rs`                                  | `cargo test`                        | `npm run test:rust`       |

There is **no end-to-end (WebdriverIO / tauri-driver) suite** at the moment.
The `npm run test:e2e` script that was once mentioned has been removed; the
only WebdriverIO mention you may still find in `package-lock.json` is a
transitive dependency of `@vitest/browser` and is not invoked by any npm
script.

If you want to add an E2E suite (for example to drive a packaged binary
through `tauri-driver`), see the "Adding an E2E suite" section below.

## Running the tests

```sh
# Unit (default, jsdom): fast, no native deps required
npm test
npm run test:watch
npm run test:coverage

# Browser (Chromium via Playwright): requires `npx playwright install chromium`
npm run test:browser
npm run test:browser:watch
npm run test:browser:ui   # headed mode for debugging

# Rust integration tests for the Tauri backend
npm run test:rust
npm run test:rust:watch        # requires `cargo install cargo-watch`
npm run test:rust:coverage     # requires Homebrew llvm + cargo-llvm-cov

# Run unit + browser together
npm run test:all
```

## Writing a unit test

Place the file next to the module it covers and use the `.test.ts` suffix.
The `unit` project uses `jsdom` plus the global setup file at
`src/test/setup.ts`. Anything that imports `@tauri-apps/api/*` must be
mocked - see `src/test/mocks/` for the existing mocks.

```ts
import { describe, expect, it } from 'vitest';
import { formatBytes } from './formatBytes';

describe('formatBytes', () => {
  it('renders KB', () => {
    expect(formatBytes(1024)).toBe('1.0 KB');
  });
});
```

## Writing a browser test

Browser tests run real DOM and CSS in Chromium. They are slower but
required for components that depend on layout, focus, IME, or the
`@xterm/xterm` renderer.

Name the file `Foo.browser.test.ts` (the `.browser` suffix is what routes
it to the `browser` project). The setup file lives at
`src/test/browser-setup.ts`.

## Writing a Rust integration test

Add a new file under `src-tauri/tests/`. Each top-level file becomes its
own test binary, so prefer per-feature files
(`fs_integration.rs`, `terminal_integration.rs`, ...) rather than one
mega-suite. Use `tempfile::TempDir` for filesystem fixtures so tests stay
hermetic.

## Adding an E2E suite

The team has chosen not to ship a WebdriverIO suite yet because the
Tauri WebDriver story still requires per-platform binaries
(`tauri-driver` on Linux/Windows; Safari WebDriver on macOS). If you
want to revive it:

1. Add `@wdio/cli`, `@wdio/local-runner`, `@wdio/mocha-framework`,
   `@wdio/spec-reporter`, and `tauri-driver` as devDependencies.
2. Generate a `wdio.conf.ts` that points at the packaged binary
   (`src-tauri/target/release/bundle/...`).
3. Add a `test:e2e` npm script that runs `wdio run wdio.conf.ts` after a
   `npm run tauri build` step in CI.
4. Document the per-platform driver setup in this file.

Until then, browser-level coverage should land in the `browser` Vitest
project.
