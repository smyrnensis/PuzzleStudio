(function attachPuzzle3VisualCore(root) {
  function cameraModelFrame(camera = {}) {
    const yaw = degreesToRadians(camera.yawDegrees ?? 0);
    const pitch = degreesToRadians(camera.pitchDegrees ?? 35);
    const roll = degreesToRadians(camera.rollDegrees ?? 0);
    const baseRight = {
      x: Math.cos(yaw),
      y: -Math.sin(yaw),
      z: 0,
    };
    const baseUp = {
      x: -Math.sin(yaw) * Math.sin(pitch),
      y: -Math.cos(yaw) * Math.sin(pitch),
      z: -Math.cos(pitch),
    };
    const depth = {
      x: -Math.sin(yaw) * Math.cos(pitch),
      y: -Math.cos(yaw) * Math.cos(pitch),
      z: Math.sin(pitch),
    };
    const cosRoll = Math.cos(roll);
    const sinRoll = Math.sin(roll);
    return {
      right: {
        x: baseRight.x * cosRoll - baseUp.x * sinRoll,
        y: baseRight.y * cosRoll - baseUp.y * sinRoll,
        z: baseRight.z * cosRoll - baseUp.z * sinRoll,
      },
      up: {
        x: baseRight.x * sinRoll + baseUp.x * cosRoll,
        y: baseRight.y * sinRoll + baseUp.y * cosRoll,
        z: baseRight.z * sinRoll + baseUp.z * cosRoll,
      },
      depth,
    };
  }

  function projectOrthographic(position, view) {
    const frame = cameraModelFrame(view.camera);
    const zoom = view.camera?.zoom ?? 1;
    const center = view.center || { x: 0, y: 0, z: 0 };
    const x = position.x - center.x;
    const y = position.y - center.y;
    const z = position.z - center.z;
    const scale = view.scale * zoom;
    return {
      x: view.origin.x + (x * frame.right.x + y * frame.right.y + z * frame.right.z) * scale,
      y: view.origin.y + (x * frame.up.x + y * frame.up.y + z * frame.up.z) * scale,
      depth: x * frame.depth.x + y * frame.depth.y + z * frame.depth.z,
    };
  }

  function degreesToRadians(value) {
    return (value * Math.PI) / 180;
  }

  function directionDepth(vector, view) {
    const { depth } = cameraModelFrame(view.camera);
    return vector.x * depth.x + vector.y * depth.y + vector.z * depth.z;
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
    const basis = cameraOrderBasis(view);
    const signed = {
      x: basis.signs.x * position.x,
      y: basis.signs.y * position.y,
      z: basis.signs.z * position.z,
    };
    return {
      ...signed,
      plane: signed.x + signed.y + signed.z,
      axes: basis.axes,
    };
  }

  function cameraGridSigns(view) {
    return cameraOrderBasis(view).signs;
  }

  function cameraOrderKey(view) {
    const basis = cameraOrderBasis(view);
    return [
      basis.signs.x,
      basis.signs.y,
      basis.signs.z,
      ...basis.axes,
    ].join(":");
  }

  function cameraOrderBasis(view) {
    const coefficients = cameraModelFrame(view.camera).depth;
    const axes = ["x", "y", "z"].sort((left, right) => {
      const magnitudeComparison = Math.abs(coefficients[right]) - Math.abs(coefficients[left]);
      if (Math.abs(magnitudeComparison) > 0.000001) {
        return magnitudeComparison;
      }
      return left < right ? -1 : (left > right ? 1 : 0);
    });
    return {
      signs: {
        x: signedAxis(coefficients.x),
        y: signedAxis(coefficients.y),
        z: signedAxis(coefficients.z),
      },
      axes,
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
    const gridDominanceComparison = compareGridDominance(a.gridOrder, b.gridOrder);
    if (gridDominanceComparison !== 0) {
      return gridDominanceComparison;
    }
    const gridComparison = compareGridOrder(a.gridOrder, b.gridOrder);
    if (gridComparison !== 0) {
      return gridComparison;
    }
    const priorityComparison = compareNumber(a.renderPriority, b.renderPriority);
    if (priorityComparison !== 0) {
      return priorityComparison;
    }
    const objectComparison = compareNumber(a.objectOrder, b.objectOrder);
    if (objectComparison !== 0) {
      return objectComparison;
    }
    return compareStablePrimitiveOrder(a, b);
  }

  function compareOwnerCellOrder(a, b) {
    if (!a.ownerCell || !b.ownerCell || a.ownerCell.key === b.ownerCell.key) {
      return 0;
    }
    const directionComparison = compareNumberArrays(
      a.ownerCell.directionPriority,
      b.ownerCell.directionPriority,
    );
    if (directionComparison !== 0) {
      return directionComparison;
    }
    const gridDominanceComparison = compareGridDominance(a.ownerCell.order, b.ownerCell.order);
    if (gridDominanceComparison !== 0) {
      return gridDominanceComparison;
    }
    const gridComparison = compareGridOrder(a.ownerCell.order, b.ownerCell.order);
    if (gridComparison !== 0) {
      return gridComparison;
    }
    const priorityComparison = compareNumber(a.ownerCell.renderPriority, b.ownerCell.renderPriority);
    if (priorityComparison !== 0) {
      return priorityComparison;
    }
    return compareStableKey(a.ownerCell.key, b.ownerCell.key);
  }

  function compareNumberArrays(a, b) {
    if (!Array.isArray(a) || !Array.isArray(b) || a.length !== b.length) {
      return 0;
    }
    for (let index = 0; index < a.length; index += 1) {
      const comparison = compareNumber(a[index], b[index]);
      if (comparison !== 0) {
        return comparison;
      }
    }
    return 0;
  }

  function compareGridOrder(a, b) {
    if (!a || !b) {
      return 0;
    }
    const planeComparison = compareNumber(a.plane, b.plane);
    if (planeComparison !== 0) {
      return planeComparison;
    }
    const axes = a.axes || b.axes || ["x", "y", "z"];
    for (const axis of axes) {
      const comparison = compareNumber(a[axis], b[axis]);
      if (comparison !== 0) {
        return comparison;
      }
    }
    return 0;
  }

  function compareGridDominance(a, b) {
    if (!a || !b) {
      return 0;
    }
    const diffs = [numberOrZero(a.x) - numberOrZero(b.x), numberOrZero(a.y) - numberOrZero(b.y), numberOrZero(a.z) - numberOrZero(b.z)];
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

  function compareStablePrimitiveOrder(a, b) {
    const frameIndexComparison = compareNumber(a.frameIndex, b.frameIndex);
    if (frameIndexComparison !== 0) {
      return frameIndexComparison;
    }
    const keyComparison = compareStableKey(a.stableKey || a.key, b.stableKey || b.key);
    if (keyComparison !== 0) {
      return keyComparison;
    }
    return compareStableKey(a.kind, b.kind);
  }

  function compareNumber(a, b) {
    const diff = numberOrZero(a) - numberOrZero(b);
    return Math.abs(diff) > 0.000001 ? diff : 0;
  }

  function numberOrZero(value) {
    const number = Number(value);
    return Number.isFinite(number) ? number : 0;
  }

  function compareStableKey(a, b) {
    const left = String(a ?? "");
    const right = String(b ?? "");
    return left < right ? -1 : (left > right ? 1 : 0);
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

    const faceRects = adapter.rectsFromCells || rectsFromCells;
    const faces = [];
    for (const group of groups.values()) {
      for (const rect of faceRects(group.cells, group)) {
        faces.push(adapter.face(group, rect));
      }
    }
    return faces;
  }

  function averageMergedVoxels(voxels, parseColor, formatColor) {
    const colors = voxels
      .map((voxel) => voxel.color || parseColor(voxel.fill))
      .filter((color) => color && color.a > 0);
    if (!colors.length) {
      return voxels[0];
    }
    const divisor = colors.length;
    const color = colors.reduce((sum, candidate) => ({
      r: sum.r + candidate.r,
      g: sum.g + candidate.g,
      b: sum.b + candidate.b,
      a: sum.a + candidate.a,
    }), { r: 0, g: 0, b: 0, a: 0 });
    color.r /= divisor;
    color.g /= divisor;
    color.b /= divisor;
    color.a /= divisor;
    return {
      ...voxels[0],
      color,
      fill: formatColor(color),
      sourceKeys: voxels.flatMap((voxel) =>
        voxel.sourceKey ? [voxel.sourceKey] : (voxel.sourceKeys || [])
      ),
    };
  }

  function objectPriority(order, object, fallbackIndex = 0) {
    const name = String(object?.name || "");
    const priority = order?.priorities?.findIndex((entry) =>
      Array.isArray(entry.objects) && entry.objects.includes(name)
    );
    if (priority >= 0) {
      return priority;
    }
    throw new Error(`compiled sprite order does not cover object: ${name || fallbackIndex}`);
  }

  function priorityDefinition(order, encodedPriority) {
    const priorities = order?.priorities;
    if (!Array.isArray(priorities) || priorities.length === 0) {
      throw new Error("compiled sprite order contract is missing");
    }
    return priorities[encodedPriority % priorities.length];
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

  root.Puzzle3VisualCore = {
    averageMergedVoxels,
    cameraModelFrame,
    cameraOrderKey,
    compareGridOrder,
    comparePrimitiveOrder,
    directionDepth,
    faceGridOrder,
    gridOrder,
    mergeVoxelFaces,
    objectPriority,
    priorityDefinition,
    projectOrthographic,
    rectsFromCells,
    stageFrameEdges,
  };
})(window);
