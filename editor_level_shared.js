"use strict";
(() => {
    if (globalThis.PuzzleEditorLevelShared) {
        throw new Error("PuzzleEditorLevelShared was loaded more than once.");
    }
    function transformGrid({ sourceWidth, sourceHeight, targetWidth, targetHeight, mapCell, readCell, emptyCell, copyCell = (value) => value, }) {
        const result = [];
        for (let y = 0; y < targetHeight; y += 1) {
            for (let x = 0; x < targetWidth; x += 1) {
                const source = mapCell(x, y, sourceWidth, sourceHeight);
                const inBounds = source
                    && source.x >= 0
                    && source.x < sourceWidth
                    && source.y >= 0
                    && source.y < sourceHeight;
                result.push(copyCell(inBounds ? readCell(source.x, source.y) : emptyCell()));
            }
        }
        return result;
    }
    function floodFill({ start, keyOf, inBounds, valueAt, paint, neighbors, sameValue = (left, right) => left === right, }) {
        if (!inBounds(start)) {
            return 0;
        }
        const targetValue = valueAt(start);
        const visited = new Set();
        const pending = [{ ...start }];
        let changed = 0;
        while (pending.length) {
            const current = pending.pop();
            const key = keyOf(current);
            if (visited.has(key) || !inBounds(current) || !sameValue(valueAt(current), targetValue)) {
                continue;
            }
            visited.add(key);
            if (paint(current)) {
                changed += 1;
            }
            pending.push(...neighbors(current));
        }
        return changed;
    }
    function beginPointerPaint({ target, pointerId, beforeSnapshot, brush, }) {
        target?.setPointerCapture?.(pointerId);
        return {
            target,
            pointerId,
            beforeSnapshot,
            brush,
            lastKey: null,
            changed: false,
        };
    }
    function applyPointerPaint(session, key, paint) {
        if (!session || key === session.lastKey) {
            return false;
        }
        session.lastKey = key;
        if (!paint(session.brush)) {
            return false;
        }
        session.changed = true;
        return true;
    }
    function finishPointerPaint(session, pointerId, commit) {
        if (!session || session.pointerId !== pointerId) {
            return false;
        }
        if (session.target?.hasPointerCapture?.(pointerId)) {
            session.target.releasePointerCapture(pointerId);
        }
        if (session.changed) {
            commit(session.beforeSnapshot);
        }
        return true;
    }
    async function startPlaytest(lifecycle) {
        if (lifecycle.isActive()) {
            return false;
        }
        const prepared = await lifecycle.prepare();
        if (!prepared || !lifecycle.validate(prepared)) {
            return false;
        }
        lifecycle.beforeStart?.(prepared);
        lifecycle.setActive(true);
        lifecycle.updateControls();
        lifecycle.render();
        lifecycle.focus();
        requestAnimationFrame(lifecycle.focus);
        lifecycle.started?.(prepared);
        return true;
    }
    function stopPlaytest(lifecycle, options = {}) {
        if (!lifecycle.isActive() && !lifecycle.hasTransient?.()) {
            lifecycle.updateControls();
            return false;
        }
        lifecycle.setActive(false);
        lifecycle.clearTransient?.();
        lifecycle.updateControls();
        if (options.syncPreview !== false) {
            lifecycle.render();
        }
        lifecycle.stopped?.();
        return true;
    }
    function togglePlaytest(lifecycle) {
        if (lifecycle.isActive()) {
            stopPlaytest(lifecycle);
            return;
        }
        startPlaytest(lifecycle).catch((error) => lifecycle.failed(error));
    }
    function sendPlaytestKey({ active, controller, event, send, unavailable, consume = false, }) {
        if (!active) {
            return false;
        }
        if (!controller?.ready || !controller.frame?.contentWindow) {
            unavailable?.();
            return false;
        }
        const commandId = send({
            key: event.key,
            code: event.code,
            repeat: event.repeat,
            altKey: event.altKey,
            ctrlKey: event.ctrlKey,
            metaKey: event.metaKey,
            shiftKey: event.shiftKey,
            trace: false,
        }, controller.frame);
        if (consume && commandId) {
            event.preventDefault();
            event.stopPropagation();
        }
        return Boolean(commandId);
    }
    async function performSourceAction({ document, source, request, executeRequest, applyMutation, }) {
        const operation = request?.operation;
        if (!request || (operation !== "format" && operation !== "insert" && operation !== "update")) {
            throw new Error(`Unsupported level source operation ${JSON.stringify(operation)}.`);
        }
        if (operation === "update" && !Number.isInteger(request.targetStart)) {
            throw new Error("No typed level source target is selected.");
        }
        const result = await executeRequest(source, request);
        if (operation === "format") {
            return result;
        }
        if (!document) {
            throw new Error("No puzzle source document is available.");
        }
        if (!applyMutation(document, source, result.source)) {
            const error = new Error("Level source changed while the edit was being prepared; retry the edit.");
            error.code = "level-source-conflict";
            throw error;
        }
        return result;
    }
    function sourceActionErrorMessage(error, prefix) {
        const typedError = error;
        if (typedError?.code === "level-source-conflict"
            && typeof typedError.message === "string") {
            return typedError.message;
        }
        const detail = typeof typedError?.message === "string" && typedError.message
            ? typedError.message
            : String(error);
        return `${prefix}: ${detail}`;
    }
    globalThis.PuzzleEditorLevelShared = Object.freeze({
        transformGrid,
        floodFill,
        beginPointerPaint,
        applyPointerPaint,
        finishPointerPaint,
        startPlaytest,
        stopPlaytest,
        togglePlaytest,
        sendPlaytestKey,
        performSourceAction,
        sourceActionErrorMessage,
    });
})();
//# sourceMappingURL=editor_level_shared.js.map