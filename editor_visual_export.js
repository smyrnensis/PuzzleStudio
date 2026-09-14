// Binary serialization for editor-owned visual drafts. Input is a detached
// snapshot: RGBA palette, indexed frames, extent and (for 2D) playback delay.
// This module has no access to source, live playback, or editor state.
(() => {
  "use strict";

  function validate(snapshot, isVox) {
    const { width, height, palette, frames, frameDelayMs } = snapshot;
    const depth = isVox ? snapshot.depth : 1;
    if (!Number.isInteger(width) || !Number.isInteger(height)
      || width < 1 || height < 1 || width > 64 || height > 64
      || !Number.isInteger(depth) || depth < 1 || depth > 64
      || !Array.isArray(palette) || palette.length > 255
      || palette.some(color => !Array.isArray(color) || color.length !== 4
        || color.some(v => !Number.isInteger(v) || v < 0 || v > 255))
      || !Array.isArray(frames) || frames.length < 1 || frames.length > 24
      || frames.some(frame => !Array.isArray(frame) || frame.length !== width * height * depth
        || frame.some(index => index !== null
          && (!Number.isInteger(index) || index < 0 || index >= palette.length)))
      || (!isVox && (!Number.isFinite(frameDelayMs) || frameDelayMs <= 0 || frameDelayMs > 5000))) {
      throw new Error(`Invalid ${isVox ? "3D" : "2D"} visual export snapshot.`);
    }
  }

  async function png(snapshot) {
    const { width, height, palette, frames } = snapshot;
    // Serialize straight RGBA. Canvas readback loses RGB precision when alpha
    // is low because its backing store uses premultiplied colors.
    const stride = width * 4 + 1;
    const scanlines = new Uint8Array(stride * height);
    frames[0].forEach((index, pixel) => {
      const row = Math.floor(pixel / width);
      if (index !== null) scanlines.set(palette[index], row * stride + 1 + (pixel % width) * 4);
    });
    const chunk = (type, data) => {
      const bytes = new Uint8Array(data.length + 12);
      const view = new DataView(bytes.buffer);
      view.setUint32(0, data.length);
      bytes.set(new TextEncoder().encode(type), 4);
      bytes.set(data, 8);
      // Shared browser binary-export checksum owner (editor_import_export.js).
      view.setUint32(bytes.length - 4, crc32(bytes.subarray(4, bytes.length - 4)));
      return bytes;
    };
    const header = new Uint8Array(13);
    const view = new DataView(header.buffer);
    view.setUint32(0, width);
    view.setUint32(4, height);
    header.set([8, 6], 8); // 8-bit straight RGBA, no interlacing.
    const compressed = await new Response(new Blob([scanlines]).stream()
      .pipeThrough(new CompressionStream("deflate"))).arrayBuffer();
    return new Blob([
      new Uint8Array([137, 80, 78, 71, 13, 10, 26, 10]),
      chunk("IHDR", header), chunk("IDAT", new Uint8Array(compressed)),
      chunk("IEND", new Uint8Array()),
    ], { type: "image/png" });
  }

  // GIF permits a clear code at any point. Small indexed sprites use bounded
  // literal runs, clearing before dictionary growth changes the code width.
  // This keeps memory and output bounded without a second color quantizer.
  function literalLzw(indexes, minimumBits) {
    const clear = 1 << minimumBits;
    const bits = minimumBits + 1;
    const bytes = [];
    let buffer = 0;
    let bufferedBits = 0;
    const emit = code => {
      buffer |= code << bufferedBits;
      bufferedBits += bits;
      while (bufferedBits >= 8) {
        bytes.push(buffer & 255);
        buffer >>>= 8;
        bufferedBits -= 8;
      }
    };
    for (let offset = 0; offset < indexes.length; offset += clear - 2) {
      emit(clear);
      for (const index of indexes.slice(offset, offset + clear - 2)) emit(index);
    }
    emit(clear + 1);
    if (bufferedBits) bytes.push(buffer & 255);
    return bytes;
  }

  function gif(snapshot) {
    const { width, height, palette, frames, frameDelayMs } = snapshot;
    // Index zero is reserved for transparent background and empty cells.
    const tableBits = Math.max(1, Math.ceil(Math.log2(palette.length + 1)));
    const minimumBits = Math.max(2, tableBits);
    const bytes = [];
    const word = value => bytes.push(value & 255, (value >> 8) & 255);
    const text = value => bytes.push(...new TextEncoder().encode(value));
    text("GIF89a");
    word(width);
    word(height);
    bytes.push(0x80 | 0x70 | (tableBits - 1), 0, 0);
    bytes.push(0, 0, 0);
    for (const color of palette) bytes.push(...color.slice(0, 3));
    for (let i = palette.length + 1; i < (1 << tableBits); i++) bytes.push(0, 0, 0);
    bytes.push(0x21, 0xff, 11);
    text("NETSCAPE2.0");
    bytes.push(3, 1, 0, 0, 0); // Repeat forever.
    const delay = Math.max(2, Math.round(frameDelayMs / 10));
    for (const frame of frames) {
      bytes.push(0x21, 0xf9, 4, 9); // Clear frame to transparent background.
      word(delay);
      bytes.push(0, 0, 0x2c);
      word(0);
      word(0);
      word(width);
      word(height);
      bytes.push(0, minimumBits);
      const data = literalLzw(frame.map(index =>
        index === null || palette[index][3] < 128 ? 0 : index + 1), minimumBits);
      for (let offset = 0; offset < data.length; offset += 255) {
        const block = data.slice(offset, offset + 255);
        bytes.push(block.length, ...block);
      }
      bytes.push(0);
    }
    bytes.push(0x3b);
    return new Blob([new Uint8Array(bytes)], { type: "image/gif" });
  }

  // MagicaVoxel v150 core plus the published scene/animation extension:
  // https://github.com/ephtracy/voxel-model
  // Positive world XYZ and Z-up are shared with the editor draft. Each shape
  // keyframe references one full model; camera and source transforms are absent.
  function vox(snapshot) {
    const { width, height, depth, palette, frames } = snapshot;
    const text = value => new TextEncoder().encode(value);
    const ints = (...values) => {
      const bytes = new Uint8Array(values.length * 4);
      const view = new DataView(bytes.buffer);
      values.forEach((value, index) => view.setInt32(index * 4, value, true));
      return bytes;
    };
    const concat = parts => {
      const bytes = new Uint8Array(parts.reduce((sum, part) => sum + part.length, 0));
      let offset = 0;
      for (const part of parts) { bytes.set(part, offset); offset += part.length; }
      return bytes;
    };
    const dict = (entries = []) => concat([ints(entries.length), ...entries.flatMap(pair =>
      pair.flatMap(value => { const bytes = text(value); return [ints(bytes.length), bytes]; }))]);
    const chunk = (id, content, children = []) => concat([
      text(id), ints(content.length, children.reduce((sum, child) => sum + child.length, 0)),
      content, ...children,
    ]);
    const children = [];
    for (const frame of frames) {
      const cells = [];
      frame.forEach((color, index) => {
        if (color === null) return;
        cells.push(index % width, Math.floor(index / width) % height,
          Math.floor(index / (width * height)), color + 1);
      });
      children.push(chunk("SIZE", ints(width, height, depth)),
        chunk("XYZI", concat([ints(cells.length / 4), new Uint8Array(cells)])));
    }
    const transform = (id, child) => chunk("nTRN", concat([
      ints(id), dict(), ints(child, -1, -1, 1), dict(),
    ]));
    children.push(transform(0, 1), chunk("nGRP", concat([ints(1), dict(), ints(1, 2)])),
      transform(2, 3), chunk("nSHP", concat([
        ints(3), dict(), ints(frames.length),
        ...frames.map((_, index) => concat([ints(index), dict([["_f", String(index)]])])),
      ])));
    const colors = new Uint8Array(1024);
    palette.forEach((color, index) => colors.set(color, index * 4));
    children.push(chunk("RGBA", colors));
    return new Blob([text("VOX "), ints(150), chunk("MAIN", new Uint8Array(), children)],
      { type: "application/octet-stream" });
  }

  async function encode(snapshot, format) {
    validate(snapshot, format === "vox");
    if (format === "vox") return vox(snapshot);
    if (format === "png") {
      if (snapshot.frames.length !== 1) throw new Error("PNG requires one selected frame.");
      return png(snapshot);
    }
    if (format === "gif") return gif(snapshot);
    throw new Error(`Unsupported visual export format: ${format}`);
  }

  window.PuzzleStudioVisualExport = Object.freeze({ encode });
})();
