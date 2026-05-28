#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

# Historical alias for the release editor generation path.
exec tools/release_editor_html.sh "$@"
