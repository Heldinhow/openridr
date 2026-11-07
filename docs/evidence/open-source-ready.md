# Open-source readiness — evidence

Issue #224; tickets #226–#229. Verification on 2026-11-07, macOS Apple Silicon.

## Checks

- `bun test tests/unit`: 111 passed, 0 failed at integration `c80f0cb`; the additional contribution-link test passed separately with `bun test tests/unit/readme.test.ts` (8 passed).
- `bun run check`: passed (TypeScript and workspace cargo check).
- `ruby -e 'require "yaml"; ARGV.each { |f| YAML.load_file(f) }' .github/ISSUE_TEMPLATE/*.yml`: all three YAML files parsed. Required bug and feature form IDs and the disabled blank-issue setting were checked separately with Ruby.
- `sh scripts/ci-pr-body-selfcheck.sh`: 18 cases passed. An untouched PR template is rejected, and a filled example with an issue and Evidence is accepted by `scripts/ci-pr-body.sh`.
- Local README and contribution-guide links were checked by `bun test tests/unit/readme.test.ts`; the `AGENTS.md#pull-request-e-ci` heading was checked manually. A `curl -L` check of each HTTP link in both documents returned 200 on 2026-11-07. Discussions, linked from the issue chooser configuration, returned 404 because it is not enabled.
- The reused `docs/evidence/dev-app/dev-app-workspace.png` predates this change and shows the public OrderDesk fixture; reviewed for private information and proprietary content. No new capture was made.
- `gh api repos/Heldinhow/openridr/community/profile`: 42% before merge. `gh api repos/Heldinhow/openridr/private-vulnerability-reporting`: `enabled:false`; repo API reports Discussions disabled, empty description and no topics.

## Pending and not verified

- GitHub issue chooser and rendering on the default branch, Discussions, private reporting end-to-end and a 100% community profile: not verified; merge and/or maintainer settings are required.
- Maintainer must enable Discussions and Private Vulnerability Reporting and set the repository description, topics and social preview. Until private reporting is enabled there is **no working private reporting channel** for either security or conduct. This is a pending configuration, not a functioning alternative; the maintainer should enable it before merge. Do not publish private reports as issues. The security advisory interface may ask for vulnerability metadata and must not inadvertently publish conduct details.
- The target of 3–5 newly suitable labelled issues remains unmet: #216 already has `help wanted` but spans seven specs and a shared helper, so is not a small first issue; #217 requires AGENTS.md (high risk); #221 requires AGENTS.md and has blockers; #203 is blocked by #201 and #202. No artificial issues or misleading labels were created.
- No desktop app or E2E: documentation/templates only, per the approved scope. Windows and macOS Intel remain unverified.

## Delegation and review

For #227, two calls to `pi/mistral/deepseek-v4-1-flash-1017` (`thinking=max`) and one to `pi/openai-codex/gpt-6.1-terra` (`thinking=medium`) failed with `Provider turn failed.` The individual failure timestamps were not exposed in the returned task summaries; all occurred on 2026-11-07 before 14:56 -03:00. The prompts were short single-paragraph implementation briefs (roughly 500–600 characters), pointing to the issue, spec, integration branch, TDD skill, commit and safety rules; no source files were inlined. The orchestrator authorized local implementation after these failures.

For #228 the first two Mistral calls failed with the same message; the authorized GPT fallback produced the guide but found no safe candidates to label. For #229 the first Mistral attempt failed with HTTP 503 `Service unavailable`; its retry succeeded. Independent review of `9e3f97f...c80f0cb` identified inaccurate Debug, license, and questions-channel wording, corrected after review. No additional dependency was added.
