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
<<<<<<< 98103c50f8b944de451f9367f6d21d34bc55e3b6
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
=======
    if (channel.kind === "rotate") {
      requireSpace(channel.space, index);
      const axis = requireVector3(channel.axis, `rotation axis ${index}`);
>>>>>>> dcbfa1ffd87009bdea112730e23f98056f777544
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
