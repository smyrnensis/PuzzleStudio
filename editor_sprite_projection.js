(() => {
  "use strict";

  const runtime = Object.freeze({
    id: "editor-sprite-projection-runtime",
    runtime: Object.freeze({
      runtimeExportJson: null,
      progressIdentityKey: null,
      playerArtifact: null,
    }),
  });
  const surfaces = new Map();

  function pointerGesture(value) {
    return value === "press"
      ? "press"
      : value === "release"
        ? "release"
        : value === "leave"
          ? "leave"
          : "move";
  }

  function flushPointer(controller) {
    if (controller.pointerCommandPending || controller.pending) {
      return;
    }
    const queued = controller.queuedSpritePointer;
    controller.queuedSpritePointer = null;
    if (queued) {
      window.setTimeout(() => sendPointer(controller, queued), 0);
    }
  }

  async function sendPointer(controller, eventData) {
    const projection = surfaces.get(controller.surfaceId);
    if (!projection?.interactive) {
      return;
    }
    const frameRevision = editorRuntimeCommittedFrames.get(controller.surfaceId);
    if (canonicalU64Identity(frameRevision) === null) {
      return;
    }
    const decision = await projection.onPointer?.(eventData) ?? {
      forward: true,
      gesture: eventData.gesture,
      mutate: false,
    };
    if (!decision.forward) {
      if (String(eventData?.gesture || "") === "press") {
        controller.queuedSpritePointer = null;
      }
      return;
    }
    if (controller.pointerCommandPending || controller.pending) {
      controller.queuedSpritePointer = { ...eventData };
      return;
    }
    const gesture = pointerGesture(decision.gesture || eventData.gesture);
    if (gesture === "press" && Number(eventData.button) !== 0) {
      return;
    }
    const commandId = postEditorPreviewEnvelope(
      "PuzzleStudioEditorPointerCommand",
      {
        surfaceId: controller.surfaceId,
        committedFrameRevision: frameRevision,
        xCss: Number(eventData.xCss),
        yCss: Number(eventData.yCss),
        gesture,
        operation: null,
      },
      controller.frame,
    );
    if (!commandId) {
      return;
    }
    controller.pointerCommandPending = true;
    editorRuntimeCommands.set(commandId, {
      kind: "spritePointer",
      consumer: controller.consumer,
      surfaceId: controller.surfaceId,
      frameRevision,
      gesture,
      controller,
      mutate: decision.mutate === true,
    });
  }

  async function acceptHit(controller, message) {
    const commandId = u32CommandIdentity(message.commandId);
    const context = commandId === null ? null : editorRuntimeCommands.get(commandId);
    const surfaceId = String(message.surfaceId || "");
    const frameRevision = canonicalU64Identity(message.frameRevision);
    if (
      !context
      || frameRevision === null
      || context.kind !== "spritePointer"
      || context.surfaceId !== surfaceId
      || context.frameRevision !== frameRevision
      || context.controller !== controller
    ) {
      return;
    }
    editorRuntimeCommands.delete(commandId);
    const projection = surfaces.get(surfaceId);
    await projection?.onHit?.(message.hit ?? null, { mutate: context.mutate });
    controller.pointerCommandPending = false;
    flushPointer(controller);
  }

  function projectionCommand(draft, presentation, key) {
    const draftKey = JSON.stringify(draft);
    return (controller) => controller.visualDraftKey === draftKey
      ? {
          key,
          dispatch: (targetFrame) => postEditorPreviewEnvelope(
            "PuzzleStudioVisual3dPresentation",
            { presentation },
            targetFrame,
          ),
        }
      : {
          key,
          dispatch: (targetFrame) => postEditorPreviewEnvelope(
            "PuzzleStudioVisual3dState",
            { state: draft, presentation },
            targetFrame,
          ),
          context: { visualDraftKey: draftKey },
        };
  }

  function project({
    host,
    surfaceId,
    draft,
    presentation,
    interactive = false,
    onPointer,
    onHit,
    onReady,
    onError,
  }) {
    if (!host || typeof surfaceId !== "string" || !surfaceId) {
      throw new Error("Sprite projection requires an explicit host and surface identity.");
    }
    if (!draft || typeof draft !== "object" || !presentation || typeof presentation !== "object") {
      throw new Error("Sprite projection requires typed draft and presentation data.");
    }
    if (presentation?.surface?.surfaceId !== surfaceId) {
      throw new Error("Sprite projection presentation must name its projection surface.");
    }
    if (interactive && (typeof onPointer !== "function" || typeof onHit !== "function")) {
      throw new Error("Interactive sprite projection requires pointer and hit consumers.");
    }
    surfaces.set(surfaceId, { interactive, onPointer, onHit, onReady, onError });
    const key = `sprite-projection:${JSON.stringify({ draft, presentation })}`;
    queueEditorRuntimeDisplay({
      host,
      consumer: "spriteProjection",
      surfaceId,
      launchProfile: VISUAL_AUTHORING_LAUNCH_PROFILE,
      key,
      selectCommand: projectionCommand(draft, presentation, key),
      runtime,
      onError,
    });
  }

  registerEditorRuntimeConsumer("spriteProjection", {
    ready(controller) {
      surfaces.get(controller.surfaceId)?.onReady?.();
    },
    error(controller, detail) {
      surfaces.get(controller.surfaceId)?.onError?.(
        new Error(`${detail.label}: ${detail.message}`),
      );
    },
    pointer: sendPointer,
    hit: acceptHit,
    displayApplied(controller) {
      flushPointer(controller);
    },
    commandError(controller) {
      controller.pointerCommandPending = false;
      controller.queuedSpritePointer = null;
    },
  });

  window.PuzzleStudioSpriteProjection = Object.freeze({ project });
})();
