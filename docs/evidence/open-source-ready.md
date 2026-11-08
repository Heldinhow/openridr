# Open-source readiness — evidence

Issue #224; tickets #226–#229. Verification on 2026-11-07, macOS Apple Silicon; #228 follow-up on
2026-11-08.

## Checks

- `bun test tests/unit`: 126 passed, 0 failed on 2026-11-08, after the contribution-guide and
  reporting tests were updated. `bunx tsc --noEmit` produced no output.
- `bun test tests/unit/readme.test.ts tests/unit/communityReportingDocs.test.ts`: 20 passed, 0 failed.
- `ruby -e 'require "yaml"; YAML.load_file(ARGV[0])' .github/ISSUE_TEMPLATE/config.yml`: parsed.
- GitHub settings, read with the API on 2026-11-08:
  - `gh api repos/Heldinhow/openridr/community/profile` → `health_percentage: 100`.
  - `gh api repos/Heldinhow/openridr` → `has_discussions: true`, the description is set and there
    are 10 topics (`code-editor`, `csharp`, `dap`, `debugger`, `desktop-app`, `dotnet`, `ide`, `lsp`,
    `rust`, `tauri`).
  - Discussions categories: Announcements, General, Ideas, Polls, Q&A, Show and tell.
  - `gh api repos/Heldinhow/openridr/private-vulnerability-reporting` → `enabled: true`.
- Labeled issues, all open and unassigned when checked with
  `gh api repos/Heldinhow/openridr/issues/<n>`:
  - #260 `good first issue`: a comment in `.gitignore` about the versioned screenshots; no product
    code, no dependency, concrete acceptance criteria.
  - #246 `help wanted`: ephemeral host port for the E2E helper; fully specified, but it touches
    `crates/host-http/`, `tests/e2e/` and `AGENTS.md`, so its PR is high risk.
  - #269 `help wanted`: a CI job for `tests/shell/*.test.sh` under `dash`; fully specified, but
    `.github/workflows/**` is high risk.
  - #280 `help wanted`: the Windows `dotnet tool install` failure in `crates/host/src/tools.rs`;
    fully specified, no blockers.
- **How "low-risk" was read.** No open, unassigned issue would produce a `risk:low` pull request:
  the only documentation-only candidate, #203, is blocked by #201 and #202, and every other open
  issue touches product code, CI or `AGENTS.md`. The criterion was therefore read as "low risk to
  hand to a newcomer": no trust boundary, no signing, no release path, no dependency change and no
  decision that belongs to the maintainer. #246 and #269 are `risk:high` by the path table, and the
  guide now says so instead of implying that a label promises an easy merge. Reducing the set to
  #260 and #280 would have left the label pages nearly empty, which is the opposite of what the
  ticket asks for; no label was put on an issue whose scope or blocker made it unusable.
- Not labeled: #228 itself (assigned and in flight), #264, #270, #275 and #287 (assigned), #247 (an
  agent-process document that edits `AGENTS.md`, high risk and not newcomer work), #203 (blocked by
  #201 and #202) and the Windows/Intel release work (#199–#202, #204–#205), which cannot be verified
  without that hardware. #216, #217 and #221 were closed when the labels were applied. No artificial
  issue was created.
- HTTP links in `README.md` and `CONTRIBUTING.md`, checked with
  `curl -s -o /dev/null -w '%{http_code} %{url_effective}\n' -L <url>` on 2026-11-08: all returned
  200.
  - `200 https://github.com/Heldinhow/openridr/actions/workflows/ci.yml/badge.svg?branch=main`
  - `200 https://github.com/Heldinhow/openridr/actions/workflows/ci.yml`
  - `200 https://img.shields.io/github/license/Heldinhow/openridr`
  - `200 https://github.com/Heldinhow/openridr/releases`
  - `200 https://rust-lang.org/tools/install/` (from `https://www.rust-lang.org/tools/install`)
  - `200 https://bun.sh/`
  - `200 https://dotnet.microsoft.com/en-us/download` (from `https://dotnet.microsoft.com/download`)
  - `200 https://github.com/Heldinhow/openridr/issues?q=is%3Aissue%20is%3Aopen%20label%3A%22good%20first%20issue%22`
  - `200 https://github.com/Heldinhow/openridr/issues?q=is%3Aissue%20is%3Aopen%20label%3A%22help%20wanted%22`
  - `200 https://github.com/Heldinhow/openridr/discussions`
- Relative links in the README, the guide, `SECURITY.md` and `CODE_OF_CONDUCT.md` are covered by
  `bun test tests/unit/readme.test.ts` and `bun test tests/unit/communityReportingDocs.test.ts`. The
  in-page `#ci-and-risk` anchor and `AGENTS.md#pull-request-e-ci` were checked by finding the matching
  headings in `CONTRIBUTING.md` and `AGENTS.md`.
- The reused `docs/evidence/dev-app/dev-app-workspace.png` predates this change and shows the public
  OrderDesk fixture; reviewed for private information and proprietary content. No new capture was
  made.

## Pending and not verified

- GitHub rendering of the updated documents and the issue chooser's Discussions description on the
  default branch: not verified until this branch merges. The files themselves are covered by the
  tests above.
- Discussions: enabled, with the categories above; creating a discussion was not exercised.
- Private vulnerability reporting: the API reports `enabled: true`; no real report was filed, so the
  end-to-end submission is not verified. Conduct reports use the same form; the security advisory
  interface may ask for vulnerability metadata and must not inadvertently publish conduct details.
- Social preview: no API exposes it, so it was not verified.
- Label suitability is a snapshot: an issue can be claimed or change scope after this evidence. The
  guide tells contributors to read the issue and comment to claim it.
- No desktop app or E2E: documentation, templates and labels only, per the approved scope. Windows
  and macOS Intel remain unverified.

## Delegation and review

For #227, two calls to `pi/mistral/deepseek-v4-1-flash-1017` (`thinking=max`) and one to `pi/openai-codex/gpt-6.1-terra` (`thinking=medium`) failed with `Provider turn failed.` The individual failure timestamps were not exposed in the returned task summaries; all occurred on 2026-11-07 before 14:56 -03:00. The prompts were short single-paragraph implementation briefs (roughly 500–600 characters), pointing to the issue, spec, integration branch, TDD skill, commit and safety rules; no source files were inlined. The orchestrator authorized local implementation after these failures.

For #228 the first two Mistral calls failed with the same message; the authorized GPT fallback produced the guide but found no safe candidates to label. For #229 the first Mistral attempt failed with HTTP 503 `Service unavailable`; its retry succeeded. Independent review of `9e3f97f...c80f0cb` identified inaccurate Debug, license, and questions-channel wording, corrected after review. No additional dependency was added.

On 2026-11-08 the #228 follow-up was implemented in a single session: the guide, README, security
and conduct documents, issue chooser and unit tests were updated, and the session applied the four
labels. Independent review ran on `d52cdc7...` with `pi/openai/gpt-6.1-terra` (`thinking=medium`), one
sub-agent per axis, fixed point `806a51a`.

Findings and what happened to each:

- **Standards, medium:** the implementation notes did not mark end-to-end private reporting as not
  verified. Fixed: the notes now list the unexercised submission, the discussion and the GitHub
  rendering.
- **Standards, judgement:** the two documents repeated the same channel assertions. Fixed with one
  shared `expectActivePrivateChannel` helper in `tests/unit/communityReportingDocs.test.ts`.
- **Standards, judgement:** the document tests assert wording, not behaviour. Kept: the published
  document is the seam, which is how `tests/unit/readme.test.ts` and
  `tests/unit/communityReportingDocs.test.ts` already tested these files before this change.
- **Spec, high:** labeling `help wanted` on `risk:high` issues reads against the criterion's
  "low-risk". Answered above and in the guide rather than reverted: no eligible `risk:low` issue
  exists, and the guide now states that the label does not promise an automatic merge.
- **Spec, medium:** `CODE_OF_CONDUCT.md` overclaimed the conduct channel. Fixed: it now says the form
  is GitHub's security advisory interface reused as the private contact channel, that the report is
  visible only to the maintainer, and that the vulnerability-specific fields are left empty where
  they do not apply.
- **Spec, note:** updating the README, security, conduct and issue-chooser channel status is wider
  than the guide-and-label contract of #228, but it is the parent #224's intent and the documents
  were factually wrong after the maintainer enabled the settings.
