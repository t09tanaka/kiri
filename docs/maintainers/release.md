# Releasing kiri

This is the maintainer-facing reference for cutting a release. The
release script automates the mechanical parts; this document covers the
decisions and policies it does not encode.

If you are not a maintainer, you do not need to read this file. The
script will refuse to push without permissions, and tag creation is
gated by the maintainer team.

## Roles

- **Release driver** — the person tagging the release. Responsible
  for the version bump, the CHANGELOG, and the GitHub Release notes.
- **Release signer** — anyone with `contents: write` on the repo.
  Today this is the same person as the driver. If the driver is not
  yet a maintainer, a second pair of eyes from the maintainer team
  must review the release branch before the tag is pushed.

There is no separate "signed artifact" step today — the macOS bundle
is not codesigned. When code signing lands, this document will grow a
signing section; until then, releases are reproducible from the tag.

## Cadence

We don't release on a calendar. Cut a release when:

- A user-visible feature has landed on `main` and is worth surfacing.
- A bug fix in `main` resolves a reported regression on the previous
  release. **This is a hotfix** (see below) — it ships out of cadence
  via cherry-pick.

Avoid trickling tiny patch releases. Bundle two or three fix commits
into one patch release where possible; a single typo PR does not
warrant a tag.

## Versioning

Semantic Versioning ([semver](https://semver.org/)). For a 0.x.y line:

| Bump | When |
|---|---|
| `major` | API or CLI breaking change. We will switch to this once we hit 1.0; for now most user-visible breakage still goes into `minor`. |
| `minor` | New user-visible behaviour: a CLI verb, a UI feature, a new keyboard shortcut. |
| `patch` | Bug fixes only. No new flags, no new commands, no schema changes. |

When in doubt, ask: "if a user has been pinning the previous version,
will upgrading surprise them?" If yes, it is at least a minor bump.

## Normal release flow

All from `main`. Skip steps that don't apply for your release.

1. **Sync.** `git checkout main && git pull --ff-only`. Stop if you
   have uncommitted changes; `scripts/release.ts` writes to tracked
   files.
2. **Audit `[Unreleased]`.** Open [`CHANGELOG.md`](../../CHANGELOG.md).
   Confirm every notable PR merged since the last release has a line
   under `[Unreleased]`. Rewrite anything that reads like a PR title;
   the CHANGELOG is for users, not commit archaeologists.
3. **Promote `[Unreleased]`.** Rename the heading to the new version
   and date (`## [X.Y.Z] - YYYY-MM-DD`). Leave a fresh empty
   `## [Unreleased]` block above it. Commit this on its own
   (`chore(changelog): cut vX.Y.Z`) so the release commit only touches
   version files.
4. **Run the release script.**

   ```bash
   npm run release patch    # or minor, major, or X.Y.Z
   ```

   This bumps `package.json`, `src-tauri/tauri.conf.json`, and
   `src-tauri/Cargo.toml`, then creates a `chore: release vX.Y.Z`
   commit and an annotated `vX.Y.Z` tag. It does **not** push.
5. **Sanity-check.**
   - `git show --stat vX.Y.Z` — three version files and nothing else.
   - `npm run lint && npm run check && npm run test` — fast suite must
     pass on the tagged commit.
6. **Push.**

   ```bash
   git push origin main
   git push origin vX.Y.Z
   ```

   The tag push triggers `.github/workflows/release.yml`, which builds
   the macOS bundle and creates a **draft** GitHub Release.
7. **Finish the GitHub Release.** Edit the auto-generated draft:
   - Title: `kiri vX.Y.Z`.
   - Body: copy the version's CHANGELOG section verbatim, then a
     "Downloads" table (already templated) and any upgrade notes.
   - Attach any extra artifacts the workflow did not produce.
   - Publish when CI is green and the artifact downloads work.

If anything in steps 4–6 fails, **do not delete the tag in the remote**
without coordinating — re-using a tag is a worse problem than the bad
release. Instead, ship a fresh patch release with the fix.

## Composing `release-notes.md` / Release body

The Release workflow seeds a templated body referring readers to
`CHANGELOG.md`. When you edit it before publishing, follow the
template the CHANGELOG already uses:

- **Headline.** One sentence: what is the most user-visible thing in
  this release? "Adds `kiri term env` for external-terminal handshake."
- **Highlights.** Bullet list of two or three biggest user-visible
  changes, each one line. Link the PR number.
- **All changes.** Paste the relevant CHANGELOG section verbatim
  (Added / Fixed / Changed / Removed / Security).
- **Upgrade notes.** Only if there are any. Schema migrations, removed
  flags, file location moves. Empty section is fine to omit.
- **Downloads.** Already templated in `release.yml`.

Tone: the user just ran an install or hit "Check for updates." Lead
with what they can now do, not what we refactored internally.

## Cherry-pick / hotfix policy

When a bug in the latest release needs to ship out of cadence:

1. **Fix on `main` first.** Land the PR with tests like any other.
   This avoids divergence and ensures the next normal release is
   already correct.
2. **Branch from the release tag.** `git checkout -b hotfix/vX.Y.Z+1 vX.Y.Z`.
3. **Cherry-pick.** `git cherry-pick <sha-on-main>` for each commit
   that constitutes the fix. Use `git cherry-pick -x` so the hotfix
   commit message includes the upstream SHA — future readers need
   that breadcrumb.
4. **Resolve conflicts conservatively.** Only port the minimum needed
   to compile and pass tests. If the fix depends on unrelated changes
   on `main`, do **not** bring them along; tighten the fix or roll a
   minor release instead.
5. **Bump and tag.** Run `npm run release patch` from the hotfix
   branch.
6. **Push the tag.** The Release workflow runs against the hotfix
   commit. Do **not** push the hotfix branch as a long-lived branch;
   delete it after the tag is published.
7. **Backfill `main`.** If you adjusted the fix during cherry-pick,
   open a follow-up PR to bring `main` in line. The next release on
   `main` should be a superset of the hotfix.

Hotfix versions are always patch bumps (`X.Y.Z` → `X.Y.Z+1`). If you
need a new feature in a hotfix, you don't; cut a minor release
instead.

## When a release goes wrong

- **Workflow failed mid-build.** Re-run the failed job from the
  GitHub UI. The tag and the draft Release are idempotent; you do not
  need to delete and re-tag.
- **Bundle is broken on download.** Delete the draft Release (not the
  tag) and re-run the workflow. If the tag is at fault, ship a patch
  release.
- **Wrong version number in files.** The release commit and tag are
  identical to the file contents — never `git tag -f` over a remote
  tag. Roll forward with a fresh patch.

## Who can push tags

The `release.yml` workflow uses the default `GITHUB_TOKEN`, which is
scoped to the repo and not bound to a person. Tag-push permissions are
controlled by the branch / tag protection rules on the repo. If you
cannot push a tag, you are not a maintainer yet; ask an existing
maintainer to either grant the permission or run the release for you.
