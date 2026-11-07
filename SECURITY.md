# Security Policy

OpenRidr is an alpha desktop IDE for C#/.NET. This policy covers the application
and the code in this repository.

## Supported Versions

Only the latest alpha release receives security fixes. Older alphas are
unsupported: a fix ships in a new release, never as a patch to an older one.

| Version | Supported |
| --- | --- |
| Latest alpha (`v0.3.0-alpha.11` or newer) | :white_check_mark: |
| Older alphas | :x: |

Releases are listed at <https://github.com/Heldinhow/openridr/releases>. They are
prereleases, so GitHub's "latest release" shortcut does not point at them.

## Supported Platforms

Only macOS on Apple Silicon (arm64) is verified: macOS 13 or newer, as described
in [docs/RELEASE-ALPHA.md](docs/RELEASE-ALPHA.md). Windows and macOS on Intel
are not verified and are not supported; a report about them is welcome, but no
fix or timeline is promised.

## Reporting a Vulnerability

Report vulnerabilities privately through GitHub's private vulnerability
reporting form:

<https://github.com/Heldinhow/openridr/security/advisories/new>

Do not open a public issue for a vulnerability, and do not post exploit details
in a public discussion.

That channel is **not yet enabled**: the maintainer must activate private
vulnerability reporting in the repository settings before the form accepts a
report. This policy does not publish a personal email address, so until the
channel is active there is no private reporting path.

## What to Include

- The OpenRidr version (from the app's About or the release tag) and the macOS
  version.
- The steps that reproduce the problem and the impact you believe it has.
- Logs or a proof of concept, with secrets and personal data removed.

## Our Process

This is a single-maintainer alpha project: reports are handled on a best-effort
basis, without a guaranteed response time.

- The report stays private until a fix is released, and the reporter is credited
  unless they ask otherwise.
- The fix ships in a new alpha release and is described in its release notes.
- A report about an unsupported version or platform may be closed with that
  reason instead of a fix.
