(function attachPuzzleVisualTweenCore(root) {
  function interpolate(tween, progress) {
    if (!tween || typeof tween !== "object" || !Array.isArray(tween.transforms)) {
      throw new Error("Prepared visual tween contract is missing or invalid.");
    }
    const amount = clampProgress(progress);
    return {
      transforms: tween.transforms.map((channel, index) => sampleTransform(channel, amount, index)),
      opacity: tween.opacity == null ? undefined : sampleScalar(tween.opacity, amount, "opacity"),
    };
  }

  function sampleTransform(channel, amount, index) {
    if (!channel || typeof channel !== "object") {
      throw new Error(`Prepared visual tween channel ${index} is invalid.`);
    }
    if (channel.kind === "rotate") {
      requireSpace(channel.space, index);
      const axis = requireVector3(channel.axis, `rotation axis ${index}`);
      return {
        kind: "rotate",
        degrees: sampleNumbers(channel.startDegrees, channel.deltaDegrees, amount, `rotation ${index}`),
        axis,
        space: channel.space,
      };
    }
    if (channel.kind === "translate" || channel.kind === "scale") {
      requireSpace(channel.space, index);
      return {
        kind: channel.kind,
        value: sampleVector(channel.start, channel.delta, amount, `${channel.kind} ${index}`),
        space: channel.space,
      };
    }
    throw new Error(`Unknown prepared visual tween channel: ${String(channel.kind)}`);
  }

  function sampleScalar(channel, amount, label) {
    if (!channel || typeof channel !== "object") {
      throw new Error(`Prepared visual tween ${label} channel is invalid.`);
    }
    return sampleNumbers(channel.start, channel.delta, amount, label);
  }

  function sampleVector(start, delta, amount, label) {
    const starts = requireVector3(start, `${label} start`);
    const deltas = requireVector3(delta, `${label} delta`);
    return starts.map((value, index) => value + deltas[index] * amount);
  }

  function sampleNumbers(start, delta, amount, label) {
    return requireFiniteNumber(start, `${label} start`)
      + requireFiniteNumber(delta, `${label} delta`) * amount;
  }

  function requireVector3(value, label) {
    if (!Array.isArray(value) || value.length !== 3) {
      throw new Error(`Prepared visual tween ${label} must be a three-component vector.`);
    }
    return value.map((entry, index) => requireFiniteNumber(entry, `${label}[${index}]`));
  }

  function requireSpace(space, index) {
    if (space !== "world" && space !== "local") {
      throw new Error(`Prepared visual tween channel ${index} has invalid space.`);
    }
  }

  function clampProgress(value) {
    return Math.min(1, Math.max(0, requireFiniteNumber(value, "progress")));
  }

  function requireFiniteNumber(value, label) {
    const number = Number(value);
    if (!Number.isFinite(number)) {
      throw new Error(`Prepared visual tween ${label} must be finite.`);
    }
    return number;
  }

  root.PuzzleVisualTweenCore = Object.freeze({ interpolate });
})(window);
