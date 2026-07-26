#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

target_dir="${CARGO_TARGET_DIR:-/private/tmp/puzzlebuilder-bevy-target}"
test_dir="$(mktemp -d)"
trap 'rm -rf "$test_dir"' EXIT
budget_file="tools/standalone_player_browser_baseline.json"

export_game() {
  cargo run --quiet --target-dir "$target_dir" -p html-play -- "$1" -o "$2"
}

smoke() {
  local profile="$1"
  shift
  local budget_args=()
  while IFS= read -r argument; do
    budget_args+=("$argument")
  done < <(
    node -e '
      const baseline = require("./tools/standalone_player_browser_baseline.json");
      const fixtureBudgets = baseline.fixtureBudgets[process.argv[1]];
      if (!fixtureBudgets) {
        throw new Error(`missing browser baseline profile ${process.argv[1]}`);
      }
      const budgets = {
        ...baseline.commonBudgets,
        ...fixtureBudgets,
      };
      const flags = {
        startupMs: "--max-startup-ms",
        inputLatencyMs: "--max-input-latency-ms",
        presentationCpuMicros: "--max-presentation-cpu-micros",
        submissionIntervalMicros: "--max-submission-interval-micros",
        steadyStateJsHeapGrowthBytes: "--max-steady-state-js-heap-growth-bytes",
        steadyStateWasmGrowthBytes: "--max-steady-state-wasm-growth-bytes",
        jsHeapBytes: "--max-js-heap-bytes",
        wasmLinearMemoryBytes: "--max-wasm-linear-memory-bytes",
        payloadBytes: "--max-payload-bytes",
      };
      for (const [name, value] of Object.entries(budgets)) {
        console.log(flags[name]);
        console.log(value);
      }
    ' "$profile"
  )
  node tools/standalone_player_browser_smoke.mjs \
    "${budget_args[@]}" \
    "$@"
}

export_game games/TENETEN.puzzle "$test_dir/teneten-2d.html"
smoke teneten2d \
  --html "$test_dir/teneten-2d.html" \
  --output "$test_dir/teneten-2d.png" \
  --metrics-output "$test_dir/teneten-2d.json" \
  --expected-focus title \
  --expected-viewport-count 0 \
  --key-step '{"key":"Enter","code":"Enter","keyCode":13,"expectedFocus":"playing","expectedViewportCount":1}' \
  --key-step '{"key":"ArrowRight","code":"ArrowRight","keyCode":39,"expectedFocus":"playing","expectedViewportCount":1}' \
  --resize-width 960 \
  --resize-height 540 \
  --expect-audio-running \
  --timeout 40000

export_game games/spec_3d.puzzle "$test_dir/spec-3d.html"
smoke spec3d \
  --html "$test_dir/spec-3d.html" \
  --output "$test_dir/spec-3d.png" \
  --metrics-output "$test_dir/spec-3d.json" \
  --expected-focus title \
  --expected-viewport-count 0 \
  --key-step '{"key":"Enter","code":"Enter","keyCode":13,"expectedFocus":"playing","expectedViewportCount":1}' \
  --resize-width 960 \
  --resize-height 540 \
  --expect-audio-running \
  --timeout 40000

export_game \
  crates/html_play/tests/fixtures/standalone_browser_visibility.puzzle \
  "$test_dir/visibility.html"
smoke visibility \
  --html "$test_dir/visibility.html" \
  --output "$test_dir/visibility.png" \
  --metrics-output "$test_dir/visibility.json" \
  --expected-focus title \
  --expected-viewport-count 0 \
  --key-step '{"key":"Enter","code":"Enter","keyCode":13,"expectedFocus":"playing","expectedViewportCount":1}' \
  --key-step '{"key":"ArrowRight","code":"ArrowRight","keyCode":39,"expectedFocus":"playing","expectedViewportCount":1}' \
  --exercise-visibility \
  --steady-state-duration-ms 3000 \
  --minimum-steady-state-submissions 10 \
  --timeout 40000

export_game \
  crates/html_play/tests/fixtures/standalone_browser_persistence.puzzle \
  "$test_dir/persistence.html"
smoke persistence \
  --html "$test_dir/persistence.html" \
  --output "$test_dir/persistence.png" \
  --metrics-output "$test_dir/persistence.json" \
  --expected-focus title \
  --expected-viewport-count 0 \
  --key-step '{"key":"Enter","code":"Enter","keyCode":13,"expectedFocus":"playing","expectedViewportCount":1}' \
  --key-step '{"key":"ArrowRight","code":"ArrowRight","keyCode":39,"expectedFocus":"playing","expectedViewportCount":1}' \
  --exercise-persistence \
  --post-reload-key-step '{"key":"Enter","code":"Enter","keyCode":13,"expectedFocus":"restored","expectedViewportCount":1}' \
  --post-reload-key-step '{"key":"Enter","code":"Enter","keyCode":13,"expectedFocus":"playing","expectedViewportCount":1}' \
  --post-clear-reload-key-step '{"key":"Enter","code":"Enter","keyCode":13,"expectedFocus":"playing","expectedViewportCount":1}' \
  --timeout 40000

external_dir="$test_dir/external-image"
mkdir -p "$external_dir"
cp crates/html_play/tests/fixtures/standalone_browser_external_image.puzzle "$external_dir/game.puzzle"
cp \
  crates/html_play/tests/fixtures/standalone_browser_external_image.png \
  "$external_dir/standalone_browser_external_image.png"
export_game "$external_dir/game.puzzle" "$external_dir/game.html"
smoke externalImage \
  --html "$external_dir/game.html" \
  --output "$external_dir/game.png" \
  --metrics-output "$external_dir/game.json" \
  --expected-focus title \
  --expected-viewport-count 0 \
  --expected-image "$external_dir/standalone_browser_external_image.png" \
  --expected-image-region '{"minX":0.05,"minY":0.1,"maxX":0.55,"maxY":0.9}' \
  --key-step '{"key":"Enter","code":"Enter","keyCode":13,"expectedFocus":"playing","expectedViewportCount":1}' \
  --timeout 40000

export_game \
  crates/html_play/tests/fixtures/standalone_browser_external_image_negative.puzzle \
  "$test_dir/external-image-negative.html"
external_image_negative_output="$test_dir/external-image-negative-output.txt"
if smoke externalImage \
  --html "$test_dir/external-image-negative.html" \
  --output "$test_dir/external-image-negative.png" \
  --expected-focus title \
  --expected-viewport-count 0 \
  --expected-image "$external_dir/standalone_browser_external_image.png" \
  --expected-image-region '{"minX":0.05,"minY":0.1,"maxX":0.55,"maxY":0.9}' \
  --key-step '{"key":"Enter","code":"Enter","keyCode":13,"expectedFocus":"playing","expectedViewportCount":1}' \
  --timeout 40000 >"$external_image_negative_output" 2>&1
then
  echo "external-image screenshot matcher accepted the no-image negative control" >&2
  exit 1
fi
if ! grep -F 'does not contain the fixture-owned image template' "$external_image_negative_output" >/dev/null
then
  echo "external-image negative control failed outside the owned template assertion" >&2
  cat "$external_image_negative_output" >&2
  exit 1
fi

missing_image_output="$test_dir/missing-image-output.txt"
missing_image_dir="$test_dir/missing-image"
mkdir -p "$missing_image_dir"
cp \
  crates/html_play/tests/fixtures/standalone_browser_external_image.puzzle \
  "$missing_image_dir/game.puzzle"
if export_game \
  "$missing_image_dir/game.puzzle" \
  "$test_dir/missing-image.html" >"$missing_image_output" 2>&1
then
  echo "standalone export unexpectedly accepted a missing external image" >&2
  exit 1
fi
if ! grep -F 'standalone visual image `standalone_browser_external_image.png` could not be resolved' "$missing_image_output" >/dev/null
then
  echo "standalone export missing-image diagnostic did not identify the owned asset" >&2
  cat "$missing_image_output" >&2
  exit 1
fi

node -e '
  const fs = require("node:fs");
  const summaries = process.argv.slice(1).map((file) => {
    const report = JSON.parse(fs.readFileSync(file, "utf8"));
    return {
      fixture: report.fixture,
      browser: report.environment.product,
      startupMs: report.startupMs,
      inputLatencyMsMax: Math.max(0, ...report.inputLatenciesMs),
      presentationCpuMicrosP95: report.presentationCpuMicros.p95,
      submissionIntervalMicrosMax:
        report.steadyState?.submissionIntervalMicros?.max ?? null,
      steadyStateJsHeapGrowthBytes:
        report.steadyState?.jsHeapGrowthBytes ?? null,
      steadyStateWasmGrowthBytes:
        report.steadyState?.wasmLinearMemoryGrowthBytes ?? null,
      jsHeapBytes: report.jsHeapBytes.used,
      wasmLinearMemoryBytes: report.wasmLinearMemoryBytes,
      payloadBytes: report.payloadBytes,
    };
  });
  console.log(JSON.stringify(summaries));
' \
  "$test_dir/teneten-2d.json" \
  "$test_dir/spec-3d.json" \
  "$test_dir/visibility.json" \
  "$test_dir/persistence.json" \
  "$external_dir/game.json"

echo "standalone browser contract suite passed"
echo "per-run metrics satisfied the budgets owned by $budget_file"
