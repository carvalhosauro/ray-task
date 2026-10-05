#!/bin/sh
# Validates a commit message file against Conventional Commits.
# Usage: check-commit-msg.sh <path-to-commit-msg-file>
set -eu
msg_file=$1
# First line that is not a git comment.
subject=$(grep -v '^#' "$msg_file" | head -n 1 || true)

case $subject in
  "Merge "* | "Revert \""* | "fixup! "* | "squash! "*) exit 0 ;;
esac

types='feat|fix|chore|docs|refactor|test|perf|ci|build|style|revert'
if printf '%s\n' "$subject" | grep -Eq "^($types)(\([a-z0-9-]+\))?!?: [^ ].*"; then
  exit 0
fi

cat >&2 <<EOF
Commit message does not follow Conventional Commits:
  $subject
Expected: <type>(<scope>)?: <summary>   e.g. "fix(app): close tag popover on Esc"
Types: feat, fix, chore, docs, refactor, test, perf, ci, build, style, revert
EOF
exit 1
