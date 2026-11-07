# Contributing to OpenRidr

Thank you for helping improve OpenRidr. The project is an alpha verified on macOS Apple Silicon; Windows and macOS Intel are not verified. Please follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## Set up

For the desktop development workflow, use macOS Apple Silicon with Xcode command-line tools, [Rust](https://www.rust-lang.org/tools/install), [Bun](https://bun.sh/), and the [.NET SDK](https://dotnet.microsoft.com/download) 8.0.408 required by the fixtures. See [AGENTS.md](AGENTS.md) for the current tool, SDK, build, test and platform details. The application detects SDKs; it does not install them or change `global.json`.

```sh
bun install
bun run check
bun run dev       # web UI
bun run tauri dev # desktop app
```

A fresh worktree needs its own `bun install`. Some checks require additional tools or SDKs; report missing prerequisites rather than treating skipped tests as passes.

## Find and propose work

Browse [good first issues](https://github.com/Heldinhow/openridr/issues?q=is%3Aissue%20is%3Aopen%20label%3A%22good%20first%20issue%22) and [help wanted issues](https://github.com/Heldinhow/openridr/issues?q=is%3Aissue%20is%3Aopen%20label%3A%22help%20wanted%22). Read the issue, comments, linked specification and any blockers before starting; check that it is unassigned and ask on the issue if its scope is unclear. For new work, discuss or open an issue first so the approach can be agreed before a PR.

## From issue to pull request

1. Create a branch from an up-to-date `main`, and claim or comment on the issue to avoid duplicated work.
2. Make a focused change with appropriate tests. Follow the project conventions and verification guidance in [AGENTS.md](AGENTS.md).
3. Use conventional commit messages such as `fix(search): handle empty query` or `docs: clarify setup`. Keep unrelated changes separate.
4. Run the checks appropriate to your change. In the pull request, use the template, link the issue with `Closes #N`, and include commands and results under **Evidence**. Explicitly state what you could not verify and why. Describe security implications and merge risk as requested by the template.
5. Open the PR against `main`, respond to review, and keep the branch current. Maintainers handle merges except where low-risk auto-merge is enabled.

## CI and risk

PR CI checks TypeScript and unit tests, Rust formatting, lint and host tests, governance and secrets. It does not prove UI behavior; provide the appropriate verification evidence separately. The [CI and risk guidance](AGENTS.md#pull-request-e-ci) is authoritative.

Risk is calculated from changed paths, not from a label: documentation, tests and CSS are generally low; product code is medium; CI, release, signing, trust and dependency changes are high. A mixed PR takes the highest tier. Low-risk PRs may auto-merge after required checks pass; medium-risk PRs await maintainer review, and high-risk PRs require the high-risk gate and a manual merge. See the [risk-tier specification](docs/SPEC-RISK-TIERS.md) for details, including the owner-author exception.
