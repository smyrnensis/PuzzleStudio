(function attachPuzzle3VisualCore(global) {
  function projectOrthographic(position, view) {
    const yaw = degreesToRadians(view.camera?.yawDegrees ?? 0);
    const pitch = degreesToRadians(view.camera?.pitchDegrees ?? 35);
    const zoom = view.camera?.zoom ?? 1;
    const center = view.center || { x: 0, y: 0, z: 0 };
    const x = position.x - center.x;
    const y = position.y - center.y;
    const z = position.z - center.z;
    const yawX = x * Math.cos(yaw) - y * Math.sin(yaw);
    const yawY = x * Math.sin(yaw) + y * Math.cos(yaw);
    const scale = view.scale * zoom;
    return {
      x: view.origin.x + yawX * scale,
      y: view.origin.y + (-yawY * Math.sin(pitch) - z * Math.cos(pitch)) * scale,
      depth: -yawY * Math.cos(pitch) + z * Math.sin(pitch),
    };
  }

  function degreesToRadians(value) {
    return (value * Math.PI) / 180;
  }

  function mergeVoxelFaces(voxels, adapter) {
    const groups = new Map();
    for (const voxel of voxels) {
      for (const face of adapter.faces(voxel)) {
        if (!adapter.isFaceVisible(voxel, face)) {
          continue;
        }
        const spec = adapter.group(voxel, face);
        let group = groups.get(spec.key);
        if (!group) {
          group = { ...spec.group, cells: new Set() };
          groups.set(spec.key, group);
        }
        group.cells.add(`${spec.u},${spec.v}`);
      }
    }

    const faces = [];
    for (const group of groups.values()) {
      for (const rect of rectsFromCells(group.cells)) {
        faces.push(adapter.face(group, rect));
      }
    }
    return faces;
  }

  function rectsFromCells(cells) {
    const remaining = new Set(cells);
    const rects = [];
    while (remaining.size > 0) {
      const [start] = remaining;
      const [startU, startV] = start.split(",").map(Number);
      let width = 1;
      while (remaining.has(`${startU + width},${startV}`)) {
        width += 1;
      }

      let height = 1;
      let canGrow = true;
      while (canGrow) {
        for (let u = startU; u < startU + width; u += 1) {
          if (!remaining.has(`${u},${startV + height}`)) {
            canGrow = false;
            break;
          }
        }
        if (canGrow) {
          height += 1;
        }
      }

      for (let v = startV; v < startV + height; v += 1) {
        for (let u = startU; u < startU + width; u += 1) {
          remaining.delete(`${u},${v}`);
        }
      }
      rects.push({ u0: startU, u1: startU + width - 1, v0: startV, v1: startV + height - 1 });
    }
    return rects;
  }

  global.Puzzle3VisualCore = {
    mergeVoxelFaces,
    projectOrthographic,
    rectsFromCells,
  };
})(window);
