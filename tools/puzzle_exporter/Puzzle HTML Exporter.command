#!/bin/zsh
set -u

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
DEFAULT_DIR="$REPO_ROOT/games"
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:$PATH"

show_error() {
  local message="$1"
  osascript -e 'display alert "Puzzle HTML Exporter" message "'"${message//\"/\\\"}"'" as critical' >/dev/null 2>&1 || true
}

if ! command -v cargo >/dev/null 2>&1; then
  show_error "cargo was not found. Install Rust or launch from a shell where cargo is available."
  exit 1
fi

cd "$REPO_ROOT" || {
  show_error "Could not enter repository folder: $REPO_ROOT"
  exit 1
}

input_path="$(
  DEFAULT_DIR="$DEFAULT_DIR" osascript <<'APPLESCRIPT'
set defaultDir to system attribute "DEFAULT_DIR"
try
  set defaultAlias to POSIX file defaultDir as alias
on error
  set defaultAlias to path to home folder
end try

try
  set picked to choose file with prompt "Choose a .puzzle file to export." default location defaultAlias
  return POSIX path of picked
on error number -128
  return ""
end try
APPLESCRIPT
)"

if [[ -z "$input_path" ]]; then
  exit 0
fi

input_path="${input_path%/}"
if [[ ! -f "$input_path" || "${input_path:e}" != "puzzle" ]]; then
  show_error "Choose a .puzzle file."
  exit 1
fi

input_name="${input_path:t:r}"
output_dir="${input_path:h}"

default_name="${input_name:-game}.html"
output_path="$(
  OUTPUT_DIR="$output_dir" OUTPUT_NAME="$default_name" osascript <<'APPLESCRIPT'
set outputDir to system attribute "OUTPUT_DIR"
set outputName to system attribute "OUTPUT_NAME"
try
  set outputAlias to POSIX file outputDir as alias
on error
  set outputAlias to path to desktop folder
end try

try
  set picked to choose file name with prompt "Save exported HTML as:" default name outputName default location outputAlias
  return POSIX path of picked
on error number -128
  return ""
end try
APPLESCRIPT
)"

if [[ -z "$output_path" ]]; then
  exit 0
fi

case "$output_path" in
  *.html|*.htm) ;;
  *) output_path="${output_path}.html" ;;
esac

echo "Puzzle HTML Exporter"
echo "Repository: $REPO_ROOT"
echo "Input:      $input_path"
echo "Output:     $output_path"
echo

if cargo run -p html-play -- "$input_path" -o "$output_path"; then
  osascript -e 'display notification "Exported HTML." with title "Puzzle HTML Exporter"' >/dev/null 2>&1 || true
  open "$output_path" >/dev/null 2>&1 || true
  echo
  echo "Exported: $output_path"
else
  show_error "Export failed. See the Terminal window for details."
  exit 1
fi
