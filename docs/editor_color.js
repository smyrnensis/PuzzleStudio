(() => {
  function clamp(value, min, max) {
    return Math.max(min, Math.min(max, Number(value) || 0));
  }

  function normalizeColor(value) {
    const raw = String(value || "").trim();
    const body = raw.startsWith("#") ? raw.slice(1) : raw;
    if (/^[0-9a-fA-F]{3,4}$/.test(body)) {
      const red = body[0] + body[0];
      const green = body[1] + body[1];
      const blue = body[2] + body[2];
      const alpha = body.length === 4 ? body[3] + body[3] : "ff";
      return `#${red}${green}${blue}${alpha === "ff" ? "" : alpha}`.toLowerCase();
    }
    if (/^[0-9a-fA-F]{6}([0-9a-fA-F]{2})?$/.test(body)) {
      return `#${body}`.toLowerCase();
    }
    return "#000000";
  }

  function parseColor(value) {
    const normalized = normalizeColor(value);
    const body = normalized.slice(1);
    return {
      red: parseInt(body.slice(0, 2), 16),
      green: parseInt(body.slice(2, 4), 16),
      blue: parseInt(body.slice(4, 6), 16),
      alpha: body.length === 8 ? parseInt(body.slice(6, 8), 16) : 255,
    };
  }

  function formatColor({ red, green, blue, alpha = 255 }) {
    const rgb = [red, green, blue]
      .map((item) => Math.round(clamp(item, 0, 255)).toString(16).padStart(2, "0"))
      .join("");
    const nextAlpha = Math.round(clamp(alpha, 0, 255));
    return `#${rgb}${nextAlpha >= 255 ? "" : nextAlpha.toString(16).padStart(2, "0")}`;
  }

  function rgbToHsv({ red, green, blue }) {
    const r = red / 255;
    const g = green / 255;
    const b = blue / 255;
    const max = Math.max(r, g, b);
    const min = Math.min(r, g, b);
    const delta = max - min;
    let hue = 0;
    if (delta > 0) {
      if (max === r) {
        hue = 60 * (((g - b) / delta) % 6);
      } else if (max === g) {
        hue = 60 * (((b - r) / delta) + 2);
      } else {
        hue = 60 * (((r - g) / delta) + 4);
      }
    }
    return {
      hue: hue < 0 ? hue + 360 : hue,
      saturation: max === 0 ? 0 : delta / max,
      value: max,
    };
  }

  function hsvToRgb({ hue, saturation, value }) {
    const h = ((Number(hue) || 0) % 360 + 360) % 360;
    const s = clamp(saturation, 0, 1);
    const v = clamp(value, 0, 1);
    const c = v * s;
    const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
    const m = v - c;
    let r = 0;
    let g = 0;
    let b = 0;
    if (h < 60) {
      r = c; g = x; b = 0;
    } else if (h < 120) {
      r = x; g = c; b = 0;
    } else if (h < 180) {
      r = 0; g = c; b = x;
    } else if (h < 240) {
      r = 0; g = x; b = c;
    } else if (h < 300) {
      r = x; g = 0; b = c;
    } else {
      r = c; g = 0; b = x;
    }
    return {
      red: Math.round((r + m) * 255),
      green: Math.round((g + m) * 255),
      blue: Math.round((b + m) * 255),
    };
  }

  function colorWithAlpha(color, alphaPercent) {
    const parsed = parseColor(color);
    parsed.alpha = Math.round((clamp(alphaPercent, 0, 100) / 100) * 255);
    return formatColor(parsed);
  }

  function create(options = {}) {
    const root = document.createElement("span");
    root.className = ["color-editor", options.className || ""].filter(Boolean).join(" ");
    const plane = document.createElement("span");
    plane.className = "color-editor-plane";
    plane.tabIndex = 0;
    plane.setAttribute("role", "slider");
    plane.setAttribute("aria-label", options.ariaLabel || "Color");
    const cursor = document.createElement("span");
    cursor.className = "color-editor-plane-cursor";
    plane.append(cursor);

    const hue = document.createElement("input");
    hue.className = "color-editor-hue";
    hue.type = "range";
    hue.min = "0";
    hue.max = "360";
    hue.step = "1";
    hue.setAttribute("aria-label", `${options.ariaLabel || "Color"} hue`);

    const alpha = document.createElement("input");
    alpha.className = "color-editor-alpha";
    alpha.type = "range";
    alpha.min = "0";
    alpha.max = "100";
    alpha.step = "1";
    alpha.setAttribute("aria-label", `${options.ariaLabel || "Color"} alpha`);

    const hex = document.createElement("input");
    hex.className = "color-editor-hex";
    hex.type = "text";
    hex.spellcheck = false;
    hex.autocomplete = "off";
    hex.autocapitalize = "off";
    hex.setAttribute("aria-label", `${options.ariaLabel || "Color"} hex`);

    const swatch = document.createElement("span");
    swatch.className = "color-editor-swatch";
    swatch.setAttribute("aria-hidden", "true");

    const fields = document.createElement("span");
    fields.className = "color-editor-fields";
    fields.append(swatch, hex);
    root.append(plane, hue, alpha, fields);

    let state = {
      ...rgbToHsv(parseColor(options.color || "#000000")),
      alpha: parseColor(options.color || "#000000").alpha,
    };

    const emit = () => {
      const rgb = hsvToRgb(state);
      const next = formatColor({ ...rgb, alpha: state.alpha });
      sync(next);
      options.onInput?.(next);
    };

    const sync = (color) => {
      const parsed = parseColor(color);
      const hsv = rgbToHsv(parsed);
      state = { ...hsv, alpha: parsed.alpha };
      const hueColor = formatColor({ ...hsvToRgb({ hue: state.hue, saturation: 1, value: 1 }), alpha: 255 });
      const next = formatColor(parsed);
      root.style.setProperty("--color-editor-hue", `${state.hue}deg`);
      root.style.setProperty("--color-editor-plane-color", hueColor);
      root.style.setProperty("--color-editor-current", next);
      root.style.setProperty("--color-editor-alpha-rgb", formatColor({ ...parsed, alpha: 255 }));
      root.style.setProperty("--color-editor-saturation", `${state.saturation * 100}%`);
      root.style.setProperty("--color-editor-value", `${(1 - state.value) * 100}%`);
      hue.value = String(Math.round(state.hue));
      alpha.value = String(Math.round((state.alpha / 255) * 100));
      hex.value = next;
      plane.setAttribute("aria-valuetext", next);
      cursor.style.left = `${state.saturation * 100}%`;
      cursor.style.top = `${(1 - state.value) * 100}%`;
    };

    const setPlaneFromEvent = (event) => {
      const rect = plane.getBoundingClientRect();
      state.saturation = clamp((event.clientX - rect.left) / Math.max(1, rect.width), 0, 1);
      state.value = 1 - clamp((event.clientY - rect.top) / Math.max(1, rect.height), 0, 1);
      emit();
    };

    plane.addEventListener("pointerdown", (event) => {
      event.preventDefault();
      plane.setPointerCapture?.(event.pointerId);
      setPlaneFromEvent(event);
    });
    plane.addEventListener("pointermove", (event) => {
      if (event.buttons !== 1 || !plane.hasPointerCapture?.(event.pointerId)) {
        return;
      }
      setPlaneFromEvent(event);
    });
    plane.addEventListener("keydown", (event) => {
      const step = event.shiftKey ? 0.1 : 0.02;
      if (event.key === "ArrowLeft") {
        state.saturation = clamp(state.saturation - step, 0, 1);
      } else if (event.key === "ArrowRight") {
        state.saturation = clamp(state.saturation + step, 0, 1);
      } else if (event.key === "ArrowUp") {
        state.value = clamp(state.value + step, 0, 1);
      } else if (event.key === "ArrowDown") {
        state.value = clamp(state.value - step, 0, 1);
      } else {
        return;
      }
      event.preventDefault();
      emit();
    });
    hue.addEventListener("input", () => {
      state.hue = Number(hue.value) || 0;
      emit();
    });
    alpha.addEventListener("input", () => {
      state.alpha = Math.round((clamp(alpha.value, 0, 100) / 100) * 255);
      emit();
    });
    hex.addEventListener("change", () => {
      sync(hex.value);
      options.onInput?.(formatColor(parseColor(hex.value)));
    });
    root.syncColor = sync;
    sync(options.color || "#000000");
    return root;
  }

  window.PuzzleStudioColorEditor = {
    create,
    normalizeColor,
    parseColor,
    formatColor,
    colorWithAlpha,
  };
})();
