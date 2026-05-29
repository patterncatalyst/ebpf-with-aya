#!/usr/bin/env bash
# Run every example's demo.sh and tally pass/fail. Examples that need the
# target VM or the stack will fail fast if those aren't up — that's expected
# when running this as a smoke check rather than a full lab run.
#
#   ./scripts/test-all-examples.sh
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/lib/_helpers.sh"
ROOT="$(repo_root)"

declare -a PASSED FAILED
for demo in "$ROOT"/examples/*/demo.sh; do
    dir="$(dirname "$demo")"
    name="$(basename "$dir")"
    step "running $name"
    if ( cd "$dir" && ./demo.sh ); then
        pass "$name"
        PASSED+=("$name")
    else
        echo -e "${RED}✗ $name failed${NC}" >&2
        FAILED+=("$name")
    fi
done

echo
echo "Passed: ${#PASSED[@]}   Failed: ${#FAILED[@]}"
if ((${#FAILED[@]})); then
    printf '  failed: %s\n' "${FAILED[@]}"
    exit 1
fi
