#!/usr/bin/env python3
"""Merge touching bitmap cells in an embedded TrueType font.

The PuzzleScript terminal font is built from square contours, one contour per
lit bitmap cell. Font rasterizers antialias every contour independently, which
can expose seams between touching cells. This tool replaces those contours with
the boundary of each connected cell region and updates the font embedded in the
theme CSS.
"""

from __future__ import annotations

import argparse
import base64
import io
import re
from collections import defaultdict
from pathlib import Path

from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.ttLib import TTFont


FONT_FAMILY = "PuzzleScript Terminal"
CELL_SIZE = 100


def embedded_font_pattern() -> re.Pattern[str]:
    return re.compile(
        rf'(@font-face\s*\{{(?:(?!@font-face).)*?font-family:\s*"{re.escape(FONT_FAMILY)}";'
        rf'(?:(?!@font-face).)*?src:\s*url\("data:font/ttf;base64,)([A-Za-z0-9+/=]+)',
        re.DOTALL,
    )


def contour_cells(
    font: TTFont, glyph_name: str
) -> tuple[set[tuple[int, int]], tuple[int, int]]:
    glyph = font["glyf"][glyph_name]
    if glyph.isComposite():
        raise ValueError(f"{glyph_name}: composite glyph is unsupported")
    if glyph.numberOfContours <= 0:
        return set(), (0, 0)

    coordinates, end_points, _flags = glyph.getCoordinates(font["glyf"])
    boxes: list[tuple[int, int]] = []
    start = 0
    for end in end_points:
        points = list(coordinates[start : end + 1])
        start = end + 1
        xs = {point[0] for point in points}
        ys = {point[1] for point in points}
        if len(points) != 4 or len(xs) != 2 or len(ys) != 2:
            raise ValueError(f"{glyph_name}: contour is not one bitmap cell")
        min_x, max_x = min(xs), max(xs)
        min_y, max_y = min(ys), max(ys)
        if max_x - min_x != CELL_SIZE or max_y - min_y != CELL_SIZE:
            raise ValueError(f"{glyph_name}: contour is not {CELL_SIZE} units square")
        boxes.append((min_x, min_y))
    origin_x = min(x for x, _y in boxes)
    origin_y = min(y for _x, y in boxes)
    if any((x - origin_x) % CELL_SIZE or (y - origin_y) % CELL_SIZE for x, y in boxes):
        raise ValueError(f"{glyph_name}: contours do not share one bitmap grid")
    cells = {
        ((x - origin_x) // CELL_SIZE, (y - origin_y) // CELL_SIZE) for x, y in boxes
    }
    return cells, (origin_x, origin_y)


def boundary_loops(cells: set[tuple[int, int]]) -> list[list[tuple[int, int]]]:
    # Directed edges keep filled space on their left. Shared edges are omitted.
    edges: set[tuple[tuple[int, int], tuple[int, int]]] = set()
    for x, y in cells:
        if (x, y - 1) not in cells:
            edges.add(((x, y), (x + 1, y)))
        if (x + 1, y) not in cells:
            edges.add(((x + 1, y), (x + 1, y + 1)))
        if (x, y + 1) not in cells:
            edges.add(((x + 1, y + 1), (x, y + 1)))
        if (x - 1, y) not in cells:
            edges.add(((x, y + 1), (x, y)))

    outgoing: dict[tuple[int, int], list[tuple[int, int]]] = defaultdict(list)
    for start, end in edges:
        outgoing[start].append(end)

    loops: list[list[tuple[int, int]]] = []
    while edges:
        first_edge = min(edges)
        start, current = first_edge
        previous = start
        edges.remove(first_edge)
        loop = [start, current]
        while current != start:
            candidates = sorted(end for end in outgoing[current] if (current, end) in edges)
            if not candidates:
                raise ValueError(f"open bitmap boundary at {current}")
            incoming = (current[0] - previous[0], current[1] - previous[1])
            directions = ((1, 0), (0, 1), (-1, 0), (0, -1))
            incoming_index = directions.index(incoming)

            def turn_priority(end: tuple[int, int]) -> int:
                outgoing_direction = (end[0] - current[0], end[1] - current[1])
                turn = (directions.index(outgoing_direction) - incoming_index) % 4
                return {1: 0, 0: 1, 3: 2, 2: 3}[turn]

            # At a diagonal cell contact, two boundaries share one vertex. A
            # left turn keeps the four-connected regions as separate contours.
            next_point = min(candidates, key=turn_priority)
            edges.remove((current, next_point))
            loop.append(next_point)
            previous = current
            current = next_point
        loops.append(loop[:-1])
    return loops


def count_shared_edges(font: TTFont) -> int:
    shared = 0
    for glyph_name in font.getGlyphOrder():
        if glyph_name == ".notdef":
            continue
        glyph = font["glyf"][glyph_name]
        if glyph.numberOfContours <= 0:
            continue
        coordinates, end_points, _flags = glyph.getCoordinates(font["glyf"])
        seen: set[tuple[tuple[int, int], tuple[int, int]]] = set()
        start = 0
        for end in end_points:
            points = list(coordinates[start : end + 1])
            start = end + 1
            for first, second in zip(points, points[1:] + points[:1]):
                if first[0] != second[0] and first[1] != second[1]:
                    raise ValueError(f"{glyph_name}: non-orthogonal bitmap boundary")
                edge = tuple(sorted((tuple(first), tuple(second))))
                if edge in seen:
                    shared += 1
                else:
                    seen.add(edge)
    return shared


def merge_glyph_contours(font: TTFont) -> tuple[int, int, int]:
    glyph_set = font.getGlyphSet()
    glyphs_changed = 0
    contours_before = 0
    contours_after = 0
    for glyph_name in font.getGlyphOrder():
        if glyph_name == ".notdef":
            continue
        cells, origin = contour_cells(font, glyph_name)
        if not cells:
            continue
        loops = boundary_loops(cells)
        old_count = font["glyf"][glyph_name].numberOfContours
        pen = TTGlyphPen(glyph_set)
        for loop in loops:
            pen.moveTo(
                (loop[0][0] * CELL_SIZE + origin[0], loop[0][1] * CELL_SIZE + origin[1])
            )
            for x, y in loop[1:]:
                pen.lineTo((x * CELL_SIZE + origin[0], y * CELL_SIZE + origin[1]))
            pen.closePath()
        font["glyf"][glyph_name] = pen.glyph()
        glyphs_changed += 1
        contours_before += old_count
        contours_after += len(loops)
    return glyphs_changed, contours_before, contours_after


def update_css(css_path: Path, check: bool) -> None:
    css = css_path.read_text(encoding="utf-8")
    pattern = embedded_font_pattern()
    match = pattern.search(css)
    if match is None:
        raise SystemExit(f'{css_path}: embedded font "{FONT_FAMILY}" was not found')

    font = TTFont(io.BytesIO(base64.b64decode(match.group(2))))
    shared_edges = count_shared_edges(font)
    if shared_edges == 0:
        print(f"{css_path}: font contours are merged (no internal shared edges)")
        return
    if check:
        raise SystemExit(
            f"{css_path}: bitmap font has {shared_edges} internal shared edges"
        )

    glyphs, before, after = merge_glyph_contours(font)
    output = io.BytesIO()
    font.save(output, reorderTables=False)
    encoded = base64.b64encode(output.getvalue()).decode("ascii")
    updated = css[: match.start(2)] + encoded + css[match.end(2) :]

    css_path.write_text(updated, encoding="utf-8")
    print(f"{css_path}: merged {before} contours into {after} across {glyphs} glyphs")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "css",
        nargs="?",
        type=Path,
        default=Path("crates/html_play/static/theme_presets.css"),
    )
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    update_css(args.css, args.check)


if __name__ == "__main__":
    main()
