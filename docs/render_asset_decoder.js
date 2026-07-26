(function () {
  async function decodeRenderImage(source) {
    const image = new Image();
    const loaded = new Promise((resolve, reject) => {
      image.addEventListener("load", resolve, { once: true });
      image.addEventListener("error", () => reject(new Error(`Render image could not be loaded: ${source}`)), { once: true });
    });
    image.src = window.PuzzleAssets.url(source);
    await loaded;
    if (image.naturalWidth < 1 || image.naturalHeight < 1
        || image.naturalWidth > 0xffff || image.naturalHeight > 0xffff) {
      throw new Error(`Render image dimensions are outside the runtime contract: ${source}`);
    }
    const canvas = document.createElement("canvas");
    canvas.width = image.naturalWidth;
    canvas.height = image.naturalHeight;
    const context = canvas.getContext("2d", { willReadFrequently: true });
    context.drawImage(image, 0, 0);
    return {
      source,
      width: image.naturalWidth,
      height: image.naturalHeight,
      rgba8Srgb: [...context.getImageData(0, 0, canvas.width, canvas.height).data],
    };
  }

  async function hydrateRenderSceneImages(wasmModule, renderScene) {
    if (typeof wasmModule?.hydrate_render_scene_images !== "function") {
      throw new Error("Puzzle game WASM runtime does not expose decoded-image hydration.");
    }
    const sources = new Set();
    for (const clip of renderScene?.clips || []) {
      for (const frame of clip?.frames || []) {
        if (frame?.kind === "external_image" && typeof frame.source === "string") {
          sources.add(frame.source);
        }
      }
    }
    if (sources.size === 0) {
      return renderScene;
    }
    const assets = await Promise.all([...sources].map(decodeRenderImage));
    return JSON.parse(wasmModule.hydrate_render_scene_images(
      JSON.stringify(renderScene),
      JSON.stringify(assets),
    ));
  }

  window.PuzzleRenderAssetDecoder = { hydrateRenderSceneImages };
}());
