#!/usr/bin/env python3
"""Experimental Sokoban generator scored by decision entropy.

This is intentionally independent from core/lang and the Rust solver. It is a
fast design probe for the puzzle-authoring loop:

  1. Generate random reverse Sokoban layouts.
  2. Solve them with a simple input-BFS.
  3. Score the found solution by weighted reasonable choices before each push.
  4. Drop obvious red herrings and optionally compact unused floor.
  5. Emit a .puzzle file that html-play can load.

The score is deliberately simple:

    decision_entropy + 0.25 * early_decision_entropy

Other checks are constraints, not extra objectives.
"""

from __future__ import annotations

import argparse
import math
import random
import time
from collections import defaultdict, deque
from dataclasses import dataclass
from pathlib import Path

DIRS = [("w", (0, -1)), ("a", (-1, 0)), ("s", (0, 1)), ("d", (1, 0))]
DIR_BY_KEY = dict(DIRS)


@dataclass(frozen=True)
class Layout:
    size: int
    walls: frozenset[tuple[int, int]]
    boxes: frozenset[tuple[int, int]]
    goals: frozenset[tuple[int, int]]
    player: tuple[int, int]


@dataclass
class Candidate:
    score: float
    layout: Layout
    actions: str
    metrics: dict


def cells(size: int) -> list[tuple[int, int]]:
    return [(x, y) for y in range(1, size - 1) for x in range(1, size - 1)]


def border(size: int) -> set[tuple[int, int]]:
    return (
        {(x, 0) for x in range(size)}
        | {(x, size - 1) for x in range(size)}
        | {(0, y) for y in range(size)}
        | {(size - 1, y) for y in range(size)}
    )


def connected(floors: set[tuple[int, int]]) -> bool:
    if not floors:
        return False
    queue = deque([next(iter(floors))])
    seen = {queue[0]}
    while queue:
        x, y = queue.popleft()
        for _, (dx, dy) in DIRS:
            pos = (x + dx, y + dy)
            if pos in floors and pos not in seen:
                seen.add(pos)
                queue.append(pos)
    return len(seen) == len(floors)


def reachable(walls: set[tuple[int, int]], boxes, start: tuple[int, int]) -> set[tuple[int, int]]:
    box_set = set(boxes)
    queue = deque([start])
    seen = {start}
    while queue:
        x, y = queue.popleft()
        for _, (dx, dy) in DIRS:
            pos = (x + dx, y + dy)
            if pos in walls or pos in box_set or pos in seen:
                continue
            seen.add(pos)
            queue.append(pos)
    return seen


def degree(pos: tuple[int, int], walls: set[tuple[int, int]]) -> int:
    x, y = pos
    return sum((x + dx, y + dy) not in walls for _, (dx, dy) in DIRS)


def connected_component_size(start: tuple[int, int], allowed: set[tuple[int, int]]) -> int:
    queue = deque([start])
    seen = {start}
    while queue:
        x, y = queue.popleft()
        for _, (dx, dy) in DIRS:
            pos = (x + dx, y + dy)
            if pos in allowed and pos not in seen:
                seen.add(pos)
                queue.append(pos)
    return len(seen)


def articulation_count(floors: set[tuple[int, int]]) -> int:
    count = 0
    for cell in floors:
        remaining = floors - {cell}
        if remaining and not connected(remaining):
            count += 1
    return count


def structural_metrics(layout: Layout) -> dict:
    floors = set(cells(layout.size)) - set(layout.walls)
    open_cells = {cell for cell in floors if degree(cell, set(layout.walls)) >= 3}
    seen = set()
    open_components = []
    for cell in open_cells:
        if cell in seen:
            continue
        queue = deque([cell])
        component = {cell}
        seen.add(cell)
        while queue:
            x, y = queue.popleft()
            for _, (dx, dy) in DIRS:
                pos = (x + dx, y + dy)
                if pos in open_cells and pos not in seen:
                    seen.add(pos)
                    component.add(pos)
                    queue.append(pos)
        open_components.append(len(component))

    open_2x2 = 0
    for y in range(1, layout.size - 2):
        for x in range(1, layout.size - 2):
            block = {(x, y), (x + 1, y), (x, y + 1), (x + 1, y + 1)}
            if block <= floors:
                open_2x2 += 1

    degrees = [degree(cell, set(layout.walls)) for cell in floors]
    return {
        "floor_count": len(floors),
        "avg_degree": sum(degrees) / len(degrees) if degrees else 0.0,
        "open_cell_count": len(open_cells),
        "max_open_component": max(open_components) if open_components else 0,
        "open_2x2": open_2x2,
        "articulation_count": articulation_count(floors),
    }


def cell_tension(pos: tuple[int, int], walls: set[tuple[int, int]]) -> float:
    # Choices inside a large room are usually navigation freedom, not puzzle
    # tension. Pushes near walls/corridors are more likely to alter future
    # access and assignment constraints.
    return {0: 0.0, 1: 1.0, 2: 0.9, 3: 0.45, 4: 0.15}.get(degree(pos, walls), 0.15)


def bad_corner(pos: tuple[int, int], walls: set[tuple[int, int]], goals) -> bool:
    if pos in goals:
        return False
    x, y = pos
    horizontal_wall = (x - 1, y) in walls or (x + 1, y) in walls
    vertical_wall = (x, y - 1) in walls or (x, y + 1) in walls
    return horizontal_wall and vertical_wall


def static_deadlock(pos: tuple[int, int], walls: set[tuple[int, int]], goals) -> bool:
    if pos in goals or bad_corner(pos, walls, goals):
        return pos not in goals

    x, y = pos
    if (x, y - 1) in walls or (x, y + 1) in walls:
        left = x
        while (left - 1, y) not in walls:
            left -= 1
        right = x
        while (right + 1, y) not in walls:
            right += 1
        if not any((gx, y) in goals for gx in range(left, right + 1)):
            return True

    if (x - 1, y) in walls or (x + 1, y) in walls:
        top = y
        while (x, top - 1) not in walls:
            top -= 1
        bottom = y
        while (x, bottom + 1) not in walls:
            bottom += 1
        if not any((x, gy) in goals for gy in range(top, bottom + 1)):
            return True

    return False


def solve_state_bfs(
    layout: Layout,
    player_start: tuple[int, int],
    boxes_start,
    max_nodes: int,
) -> tuple[str, list[tuple[int, tuple[int, int], tuple[int, int], str]]] | None:
    walls = set(layout.walls)
    goals = set(layout.goals)
    start = (player_start, tuple(sorted(boxes_start)))
    queue = deque([start])
    parent = {start: None}
    action = {}

    while queue:
        if len(parent) > max_nodes:
            return None
        player, boxes = queue.popleft()
        box_set = set(boxes)
        for key, (dx, dy) in DIRS:
            next_player = (player[0] + dx, player[1] + dy)
            if next_player in walls:
                continue
            next_boxes = boxes
            pushed = None
            if next_player in box_set:
                index = boxes.index(next_player)
                box_to = (next_player[0] + dx, next_player[1] + dy)
                if box_to in walls or box_to in box_set:
                    continue
                mutable = list(boxes)
                mutable[index] = box_to
                next_boxes = tuple(sorted(mutable))
                pushed = (index, next_player, box_to, key)
            state = (next_player, next_boxes)
            if state in parent:
                continue
            parent[state] = (player, boxes)
            action[state] = (key, pushed)
            if set(next_boxes) == goals:
                actions = []
                pushes = []
                current = state
                while parent[current] is not None:
                    key_out, pushed_out = action[current]
                    actions.append(key_out)
                    if pushed_out is not None:
                        pushes.append(pushed_out)
                    current = parent[current]
                return "".join(reversed(actions)), list(reversed(pushes))
            queue.append(state)
    return None


def solve_input_bfs(layout: Layout, max_nodes: int) -> tuple[str, list[tuple[int, tuple[int, int], tuple[int, int], str]]] | None:
    return solve_state_bfs(layout, layout.player, layout.boxes, max_nodes)


def reasonable_pushes(layout: Layout, boxes: tuple[tuple[int, int], ...], player: tuple[int, int]):
    walls = set(layout.walls)
    goals = set(layout.goals)
    reachable_cells = reachable(walls, boxes, player)
    box_set = set(boxes)
    options = []
    for index, box in enumerate(boxes):
        for key, (dx, dy) in DIRS:
            stand = (box[0] - dx, box[1] - dy)
            dest = (box[0] + dx, box[1] + dy)
            if stand not in reachable_cells or dest in walls or dest in box_set:
                continue
            if static_deadlock(dest, walls, goals):
                continue
            before = min(abs(box[0] - goal[0]) + abs(box[1] - goal[1]) for goal in goals)
            after = min(abs(dest[0] - goal[0]) + abs(dest[1] - goal[1]) for goal in goals)
            if box in goals and dest not in goals:
                weight = 0.25
            elif dest in goals and box not in goals:
                weight = 1.25
            elif after < before:
                weight = 1.0
            elif after == before:
                weight = 0.65
            else:
                weight = 0.4
            tension = max(cell_tension(stand, walls), cell_tension(box, walls), cell_tension(dest, walls))
            constrained_weight = weight * tension
            options.append((index, box, dest, key, weight, constrained_weight, before, after, tension))
    return options


def apply_push(boxes: tuple[tuple[int, int], ...], option) -> tuple[tuple[int, int], tuple[tuple[int, int], ...]]:
    index, box, dest = option[0], option[1], option[2]
    mutable = list(boxes)
    mutable[index] = dest
    return box, tuple(sorted(mutable))


def replay_metrics(layout: Layout, actions: str) -> dict:
    walls = set(layout.walls)
    goals = set(layout.goals)
    boxes = list(sorted(layout.boxes))
    player = layout.player
    used_cells = {player} | set(boxes) | goals
    counts = defaultdict(int)
    last_push = {}
    entropy = 0.0
    early_entropy = 0.0
    forced_away = 0
    choices = []
    push_sequence = []
    push_directions = []
    plausible_solution_pushes = 0
    weak_solution_pushes = 0
    push_no = 0
    bad = False

    for key in actions:
        dx, dy = DIR_BY_KEY[key]
        next_player = (player[0] + dx, player[1] + dy)
        used_cells.add(next_player)
        if next_player in boxes:
            options = reasonable_pushes(layout, tuple(boxes), player)
            raw_choices = sum(option[4] for option in options)
            effective_choices = sum(option[5] for option in options)
            solution_option = None
            for option in options:
                if option[1] == next_player and option[2] == (next_player[0] + dx, next_player[1] + dy):
                    solution_option = option
                    break
            local_entropy = math.log2(max(1.0, effective_choices))
            entropy += local_entropy
            if push_no < 6:
                early_entropy += local_entropy
            choices.append((len(options), round(raw_choices, 2), round(effective_choices, 2)))

            index = boxes.index(next_player)
            box_to = (next_player[0] + dx, next_player[1] + dy)
            before = min(abs(next_player[0] - goal[0]) + abs(next_player[1] - goal[1]) for goal in goals)
            after = min(abs(box_to[0] - goal[0]) + abs(box_to[1] - goal[1]) for goal in goals)
            if after > before and effective_choices <= 1.05:
                forced_away += 1
            if static_deadlock(box_to, walls, goals):
                bad = True
            if solution_option is not None:
                _, _, dest, _, base_weight, constrained_weight, before_opt, after_opt, _ = solution_option
                plausible = base_weight >= 0.65 and constrained_weight >= 0.5 and not (
                    after_opt > before_opt and dest not in goals
                )
                if plausible:
                    plausible_solution_pushes += 1
                else:
                    weak_solution_pushes += 1
            boxes[index] = box_to
            used_cells.add(box_to)
            counts[index] += 1
            last_push[index] = push_no + 1
            push_sequence.append(index)
            push_directions.append(key)
            push_no += 1
        player = next_player

    box_count = len(layout.boxes)
    push_counts = tuple(sorted(counts.get(index, 0) for index in range(box_count)))
    last_tuple = tuple(sorted(last_push.get(index, 0) for index in range(box_count)))
    runs = []
    for index in push_sequence:
        if not runs or runs[-1][0] != index:
            runs.append([index, 1])
        else:
            runs[-1][1] += 1
    run_boxes = [box for box, _ in runs]
    run_lengths = [length for _, length in runs]
    switch_count = max(0, len(runs) - 1)
    seen_runs = set()
    revisit_switch_count = 0
    for box in run_boxes:
        if box in seen_runs:
            revisit_switch_count += 1
        seen_runs.add(box)
    alternation_count = sum(
        1
        for i in range(2, len(run_boxes))
        if run_boxes[i] == run_boxes[i - 2] and run_boxes[i] != run_boxes[i - 1]
    )
    total_pushes = len(push_sequence)
    push_balance = 0.0
    if total_pushes:
        probabilities = [counts.get(index, 0) / total_pushes for index in range(box_count) if counts.get(index, 0)]
        push_balance = -sum(p * math.log2(p) for p in probabilities)
    max_run_length = max(run_lengths) if run_lengths else 0
    direction_counts = defaultdict(int)
    for direction in push_directions:
        direction_counts[direction] += 1
    dominant_direction_count = max(direction_counts.values()) if direction_counts else 0
    dominant_direction_ratio = dominant_direction_count / total_pushes if total_pushes else 0.0
    direction_balance = 0.0
    if total_pushes:
        direction_balance = -sum(
            (count / total_pushes) * math.log2(count / total_pushes)
            for count in direction_counts.values()
            if count
        )
    plausible_solution_ratio = plausible_solution_pushes / total_pushes if total_pushes else 0.0
    interleaving_score = (
        switch_count
        + 1.5 * revisit_switch_count
        + 1.25 * alternation_count
        + push_balance
        + 0.5 * direction_balance
        - 0.9 * max(0, max_run_length - 4)
        - 3.0 * max(0.0, dominant_direction_ratio - 0.58)
        - 2.0 * max(0.0, 0.55 - plausible_solution_ratio)
    )
    return {
        "entropy": entropy,
        "early_entropy": early_entropy,
        "push_counts": push_counts,
        "last_pushes": last_tuple,
        "push_sequence": tuple(push_sequence),
        "push_directions": tuple(push_directions),
        "active_runs": tuple((box, length) for box, length in runs),
        "switch_count": switch_count,
        "revisit_switch_count": revisit_switch_count,
        "alternation_count": alternation_count,
        "push_balance": push_balance,
        "direction_balance": direction_balance,
        "dominant_direction_ratio": dominant_direction_ratio,
        "plausible_solution_pushes": plausible_solution_pushes,
        "weak_solution_pushes": weak_solution_pushes,
        "plausible_solution_ratio": plausible_solution_ratio,
        "max_run_length": max_run_length,
        "interleaving_score": interleaving_score,
        "forced_away": forced_away,
        "choices": choices,
        "bad": bad,
        "used_cells": used_cells,
        "pushes": push_no,
    }


def trap_metrics(layout: Layout, actions: str, branch_nodes: int) -> dict:
    boxes = list(sorted(layout.boxes))
    player = layout.player
    memo = {}
    trap_score = 0.0
    early_trap_score = 0.0
    trap_count = 0
    early_trap_count = 0
    solvable_alt_count = 0
    profile = []
    push_no = 0

    for key in actions:
        dx, dy = DIR_BY_KEY[key]
        next_player = (player[0] + dx, player[1] + dy)
        if next_player in boxes:
            box_to = (next_player[0] + dx, next_player[1] + dy)
            current_boxes = tuple(boxes)
            options = reasonable_pushes(layout, current_boxes, player)
            local_trap_weight = 0.0
            local_traps = 0
            local_solvable = 0

            for option in options:
                _, box, dest, _, base_weight, constrained_weight, before, after = option[:8]
                if box == next_player and dest == box_to:
                    continue
                # Trap value should mean "this looks locally plausible but is
                # globally wrong". Purely away-from-goal pushes are usually
                # recognized as suspicious, so they are not counted here.
                if base_weight < 0.65 or constrained_weight < 0.5:
                    continue
                if after > before and dest not in layout.goals:
                    continue

                alt_player, alt_boxes = apply_push(current_boxes, option)
                state_key = (alt_player, alt_boxes)
                if state_key not in memo:
                    memo[state_key] = solve_state_bfs(layout, alt_player, alt_boxes, branch_nodes) is not None
                if memo[state_key]:
                    local_solvable += 1
                else:
                    local_traps += 1
                    local_trap_weight += constrained_weight

            if local_traps:
                local_score = math.log2(1.0 + local_trap_weight)
                trap_score += local_score
                trap_count += local_traps
                if push_no < 6:
                    early_trap_score += local_score
                    early_trap_count += local_traps
            solvable_alt_count += local_solvable
            profile.append((local_traps, round(local_trap_weight, 2), local_solvable))

            index = boxes.index(next_player)
            boxes[index] = box_to
            push_no += 1
        player = next_player

    return {
        "trap_score": trap_score,
        "early_trap_score": early_trap_score,
        "trap_count": trap_count,
        "early_trap_count": early_trap_count,
        "solvable_alt_count": solvable_alt_count,
        "trap_profile": profile,
    }


def verbose_push_analysis(layout: Layout, actions: str, branch_nodes: int) -> list[dict]:
    boxes = list(sorted(layout.boxes))
    player = layout.player
    memo = {}
    rows = []
    push_no = 0

    for key in actions:
        dx, dy = DIR_BY_KEY[key]
        next_player = (player[0] + dx, player[1] + dy)
        if next_player in boxes:
            box_to = (next_player[0] + dx, next_player[1] + dy)
            current_boxes = tuple(boxes)
            options = reasonable_pushes(layout, current_boxes, player)
            detail = []

            for option in options:
                _, box, dest, option_key, base_weight, constrained_weight, before, after, tension = option
                alt_player, alt_boxes = apply_push(current_boxes, option)
                state_key = (alt_player, alt_boxes)
                if state_key not in memo:
                    memo[state_key] = solve_state_bfs(layout, alt_player, alt_boxes, branch_nodes) is not None
                plausible = base_weight >= 0.65 and constrained_weight >= 0.5 and not (
                    after > before and dest not in layout.goals
                )
                detail.append(
                    {
                        "move": f"{box}->{dest}{option_key}",
                        "solution": box == next_player and dest == box_to,
                        "base": round(base_weight, 2),
                        "tension": round(tension, 2),
                        "weighted": round(constrained_weight, 2),
                        "dist": (before, after),
                        "plausible": plausible,
                        "solvable": memo[state_key],
                    }
                )

            rows.append(
                {
                    "push": push_no + 1,
                    "solution": f"{next_player}->{box_to}{key}",
                    "player": player,
                    "boxes": current_boxes,
                    "options": detail,
                }
            )

            index = boxes.index(next_player)
            boxes[index] = box_to
            push_no += 1
        player = next_player

    return rows


def compact_layout(layout: Layout, actions: str) -> Layout:
    metrics = replay_metrics(layout, actions)
    walls = set(layout.walls)
    for cell in cells(layout.size):
        if (
            cell not in metrics["used_cells"]
            and cell not in layout.boxes
            and cell not in layout.goals
            and cell != layout.player
        ):
            walls.add(cell)
    return Layout(layout.size, frozenset(walls), layout.boxes, layout.goals, layout.player)


def evaluate(layout: Layout, max_nodes: int, branch_nodes: int) -> Candidate | None:
    if len(layout.boxes) != len(layout.goals):
        return None
    if layout.boxes & layout.goals or layout.player in layout.boxes or layout.player in layout.goals:
        return None
    walls = set(layout.walls)
    if any(bad_corner(box, walls, layout.goals) for box in layout.boxes):
        return None
    if not connected(set(cells(layout.size)) - walls):
        return None
    structure = structural_metrics(layout)
    inside_count = (layout.size - 2) * (layout.size - 2)
    if structure["floor_count"] > int(inside_count * 0.78):
        return None
    if structure["max_open_component"] > max(14, int(structure["floor_count"] * 0.65)):
        return None
    if structure["open_2x2"] > max(8, structure["floor_count"] // 2):
        return None

    solved = solve_input_bfs(layout, max_nodes)
    if solved is None:
        return None
    actions, _ = solved
    metrics = replay_metrics(layout, actions)

    if not (12 <= metrics["pushes"] <= 30 and 24 <= len(actions) <= 140):
        return None
    if metrics["bad"]:
        return None
    if metrics["forced_away"] > 1:
        return None
    if metrics["early_entropy"] < 1.5:
        return None
    if min(metrics["push_counts"]) < 2:
        return None
    if min(metrics["last_pushes"]) < max(5, int(0.30 * metrics["pushes"])):
        return None

    if metrics["switch_count"] < 4:
        return None
    if metrics["revisit_switch_count"] < 1:
        return None
    if metrics["interleaving_score"] < 5.5:
        return None
    if metrics["max_run_length"] > 9:
        return None
    if metrics["dominant_direction_ratio"] > 0.60:
        return None
    if metrics["plausible_solution_ratio"] < 0.52:
        return None

    traps = trap_metrics(layout, actions, branch_nodes)

    metrics.update(traps)
    metrics["structure"] = structure
    score = (
        metrics["interleaving_score"]
        + 0.18 * metrics["entropy"]
        + 0.12 * metrics["trap_score"]
        + 0.4 * min(metrics["early_entropy"], 6.0)
    )
    return Candidate(score, layout, actions, metrics)


def random_reverse_layout(rng: random.Random, size: int) -> Layout | None:
    board_cells = cells(size)
    board_border = border(size)
    for _ in range(100):
        box_count = rng.choice([3, 3, 3, 4])
        wall_count = rng.choice([4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 10, 11, 12])
        walls = set(board_border) | set(rng.sample(board_cells, wall_count))
        floors = [cell for cell in board_cells if cell not in walls]
        if len(floors) < 2 * box_count + 1 or not connected(set(floors)):
            continue
        goals = set(rng.sample(floors, box_count))
        if sum(degree(goal, walls) >= 3 for goal in goals) < min(box_count, 2):
            continue

        boxes = tuple(sorted(goals))
        player = rng.choice([cell for cell in floors if cell not in boxes])
        floor_set = set(floors)

        for _ in range(rng.randint(12, 42)):
            reachable_cells = reachable(walls, boxes, player)
            box_set = set(boxes)
            options = []
            for index, box_now in enumerate(boxes):
                for _, (dx, dy) in DIRS:
                    old_box = (box_now[0] - dx, box_now[1] - dy)
                    old_player = (old_box[0] - dx, old_box[1] - dy)
                    if old_box in walls or old_player in walls or old_box in box_set or old_player in box_set:
                        continue
                    if old_box not in floor_set or old_player not in floor_set or old_box not in reachable_cells:
                        continue
                    options.append((index, old_box, old_player))
            if not options:
                break
            index, old_box, old_player = rng.choice(options)
            mutable = list(boxes)
            mutable[index] = old_box
            boxes = tuple(sorted(mutable))
            player = old_player

        box_set = set(boxes)
        if (
            box_set != goals
            and not (box_set & goals)
            and player not in box_set
            and player not in goals
            and not any(bad_corner(box, walls, goals) for box in box_set)
        ):
            return Layout(size, frozenset(walls), frozenset(box_set), frozenset(goals), player)
    return None


def mutate_layout(layout: Layout, rng: random.Random) -> Layout:
    walls = set(layout.walls)
    boxes = set(layout.boxes)
    goals = set(layout.goals)
    player = layout.player
    floors = [cell for cell in cells(layout.size) if cell not in walls]
    op = rng.choice(["move_box", "move_goal", "move_player", "toggle_wall", "toggle_wall", "swap_bg"])

    if op == "move_box" and boxes:
        options = [cell for cell in floors if cell not in boxes and cell not in goals and cell != player]
        if options:
            boxes.remove(rng.choice(tuple(boxes)))
            boxes.add(rng.choice(options))
    elif op == "move_goal" and goals:
        options = [
            cell
            for cell in floors
            if cell not in boxes and cell not in goals and cell != player and degree(cell, walls) >= 2
        ]
        if options:
            goals.remove(rng.choice(tuple(goals)))
            goals.add(rng.choice(options))
    elif op == "move_player":
        options = [cell for cell in floors if cell not in boxes and cell not in goals]
        if options:
            player = rng.choice(options)
    elif op == "toggle_wall":
        occupied = boxes | goals | {player}
        cell = rng.choice(cells(layout.size))
        if cell not in occupied:
            if cell in walls:
                walls.remove(cell)
            else:
                walls.add(cell)
            if not connected(set(cells(layout.size)) - walls):
                if cell in walls:
                    walls.remove(cell)
                else:
                    walls.add(cell)
    elif op == "swap_bg" and boxes and goals:
        box_options = [cell for cell in floors if cell not in boxes and cell not in goals and cell != player]
        goal_options = [
            cell
            for cell in floors
            if cell not in boxes and cell not in goals and cell != player and degree(cell, walls) >= 2
        ]
        if box_options and goal_options:
            boxes.remove(rng.choice(tuple(boxes)))
            goals.remove(rng.choice(tuple(goals)))
            boxes.add(rng.choice(box_options))
            goals.add(rng.choice(goal_options))

    return Layout(layout.size, frozenset(walls), frozenset(boxes), frozenset(goals), player)


def render_rows(layout: Layout) -> list[str]:
    rows = []
    for y in range(layout.size):
        row = ""
        for x in range(layout.size):
            pos = (x, y)
            if pos in layout.walls:
                row += "#"
            elif pos == layout.player:
                row += "P"
            elif pos in layout.boxes:
                row += "B"
            elif pos in layout.goals:
                row += "G"
            else:
                row += "."
        rows.append(row)
    return rows


def puzzle_header(name: str) -> str:
    return f"""name {name}

objects {{
layer {{
Goal G
}}

layer {{
Player P
Box B
Wall #
}}

solid = Player Box Wall
}}

legend {{
. = empty
* = Goal Box
+ = Goal Player
}}

goal = exists(Goal) and count([ Goal no Box ]) == 0

puzzle_inputs directions

main {{
once input [ Player | Box | no solid ] -> [ | Player | Box ]
once input [ Player | no solid ] -> [ | Player ]
}}

scene playing {{
state {{
board = puzzle current_level
mode = play
}}
view {{
puzzle board
}}
keys {{
d ArrowRight -> right
a ArrowLeft -> left
w ArrowUp -> up
s ArrowDown -> down
q -> level_select
Escape -> goto menu
}}
}}

scene level_clear {{
state {{
board = puzzle current_level
mode = clear
message = \"Level clear\"
}}
view {{
puzzle board
panel {{
text message
button \"Next Level\" -> next_level
button \"Restart\" -> restart_level
button \"Level Select\" -> goto level_select
}}
}}
keys {{
Enter Space -> next_level
r -> restart_level
q -> level_select
}}
}}

scene menu using menu {{
view {{
text \"Menu\"
column {{
button \"Back\" -> back
button \"Restart\" -> restart_level
button \"Level Select\" -> goto level_select
}}
}}
keys {{
Escape Enter Space -> back
r -> restart_level
q -> level_select
}}
}}

scene level_select {{
state {{
mode = browse
message = \"Select a level\"
}}
view {{
text message
column {{
for level in levels {{
button level.label -> playing.goto level, mode = retry
}}
}}
button \"Start\" -> confirm
button \"Back\" -> back
}}
keys {{
w ArrowUp -> menu_up
s ArrowDown -> menu_down
Enter Space -> confirm
Escape q -> back
}}
}}
"""


def write_puzzle(path: Path, name: str, candidates: list[Candidate]) -> None:
    chunks = [puzzle_header(name)]
    for index, candidate in enumerate(candidates, 1):
        pushes = candidate.metrics["pushes"]
        chunks.append(f"level entropy_{pushes}_{index} {{")
        chunks.extend(render_rows(candidate.layout))
        chunks.append("}")
        chunks.append("")
    path.write_text("\n".join(chunks), encoding="utf-8")


def parse_level(path: Path, level_name: str | None = None) -> Layout:
    rows = []
    in_level = False
    for line in path.read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        if stripped.startswith("level ") and stripped.endswith("{"):
            name = stripped.split()[1]
            in_level = level_name is None or name == level_name
            rows = []
            continue
        if in_level and stripped == "}":
            break
        if in_level and stripped:
            rows.append(stripped)

    if not rows:
        raise ValueError(f"no level found in {path}")

    walls = set()
    boxes = set()
    goals = set()
    player = None
    for y, row in enumerate(rows):
        for x, char in enumerate(row):
            if char == "#":
                walls.add((x, y))
            elif char == "B":
                boxes.add((x, y))
            elif char == "G":
                goals.add((x, y))
            elif char == "P":
                player = (x, y)
            elif char == "*":
                boxes.add((x, y))
                goals.add((x, y))
            elif char == "+":
                goals.add((x, y))
                player = (x, y)
    if player is None:
        raise ValueError(f"no player found in {path}")
    return Layout(len(rows), frozenset(walls), frozenset(boxes), frozenset(goals), player)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--size", type=int, default=8)
    parser.add_argument("--seconds", type=float, default=120.0)
    parser.add_argument("--seed", type=int, default=20260517)
    parser.add_argument("--keep", type=int, default=4)
    parser.add_argument("--hill-steps", type=int, default=80)
    parser.add_argument("--max-nodes", type=int, default=260_000)
    parser.add_argument("--branch-nodes", type=int, default=45_000)
    parser.add_argument("--out", type=Path, default=Path("games/sokoban_entropy_generated.puzzle"))
    parser.add_argument("--analyze", type=Path)
    parser.add_argument("--level")
    parser.add_argument("--verbose", action="store_true")
    args = parser.parse_args()

    if args.analyze:
        layout = parse_level(args.analyze, args.level)
        solved = solve_input_bfs(layout, args.max_nodes)
        metrics = replay_metrics(layout, solved[0]) if solved else None
        traps = trap_metrics(layout, solved[0], args.branch_nodes) if solved else None
        print(f"file={args.analyze}")
        for row in render_rows(layout):
            print(row)
        print(f"structure={structural_metrics(layout)}")
        if solved:
            print(
                f"solution_steps={len(solved[0])} pushes={metrics['pushes']} "
                f"entropy={metrics['entropy']:.2f} early={metrics['early_entropy']:.2f} "
                f"interleave={metrics['interleaving_score']:.2f} switches={metrics['switch_count']} "
                f"revisits={metrics['revisit_switch_count']} alternations={metrics['alternation_count']} "
                f"max_run={metrics['max_run_length']} dominant_dir={metrics['dominant_direction_ratio']:.2f} "
                f"plausible_solution={metrics['plausible_solution_ratio']:.2f} runs={metrics['active_runs']} "
                f"trap={traps['trap_score']:.2f} early_trap={traps['early_trap_score']:.2f} "
                f"trap_count={traps['trap_count']} early_traps={traps['early_trap_count']} "
                f"counts={metrics['push_counts']} last={metrics['last_pushes']}"
            )
            print(f"choices=(legal, raw, constrained) {metrics['choices']}")
            print(f"trap_profile=(traps, trap_weight, solvable_alts) {traps['trap_profile']}")
            print(f"actions={solved[0]}")
            if args.verbose:
                for row in verbose_push_analysis(layout, solved[0], args.branch_nodes):
                    print(
                        f"push {row['push']} solution={row['solution']} "
                        f"player={row['player']} boxes={row['boxes']}"
                    )
                    for option in row["options"]:
                        marker = "*" if option["solution"] else " "
                        print(f"  {marker} {option}")
        else:
            print("unsolved")
        return

    rng = random.Random(args.seed)
    start = time.time()
    deadline = start + args.seconds
    best: list[Candidate] = []
    seen = set()
    evaluations = 0

    while time.time() < deadline:
        layout = random_reverse_layout(rng, args.size)
        if layout is None:
            continue
        current = evaluate(layout, args.max_nodes, args.branch_nodes)
        evaluations += 1
        local_best = current
        current_layout = layout
        current_score = current.score if current else -9999.0
        temperature = 4.0

        for _ in range(args.hill_steps):
            if time.time() >= deadline:
                break
            next_layout = mutate_layout(current_layout, rng)
            next_candidate = evaluate(next_layout, args.max_nodes, args.branch_nodes)
            evaluations += 1
            next_score = next_candidate.score if next_candidate else -9999.0
            if next_score > current_score or rng.random() < math.exp((next_score - current_score) / max(temperature, 0.2)):
                current_layout = next_layout
                current_score = next_score
                current = next_candidate
            if next_candidate and (local_best is None or next_candidate.score > local_best.score):
                local_best = next_candidate
            temperature *= 0.94

        if local_best is None:
            continue

        compacted = compact_layout(local_best.layout, local_best.actions)
        compacted_candidate = evaluate(compacted, args.max_nodes, args.branch_nodes)
        if compacted_candidate and compacted_candidate.score >= local_best.score - 0.8:
            local_best = compacted_candidate

        rows = tuple(render_rows(local_best.layout))
        if rows in seen:
            continue
        seen.add(rows)
        best.append(local_best)
        best.sort(key=lambda item: item.score, reverse=True)
        best = best[: args.keep]

    write_puzzle(args.out, f"sokoban_{args.size}x{args.size}_entropy", best)

    print(f"evaluations={evaluations} kept={len(best)} out={args.out}")
    for index, candidate in enumerate(best, 1):
        metrics = candidate.metrics
        print(
            f"{index}. score={candidate.score:.2f} interleave={metrics['interleaving_score']:.2f} "
            f"switches={metrics['switch_count']} revisits={metrics['revisit_switch_count']} "
            f"alternations={metrics['alternation_count']} max_run={metrics['max_run_length']} "
            f"dominant_dir={metrics['dominant_direction_ratio']:.2f} "
            f"plausible_solution={metrics['plausible_solution_ratio']:.2f} "
            f"runs={metrics['active_runs']} trap={metrics['trap_score']:.2f} "
            f"early_trap={metrics['early_trap_score']:.2f} traps={metrics['trap_count']} "
            f"entropy={metrics['entropy']:.2f} "
            f"early={metrics['early_entropy']:.2f} pushes={metrics['pushes']} "
            f"counts={metrics['push_counts']} last={metrics['last_pushes']} "
            f"forced_away={metrics['forced_away']} structure={metrics['structure']} "
            f"actions={candidate.actions}"
        )
        for row in render_rows(candidate.layout):
            print(row)


if __name__ == "__main__":
    main()
