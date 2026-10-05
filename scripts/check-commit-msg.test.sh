#!/bin/sh
# Table test for check-commit-msg.sh. Usage: scripts/check-commit-msg.test.sh
set -u
here=$(cd "$(dirname "$0")" && pwd)
check="$here/check-commit-msg.sh"
tmp=$(mktemp)
trap 'rm -f "$tmp"' EXIT
fail=0

expect() { # expect <0|1> <message>
  printf '%s\n' "$2" >"$tmp"
  "$check" "$tmp" 2>/dev/null
  got=$?
  if [ "$got" -ne "$1" ]; then
    echo "FAIL: expected exit $1, got $got for: $2"
    fail=1
  fi
}

expect 0 "feat(app): add tag popover"
expect 0 "fix: close keyboard gaps"
expect 0 "chore(deps)!: bump slint to 2.0"
expect 0 "docs: add oss readiness design spec"
expect 0 "ci: add coverage job"
expect 0 "Merge branch 'main' into feat/x"
expect 0 "Merge pull request #12 from someone/branch"
expect 0 "Revert \"feat(app): add tag popover\""
expect 0 "fixup! feat(app): add tag popover"
expect 0 "squash! fix: close keyboard gaps"
expect 0 "# only a comment line
feat(core): real subject after comment"
expect 1 "added stuff"
expect 1 "Feat: capitalized type"
expect 1 "feat(app) missing colon"
expect 1 "feat(App): uppercase scope"
expect 1 "feature: unknown type"
expect 1 "feat: "
expect 1 ""

[ "$fail" -eq 0 ] && echo "all commit-msg cases pass"
exit "$fail"
