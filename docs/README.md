# `docs/`

Documentation index. Most kiri documentation lives in one of three places;
this README explains which is which so contributors and AI agents don't
have to spelunk.

| Path | What lives here | Audience |
|---|---|---|
| [`contributing/`](contributing/) | Canonical English contributor docs (testing policy, label taxonomy). | Anyone sending a PR. Linked from the root [`CONTRIBUTING.md`](../CONTRIBUTING.md). |
| [`maintainers/`](maintainers/) | Maintainer-only material (release flow, cherry-pick policy). | People who tag releases or sign artifacts. |
| [`plans/`](plans/) | Design + plan documents for app-level features, by date. | Anyone implementing or reviewing a feature. See [`plans/README.md`](plans/README.md) for lifecycle. |
| [`superpowers/plans/`](superpowers/plans/) | Plans for "superpower" skills the agent installs into Claude Code. | Agent developers. |
| [`superpowers/specs/`](superpowers/specs/) | Detailed designs that pair with the plans above. | Same. |

## What about `docs/features/`?

Earlier drafts of the project referred to a `docs/features/` directory.
That layout was never finalised: live feature documentation lives in
`docs/plans/` (app-level features) and `docs/superpowers/specs/` (skills),
and reference user-facing docs live in the root [`README.md`](../README.md)
and [`resources/skills/kiri-cli/SKILL.md`](../resources/skills/kiri-cli/SKILL.md).
If you are looking for a specific feature, search those directories.

## Adding a document

- A design or implementation plan for a new feature → `docs/plans/`,
  prefixed with the date you started it (`YYYY-MM-DD-short-slug.md`).
  See [`plans/README.md`](plans/README.md) for the lifecycle.
- Guidance new contributors need before sending a PR → either
  `CONTRIBUTING.md` (top-level) or `docs/contributing/` (longer form).
- Release / signing / cherry-pick policy → `docs/maintainers/`.
- AI-agent-only notes → `.claude/` (gitignored). If a human will ever
  need to read it, it does **not** belong there.
