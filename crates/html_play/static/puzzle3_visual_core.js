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

  function directionDepth(vector, view) {
    const yaw = degreesToRadians(view.camera?.yawDegrees ?? 0);
    const pitch = degreesToRadians(view.camera?.pitchDegrees ?? 35);
    const yawY = vector.x * Math.sin(yaw) + vector.y * Math.cos(yaw);
    return -yawY * Math.cos(pitch) + vector.z * Math.sin(pitch);
  }

  function faceGridOrder(corners, view) {
    const center = corners.reduce(
      (total, corner) => ({
        x: total.x + corner.x / corners.length,
        y: total.y + corner.y / corners.length,
        z: total.z + corner.z / corners.length,
      }),
      { x: 0, y: 0, z: 0 },
    );
    return gridOrder(center, view);
  }

  function gridOrder(position, view) {
    const signs = cameraGridSigns(view);
    return {
      x: signs.x * position.x,
      y: signs.y * position.y,
      z: signs.z * position.z,
    };
  }

  function cameraGridSigns(view) {
    const yaw = degreesToRadians(view.camera?.yawDegrees ?? 0);
    const pitch = degreesToRadians(view.camera?.pitchDegrees ?? 35);
    return {
      x: signedAxis(-Math.sin(yaw) * Math.cos(pitch)),
      y: signedAxis(-Math.cos(yaw) * Math.cos(pitch)),
      z: signedAxis(Math.sin(pitch)),
    };
  }

  function signedAxis(value) {
    if (Math.abs(value) < 0.000001) {
      return 0;
    }
    return value > 0 ? 1 : -1;
  }

  function comparePrimitiveOrder(a, b) {
    const ownerComparison = compareOwnerCellOrder(a, b);
    if (ownerComparison !== 0) {
      return ownerComparison;
    }
    const localPriorityComparison = compareLocalRenderPriority(a, b);
    if (localPriorityComparison !== 0) {
      return localPriorityComparison;
    }
    const gridComparison = compareGridOrder(a.gridOrder, b.gridOrder);
    if (gridComparison !== 0) {
      return gridComparison;
    }
    const depthDiff = (a.depth ?? 0) - (b.depth ?? 0);
    if (Math.abs(depthDiff) > 0.000001) {
      return depthDiff;
    }
    return (a.renderPriority ?? 0) - (b.renderPriority ?? 0);
  }

  function compareOwnerCellOrder(a, b) {
    if (!a.ownerCell || !b.ownerCell || a.ownerCell.key === b.ownerCell.key) {
      return 0;
    }
    const gridComparison = compareGridOrder(a.ownerCell.order, b.ownerCell.order);
    if (gridComparison !== 0) {
      return gridComparison;
    }
    const depthDiff = a.ownerCell.depth - b.ownerCell.depth;
    if (Math.abs(depthDiff) > 0.000001) {
      return depthDiff;
    }
    return 0;
  }

  function compareLocalRenderPriority(a, b) {
    if (!a.ownerCell || !b.ownerCell || a.ownerCell.key !== b.ownerCell.key) {
      return 0;
    }
    const priorityDiff = (a.renderPriority ?? 0) - (b.renderPriority ?? 0);
    if (Math.abs(priorityDiff) > 0.000001) {
      return priorityDiff;
    }
    return 0;
  }

  function compareGridOrder(a, b) {
    if (!a || !b) {
      return 0;
    }
    const diffs = [a.x - b.x, a.y - b.y, a.z - b.z];
    const hasPositive = diffs.some((diff) => diff > 0.000001);
    const hasNegative = diffs.some((diff) => diff < -0.000001);
    if (hasPositive && !hasNegative) {
      return 1;
    }
    if (hasNegative && !hasPositive) {
      return -1;
    }
    return 0;
  }

  function stageFrameEdges(size) {
    const width = Math.max(1, Number(size?.width) || 1);
    const depth = Math.max(1, Number(size?.depth) || 1);
    const height = Math.max(1, Number(size?.height) || 1);
    const x0 = -0.5;
    const x1 = width - 0.5;
    const y0 = -0.5;
    const y1 = depth - 0.5;
    const z0 = -0.5;
    const z1 = height - 0.5;
    const corners = {
      leftBackBottom: { x: x0, y: y0, z: z0 },
      rightBackBottom: { x: x1, y: y0, z: z0 },
      rightFrontBottom: { x: x1, y: y1, z: z0 },
      leftFrontBottom: { x: x0, y: y1, z: z0 },
      leftBackTop: { x: x0, y: y0, z: z1 },
      rightBackTop: { x: x1, y: y0, z: z1 },
      rightFrontTop: { x: x1, y: y1, z: z1 },
      leftFrontTop: { x: x0, y: y1, z: z1 },
    };
    return [
      { from: corners.leftBackBottom, to: corners.rightBackBottom },
      { from: corners.rightBackBottom, to: corners.rightFrontBottom },
      { from: corners.rightFrontBottom, to: corners.leftFrontBottom },
      { from: corners.leftFrontBottom, to: corners.leftBackBottom },
      { from: corners.leftBackTop, to: corners.rightBackTop },
      { from: corners.rightBackTop, to: corners.rightFrontTop },
      { from: corners.rightFrontTop, to: corners.leftFrontTop },
      { from: corners.leftFrontTop, to: corners.leftBackTop },
      { from: corners.leftBackBottom, to: corners.leftBackTop },
      { from: corners.rightBackBottom, to: corners.rightBackTop },
      { from: corners.rightFrontBottom, to: corners.rightFrontTop },
      { from: corners.leftFrontBottom, to: corners.leftFrontTop },
    ];
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
    compareGridOrder,
    comparePrimitiveOrder,
    directionDepth,
    faceGridOrder,
    gridOrder,
    mergeVoxelFaces,
    projectOrthographic,
    rectsFromCells,
    stageFrameEdges,
  };
})(window);
