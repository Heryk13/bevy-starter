#!/bin/sh
# Tries sync.sh on a throwaway repository with the same branch tree, whose `just check` fails
# while a file named `broken` exists. Touches nothing in the real repository.
set -eu

fail() {
    echo "sync-test: FAILED: $*" >&2
    exit 1
}

here="$(cd "$(dirname "$0")" && pwd)"
repo="$(mktemp -d)"
trap 'rm -rf "$repo"' EXIT
cd "$repo"

git init --quiet --initial-branch main
git config user.name "sync test"
git config user.email "sync-test@example.com"
git config core.autocrlf false
printf 'check:\n    test ! -e broken\n' >justfile
echo shared >shared.txt
# in git from the first commit, as in the template: every branch carries the script
mkdir tools
cp "$here/sync.sh" tools/
git add --all
git commit --quiet --message "chore: start"
# every branch gets a commit of its own, so the merges down are real merges
for edge in main:3d/base 3d/base:3d/player main:2d/base; do
    parent="${edge%%:*}"
    child="${edge#*:}"
    git switch --quiet --create "$child" "$parent"
    echo "$child" >"own-$(echo "$child" | tr / -).txt"
    git add --all
    git commit --quiet --message "feat: start $child"
done

echo "---- a change made on 3d/player lands on main and reaches every branch"
git switch --quiet 3d/player
echo changed >>shared.txt
echo new >new.txt
sh tools/sync.sh commit main "feat: change"
[ "$(git branch --show-current)" = "3d/player" ] || fail "did not return to the starting branch"
[ -z "$(git status --porcelain)" ] || fail "left changes behind"
[ "$(git log -1 --format=%s main)" = "feat: change" ] || fail "main did not get the commit"
for branch in main 3d/base 3d/player 2d/base; do
    git show "$branch:shared.txt" | grep -q changed || fail "$branch missed the edit"
    git show "$branch:new.txt" >/dev/null 2>&1 || fail "$branch missed the new file"
done
if git show main:own-3d-base.txt >/dev/null 2>&1; then
    fail "main got a child's file"
fi

echo "---- a failed check stops before the commit, on the target branch"
touch broken
main_before="$(git rev-parse main)"
if sh tools/sync.sh commit main "feat: broken" 2>/dev/null; then
    fail "a failed check did not stop it"
fi
[ "$(git rev-parse main)" = "$main_before" ] || fail "committed despite the failed check"
[ "$(git branch --show-current)" = main ] || fail "did not stop on the target branch"
rm broken

echo "---- no changes is refused"
if sh tools/sync.sh commit main "feat: nothing" 2>/dev/null; then
    fail "committed nothing"
fi

echo "---- an unknown branch is refused, and the changes stay where they were"
echo more >>shared.txt
if sh tools/sync.sh commit nowhere "feat: lost" 2>/dev/null; then
    fail "accepted a branch that does not exist"
fi
grep -q more shared.txt || fail "lost the changes"
git checkout --quiet -- shared.txt

echo "---- a change to the script itself survives the stash and the switches that rewrite it"
git switch --quiet 3d/player
echo "# changed" >>tools/sync.sh
sh tools/sync.sh commit main "build: change the script"
for branch in main 3d/base 3d/player 2d/base; do
    git show "$branch:tools/sync.sh" | grep -q "# changed" || fail "$branch missed the script's change"
done

echo "---- merging down with nothing new changes nothing"
tips_before="$(git for-each-ref --format='%(objectname)' refs/heads)"
sh tools/sync.sh down
[ "$tips_before" = "$(git for-each-ref --format='%(objectname)' refs/heads)" ] || fail "an idle merge down moved a branch"

echo
echo "sync-test: all passed"
