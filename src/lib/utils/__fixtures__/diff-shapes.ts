/**
 * Sample diff payloads in the shape the Rust backend (`git_diff.rs`)
 * emits them. The backend uses `git2`'s patch printer with `+ `, `- `,
 * `  ` prefixes for body lines and an empty prefix for everything else
 * (file header, hunk header, binary marker, "no newline" markers...).
 *
 * These fixtures intentionally exercise the shapes git produces but the
 * UI rarely sees explicitly: rename, copy, binary, mode change,
 * submodule pointer bump, and "no newline at EOF". They live as one
 * exported list so tests can `it.each(...)` over them and any future
 * shape can be added without touching test wiring.
 */

export interface DiffShapeFixture {
  /** Human-friendly name for `it.each` output. */
  readonly name: string;
  /** Raw diff text as produced by `get_file_diff_internal`. */
  readonly diff: string;
  /** Lines we expect `parseDiff` to mark as additions. */
  readonly expectedAdded: readonly number[];
  /** Lines we expect `parseDiff` to mark as modifications. */
  readonly expectedModified: readonly number[];
  /** Line positions we expect `parseDiff` to mark as deletions. */
  readonly expectedDeletedAt: readonly number[];
}

export const DIFF_SHAPE_FIXTURES: readonly DiffShapeFixture[] = [
  {
    name: 'rename without content change',
    diff: `diff --git a/old/path.ts b/new/path.ts
similarity index 100%
rename from old/path.ts
rename to new/path.ts
`,
    expectedAdded: [],
    expectedModified: [],
    expectedDeletedAt: [],
  },

  {
    name: 'rename with small content change',
    diff: `diff --git a/old/name.ts b/new/name.ts
similarity index 92%
rename from old/name.ts
rename to new/name.ts
@@ -1,3 +1,3 @@
  unchanged
- old line
+ new line
  unchanged
`,
    expectedAdded: [],
    expectedModified: [2],
    expectedDeletedAt: [],
  },

  {
    name: 'copy without content change',
    diff: `diff --git a/src/a.ts b/src/b.ts
similarity index 100%
copy from src/a.ts
copy to src/b.ts
`,
    expectedAdded: [],
    expectedModified: [],
    expectedDeletedAt: [],
  },

  {
    name: 'binary file added',
    diff: `diff --git a/logo.png b/logo.png
new file mode 100644
Binary files /dev/null and b/logo.png differ
`,
    expectedAdded: [],
    expectedModified: [],
    expectedDeletedAt: [],
  },

  {
    name: 'binary file modified',
    diff: `diff --git a/logo.png b/logo.png
index abc..def 100644
Binary files a/logo.png and b/logo.png differ
`,
    expectedAdded: [],
    expectedModified: [],
    expectedDeletedAt: [],
  },

  {
    name: 'mode change only (no content delta)',
    diff: `diff --git a/script.sh b/script.sh
old mode 100644
new mode 100755
`,
    expectedAdded: [],
    expectedModified: [],
    expectedDeletedAt: [],
  },

  {
    name: 'submodule pointer bump',
    diff: `diff --git a/vendor/lib b/vendor/lib
index 1111111..2222222 160000
--- a/vendor/lib
+++ b/vendor/lib
@@ -1 +1 @@
- Subproject commit 1111111
+ Subproject commit 2222222
`,
    expectedAdded: [],
    expectedModified: [1],
    expectedDeletedAt: [],
  },

  {
    name: 'no newline at EOF marker',
    diff: `@@ -1,2 +1,2 @@
  before
- old last line
\\ No newline at end of file
+ new last line
\\ No newline at end of file
`,
    expectedAdded: [],
    expectedModified: [2],
    expectedDeletedAt: [],
  },

  {
    name: 'new file with body',
    diff: `diff --git a/notes.md b/notes.md
new file mode 100644
index 0000000..abc1234
--- /dev/null
+++ b/notes.md
@@ -0,0 +1,2 @@
+ first
+ second
`,
    expectedAdded: [1, 2],
    expectedModified: [],
    expectedDeletedAt: [],
  },

  {
    name: 'deleted file',
    diff: `diff --git a/notes.md b/notes.md
deleted file mode 100644
index abc1234..0000000
--- a/notes.md
+++ /dev/null
@@ -1,2 +0,0 @@
- first
- second
`,
    expectedAdded: [],
    expectedModified: [],
    // Hunk header is `@@ -1,2 +0,0 @@`, so the parser seeds
    // currentLineNumber = 0 - 1 = -1; the first deletion records its
    // start at currentLineNumber + 1 = 0. Documents current behavior.
    expectedDeletedAt: [0],
  },
];
