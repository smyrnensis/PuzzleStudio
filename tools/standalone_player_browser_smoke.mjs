#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";
import zlib from "node:zlib";
import { Browser, resolveChrome } from "./editor_browser_smoke.mjs";

const options = parseArgs(process.argv.slice(2));
const htmlPath = requiredFile(options.html, "--html");
const outputPath = path.resolve(requiredValue(options.output, "--output"));
const width = positiveInteger(options.width || "1280", "--width");
const height = positiveInteger(options.height || "720", "--height");
const timeoutMs = positiveInteger(options.timeout || "10000", "--timeout");
const keyboardSteps = options.keyboardSteps.map(parseKeyboardStep);
const postReloadKeyboardSteps = options.postReloadKeyboardSteps.map(parseKeyboardStep);
const postClearReloadKeyboardSteps = options.postClearReloadKeyboardSteps.map(parseKeyboardStep);
const expectedImagePath = options.expectedImage
  ? requiredFile(options.expectedImage, "--expected-image")
  : "";
const expectedImageRegion = options.expectedImageRegion
  ? parseNormalizedRegion(options.expectedImageRegion, "--expected-image-region")
  : { minX: 0, minY: 0, maxX: 1, maxY: 1 };
if (options.expectedImageRegion && !expectedImagePath) {
  throw new Error("--expected-image-region requires --expected-image");
}
const resizeWidth = options.resizeWidth
  ? positiveInteger(options.resizeWidth, "--resize-width")
  : 0;
const resizeHeight = options.resizeHeight
  ? positiveInteger(options.resizeHeight, "--resize-height")
  : 0;
if ((resizeWidth === 0) !== (resizeHeight === 0)) {
  throw new Error("--resize-width and --resize-height must be provided together");
}
const captureWidth = resizeWidth || width;
const captureHeight = resizeHeight || height;
const steadyStateDurationMs = options.steadyStateDurationMs
  ? positiveInteger(options.steadyStateDurationMs, "--steady-state-duration-ms")
  : 0;
const minimumSteadyStateSubmissions = options.minimumSteadyStateSubmissions
  ? positiveInteger(
      options.minimumSteadyStateSubmissions,
      "--minimum-steady-state-submissions",
    )
  : 0;
if ((steadyStateDurationMs === 0) !== (minimumSteadyStateSubmissions === 0)) {
  throw new Error(
    "--steady-state-duration-ms and --minimum-steady-state-submissions must be provided together",
  );
}
const chromePath = resolveChrome(options.chrome);
const browser = new Browser(chromePath, {
  headless: true,
  width,
  height,
  enableGpu: true,
  swiftShader: true,
});
let page;
let temporaryOutputPath = "";

try {
  await browser.start();
  page = await browser.newPage();
  await browser.activatePage(page);
  await page.send("Performance.enable");
  await installBoundaryInstrumentation(page);
  await page.send("Emulation.setDeviceMetricsOverride", {
    width,
    height,
    deviceScaleFactor: 1,
    mobile: false,
  });
  const navigationStartedAt = performance.now();
  await page.navigate(pathToFileURL(htmlPath).href);
  await page.waitForTop(
    `(() => {
      const status = document.querySelector("#puzzle-bevy-status");
      const fatal = document.querySelector("#puzzle-bevy-fatal");
      return status?.dataset.state === "ready"
        || status?.dataset.state === "fatal"
        || Boolean(fatal?.textContent?.trim());
    })()`,
    "standalone player ready or fatal observation",
    { timeoutMs },
  );

  let observation = await readPlayerObservation(page);
  assertPlayerReady(observation, "standalone player did not become ready");
  let sequence = requiredUnsignedInteger(observation.sequence, "player observation sequence", 1);
  let submissionSequence = requiredUnsignedInteger(
    observation.submissionSequence,
    "player observation submission sequence",
    1,
  );
  let revision = requiredUnsignedInteger(observation.revision, "player observation revision", 0);
  let viewportCount = requiredUnsignedInteger(
    observation.viewportCount,
    "player observation viewport count",
    0,
  );
  if (!observation.surfaceFocus) {
    throw new Error("player observation surface focus is missing");
  }
  const initialSurfaceFocus = observation.surfaceFocus;
  const initialProgressFingerprint = requiredUnsignedString(
    observation.progressFingerprint,
    "initial progress fingerprint",
  );
  if (options.expectedFocus && observation.surfaceFocus !== options.expectedFocus) {
    throw new Error(
      `expected initial surface focus ${JSON.stringify(options.expectedFocus)}, got ${JSON.stringify(observation.surfaceFocus)}`,
    );
  }
  if (
    options.expectedViewportCount !== undefined
    && viewportCount !== options.expectedViewportCount
  ) {
    throw new Error(
      `expected initial viewport count ${options.expectedViewportCount}, got ${viewportCount}`,
    );
  }
  if (observation.canvasWidth < 1 || observation.canvasHeight < 1) {
    throw new Error(
      `standalone player canvas has invalid backing size ${observation.canvasWidth}x${observation.canvasHeight}`,
    );
  }
  assertNoBrowserErrors(page);
  await awaitBrowserFrame(page, "startup");
  const startupMs = performance.now() - navigationStartedAt;
  const initialPresentationCpuMicros = observation.presentationCpuMicros;
  const firstKeyboardRun = await applyKeyboardSteps(page, keyboardSteps, {
    observation,
    sequence,
    submissionSequence,
    revision,
    viewportCount,
    timeoutMs,
    label: "keyboard",
  });
  ({ observation, sequence, submissionSequence, revision, viewportCount } = firstKeyboardRun);
  const inputLatenciesMs = [...firstKeyboardRun.inputLatenciesMs];
  const presentationCpuMicros = [
    initialPresentationCpuMicros,
    ...firstKeyboardRun.presentationCpuMicros,
  ];
  console.log("standalone smoke stage: typed input complete");

  if (options.exercisePersistence) {
    const savedProgressFingerprint = requiredUnsignedString(
      observation.progressFingerprint,
      "mutated progress fingerprint",
    );
    if (savedProgressFingerprint === initialProgressFingerprint) {
      throw new Error(
        "persistence exercise did not mutate the Rust-owned progress state before saving",
      );
    }
    await page.waitForTop(
      `window.__puzzleStandaloneSmoke?.counters?.storageSet >= 1`,
      "typed progress persistence write",
      { timeoutMs },
    );
    const beforeReloadCounters = await readBoundaryCounters(page);
    if (beforeReloadCounters.storageSet < 1) {
      throw new Error(
        `persistence exercise expected a typed write before reload: ${JSON.stringify(beforeReloadCounters)}`,
      );
    }
    const expectedRestoredFocus = initialSurfaceFocus;
    await page.navigate(pathToFileURL(htmlPath).href);
    await page.waitForTop(
      `(() => {
        const status = document.querySelector("#puzzle-bevy-status");
        const fatal = document.querySelector("#puzzle-bevy-fatal");
        return status?.dataset.state === "ready"
          || status?.dataset.state === "fatal"
          || Boolean(fatal?.textContent?.trim());
      })()`,
      "standalone player persistence reload",
      { timeoutMs },
    );
    observation = await readPlayerObservation(page);
    assertPlayerReady(observation, "standalone player failed to restore persisted progress");
    if (observation.progressFingerprint !== savedProgressFingerprint) {
      throw new Error(
        `restored Rust-owned progress state differs from the saved state: ${JSON.stringify({
          savedProgressFingerprint,
          restoredProgressFingerprint: observation.progressFingerprint,
        })}`,
      );
    }
    if (observation.surfaceFocus !== expectedRestoredFocus) {
      throw new Error(
        `persisted focus was not restored: expected ${JSON.stringify(expectedRestoredFocus)}, got ${JSON.stringify(observation.surfaceFocus)}`,
      );
    }
    sequence = requiredUnsignedInteger(
      observation.sequence,
      "reloaded player observation sequence",
      1,
    );
    submissionSequence = requiredUnsignedInteger(
      observation.submissionSequence,
      "reloaded player observation submission sequence",
      1,
    );
    revision = requiredUnsignedInteger(
      observation.revision,
      "reloaded player observation revision",
      0,
    );
    viewportCount = requiredUnsignedInteger(
      observation.viewportCount,
      "reloaded player observation viewport count",
      0,
    );
    const afterReloadCounters = await readBoundaryCounters(page);
    if (afterReloadCounters.storageGet < 1) {
      throw new Error(
        `persistence exercise expected a typed read during reload: ${JSON.stringify(afterReloadCounters)}`,
      );
    }
    const postReloadRun = await applyKeyboardSteps(page, postReloadKeyboardSteps, {
      observation,
      sequence,
      submissionSequence,
      revision,
      viewportCount,
      timeoutMs,
      label: "post-reload keyboard",
    });
    ({ observation, sequence, submissionSequence, revision, viewportCount } = postReloadRun);
    inputLatenciesMs.push(...postReloadRun.inputLatenciesMs);
    presentationCpuMicros.push(...postReloadRun.presentationCpuMicros);
    if (postReloadKeyboardSteps.length) {
      await page.waitForTop(
        `window.__puzzleStandaloneSmoke?.counters?.storageDelete >= 1`,
        "typed progress persistence delete",
        { timeoutMs },
      );
    }
    const afterPostReloadCounters = await readBoundaryCounters(page);
    if (
      postReloadKeyboardSteps.length
      && afterPostReloadCounters.storageDelete < 1
    ) {
      throw new Error(
        `persistence exercise expected a typed delete after reload: ${JSON.stringify(afterPostReloadCounters)}`,
      );
    }
    await page.navigate(pathToFileURL(htmlPath).href);
    await page.waitForTop(
      `document.querySelector("#puzzle-bevy-status")?.dataset.state === "ready"`,
      "standalone player post-clear reload",
      { timeoutMs },
    );
    observation = await readPlayerObservation(page);
    assertPlayerReady(observation, "standalone player failed after clearing persisted progress");
    if (observation.progressFingerprint !== initialProgressFingerprint) {
      throw new Error(
        `cleared Rust-owned progress state did not return to its initial value: ${JSON.stringify({
          initialProgressFingerprint,
          clearedProgressFingerprint: observation.progressFingerprint,
        })}`,
      );
    }
    sequence = requiredUnsignedInteger(observation.sequence, "post-clear sequence", 1);
    submissionSequence = requiredUnsignedInteger(
      observation.submissionSequence,
      "post-clear submission sequence",
      1,
    );
    revision = requiredUnsignedInteger(observation.revision, "post-clear revision", 0);
    viewportCount = requiredUnsignedInteger(
      observation.viewportCount,
      "post-clear viewport count",
      0,
    );
    const postClearReloadRun = await applyKeyboardSteps(page, postClearReloadKeyboardSteps, {
      observation,
      sequence,
      submissionSequence,
      revision,
      viewportCount,
      timeoutMs,
      label: "post-clear reload keyboard",
    });
    ({ observation, sequence, submissionSequence, revision, viewportCount } =
      postClearReloadRun);
    inputLatenciesMs.push(...postClearReloadRun.inputLatenciesMs);
    presentationCpuMicros.push(...postClearReloadRun.presentationCpuMicros);
  } else if (postReloadKeyboardSteps.length) {
    throw new Error("--post-reload-key-step requires --exercise-persistence");
  } else if (postClearReloadKeyboardSteps.length) {
    throw new Error("--post-clear-reload-key-step requires --exercise-persistence");
  }

  if (resizeWidth) {
    const beforeResize = {
      width: observation.canvasWidth,
      height: observation.canvasHeight,
    };
    await page.send("Emulation.setDeviceMetricsOverride", {
      width: resizeWidth,
      height: resizeHeight,
      deviceScaleFactor: 1,
      mobile: false,
    });
    await page.waitForTop(
      `(() => {
        const canvas = document.querySelector("#puzzle-bevy");
        const status = document.querySelector("#puzzle-bevy-status");
        return canvas?.width > 0
          && canvas?.height > 0
          && Number(status?.dataset.submissionSequence) > ${submissionSequence}
          && (canvas.width !== ${beforeResize.width} || canvas.height !== ${beforeResize.height});
      })()`,
      "standalone player resize",
      { timeoutMs },
    );
    observation = await readPlayerObservation(page);
    assertPlayerReady(observation, "standalone player failed after resize");
    submissionSequence = requiredUnsignedInteger(
      observation.submissionSequence,
      "resized player observation submission sequence",
      submissionSequence + 1,
    );
    presentationCpuMicros.push(observation.presentationCpuMicros);
    assertNoBrowserErrors(page);
  }
  console.log("standalone smoke stage: resize complete");

  if (options.exerciseVisibility) {
    console.log("standalone smoke stage: visibility lifecycle starting");
    const submissionBeforeVisibility = submissionSequence;
    const lifecyclePage = await browser.newPage();
    let lifecycleTargetClosed = false;
    try {
      await lifecyclePage.navigate("about:blank");
      await browser.activatePage(lifecyclePage);
      await page.waitForTop(
        `document.visibilityState === "hidden"`,
        "standalone player hidden visibility",
        { timeoutMs },
      );
      await browser.activatePage(page);
      await page.waitForTop(
        `document.visibilityState === "visible"`,
        "standalone player restored visibility",
        { timeoutMs },
      );
      await browser.closePage(lifecyclePage);
      lifecycleTargetClosed = true;
      await page.waitForTop(
        `(() => {
          const status = document.querySelector("#puzzle-bevy-status");
          return status?.dataset.state === "ready"
            && Number(status.dataset.submissionSequence) > ${submissionBeforeVisibility};
        })()`,
        "standalone player fresh submission after visibility restore",
        { timeoutMs },
      );
      observation = await readPlayerObservation(page);
      assertPlayerReady(observation, "standalone player failed after visibility restore");
      submissionSequence = requiredUnsignedInteger(
        observation.submissionSequence,
        "post-visibility submission sequence",
        submissionBeforeVisibility + 1,
      );
      presentationCpuMicros.push(observation.presentationCpuMicros);
      assertNoBrowserErrors(page);
    } finally {
      if (!lifecycleTargetClosed) {
        await browser.closePage(lifecyclePage).catch(() => {});
      }
    }
  }
  console.log("standalone smoke stage: visibility lifecycle complete");

  if (options.expectAudioRunning) {
    await page.waitForTop(
      `document.querySelector("#puzzle-bevy-status")?.dataset.audioCapability === "ready"`,
      "Rust-owned browser audio capability ready",
      { timeoutMs },
    );
    observation = await readPlayerObservation(page);
    if (observation.audioCapability !== "ready") {
      throw new Error(
        `expected Rust-owned AudioContext capability ready, got ${JSON.stringify(observation.audioCapability)}`,
      );
    }
  }

  const steadyState = steadyStateDurationMs
    ? await sampleSteadyState(page, {
        durationMs: steadyStateDurationMs,
        minimumSubmissions: minimumSteadyStateSubmissions,
        initialObservation: observation,
        presentationCpuMicros,
      })
    : null;
  if (steadyState) {
    observation = steadyState.observation;
    submissionSequence = requiredUnsignedInteger(
      observation.submissionSequence,
      "post-steady-state submission sequence",
      submissionSequence + minimumSteadyStateSubmissions,
    );
  }

  const finalState = await page.evaluateTop(`(() => ({
    state: document.querySelector("#puzzle-bevy-status")?.dataset.state || "",
    fatal: document.querySelector("#puzzle-bevy-fatal")?.textContent?.trim() || "",
  }))()`);
  if (finalState.state !== "ready" || finalState.fatal) {
    throw new Error(`standalone player failed before capture: ${JSON.stringify(finalState)}`);
  }
  assertNoBrowserErrors(page);

  const screenshot = await page.send("Page.captureScreenshot", {
    format: "png",
    fromSurface: true,
    captureBeyondViewport: false,
  });
  const png = Buffer.from(screenshot.data || "", "base64");
  assertPngDimensions(png, captureWidth, captureHeight);
  const pixelSignal = assertRenderedPixelSignal(png);
  const expectedImageSignal = expectedImagePath
    ? assertExpectedImageTemplateSignal(
        png,
        fs.readFileSync(expectedImagePath),
        expectedImageRegion,
      )
    : null;
  const performanceMetrics = metricsByName(await page.send("Performance.getMetrics"));
  const browserVersion = await page.send("Browser.getVersion");
  const boundaryCounters = await readBoundaryCounters(page);
  observation = await readPlayerObservation(page);
  assertPlayerReady(observation, "standalone player failed before metric report");
  if (
    !Number.isSafeInteger(observation.wasmLinearMemoryBytes)
    || observation.wasmLinearMemoryBytes <= 0
  ) {
    throw new Error(
      `Rust-owned WASM linear memory metric is unavailable: ${JSON.stringify(observation.wasmLinearMemoryBytes)}`,
    );
  }
  if (
    presentationCpuMicros.length === 0
    || presentationCpuMicros.some((sample) => !Number.isFinite(sample) || sample < 0)
    || !presentationCpuMicros.some((sample) => sample > 0)
  ) {
    throw new Error(
      `Rust-owned presentation CPU metric is unavailable: ${JSON.stringify(presentationCpuMicros)}`,
    );
  }
  const maximumStatusWrites = Number(observation.submissionSequence) * 11;
  if (boundaryCounters.statusAttributeWrites > maximumStatusWrites) {
    throw new Error(
      `typed status boundary writes exceeded the per-submission bound: ${JSON.stringify({
        statusAttributeWrites: boundaryCounters.statusAttributeWrites,
        submissionSequence: observation.submissionSequence,
        maximumStatusWrites,
      })}`,
    );
  }
  const report = {
    version: 1,
    fixture: path.relative(process.cwd(), htmlPath),
    viewport: { width, height },
    environment: {
      product: browserVersion.product,
      userAgent: browserVersion.userAgent,
      jsVersion: browserVersion.jsVersion,
      headless: true,
      swiftShader: true,
      deviceScaleFactor: 1,
    },
    startupMs: roundMetric(startupMs),
    inputLatenciesMs: inputLatenciesMs.map(roundMetric),
    presentationCpuMicros: summarizeSamples(presentationCpuMicros),
    steadyState: steadyState
      ? {
          durationMs: steadyState.durationMs,
          submissions: steadyState.submissions,
          submissionIntervalMicros: steadyState.submissionIntervalMicros,
          jsHeapGrowthBytes: steadyState.jsHeapGrowthBytes,
          wasmLinearMemoryGrowthBytes: steadyState.wasmLinearMemoryGrowthBytes,
        }
      : null,
    hostAdapterCalls: {
      total: boundaryCounters,
      submittedFrames: Number(observation.submissionSequence),
      maximumStatusWritesPerSubmission: 11,
    },
    jsHeapBytes: {
      used: performanceMetrics.JSHeapUsedSize ?? null,
      total: performanceMetrics.JSHeapTotalSize ?? null,
    },
    wasmLinearMemoryBytes: observation.wasmLinearMemoryBytes,
    payloadBytes: fs.statSync(htmlPath).size,
    pixelSignal,
    expectedImageSignal,
    finalObservation: {
      focus: observation.surfaceFocus,
      revision: Number(observation.revision),
      viewportCount: Number(observation.viewportCount),
      audioCapability: observation.audioCapability,
    },
  };
  if (options.metricsOutput) {
    const metricsOutputPath = path.resolve(options.metricsOutput);
    fs.mkdirSync(path.dirname(metricsOutputPath), { recursive: true });
    fs.writeFileSync(metricsOutputPath, `${JSON.stringify(report, null, 2)}\n`);
  }
  enforcePerformanceBudgets(report, options);
  fs.mkdirSync(path.dirname(outputPath), { recursive: true });
  temporaryOutputPath = `${outputPath}.tmp-${process.pid}`;
  fs.writeFileSync(temporaryOutputPath, png);
  fs.renameSync(temporaryOutputPath, outputPath);
  temporaryOutputPath = "";
  console.log(
    `standalone player browser smoke passed: focus=${observation.surfaceFocus} revision=${revision} submissions=${observation.submissionSequence} viewports=${viewportCount} keyboardSteps=${keyboardSteps.length} pixels=${pixelSignal.quantizedColorCount} startupMs=${report.startupMs} presentationCpuP95us=${report.presentationCpuMicros.p95}`,
  );
} catch (error) {
  const browserOutput = browser.output.trim();
  if (browserOutput) {
    error.message += `\nBrowser output:\n${browserOutput}`;
  }
  throw error;
} finally {
  if (temporaryOutputPath) {
    fs.rmSync(temporaryOutputPath, { force: true });
  }
  await page?.close().catch(() => {});
  await browser.close();
}

function enforcePerformanceBudgets(report, options) {
  const budgets = [
    ["startupMs", options.maxStartupMs, report.startupMs],
    [
      "inputLatencyMs",
      options.maxInputLatencyMs,
      Math.max(0, ...report.inputLatenciesMs),
    ],
    [
      "presentationCpuMicros",
      options.maxPresentationCpuMicros,
      report.presentationCpuMicros.p95,
    ],
    [
      "submissionIntervalMicros",
      options.maxSubmissionIntervalMicros,
      report.steadyState?.submissionIntervalMicros.max ?? null,
    ],
    [
      "steadyStateJsHeapGrowthBytes",
      options.maxSteadyStateJsHeapGrowthBytes,
      report.steadyState?.jsHeapGrowthBytes ?? null,
    ],
    [
      "steadyStateWasmGrowthBytes",
      options.maxSteadyStateWasmGrowthBytes,
      report.steadyState?.wasmLinearMemoryGrowthBytes ?? null,
    ],
    ["jsHeapBytes", options.maxJsHeapBytes, report.jsHeapBytes.used],
    [
      "wasmLinearMemoryBytes",
      options.maxWasmLinearMemoryBytes,
      report.wasmLinearMemoryBytes,
    ],
    ["payloadBytes", options.maxPayloadBytes, report.payloadBytes],
  ];
  for (const [name, rawLimit, measured] of budgets) {
    if (rawLimit === undefined) {
      continue;
    }
    const limit = positiveInteger(rawLimit, `--max-${performanceBudgetOption(name)}`);
    if (measured === null || measured > limit) {
      throw new Error(
        `standalone player performance budget ${name} exceeded: measured=${measured} limit=${limit}`,
      );
    }
  }
}

function performanceBudgetOption(name) {
  return {
    startupMs: "startup-ms",
    inputLatencyMs: "input-latency-ms",
    presentationCpuMicros: "presentation-cpu-micros",
    submissionIntervalMicros: "submission-interval-micros",
    steadyStateJsHeapGrowthBytes: "steady-state-js-heap-growth-bytes",
    steadyStateWasmGrowthBytes: "steady-state-wasm-growth-bytes",
    jsHeapBytes: "js-heap-bytes",
    wasmLinearMemoryBytes: "wasm-linear-memory-bytes",
    payloadBytes: "payload-bytes",
  }[name];
}

async function sampleSteadyState(page, {
  durationMs,
  minimumSubmissions,
  initialObservation,
  presentationCpuMicros,
}) {
  const startedAt = performance.now();
  const initialPerformance = metricsByName(await page.send("Performance.getMetrics"));
  let observation = initialObservation;
  let previousSubmission = requiredUnsignedInteger(
    observation.submissionSequence,
    "steady-state initial submission sequence",
    1,
  );
  const intervals = [];
  let submissions = 0;
  while (performance.now() - startedAt < durationMs) {
    await new Promise((resolve) => setTimeout(resolve, 25));
    const next = await readPlayerObservation(page);
    assertPlayerReady(next, "standalone player failed during steady-state sampling");
    const nextSubmission = requiredUnsignedInteger(
      next.submissionSequence,
      "steady-state submission sequence",
      previousSubmission,
    );
    if (nextSubmission > previousSubmission) {
      submissions += nextSubmission - previousSubmission;
      if (next.submissionIntervalMicros > 0) {
        intervals.push(next.submissionIntervalMicros);
      }
      if (next.presentationCpuMicros > 0) {
        presentationCpuMicros.push(next.presentationCpuMicros);
      }
      previousSubmission = nextSubmission;
      observation = next;
    }
  }
  if (submissions < minimumSubmissions) {
    throw new Error(
      `steady-state sampling observed ${submissions} submissions, expected at least ${minimumSubmissions}`,
    );
  }
  if (intervals.length === 0) {
    throw new Error("steady-state sampling observed no typed submission intervals");
  }
  const finalPerformance = metricsByName(await page.send("Performance.getMetrics"));
  return {
    durationMs: roundMetric(performance.now() - startedAt),
    submissions,
    submissionIntervalMicros: summarizeSamples(intervals),
    jsHeapGrowthBytes: Math.max(
      0,
      (finalPerformance.JSHeapUsedSize ?? 0)
        - (initialPerformance.JSHeapUsedSize ?? 0),
    ),
    wasmLinearMemoryGrowthBytes: Math.max(
      0,
      observation.wasmLinearMemoryBytes
        - initialObservation.wasmLinearMemoryBytes,
    ),
    observation,
  };
}

async function applyKeyboardSteps(page, steps, state) {
  let {
    observation,
    sequence,
    submissionSequence,
    revision,
    viewportCount,
    timeoutMs,
    label,
  } = state;
  const inputLatenciesMs = [];
  const presentationCpuMicros = [];
  for (const [index, step] of steps.entries()) {
    const stepLabel = `${label} step ${index + 1}`;
    const inputStartedAt = performance.now();
    await dispatchKey(page, step);
    try {
      await page.waitForTop(
        `(() => {
          const status = document.querySelector("#puzzle-bevy-status");
          const fatal = document.querySelector("#puzzle-bevy-fatal");
          return status?.dataset.state === "fatal"
            || Boolean(fatal?.textContent?.trim())
            || (
              status?.dataset.state === "ready"
              && Number(status.dataset.sequence) > ${sequence}
              && Number(status.dataset.revision) > ${revision}
            );
        })()`,
        `${stepLabel} typed transition`,
        { timeoutMs },
      );
    } catch (error) {
      const current = await readPlayerObservation(page);
      const pageErrors = page.pageErrors.length
        ? `; browser errors: ${page.pageErrors.join(" | ")}`
        : "";
      error.message += `; current observation: ${JSON.stringify(current)}${pageErrors}`;
      throw error;
    }
    const next = await readPlayerObservation(page);
    assertPlayerReady(next, `${stepLabel} failed`);
    const nextSequence = requiredUnsignedInteger(
      next.sequence,
      `${stepLabel} observation sequence`,
      1,
    );
    const nextRevision = requiredUnsignedInteger(
      next.revision,
      `${stepLabel} observation revision`,
      0,
    );
    const nextSubmissionSequence = requiredUnsignedInteger(
      next.submissionSequence,
      `${stepLabel} observation submission sequence`,
      submissionSequence + 1,
    );
    const nextViewportCount = requiredUnsignedInteger(
      next.viewportCount,
      `${stepLabel} observation viewport count`,
      0,
    );
    if (nextSequence <= sequence || nextRevision <= revision) {
      throw new Error(
        `${stepLabel} did not advance typed observation: before sequence=${sequence} revision=${revision}, after sequence=${nextSequence} revision=${nextRevision}`,
      );
    }
    if (next.surfaceFocus !== step.expectedFocus) {
      throw new Error(
        `${stepLabel} expected surface focus ${JSON.stringify(step.expectedFocus)}, got ${JSON.stringify(next.surfaceFocus)}`,
      );
    }
    if (nextViewportCount !== step.expectedViewportCount) {
      throw new Error(
        `${stepLabel} expected viewport count ${step.expectedViewportCount}, got ${nextViewportCount}`,
      );
    }
    assertNoBrowserErrors(page);
    await awaitBrowserFrame(page, stepLabel);
    inputLatenciesMs.push(performance.now() - inputStartedAt);
    presentationCpuMicros.push(next.presentationCpuMicros);
    observation = next;
    sequence = nextSequence;
    submissionSequence = nextSubmissionSequence;
    revision = nextRevision;
    viewportCount = nextViewportCount;
  }
  return {
    observation,
    sequence,
    submissionSequence,
    revision,
    viewportCount,
    inputLatenciesMs,
    presentationCpuMicros,
  };
}

async function awaitBrowserFrame(page, label) {
  const completed = await page.evaluateTop(`new Promise((resolve) => {
    requestAnimationFrame(() => requestAnimationFrame(() => resolve(true)));
  })`);
  if (completed !== true) {
    throw new Error(`${label} did not reach the next browser frame`);
  }
}

async function installBoundaryInstrumentation(page) {
  await page.send("Page.addScriptToEvaluateOnNewDocument", {
    source: `(() => {
      const counters = {
        storageGet: 0,
        storageSet: 0,
        storageDelete: 0,
        statusAttributeWrites: 0,
      };
      Object.defineProperty(window, "__puzzleStandaloneSmoke", {
        configurable: false,
        enumerable: false,
        writable: false,
        value: { counters },
      });
      for (const [name, counter] of [
        ["getItem", "storageGet"],
        ["setItem", "storageSet"],
        ["removeItem", "storageDelete"],
      ]) {
        const original = Storage.prototype[name];
        Storage.prototype[name] = function (...args) {
          counters[counter] += 1;
          return Reflect.apply(original, this, args);
        };
      }
      addEventListener("DOMContentLoaded", () => {
        const status = document.querySelector("#puzzle-bevy-status");
        if (!status) {
          return;
        }
        new MutationObserver((records) => {
          counters.statusAttributeWrites += records.filter(
            (record) => record.type === "attributes",
          ).length;
        }).observe(status, { attributes: true });
      }, { once: true });
    })();`,
  });
}

async function readBoundaryCounters(page) {
  const counters = await page.evaluateTop(
    `window.__puzzleStandaloneSmoke?.counters || null`,
  );
  if (!counters) {
    throw new Error("standalone player boundary instrumentation is unavailable");
  }
  return counters;
}

async function readPlayerObservation(page) {
  return page.evaluateTop(`(() => {
    const status = document.querySelector("#puzzle-bevy-status");
    const fatal = document.querySelector("#puzzle-bevy-fatal");
    const canvas = document.querySelector("#puzzle-bevy");
    return {
      state: status?.dataset.state || "",
      sequence: status?.dataset.sequence || "",
      submissionSequence: status?.dataset.submissionSequence || "",
      revision: status?.dataset.revision || "",
      surfaceFocus: status?.dataset.surfaceFocus || "",
      viewportCount: status?.dataset.viewportCount || "",
      submissionIntervalMicros: Number(status?.dataset.submissionIntervalMicros || 0),
      presentationCpuMicros: Number(status?.dataset.presentationCpuMicros || 0),
      wasmLinearMemoryBytes: Number(status?.dataset.wasmLinearMemoryBytes || 0),
      progressFingerprint: status?.dataset.progressFingerprint || "",
      audioCapability: status?.dataset.audioCapability || "",
      fatal: fatal?.textContent?.trim() || "",
      canvasWidth: Number(canvas?.width || 0),
      canvasHeight: Number(canvas?.height || 0),
    };
  })()`);
}

function assertPlayerReady(observation, label) {
  if (observation.state !== "ready") {
    throw new Error(`${label}: ${observation.fatal || JSON.stringify(observation)}`);
  }
  if (observation.fatal) {
    throw new Error(`${label}: ${observation.fatal}`);
  }
}

function assertNoBrowserErrors(page) {
  if (page.pageErrors.length) {
    throw new Error(`standalone player browser errors:\n- ${page.pageErrors.join("\n- ")}`);
  }
}

async function dispatchKey(page, step) {
  const event = {
    key: step.key,
    code: step.code,
    modifiers: step.modifiers,
    windowsVirtualKeyCode: step.keyCode,
    nativeVirtualKeyCode: step.keyCode,
    unmodifiedText: step.text,
    text: step.text,
  };
  await page.send("Input.dispatchKeyEvent", { ...event, type: "keyDown" });
  await page.send("Input.dispatchKeyEvent", { ...event, type: "keyUp", text: "" });
}

function parseKeyboardStep(value, index) {
  let parsed;
  try {
    parsed = JSON.parse(value);
  } catch (error) {
    throw new Error(`--key-step ${index + 1} must be valid JSON: ${error.message}`);
  }
  if (!parsed || Array.isArray(parsed) || typeof parsed !== "object") {
    throw new Error(`--key-step ${index + 1} must be a JSON object`);
  }
  const allowedFields = new Set([
    "key",
    "code",
    "keyCode",
    "modifiers",
    "text",
    "expectedFocus",
    "expectedViewportCount",
  ]);
  const unknownFields = Object.keys(parsed).filter((field) => !allowedFields.has(field));
  if (unknownFields.length) {
    throw new Error(
      `--key-step ${index + 1} has unknown fields: ${unknownFields.join(", ")}`,
    );
  }
  for (const field of ["key", "code", "expectedFocus"]) {
    if (typeof parsed[field] !== "string" || !parsed[field]) {
      throw new Error(`--key-step ${index + 1} field ${field} must be a non-empty string`);
    }
  }
  const keyCode = nonNegativeInteger(parsed.keyCode, `--key-step ${index + 1} keyCode`);
  const modifiers = nonNegativeInteger(
    parsed.modifiers ?? 0,
    `--key-step ${index + 1} modifiers`,
  );
  const expectedViewportCount = nonNegativeInteger(
    parsed.expectedViewportCount,
    `--key-step ${index + 1} expectedViewportCount`,
  );
  if (parsed.text !== undefined && typeof parsed.text !== "string") {
    throw new Error(`--key-step ${index + 1} field text must be a string`);
  }
  return {
    key: parsed.key,
    code: parsed.code,
    keyCode,
    modifiers,
    text: parsed.text ?? "",
    expectedFocus: parsed.expectedFocus,
    expectedViewportCount,
  };
}

function parseNormalizedRegion(value, label) {
  let parsed;
  try {
    parsed = JSON.parse(value);
  } catch (error) {
    throw new Error(`${label} must be valid JSON: ${error.message}`);
  }
  const fields = ["minX", "minY", "maxX", "maxY"];
  if (
    !parsed
    || Array.isArray(parsed)
    || typeof parsed !== "object"
    || Object.keys(parsed).some((field) => !fields.includes(field))
    || fields.some((field) => !Number.isFinite(parsed[field]))
    || parsed.minX < 0
    || parsed.minY < 0
    || parsed.maxX > 1
    || parsed.maxY > 1
    || parsed.minX >= parsed.maxX
    || parsed.minY >= parsed.maxY
  ) {
    throw new Error(
      `${label} must contain normalized minX/minY/maxX/maxY bounds`,
    );
  }
  return parsed;
}

function requiredUnsignedInteger(value, label, minimum) {
  if (!/^(0|[1-9][0-9]*)$/.test(value)) {
    throw new Error(`${label} is missing or malformed: ${JSON.stringify(value)}`);
  }
  const parsed = Number(value);
  if (!Number.isSafeInteger(parsed) || parsed < minimum) {
    throw new Error(`${label} is outside the supported range: ${JSON.stringify(value)}`);
  }
  return parsed;
}

function requiredUnsignedString(value, label) {
  if (!/^(0|[1-9][0-9]*)$/.test(value)) {
    throw new Error(`${label} is missing or malformed: ${JSON.stringify(value)}`);
  }
  return value;
}

function assertPngDimensions(bytes, expectedWidth, expectedHeight) {
  const signature = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
  if (bytes.length < 24 || !bytes.subarray(0, 8).equals(signature)) {
    throw new Error("browser screenshot is not a decodable PNG header");
  }
  const width = bytes.readUInt32BE(16);
  const height = bytes.readUInt32BE(20);
  if (width !== expectedWidth || height !== expectedHeight) {
    throw new Error(
      `browser screenshot is ${width}x${height}, expected ${expectedWidth}x${expectedHeight}`,
    );
  }
}

function assertRenderedPixelSignal(bytes) {
  const decoded = decodePng(bytes);
  const quantizedColors = new Set();
  let minimumLuminance = 255;
  let maximumLuminance = 0;
  let sampledPixels = 0;
  const stride = Math.max(1, Math.floor(Math.min(decoded.width, decoded.height) / 180));
  for (let y = 0; y < decoded.height; y += stride) {
    for (let x = 0; x < decoded.width; x += stride) {
      const offset = (y * decoded.width + x) * decoded.channels;
      const red = decoded.pixels[offset];
      const green = decoded.pixels[offset + 1];
      const blue = decoded.pixels[offset + 2];
      const alpha = decoded.channels === 4 ? decoded.pixels[offset + 3] : 255;
      if (alpha === 0) {
        continue;
      }
      const luminance = Math.round(0.2126 * red + 0.7152 * green + 0.0722 * blue);
      minimumLuminance = Math.min(minimumLuminance, luminance);
      maximumLuminance = Math.max(maximumLuminance, luminance);
      quantizedColors.add(`${red >> 4}:${green >> 4}:${blue >> 4}`);
      sampledPixels += 1;
    }
  }
  if (sampledPixels < 100 || quantizedColors.size < 2 || maximumLuminance - minimumLuminance < 24) {
    throw new Error(
      `standalone player screenshot has no representative rendered pixel signal: ${JSON.stringify({
        sampledPixels,
        quantizedColorCount: quantizedColors.size,
        luminanceRange: maximumLuminance - minimumLuminance,
      })}`,
    );
  }
  return {
    sampledPixels,
    quantizedColorCount: quantizedColors.size,
    luminanceRange: maximumLuminance - minimumLuminance,
  };
}

function assertExpectedImageTemplateSignal(
  screenshotBytes,
  expectedImageBytes,
  normalizedRegion,
) {
  const screenshot = decodePng(screenshotBytes);
  const expected = decodePng(expectedImageBytes);
  const opaqueColors = [];
  for (let index = 0; index < expected.pixels.length; index += expected.channels) {
    const alpha = expected.channels === 4 ? expected.pixels[index + 3] : 255;
    if (alpha >= 224) {
      opaqueColors.push([
        expected.pixels[index],
        expected.pixels[index + 1],
        expected.pixels[index + 2],
      ]);
    }
  }
  if (opaqueColors.length < 64) {
    throw new Error("expected image fixture has too few opaque template pixels");
  }
  const channelMeans = [0, 1, 2].map((channel) =>
    opaqueColors.reduce((sum, color) => sum + color[channel], 0) / opaqueColors.length
  );
  const uniformRms = Math.sqrt(
    opaqueColors.reduce(
      (sum, color) => sum + colorDistanceSquared(color, channelMeans),
      0,
    ) / (opaqueColors.length * 3),
  );
  if (uniformRms < 20) {
    throw new Error(
      `expected image fixture lacks a spatially distinguishable color template: uniformRms=${uniformRms}`,
    );
  }

  const match = findBestImageTemplate(
    screenshot,
    expected,
    normalizedRegion,
  );
  if (!match || match.rms > 48) {
    throw new Error(
      `standalone player screenshot does not contain the fixture-owned image template in its expected viewport region: ${JSON.stringify({
        normalizedRegion,
        uniformRms,
        match,
      })}`,
    );
  }
  return {
    normalizedRegion,
    uniformRms: roundMetric(uniformRms),
    templateRms: roundMetric(match.rms),
    bounds: {
      x: match.x,
      y: match.y,
      width: match.width,
      height: match.height,
    },
  };
}

function findBestImageTemplate(screenshot, expected, normalizedRegion) {
  const region = {
    minX: Math.floor(normalizedRegion.minX * screenshot.width),
    minY: Math.floor(normalizedRegion.minY * screenshot.height),
    maxX: Math.ceil(normalizedRegion.maxX * screenshot.width),
    maxY: Math.ceil(normalizedRegion.maxY * screenshot.height),
  };
  const regionWidth = region.maxX - region.minX;
  const regionHeight = region.maxY - region.minY;
  const aspect = expected.width / expected.height;
  const maximumWidth = Math.min(regionWidth, Math.floor(regionHeight * aspect));
  let best = null;
  for (let width = 24; width <= maximumWidth; width += 12) {
    const height = Math.max(1, Math.round(width / aspect));
    const step = Math.max(4, Math.floor(width / 8));
    for (let y = region.minY; y + height <= region.maxY; y += step) {
      for (let x = region.minX; x + width <= region.maxX; x += step) {
        const rms = imageTemplateRms(screenshot, expected, { x, y, width, height }, 4);
        if (!best || rms < best.rms) {
          best = { x, y, width, height, rms };
        }
      }
    }
  }
  if (!best) {
    return null;
  }
  const refinement = Math.max(4, Math.floor(best.width / 8));
  let refined = best;
  for (
    let width = Math.max(16, best.width - refinement);
    width <= best.width + refinement;
    width += 2
  ) {
    const height = Math.max(1, Math.round(width / aspect));
    for (let y = best.y - refinement; y <= best.y + refinement; y += 2) {
      for (let x = best.x - refinement; x <= best.x + refinement; x += 2) {
        if (
          x < region.minX
          || y < region.minY
          || x + width > region.maxX
          || y + height > region.maxY
        ) {
          continue;
        }
        const rms = imageTemplateRms(screenshot, expected, { x, y, width, height }, 2);
        if (rms < refined.rms) {
          refined = { x, y, width, height, rms };
        }
      }
    }
  }
  refined.rms = imageTemplateRms(screenshot, expected, refined, 1);
  return refined;
}

function imageTemplateRms(screenshot, expected, bounds, stride) {
  let distance = 0;
  let samples = 0;
  for (let expectedY = 0; expectedY < expected.height; expectedY += stride) {
    for (let expectedX = 0; expectedX < expected.width; expectedX += stride) {
      const expectedOffset =
        (expectedY * expected.width + expectedX) * expected.channels;
      const alpha =
        expected.channels === 4 ? expected.pixels[expectedOffset + 3] : 255;
      if (alpha < 224) {
        continue;
      }
      const screenshotX = Math.min(
        screenshot.width - 1,
        bounds.x
          + Math.floor(((expectedX + 0.5) / expected.width) * bounds.width),
      );
      const screenshotY = Math.min(
        screenshot.height - 1,
        bounds.y
          + Math.floor(((expectedY + 0.5) / expected.height) * bounds.height),
      );
      const screenshotOffset =
        (screenshotY * screenshot.width + screenshotX) * screenshot.channels;
      distance += colorDistanceSquared(
        [
          expected.pixels[expectedOffset],
          expected.pixels[expectedOffset + 1],
          expected.pixels[expectedOffset + 2],
        ],
        [
          screenshot.pixels[screenshotOffset],
          screenshot.pixels[screenshotOffset + 1],
          screenshot.pixels[screenshotOffset + 2],
        ],
      );
      samples += 1;
    }
  }
  return Math.sqrt(distance / Math.max(1, samples * 3));
}

function colorDistanceSquared(left, right) {
  return left.reduce((sum, channel, index) => {
    const delta = channel - right[index];
    return sum + delta * delta;
  }, 0);
}

function decodePng(bytes) {
  const chunks = [];
  let offset = 8;
  let width = 0;
  let height = 0;
  let bitDepth = 0;
  let colorType = 0;
  while (offset + 12 <= bytes.length) {
    const length = bytes.readUInt32BE(offset);
    const type = bytes.toString("ascii", offset + 4, offset + 8);
    const data = bytes.subarray(offset + 8, offset + 8 + length);
    if (type === "IHDR") {
      width = data.readUInt32BE(0);
      height = data.readUInt32BE(4);
      bitDepth = data[8];
      colorType = data[9];
      if (data[10] !== 0 || data[11] !== 0 || data[12] !== 0) {
        throw new Error("browser screenshot uses unsupported PNG encoding");
      }
    } else if (type === "IDAT") {
      chunks.push(data);
    } else if (type === "IEND") {
      break;
    }
    offset += 12 + length;
  }
  const channels = colorType === 6 ? 4 : colorType === 2 ? 3 : 0;
  if (!width || !height || bitDepth !== 8 || !channels || !chunks.length) {
    throw new Error(
      `browser screenshot uses unsupported PNG pixel format: bitDepth=${bitDepth} colorType=${colorType}`,
    );
  }
  const rowBytes = width * channels;
  const inflated = zlib.inflateSync(Buffer.concat(chunks));
  if (inflated.length !== height * (rowBytes + 1)) {
    throw new Error("browser screenshot PNG scanline length is invalid");
  }
  const pixels = Buffer.alloc(width * height * channels);
  for (let y = 0; y < height; y += 1) {
    const input = y * (rowBytes + 1);
    const output = y * rowBytes;
    const filter = inflated[input];
    for (let x = 0; x < rowBytes; x += 1) {
      const raw = inflated[input + 1 + x];
      const left = x >= channels ? pixels[output + x - channels] : 0;
      const above = y > 0 ? pixels[output + x - rowBytes] : 0;
      const upperLeft =
        y > 0 && x >= channels ? pixels[output + x - rowBytes - channels] : 0;
      pixels[output + x] = unfilterPngByte(filter, raw, left, above, upperLeft);
    }
  }
  return { width, height, channels, pixels };
}

function unfilterPngByte(filter, raw, left, above, upperLeft) {
  if (filter === 0) {
    return raw;
  }
  if (filter === 1) {
    return (raw + left) & 0xff;
  }
  if (filter === 2) {
    return (raw + above) & 0xff;
  }
  if (filter === 3) {
    return (raw + Math.floor((left + above) / 2)) & 0xff;
  }
  if (filter === 4) {
    const predictor = left + above - upperLeft;
    const leftDistance = Math.abs(predictor - left);
    const aboveDistance = Math.abs(predictor - above);
    const upperLeftDistance = Math.abs(predictor - upperLeft);
    const paeth =
      leftDistance <= aboveDistance && leftDistance <= upperLeftDistance
        ? left
        : aboveDistance <= upperLeftDistance
          ? above
          : upperLeft;
    return (raw + paeth) & 0xff;
  }
  throw new Error(`browser screenshot PNG uses unknown scanline filter ${filter}`);
}

function metricsByName(result) {
  return Object.fromEntries((result.metrics || []).map(({ name, value }) => [name, value]));
}

function summarizeSamples(samples) {
  const sorted = samples.slice().sort((left, right) => left - right);
  const percentile = (fraction) =>
    sorted[Math.min(sorted.length - 1, Math.floor(sorted.length * fraction))] ?? 0;
  const total = samples.reduce((sum, value) => sum + value, 0);
  return {
    count: samples.length,
    mean: roundMetric(samples.length ? total / samples.length : 0),
    p50: roundMetric(percentile(0.5)),
    p95: roundMetric(percentile(0.95)),
    max: roundMetric(sorted.at(-1) ?? 0),
  };
}

function roundMetric(value) {
  return Math.round(Number(value) * 100) / 100;
}

function requiredFile(value, label) {
  const resolved = path.resolve(requiredValue(value, label));
  if (!fs.statSync(resolved, { throwIfNoEntry: false })?.isFile()) {
    throw new Error(`${label} does not exist: ${resolved}`);
  }
  return resolved;
}

function requiredValue(value, label) {
  if (!value) {
    throw new Error(`${label} is required`);
  }
  return value;
}

function positiveInteger(value, label) {
  const parsed = Number(value);
  if (!Number.isSafeInteger(parsed) || parsed <= 0) {
    throw new Error(`${label} must be a positive integer`);
  }
  return parsed;
}

function nonNegativeInteger(value, label) {
  const parsed = Number(value);
  if (!Number.isSafeInteger(parsed) || parsed < 0) {
    throw new Error(`${label} must be a non-negative integer`);
  }
  return parsed;
}

function parseArgs(argv) {
  const parsed = {
    keyboardSteps: [],
    postReloadKeyboardSteps: [],
    postClearReloadKeyboardSteps: [],
  };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--key-step") {
      parsed.keyboardSteps.push(requiredValue(argv[++index], "--key-step"));
      continue;
    }
    if (arg === "--post-reload-key-step") {
      parsed.postReloadKeyboardSteps.push(
        requiredValue(argv[++index], "--post-reload-key-step"),
      );
      continue;
    }
    if (arg === "--post-clear-reload-key-step") {
      parsed.postClearReloadKeyboardSteps.push(
        requiredValue(argv[++index], "--post-clear-reload-key-step"),
      );
      continue;
    }
    const field = {
      "--html": "html",
      "--output": "output",
      "--chrome": "chrome",
      "--width": "width",
      "--height": "height",
      "--timeout": "timeout",
      "--resize-width": "resizeWidth",
      "--resize-height": "resizeHeight",
      "--metrics-output": "metricsOutput",
      "--max-startup-ms": "maxStartupMs",
      "--max-input-latency-ms": "maxInputLatencyMs",
      "--max-presentation-cpu-micros": "maxPresentationCpuMicros",
      "--max-submission-interval-micros": "maxSubmissionIntervalMicros",
      "--max-steady-state-js-heap-growth-bytes": "maxSteadyStateJsHeapGrowthBytes",
      "--max-steady-state-wasm-growth-bytes": "maxSteadyStateWasmGrowthBytes",
      "--max-js-heap-bytes": "maxJsHeapBytes",
      "--max-wasm-linear-memory-bytes": "maxWasmLinearMemoryBytes",
      "--max-payload-bytes": "maxPayloadBytes",
      "--expected-image": "expectedImage",
      "--expected-image-region": "expectedImageRegion",
      "--expected-focus": "expectedFocus",
      "--expected-viewport-count": "expectedViewportCount",
      "--steady-state-duration-ms": "steadyStateDurationMs",
      "--minimum-steady-state-submissions": "minimumSteadyStateSubmissions",
    }[arg];
    if (arg === "--exercise-visibility") {
      parsed.exerciseVisibility = true;
      continue;
    }
    if (arg === "--exercise-persistence") {
      parsed.exercisePersistence = true;
      continue;
    }
    if (arg === "--expect-audio-running") {
      parsed.expectAudioRunning = true;
      continue;
    }
    if (!field) {
      throw new Error(`unknown argument: ${arg}`);
    }
    parsed[field] = argv[++index];
  }
  if (parsed.expectedViewportCount !== undefined) {
    parsed.expectedViewportCount = nonNegativeInteger(
      parsed.expectedViewportCount,
      "--expected-viewport-count",
    );
  }
  return parsed;
}
