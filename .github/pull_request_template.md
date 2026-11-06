<!--
`Closes #N` and a non-empty `## Evidence` are checked by the `governance` job
(scripts/ci-pr-body.sh): a body without them fails. `## Summary` and the rest are what a human reads
before approving — fill them for real.
-->

Closes #

## Summary

<!-- What changed and why, in the issue's vocabulary. -->

## Evidence

<!-- Replace this comment with what you actually ran: the command and the result (`cargo test -p
openridr-host`, the spec that passed, the agent-desktop snapshot of the app). What did not run goes
here as "not verified: <why>". A green `ci` is not evidence of UI — it runs tsc, unit tests, fmt,
clippy and the host tests only. -->

## Security review

<!-- What this PR touches at a trust boundary (argv, paths, subprocesses, network, credentials) and
why it is safe. "Nothing outside the host's existing boundaries" is a valid answer when it is true.
-->

## Merge danger

<!-- Low / medium / high, and what breaks if it is wrong. `.github/workflows/**`, `scripts/release*`,
signing, `scripts/vendor-*`, the trust model, the updater and dependency changes are high: they need
the maintainer, not only a green CI. -->
