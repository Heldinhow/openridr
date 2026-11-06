#!/bin/sh
# Disposable local bare remote + two clones of the public fixtures/OrderDesk content, for Git
# acceptance work (docs/SPEC-GIT-REMOTE-ACCEPTANCE.md). No network, no login, no global git config:
# every setting lives inside these throwaway repositories.
#
#   user   <state>/user   repository root == workspace root; the app opens <state>/user/OrderDesk.sln
#   peer   <state>/peer   the counterpart; publish commits and branches from here with plain git
#   origin <state>/origin.git   the bare remote both clones share
#
# Usage: scripts/git-remote-fixture.sh [new-state-dir]
#
# Non-destructive by contract: this script never removes or reuses an existing directory. With no
# argument it creates a fresh private mktemp directory under ${TMPDIR:-/tmp}. An argument must be a
# new absolute path inside the (real) temporary root; anything that already exists — a directory, a
# file, or a dangling symlink — is refused and the path is left exactly as it was. To start over,
# run it again with no argument (or with another new path); the previous state is kept.
set -eu

repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=${TMPDIR:-/tmp}; tmp=${tmp%/}
name="OpenRidr Acceptance"
email="acceptance@openridr.invalid"

die() { echo "git-remote-fixture: $*" >&2; exit 2; }

# The real temporary root: symlinked TMPDIR (macOS /var -> /private/var) resolves once, here.
tmp_real=$(CDPATH= cd -- "$tmp" && pwd -P) || die "temporary root is not reachable: $tmp"

if [ "$#" -gt 1 ]; then
  die "usage: scripts/git-remote-fixture.sh [new-state-dir]"
fi

if [ "$#" -eq 0 ]; then
  state=$(mktemp -d "$tmp_real/openridr-git-remote.XXXXXX") || die "could not create a temporary state"
else
  requested=$1
  case "$requested" in
  /*) ;;
  *) die "the state path must be absolute: $requested" ;;
  esac
  case "$requested" in
  */.. | */../* | */. | */./* | /.) die "the state path must not contain '.' or '..': $requested" ;;
  esac
  # Existing in any shape (directory, file, dangling symlink) is a refusal, never a target.
  if [ -e "$requested" ] || [ -L "$requested" ]; then
    die "refusing an existing path; run again with a new path or with no argument: $requested"
  fi
  parent=$(dirname -- "$requested")
  base=$(basename -- "$requested")
  [ -n "$base" ] || die "the state path needs a name: $requested"
  [ -d "$parent" ] || die "the parent directory does not exist: $parent"
  # Resolve the real parent (no symlink followed silently) and require it under the temporary root.
  parent_real=$(CDPATH= cd -- "$parent" && pwd -P) || die "parent directory is not reachable: $parent"
  case "$parent_real/" in
  "$tmp_real"/*) ;;
  *) die "the state path must live under the temporary root $tmp_real: $requested" ;;
  esac
  state="$parent_real/$base"
  # Final check on the resolved path, so a parent symlink cannot land outside the temporary root.
  case "$state/" in
  "$tmp_real"/*) ;;
  *) die "the resolved state path escapes the temporary root: $state" ;;
  esac
  mkdir "$state" 2>/dev/null || die "could not create the state directory: $state"
fi

# A state inside the checkout would write throwaway repositories into the product tree.
case "$(CDPATH= cd -- "$state" && pwd -P)/" in
"$repo"/*) die "refusing a state inside the checkout: $state" ;;
esac

command -v git >/dev/null 2>&1 || die "git is required"
command -v rsync >/dev/null 2>&1 || die "rsync is required"

mkdir -p "$state/user"
# Copy the public fixture into the clone root so the repository root is the workspace root.
rsync -a --exclude bin --exclude obj "$repo/fixtures/OrderDesk/" "$state/user/"

git init --bare --quiet --initial-branch=main "$state/origin.git"

git -C "$state/user" init --quiet --initial-branch=main
git -C "$state/user" config user.name "$name"
git -C "$state/user" config user.email "$email"
git -C "$state/user" add -A .
git -C "$state/user" commit --quiet -m "Baseline for issue 50 Git remote acceptance"
git -C "$state/user" remote add origin "$state/origin.git"
git -C "$state/user" push --quiet --set-upstream origin main

git clone --quiet "$state/origin.git" "$state/peer"
git -C "$state/peer" config user.name "$name"
git -C "$state/peer" config user.email "$email"

baseline=$(git -C "$state/user" rev-parse HEAD)
[ "$baseline" = "$(git -C "$state/origin.git" rev-parse refs/heads/main)" ] \
  || die "origin/main does not match the baseline"
[ "$baseline" = "$(git -C "$state/peer" rev-parse HEAD)" ] \
  || die "peer does not share the baseline"

echo "state $state"
echo "origin $state/origin.git"
echo "solution $state/user/OrderDesk.sln"
echo "peer $state/peer"
echo "baseline $baseline"
echo "launch VERIFY_OPENRIDR_DESKTOP_SOLUTION=$state/user/OrderDesk.sln"
