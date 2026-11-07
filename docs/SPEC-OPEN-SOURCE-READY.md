# Open-source contribution readiness

Parent: [#224](https://github.com/Heldinhow/openridr/issues/224)

## Problem Statement

A visitor cannot readily understand the alpha's supported platforms, find a contribution path, or privately report a concern. Missing community documents and structured issue forms make the public repository harder to use and contribute to.

## Solution

Provide an accurate English-language public entry point, contribution and conduct guidance, a security reporting policy, and GitHub issue and pull request templates. Identify approachable existing issues for new contributors. GitHub repository settings remain the maintainer's responsibility.

## User Stories

1. As a visitor, I want to understand what OpenRidr is and whom it serves, so that I can decide whether to try it.
2. As a visitor, I want to see its alpha status and verified platforms, so that I understand its limitations.
3. As a visitor, I want an authentic product image, or an explicit placeholder, so that I am not misled by third-party material.
4. As a user, I want to find published releases, so that I can download the application.
5. As a user, I want a summary of available features, so that I know what to expect.
6. As a developer, I want build prerequisites and commands, so that I can build the project.
7. As a contributor, I want setup and test guidance linked to its authoritative source, so that instructions do not drift.
8. As a contributor, I want to understand the issue-to-PR workflow and conventional commits, so that I can contribute effectively.
9. As a contributor, I want to understand CI and risk tiers, so that I know how my PR will be reviewed.
10. As a first-time contributor, I want appropriately labelled issues with enough context, so that I can select useful work.
11. As a community member, I want a code of conduct and a private reporting path, so that I can report a concern.
12. As a security researcher, I want supported versions and a private reporting path, so that I can report vulnerabilities responsibly.
13. As a reporter, I want a structured bug form with environment, reproduction and logs, so that maintainers can investigate.
14. As a proposer, I want a structured feature request, so that I can explain a problem and proposed solution.
15. As a visitor, I want questions directed to Discussions, so that issues remain actionable.
16. As a contributor, I want a short PR template with evidence, so that maintainers can assess my change.
17. As a maintainer, I want remaining GitHub settings identified explicitly, so that I can complete the community profile.

## Implementation Decisions

- Write the new public-facing documents and templates in English only; do not add a parallel translation in this PR.
- Describe only verified support: alpha on macOS Apple Silicon; do not claim Windows or macOS Intel support without evidence.
- Link to the release list: prereleases are not reliably addressed by GitHub's latest-release endpoint.
- Reuse an existing OpenRidr screenshot only after checking that it is suitable and has no private data; otherwise document a placeholder. Do not create a new capture or use proprietary material.
- Provide human-oriented contributing guidance without duplicating the authoritative agent instructions. Explain issue-to-PR flow, conventional commits, CI, risk tiers, and finding a good first issue.
- Use Contributor Covenant 2.1. Both conduct and security reports use GitHub Private Vulnerability Reporting, without publishing a personal email. Clearly state that this channel requires maintainer activation; do not claim it is already available. State the supported release policy accurately.
- Supply bug and feature issue forms and disable blank issues; direct questions to Discussions, whose activation remains pending. Retain the existing PR governance contract, including a closing issue reference and nonempty Evidence, with a concise external-contributor checklist.
- Label three to five suitable existing issues good first issue and/or help wanted only after checking their context and suitability.
- Repository settings, Discussions, private reporting, description, topics and social preview are maintainer actions, listed in PR evidence; do not change them in this work.
- Do not modify the existing agent instructions, product design documents, or product code. Do not add dependencies.

## Testing Decisions

- Verify observable document and template behavior at the repository surface, not product internals: validate YAML and form fields, PR governance compatibility, and the required content.
- Check links in the README and contribution guide, local and HTTP, and record commands, results, and inaccessible links. Check screenshot provenance and privacy if used.
- Record GitHub's issue chooser, Discussions, private reporting and a 100% community profile as not verified until changes reach the default branch and the maintainer enables the relevant settings. Never treat a configuration-dependent outcome as passed prematurely.
- No product seam, app launch, UI E2E, or new dependency is needed. Run applicable existing checks and document what was not run.

## Out of Scope

Product code changes, new captures, third-party proprietary material, changes to repository settings, translated duplicate documentation, and changes to the agent instructions or product design documents.

## Further Notes

The GitHub community profile was 42% at the start of this work. Acceptance at 100% depends on subsequent maintainer configuration and GitHub's assessment, not merely this PR.
