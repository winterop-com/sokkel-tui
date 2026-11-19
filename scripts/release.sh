#!/usr/bin/env bash
# Cut release VERSION (X.Y.Z): the workspace's version set, the CHANGELOG's Unreleased section
# dated as it, the gates run, a signed commit and a signed tag vVERSION. It pushes nothing: it
# prints the push that publishes it (the release workflow then checks it and makes the GitHub
# Release).
set -euo pipefail

version="${1:-}"
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "usage: make release VERSION=X.Y.Z" >&2; exit 2; }
tag="v$version"

[[ -z "$(git status --porcelain)" ]] || { echo "the tree is not clean" >&2; exit 1; }
[[ "$(git branch --show-current)" == "main" ]] || { echo "not on main" >&2; exit 1; }
git fetch -q origin main
[[ "$(git rev-parse HEAD)" == "$(git rev-parse origin/main)" ]] || { echo "main is not origin/main" >&2; exit 1; }
git rev-parse -q --verify "refs/tags/$tag" >/dev/null && { echo "$tag exists" >&2; exit 1; }
grep -q '^## Unreleased$' CHANGELOG.md || { echo "CHANGELOG.md has no Unreleased section" >&2; exit 1; }

# The version, and Cargo.lock with it.
sed -i.bak -E "/^\[workspace.package\]/,/^\[/ s/^version = \".*\"/version = \"$version\"/" Cargo.toml
rm -f Cargo.toml.bak
cargo check -q --workspace

# The Unreleased section becomes this version's, a new empty one over it.
date="$(date +%Y-%m-%d)"
awk -v v="$version" -v d="$date" '
    /^## Unreleased$/ && !done { print "## Unreleased\n\n## " v " (" d ")"; done = 1; next }
    { print }
' CHANGELOG.md > CHANGELOG.md.new
mv CHANGELOG.md.new CHANGELOG.md

make all

git add Cargo.toml Cargo.lock CHANGELOG.md
git commit -S -q -m "chore: release $tag"
git tag -s "$tag" -m "sokkel-tui $version ($(python3 -c 'import json;print(json.load(open("sokkel-design.json"))["commit"][:7])') of sokkel-design)"

echo "Release $tag is ready, committed and tagged (signed). To publish it:"
echo "    git push origin main $tag"
