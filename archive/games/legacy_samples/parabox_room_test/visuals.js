window.PuzzleStudio.registerAssetScript({
  name: "parabox-room-visuals",
  setup(api) {
    api.setBoardClass("is-parabox");
    api.setThemeClass("theme-parabox");

    const rooms = [
      { id: "room-a", label: "Room A", start: 0 },
      { id: "room-b", label: "Room B", start: 7 },
      { id: "room-c", label: "Room C", start: 14 },
    ];
    const roomViews = [
      { sprite: "Room-A", mini: "room-a", start: 0 },
      { sprite: "Room-B", mini: "room-b", start: 7 },
      { sprite: "Room-C", mini: "room-c", start: 14 },
    ];
    const roomSize = 7;
    const miniLayers = [
      { sprite: "Box", mini: "box" },
      { sprite: "Player", mini: "player" },
      { sprite: "Room-A", mini: "room-a" },
      { sprite: "Room-B", mini: "room-b" },
      { sprite: "Room-C", mini: "room-c" },
      { sprite: "Goal", mini: "goal" },
      { sprite: "Wall-A", mini: "wall-a" },
      { sprite: "Wall-B", mini: "wall-b" },
      { sprite: "Wall-C", mini: "wall-c" },
      { sprite: "Wall", mini: "wall" },
    ];

    function roomForX(x) {
      let active = rooms[0];
      for (const room of rooms) {
        if (x >= room.start) {
          active = room;
        }
      }
      return active;
    }

    function ensureHud(screenView) {
      let hud = screenView.querySelector(".parabox-hud");
      if (hud) {
        return hud;
      }

      hud = document.createElement("div");
      hud.className = "parabox-hud";

      const name = document.createElement("span");
      name.className = "parabox-room-name";
      hud.append(name);

      const stack = document.createElement("span");
      stack.className = "parabox-room-stack";
      for (const room of rooms) {
        const dot = document.createElement("span");
        dot.className = "parabox-room-dot";
        dot.dataset.room = room.id;
        stack.append(dot);
      }
      hud.append(stack);

      screenView.append(hud);
      return hud;
    }

    function cellAt(board, x, y) {
      return board.querySelector(`.cell[data-x="${x}"][data-y="${y}"]`);
    }

    function sceneCellAt(scene, x, y) {
      return scene?.cells?.find((cell) => cell.x === x && cell.y === y);
    }

    function miniClassFor(cell) {
      return miniLayers.find((layer) => cell.classList.contains(`has-${layer.sprite}`))?.mini;
    }

    function miniClassForSceneCell(cell) {
      return miniLayers.find((layer) => cell?.layers?.some((item) => item.sprite === layer.sprite))?.mini;
    }

    function currentRoomViews(scene) {
      const regions = scene?.regions || [];
      const useRegions = regions.length >= roomViews.length
        && regions.every((region) => region.width === roomSize && region.height === roomSize);
      return roomViews.map((view, index) => ({
        ...view,
        start: useRegions ? regions[index].x : view.start,
        top: useRegions ? regions[index].y : 0,
        width: roomSize,
        height: roomSize,
      }));
    }

    function updateMiniRooms(board, scene) {
      board.querySelectorAll(".parabox-mini-room").forEach((mini) => mini.remove());

      for (const view of currentRoomViews(scene)) {
        const roomCells = board.querySelectorAll(`.cell.has-${view.sprite}`);
        for (const roomCell of roomCells) {
          const mini = document.createElement("span");
          mini.className = `parabox-mini-room parabox-mini-${view.mini}`;
          mini.setAttribute("aria-hidden", "true");

          for (let y = 0; y < view.height; y += 1) {
            for (let x = 0; x < view.width; x += 1) {
              const source = cellAt(board, view.start + x, view.top + y);
              const sceneSource = sceneCellAt(scene, view.start + x, view.top + y);
              const tile = document.createElement("span");
              tile.className = "parabox-mini-tile";
              const layer = miniClassForSceneCell(sceneSource) || (source ? miniClassFor(source) : null);
              if (layer) {
                tile.classList.add(`mini-${layer}`);
              }
              mini.append(tile);
            }
          }

          roomCell.append(mini);
        }
      }
    }

    function updateFocus({ board, screenView, scene }) {
      if (!board || !screenView) {
        return;
      }

      updateMiniRooms(board, scene);

      const playerCell = board.querySelector(".cell.has-Player");
      if (!playerCell) {
        return;
      }

      const x = Number(playerCell.dataset.x || 0);
      const room = roomForX(x);
      board.style.setProperty("--focus-left", "0px");
      board.dataset.focus = room.id;
      document.body.dataset.roomFocus = room.id;

      const hud = ensureHud(screenView);
      const name = hud.querySelector(".parabox-room-name");
      if (name) {
        name.textContent = room.label;
      }
    }

    api.onRender(updateFocus);
  },
});
