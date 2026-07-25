#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { Browser, resolveChrome } from "./editor_browser_smoke.mjs";

const options = parseArgs(process.argv.slice(2));
const htmlPath = requiredFile(options.html, "--html");
const outputPath = path.resolve(requiredValue(options.output, "--output"));
const width = positiveInteger(options.width || "1280", "--width");
const height = positiveInteger(options.height || "720", "--height");
const timeoutMs = positiveInteger(options.timeout || "10000", "--timeout");
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
  await page.send("Emulation.setDeviceMetricsOverride", {
    width,
    height,
    deviceScaleFactor: 1,
    mobile: false,
  });
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

  const observation = await page.evaluateTop(`(() => {
    const status = document.querySelector("#puzzle-bevy-status");
    const fatal = document.querySelector("#puzzle-bevy-fatal");
    const canvas = document.querySelector("#puzzle-bevy");
    return {
      state: status?.dataset.state || "",
      sequence: status?.dataset.sequence || "",
      revision: status?.dataset.revision || "",
      surfaceFocus: status?.dataset.surfaceFocus || "",
      viewportCount: status?.dataset.viewportCount || "",
      fatal: fatal?.textContent?.trim() || "",
      canvasWidth: Number(canvas?.width || 0),
      canvasHeight: Number(canvas?.height || 0),
    };
  })()`);
  if (observation.state !== "ready") {
    throw new Error(
      `standalone player did not become ready: ${observation.fatal || JSON.stringify(observation)}`,
    );
  }
  if (observation.fatal) {
    throw new Error(`standalone player exposed a fatal diagnostic: ${observation.fatal}`);
  }
  const sequence = requiredUnsignedInteger(observation.sequence, "player observation sequence", 1);
  const revision = requiredUnsignedInteger(observation.revision, "player observation revision", 0);
  const viewportCount = requiredUnsignedInteger(
    observation.viewportCount,
    "player observation viewport count",
    0,
  );
  if (!observation.surfaceFocus) {
    throw new Error("player observation surface focus is missing");
  }
  if (options.expectedFocus && observation.surfaceFocus !== options.expectedFocus) {
    throw new Error(
      `expected initial surface focus ${JSON.stringify(options.expectedFocus)}, got ${JSON.stringify(observation.surfaceFocus)}`,
    );
  }
  if (observation.canvasWidth < 1 || observation.canvasHeight < 1) {
    throw new Error(
      `standalone player canvas has invalid backing size ${observation.canvasWidth}x${observation.canvasHeight}`,
    );
  }

  await page.evaluateTop(
    `new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)))`,
  );
  const finalState = await page.evaluateTop(`(() => ({
    state: document.querySelector("#puzzle-bevy-status")?.dataset.state || "",
    fatal: document.querySelector("#puzzle-bevy-fatal")?.textContent?.trim() || "",
  }))()`);
  if (finalState.state !== "ready" || finalState.fatal) {
    throw new Error(`standalone player failed before capture: ${JSON.stringify(finalState)}`);
  }
  if (page.pageErrors.length) {
    throw new Error(`standalone player browser errors:\n- ${page.pageErrors.join("\n- ")}`);
  }

  const screenshot = await page.send("Page.captureScreenshot", {
    format: "png",
    fromSurface: true,
    captureBeyondViewport: false,
  });
  const png = Buffer.from(screenshot.data || "", "base64");
  assertPngDimensions(png, width, height);
  fs.mkdirSync(path.dirname(outputPath), { recursive: true });
  temporaryOutputPath = `${outputPath}.tmp-${process.pid}`;
  fs.writeFileSync(temporaryOutputPath, png);
  fs.renameSync(temporaryOutputPath, outputPath);
  temporaryOutputPath = "";
  console.log(
    `standalone player browser smoke passed: focus=${observation.surfaceFocus} revision=${revision} viewports=${viewportCount}`,
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

function parseArgs(argv) {
  const parsed = {};
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    const field = {
      "--html": "html",
      "--output": "output",
      "--chrome": "chrome",
      "--width": "width",
      "--height": "height",
      "--timeout": "timeout",
      "--expected-focus": "expectedFocus",
    }[arg];
    if (!field) {
      throw new Error(`unknown argument: ${arg}`);
    }
    parsed[field] = argv[++index];
  }
  return parsed;
}
