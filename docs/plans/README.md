# `docs/plans/`

Design and implementation plans for app-level features (the Tauri host
and the Svelte frontend). One file per design or plan, prefixed with the
date the document was started (`YYYY-MM-DD-short-slug.md`).

Skills that the agent installs into Claude Code use a parallel directory
at [`docs/superpowers/`](../superpowers/) — keep app-level plans here.

## Lifecycle

A document in this directory progresses through three states. The state
is implicit in the filename and the contents, not in a label, so keep
docs short enough that you can update them in place.

1. **Proposal.** First draft, sketched before code is written. Names
   the problem, the proposed shape, and any alternatives considered.
   File starts with `YYYY-MM-DD-<slug>-design.md` (or just
   `YYYY-MM-DD-<slug>.md` for a single combined document).
2. **Plan.** Once the proposal is agreed, the implementation plan is
   added alongside as `YYYY-MM-DD-<slug>-plan.md`. The plan breaks the
   work into reviewable steps; AI agents use it as the checklist for
   `superpowers:executing-plans` / `superpowers:subagent-driven-development`.
3. **Archive.** Once the feature ships, leave the documents in place.
   Add a single line at the top noting the shipping PR and version
   (`Shipped: v0.5.0, #42`). Do not delete: the history matters for
   future incident response and onboarding.

## Conventions

- **Dates are the start date**, not the ship date — the file is named
  the day it is created and does not get renamed when work moves on.
- **English is preferred** so external contributors can follow along.
  Japanese is acceptable for internal-only experiments, but anything
  that may attract a PR should be in English.
- **Link rather than duplicate.** If a section of a plan applies to
  multiple features, factor it into a separate document under
  `contributing/` or `maintainers/` and link to it.
- **Diagrams are welcome.** ASCII art renders everywhere; Mermaid is
  acceptable but be aware GitHub limits some renderings.

## Index

The current set of plans is whatever you find with `ls`. Filenames are
descriptive enough that an explicit index would drift.
