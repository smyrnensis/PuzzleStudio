(function attachPuzzleVisualTweenCore(root) {
  function interpolate(tween, progress) {
    if (!tween || typeof tween !== "object" || Array.isArray(tween)) {
      throw new Error("Visual tween contract is missing or invalid.");
    }
    const from = requireVisualState(tween.from, "from");
    const to = requireVisualState(tween.to, "to");
    if (from.transforms.length !== to.transforms.length) {
      throw new Error("Visual tween transform counts differ.");
    }
    const amount = clampProgress(progress);
    const transforms = to.transforms.map((target, index) => (
      interpolateTransform(from.transforms[index], target, amount, index)
    ));
    return {
      transforms,
      opacity: interpolateOptionalNumber(from.opacity, to.opacity, amount, "opacity"),
    };
  }

  function resolveAnimationChannels(events) {
    if (!Array.isArray(events)) {
      throw new Error("Animation events must be an array.");
    }
    const occurrences = new Map();
    const ordered = [];
    for (const event of events) {
      if (!isTweenMove(event)) {
        ordered.push({ event });
        continue;
      }
      const occurrenceId = Number(event.occurrenceId);
      if (!Number.isSafeInteger(occurrenceId) || occurrenceId <= 0) {
        throw new Error("Tween move occurrenceId must be a positive safe integer.");
      }
      let occurrence = occurrences.get(occurrenceId);
      if (!occurrence) {
        occurrence = { occurrenceId, events: [] };
        occurrences.set(occurrenceId, occurrence);
        ordered.push({ occurrence });
      }
      occurrence.events.push(event);
    }
    return ordered.map((entry) => entry.event || composeOccurrence(entry.occurrence));
  }

  function composeOccurrence(occurrence) {
    const positionEvents = occurrence.events.filter((event) => !sameCoord(event.from, event.to));
    const visualEvents = occurrence.events.filter((event) => Boolean(event.visualTween));
    const finalEvent = occurrence.events[occurrence.events.length - 1];
    const base = positionEvents[positionEvents.length - 1] || finalEvent;
    const result = {
      ...base,
      occurrenceId: occurrence.occurrenceId,
      objectId: finalEvent.objectId,
    };
    if (positionEvents.length > 0) {
      result.from = positionEvents[0].from;
      result.to = positionEvents[positionEvents.length - 1].to;
    }
    if (visualEvents.length > 0) {
      result.visualTween = {
        from: visualEvents[0].visualTween.from,
        to: visualEvents[visualEvents.length - 1].visualTween.to,
      };
    } else {
      delete result.visualTween;
    }
    return result;
  }

  function isTweenMove(event) {
    return event?.kind === "move"
      && event?.name === "tween";
  }

  function sameCoord(left, right) {
    return Number(left?.x) === Number(right?.x)
      && Number(left?.y) === Number(right?.y)
      && Number(left?.z || 0) === Number(right?.z || 0);
  }

  function requireVisualState(value, label) {
    if (!value || typeof value !== "object" || Array.isArray(value)) {
      throw new Error(`Visual tween ${label} state is missing or invalid.`);
    }
    if (!Array.isArray(value.transforms)) {
      throw new Error(`Visual tween ${label} transforms are missing or invalid.`);
    }
    return value;
  }

  function interpolateTransform(source, target, amount, index) {
    if (!source || !target || source.kind !== target.kind) {
      throw new Error(`Visual tween transform ${index} is incompatible.`);
    }
    if (source.kind === "rotate") {
      requireSameSpace(source, target, index);
      requireSameVector(source.axis, target.axis, `rotation axis ${index}`);
      return {
        ...target,
        degrees: interpolateAngle(
          requireFiniteNumber(source.degrees, `source rotation ${index}`),
          requireFiniteNumber(target.degrees, `target rotation ${index}`),
          amount,
        ),
      };
    }
    if (source.kind === "translate" || source.kind === "scale") {
      requireSameSpace(source, target, index);
      return {
        ...target,
        value: interpolateVector(source.value, target.value, amount, `${source.kind} ${index}`),
      };
    }
    if (source.kind === "flip") {
      if (typeof source.enabled !== "boolean" || typeof target.enabled !== "boolean") {
        throw new Error(`Visual tween flip ${index} must use boolean endpoints.`);
      }
      if (source.enabled === target.enabled) {
        return target;
      }
      const fromScale = source.enabled ? -1 : 1;
      const toScale = target.enabled ? -1 : 1;
      return {
        kind: "scale",
        space: "local",
        value: [fromScale + (toScale - fromScale) * amount, 1, 1],
      };
    }
    throw new Error(`Unknown visual tween transform: ${String(source.kind)}`);
  }

  function requireSameSpace(source, target, index) {
    if ((source.space !== "world" && source.space !== "local") || source.space !== target.space) {
      throw new Error(`Visual tween transform ${index} changes space.`);
    }
  }

  function requireSameVector(source, target, label) {
    const left = requireVector3(source, `source ${label}`);
    const right = requireVector3(target, `target ${label}`);
    if (left.some((value, index) => Math.abs(value - right[index]) > 0.000000001)) {
      throw new Error(`Visual tween ${label} changes.`);
    }
  }

  function interpolateVector(source, target, amount, label) {
    const left = requireVector3(source, `source ${label}`);
    const right = requireVector3(target, `target ${label}`);
    return left.map((value, index) => value + (right[index] - value) * amount);
  }

  function requireVector3(value, label) {
    if (!Array.isArray(value) || value.length !== 3) {
      throw new Error(`Visual tween ${label} must be a three-component vector.`);
    }
    return value.map((entry, index) => requireFiniteNumber(entry, `${label}[${index}]`));
  }

  function interpolateOptionalNumber(source, target, amount, label) {
    if (source === undefined && target === undefined) {
      return undefined;
    }
    if (source === undefined || target === undefined) {
      throw new Error(`Visual tween ${label} must exist in both endpoint states.`);
    }
    const from = requireFiniteNumber(source, `source ${label}`);
    const to = requireFiniteNumber(target, `target ${label}`);
    return from + (to - from) * amount;
  }

  function interpolateAngle(source, target, amount) {
    let delta = ((target - source + 180) % 360 + 360) % 360 - 180;
    if (delta === -180) {
      delta = 180;
    }
    return source + delta * amount;
  }

  function clampProgress(value) {
    return Math.min(1, Math.max(0, requireFiniteNumber(value, "progress")));
  }

  function requireFiniteNumber(value, label) {
    const number = Number(value);
    if (!Number.isFinite(number)) {
      throw new Error(`Visual tween ${label} must be finite.`);
    }
    return number;
  }

  root.PuzzleVisualTweenCore = Object.freeze({ interpolate, resolveAnimationChannels });
})(window);
