const embeddedBytes = __PUZZLE_AUDIO_WORKLET_WASM_BYTES__;

class PuzzleMusicProcessor extends AudioWorkletProcessor {
  constructor(options) {
    super();
    this.renderer = null;
    this.stopped = false;
    try {
      initSync({ module: embeddedBytes });
      const contract = options?.processorOptions;
      if (!contract || contract.version !== 1 || !(contract.asset instanceof Uint8Array)) {
        throw new Error("Puzzle music processor requires typed contract version 1.");
      }
      this.renderer = new WorkletMusicRenderer(
        contract.asset,
        sampleRate,
        BigInt(contract.startFrame),
      );
      this.port.onmessage = (event) => this.applyCommand(event.data);
      this.port.postMessage({ kind: "ready", version: 1 });
    } catch (error) {
      this.fail("initialize", error);
    }
  }

  applyCommand(command) {
    try {
      if (!command || command.version !== 1) {
        throw new Error("Puzzle music command requires typed contract version 1.");
      }
      if (command.kind === "pause") {
        this.renderer.pause(BigInt(command.atFrame));
      } else if (command.kind === "resume") {
        this.renderer.resume(BigInt(command.atFrame));
      } else if (command.kind === "stop") {
        this.stopped = true;
      } else {
        throw new Error(`Unknown Puzzle music command: ${String(command.kind)}`);
      }
      this.port.postMessage({ kind: "ack", version: 1, command: command.kind });
    } catch (error) {
      this.fail("command", error);
    }
  }

  process(_inputs, outputs) {
    if (this.stopped || !this.renderer) return false;
    try {
      const channels = outputs[0];
      if (!channels || channels.length !== 2) {
        throw new Error("Puzzle music processor requires exactly two output channels.");
      }
      this.renderer.render(channels[0].length);
      this.renderer.copy_left(channels[0]);
      this.renderer.copy_right(channels[1]);
      return true;
    } catch (error) {
      this.fail("render", error);
      return false;
    }
  }

  fail(operation, error) {
    this.stopped = true;
    this.port.postMessage({
      kind: "error",
      version: 1,
      operation,
      error: error instanceof Error ? error.message : String(error),
    });
  }
}

registerProcessor("puzzle-music-processor-v1", PuzzleMusicProcessor);
