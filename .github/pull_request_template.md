<!--
Fill every section. `Closes #N`, `## Summary` and `## Evidence` are checked by the `governance` job
(scripts/ci-pr-body.sh); a body without them fails. Keep "Merge danger" honest: it is what a human
reads before approving.
-->

Closes #

## Summary

What changed and why, in the issue's vocabulary.

## Evidence

What you actually ran, with the command and the result — `cargo test -p openridr-host`, the E2E spec
that passed, the agent-desktop snapshot of the app. What did not run goes here as
"not verified: <why>". A green `ci` is not evidence of UI: the workflow runs tsc, unit tests, fmt,
clippy and the host tests only.

## Security review

What this PR touches at a trust boundary (argv, paths, subprocesses, network, credentials) and why it
is safe. "Nothing outside the host's existing boundaries" is a valid answer when it is true.

## Merge danger

Low / medium / high, and what could break if it is wrong. `.github/workflows/**`, `scripts/release*`,
signing, `scripts/vendor-*`, the trust model, the updater and dependency changes are high: they need
the maintainer, not only a green CI.
