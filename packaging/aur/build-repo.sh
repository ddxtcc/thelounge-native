#!/bin/sh
set -eu

repo=${REPO_NAME:-lounge-native}
repo_dir=${1:-"$PWD/repo"}
shift || true

if ! command -v repo-add >/dev/null 2>&1; then
    echo "repo-add is required (pacman package)." >&2
    exit 1
fi
if [ "$#" -eq 0 ]; then
    set -- "$PWD"/*.pkg.tar.zst
fi
if [ ! -e "$1" ]; then
    echo "No packages found. Pass package paths or build one first." >&2
    exit 1
fi

mkdir -p "$repo_dir"
for package in "$@"; do
    cp -f "$package" "$repo_dir/"
done

set -- "$repo_dir"/*.pkg.tar.zst
if [ -n "${GPG_KEY:-}" ]; then
    repo-add --sign --key "$GPG_KEY" "$repo_dir/$repo.db.tar.gz" "$@"
else
    repo-add "$repo_dir/$repo.db.tar.gz" "$@"
fi
echo "Repository database: $repo_dir/$repo.db.tar.gz"
