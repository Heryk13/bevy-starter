#!/bin/sh
# Commits the working changes on the branch they belong to and merges them down the branch
# tree, running `just check` on every branch it changes. It is the template's own tool: a game
# has no branch tree, so `create-bevy-starter` deletes `tools/` from the games it makes.
#
#   just tools/sync main "feat: make the cards glass"   every change -> a commit on main -> down
#   just tools/down                                      only the merges down
#
# It stops at the first conflict or failed check, on the branch where it happened, and says
# what to do there. It pushes nothing without asking.
#
# Plain `sh` on purpose, and run as `sh sync.sh`: on Windows the name `bash` finds WSL's before
# Git's, a Linux with another git and no `just`, while `sh` is the one shell `just` itself runs.
set -eu

# the branch tree as parent:child, in the order the merges run: a new branch is a new pair
TREE="main:3d/base 3d/base:3d/player main:2d/base"

die() {
    echo "sync: $*" >&2
    exit 1
}

step() {
    printf '\n==> %s\n' "$*"
}

# runs `just check` on the current branch; $1 is what to do when it fails
check() {
    branch="$(git branch --show-current)"
    step "just check on $branch"
    just check || die "just check failed on $branch: $1"
}

# moves every working change to the branch $1 and commits it there with the message $2
commit_on() {
    target="$1"
    message="$2"
    git rev-parse --verify --quiet "refs/heads/$target" >/dev/null || die "no branch named '$target'"
    [ -n "$message" ] || die "the commit needs a message"
    [ -n "$(git status --porcelain)" ] || die "no changes to commit"
    if [ "$start" != "$target" ]; then
        step "move the changes from $start to $target"
        git stash push --include-untracked --quiet
        git switch --quiet "$target" || die "could not switch to $target: the changes are in the stash, 'git stash pop' brings them back"
        git stash pop --quiet || die "the changes conflict with $target: resolve them here, run 'git stash drop', then run the same command again"
    fi
    check "fix it here, then run the same command again"
    step "commit on $target"
    git add --all
    git commit --quiet --message "$message"
}

# merges every parent into its child, down the whole tree, checking each branch a merge changed
merge_down() {
    [ -z "$(git status --porcelain)" ] || die "commit or stash the changes before merging down"
    for edge in $TREE; do
        parent="${edge%%:*}"
        child="${edge#*:}"
        git switch --quiet "$child"
        before="$(git rev-parse HEAD)"
        git merge --no-edit --quiet "$parent" || die "merging $parent into $child conflicts: resolve it here, commit, then run 'just tools/down'"
        # already up to date: nothing new to check
        [ "$before" != "$(git rev-parse HEAD)" ] || continue
        step "merged $parent into $child"
        check "fix it here, commit, then run 'just tools/down'"
    done
    git switch --quiet "$start"
}

# pushes every branch of the tree, only on a typed "y"
offer_push() {
    git remote get-url origin >/dev/null 2>&1 || return 0
    # each branch of the tree once, by hand: `sort -u` would do it, but on Windows that name
    # finds the system's own sort before Git's, as `bash` finds WSL's
    branches=""
    for edge in $TREE; do
        for branch in "${edge%%:*}" "${edge#*:}"; do
            case " $branches " in
            *" $branch "*) ;;
            *) branches="$branches $branch" ;;
            esac
        done
    done
    step "every branch is merged down, back on $start"
    printf 'push%s? [y/N] ' "$branches"
    read -r answer || answer=n
    if [ "$answer" != y ]; then
        echo "not pushed"
        return 0
    fi
    # unquoted on purpose: one argument per branch
    git push origin $branches
}

main() {
    here="$(cd "$(dirname "$0")" && pwd)"
    cd "$(git rev-parse --show-toplevel)"
    # a stash takes untracked files along, and this script must not leave with them mid-run
    git ls-files --error-unmatch -- "$here/sync.sh" >/dev/null 2>&1 || die "sync.sh is not in git yet: commit tools/ by hand once"
    start="$(git branch --show-current)"
    [ -n "$start" ] || die "not on a branch"
    [ ! -e "$(git rev-parse --git-path MERGE_HEAD)" ] || die "a merge is in progress: finish it first"

    case "${1:-}" in
    commit)
        commit_on "${2:-}" "${3:-}"
        merge_down
        offer_push
        ;;
    down)
        merge_down
        offer_push
        ;;
    *)
        die "usage: sync.sh commit <branch> <message> | sync.sh down"
        ;;
    esac
}

# everything above is a function and the run starts on this last line, so `sh` has read the
# whole file before a stash or a switch rewrites it under the run, as when this file changed
main "$@"; exit
