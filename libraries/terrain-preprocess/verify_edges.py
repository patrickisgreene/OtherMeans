#!/usr/bin/env python3
"""
Verifies that cubesphere tile data is seamless across the 12 edges where the
6 base cube faces meet, at every LOD.

Faithfully re-implements (in Python) the pixel-level math from:
  - libraries/terrain/src/math/mod.rs (NEIGHBOURING_FACES, FaceRotation)
  - libraries/terrain/src/math/coordinate.rs (TileCoordinate::neighbours, path)
  - libraries/terrain-preprocess/src/process/stitch.rs (border stitching)
  - libraries/terrain-preprocess/src/process/reconcile.rs (real-edge averaging)

Two independent checks per boundary tile pair:
  Check A - border stitch fidelity: is the padding border of a tile bit-exact
            with what stitch.rs would currently copy from its cross-face
            neighbour?
  Check B - real edge continuity: do the outermost *real* (non-border) pixels
            of two neighbouring tiles, on either side of a base-face edge,
            actually agree?
"""

import argparse
from collections import defaultdict
from pathlib import Path

import numpy as np
from osgeo import gdal

gdal.UseExceptions()

# --- Rust math/mod.rs: NEIGHBOURING_FACES -----------------------------------
NEIGHBOURING_FACES = [
    [0, 2, 1, 5, 4],
    [1, 2, 3, 5, 0],
    [2, 4, 3, 1, 0],
    [3, 4, 5, 1, 2],
    [4, 0, 5, 3, 2],
    [5, 0, 1, 3, 4],
]

# --- Rust math/mod.rs: FaceRotation ------------------------------------------
IDENTICAL, SHIFT_U, ROTATE_CCW, BACKSIDE, ROTATE_CW, SHIFT_V = range(6)
ROTATION_NAMES = {
    IDENTICAL: "Identical",
    SHIFT_U: "ShiftU",
    ROTATE_CCW: "RotateCCW",
    BACKSIDE: "Backside",
    ROTATE_CW: "RotateCW",
    SHIFT_V: "ShiftV",
}

# up, right, down, left - matches NEIGHBOUR_OFFSETS[0..4] / edge_index 0..3
CARDINAL_OFFSETS = [(0, -1), (1, 0), (0, 1), (-1, 0)]
CARDINAL_NAMES = ["up", "right", "down", "left"]


def face_rotation(face: int, other_face: int) -> int:
    """Rust: FaceRotation::new (math/mod.rs:88-99)."""
    if face % 2 == 0:
        index = (6 + other_face - face) % 6
    else:
        index = (6 + face - other_face) % 6
    return index


def project_uv(rotation: int, face: int, u: float, v: float) -> tuple[float, float]:
    """Rust: FaceRotation::project_uv (math/mod.rs:74-86)."""
    odd = face % 2
    if rotation == IDENTICAL:
        return (u, v)
    if rotation == SHIFT_U:
        return (odd, v)
    if rotation == ROTATE_CCW:
        return (odd, u)
    if rotation == BACKSIDE:
        return (v, u)
    if rotation == ROTATE_CW:
        return (v, odd)
    if rotation == SHIFT_V:
        return (u, odd)
    raise ValueError(rotation)


def tile_neighbour(face: int, lod: int, x: int, y: int, edge_index: int):
    """Cardinal-only port of TileCoordinate::neighbours (coordinate.rs:141-181,
    spherical branch). Only valid to call when (x, y) is actually a boundary
    tile in the direction of edge_index - guaranteed by the caller.

    Returns (neighbour_face, nx, ny, rotation).
    """
    dx, dy = CARDINAL_OFFSETS[edge_index]
    tile_count = 1 << lod
    scale = tile_count - 1

    ex, ey = x + dx, y + dy
    neighbour_face = NEIGHBOURING_FACES[face][edge_index + 1]

    if scale == 0:
        eu, ev = 0.0, 0.0
    else:
        eu = min(max(ex / scale, 0.0), 1.0)
        ev = min(max(ey / scale, 0.0), 1.0)

    rotation = face_rotation(face, neighbour_face)
    nu, nv = project_uv(rotation, face, eu, ev)

    nx = int(nu * scale)
    ny = int(nv * scale)
    return neighbour_face, nx, ny, rotation


def neighbour_edge_index(edge_index: int, rotation: int) -> int:
    """Rust: reconcile.rs::neighbour_edge_index (reconcile.rs:67-76). The
    neighbour's own edge index that physically touches `edge_index` on this
    tile."""
    if rotation in (IDENTICAL, SHIFT_U, SHIFT_V):
        return (edge_index + 2) % 4
    if rotation == ROTATE_CW:
        return (edge_index + 1) % 4
    if rotation == ROTATE_CCW:
        return (edge_index + 3) % 4
    raise ValueError(rotation)


def apply_rotation(arr: np.ndarray, rotation: int) -> np.ndarray:
    """Rust: stitch.rs::neighbour_data's array transform (stitch.rs:125-140)."""
    if rotation in (IDENTICAL, SHIFT_U, SHIFT_V):
        return arr
    if rotation == ROTATE_CW:
        return np.swapaxes(arr, 0, 1)[:, ::-1]
    if rotation == ROTATE_CCW:
        return np.swapaxes(arr, 0, 1)[::-1, :]
    raise ValueError(rotation)


def read_window(arr: np.ndarray, xoff: int, yoff: int, xsize: int, ysize: int) -> np.ndarray:
    return arr[yoff : yoff + ysize, xoff : xoff + xsize]


class TileGrid:
    """Loads/caches tile rasters and resolves tile file paths."""

    def __init__(self, terrain_path: Path, attachment: str, block_size: int = 8):
        self.root = terrain_path / attachment
        self.block_size = block_size
        self._cache: dict[tuple[int, int, int, int], np.ndarray | None] = {}

    def tile_path(self, face: int, lod: int, x: int, y: int) -> Path:
        bx, by = x // self.block_size, y // self.block_size
        return self.root / str(lod) / f"{bx}_{by}" / f"{face}_{lod}_{x}_{y}.tif"

    def load(self, face: int, lod: int, x: int, y: int) -> np.ndarray | None:
        key = (face, lod, x, y)
        if key in self._cache:
            return self._cache[key]
        path = self.tile_path(face, lod, x, y)
        if not path.exists():
            self._cache[key] = None
            return None
        ds = gdal.Open(str(path))
        arr = ds.GetRasterBand(1).ReadAsArray()
        self._cache[key] = arr
        return arr

    def present_lods(self) -> list[int]:
        if not self.root.exists():
            return []
        return sorted(int(p.name) for p in self.root.iterdir() if p.is_dir() and p.name.isdigit())


def boundary_tiles(face: int, lod: int, edge_index: int):
    """All tile (x, y) on `face` that sit on the given cardinal boundary."""
    tile_count = 1 << lod
    rng = range(tile_count)
    if edge_index == 0:  # up -> y == 0
        return [(x, 0) for x in rng]
    if edge_index == 1:  # right -> x == tile_count - 1
        return [(tile_count - 1, y) for y in rng]
    if edge_index == 2:  # down -> y == tile_count - 1
        return [(x, tile_count - 1) for x in rng]
    if edge_index == 3:  # left -> x == 0
        return [(0, y) for y in rng]
    raise ValueError(edge_index)


class EdgeGeometry:
    """Precomputes the stitch.rs / reconcile.rs pixel offset tables for the
    4 cardinal directions, given texture/border size."""

    def __init__(self, texture_size: int, border_size: int):
        self.texture_size = texture_size
        self.border_size = border_size
        self.center_size = texture_size - 2 * border_size
        self.offset_size = self.center_size + border_size

        b, c, o = border_size, self.center_size, self.offset_size

        # stitch.rs src_offsets/dst_offsets/sizes (stitch.rs:157-188), cardinal (i=0..3) only.
        self.src_offsets = [(b, c), (b, b), (b, b), (c, b)]
        self.dst_offsets = [(b, 0), (o, b), (b, o), (0, b)]
        self.sizes = [(c, b), (b, c), (c, b), (b, c)]

        # This tile's own outermost *real* row/col nearest a given cardinal side.
        self.own_edge_offsets = [(b, b), (o - 1, b), (b, o - 1), (b, b)]
        self.own_edge_sizes = [(c, 1), (1, c), (c, 1), (1, c)]

    def stitch_src(self, edge_index: int, rotation: int):
        """Rust: stitch.rs::neighbour_data src_offset/src_size selection
        (stitch.rs:96-115), restricted to i < 4."""
        size = self.sizes[edge_index]
        if rotation in (IDENTICAL, SHIFT_U, SHIFT_V):
            return self.src_offsets[edge_index], size
        if rotation == ROTATE_CW:
            return self.src_offsets[(edge_index + 3) % 4], (size[1], size[0])
        if rotation == ROTATE_CCW:
            return self.src_offsets[(edge_index + 1) % 4], (size[1], size[0])
        raise ValueError(rotation)


def check_border_fidelity(grid: TileGrid, geom: EdgeGeometry, face, lod, x, y, edge_index, results):
    tile_arr = grid.load(face, lod, x, y)
    if tile_arr is None:
        return

    nface, nx, ny, rotation = tile_neighbour(face, lod, x, y, edge_index)
    neighbour_arr = grid.load(nface, lod, nx, ny)
    if neighbour_arr is None:
        return

    src_offset, src_size = geom.stitch_src(edge_index, rotation)
    expected = read_window(neighbour_arr, src_offset[0], src_offset[1], src_size[0], src_size[1])
    expected = apply_rotation(expected, rotation)

    dst_offset = geom.dst_offsets[edge_index]
    dst_size = geom.sizes[edge_index]
    actual = read_window(tile_arr, dst_offset[0], dst_offset[1], dst_size[0], dst_size[1])

    if expected.shape != actual.shape:
        results["A_shape_mismatch"].append(
            (face, lod, x, y, edge_index, nface, nx, ny, expected.shape, actual.shape)
        )
        return

    diff = np.abs(expected.astype(np.float64) - actual.astype(np.float64))
    if np.array_equal(expected, actual):
        results["A_pass"] += 1
    else:
        results["A_fail"] += 1
        results["A_failures"].append(
            (face, lod, x, y, edge_index, nface, nx, ny, float(diff.max()), int(np.count_nonzero(diff)))
        )


def check_edge_continuity(grid: TileGrid, geom: EdgeGeometry, face, lod, x, y, edge_index, results, seen_pairs):
    tile_arr = grid.load(face, lod, x, y)
    if tile_arr is None:
        return

    nface, nx, ny, rotation = tile_neighbour(face, lod, x, y, edge_index)

    pair_key = (lod, min((face, x, y), (nface, nx, ny)), max((face, x, y), (nface, nx, ny)))
    if pair_key in seen_pairs:
        return
    seen_pairs.add(pair_key)

    neighbour_arr = grid.load(nface, lod, nx, ny)
    if neighbour_arr is None:
        return

    own_off = geom.own_edge_offsets[edge_index]
    own_size = geom.own_edge_sizes[edge_index]
    own_strip = read_window(tile_arr, own_off[0], own_off[1], own_size[0], own_size[1])

    n_edge_index = neighbour_edge_index(edge_index, rotation)
    n_off = geom.own_edge_offsets[n_edge_index]
    n_size = geom.own_edge_sizes[n_edge_index]
    neighbour_strip = read_window(neighbour_arr, n_off[0], n_off[1], n_size[0], n_size[1])
    neighbour_strip = apply_rotation(neighbour_strip, rotation)

    if own_strip.shape != neighbour_strip.shape:
        results["B_shape_mismatch"].append(
            (face, lod, x, y, edge_index, nface, nx, ny, own_strip.shape, neighbour_strip.shape)
        )
        return

    diff = np.abs(own_strip.astype(np.float64) - neighbour_strip.astype(np.float64))
    max_diff = float(diff.max())
    mean_diff = float(diff.mean())
    results["B_total"] += 1
    results["B_records"].append((face, lod, x, y, edge_index, nface, nx, ny, max_diff, mean_diff))
    if max_diff == 0.0:
        results["B_exact"] += 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--terrain-path", type=Path, default=Path("/home/patrick/Documents/OtherMeans/assets/earth"))
    parser.add_argument("--attachment", default="height")
    parser.add_argument("--texture-size", type=int, default=512)
    parser.add_argument("--border-size", type=int, default=4)
    parser.add_argument("--lod", type=int, default=None, help="restrict to a single LOD")
    parser.add_argument("--face", type=int, default=None, help="restrict to a single face (0-5)")
    parser.add_argument("--threshold", type=float, default=1e-6, help="Check B diff considered notable above this")
    parser.add_argument("--verbose", action="store_true")
    args = parser.parse_args()

    grid = TileGrid(args.terrain_path, args.attachment)
    geom = EdgeGeometry(args.texture_size, args.border_size)

    lods = [args.lod] if args.lod is not None else grid.present_lods()
    if not lods:
        print(f"No LOD directories found under {grid.root}")
        return 1

    faces = [args.face] if args.face is not None else list(range(6))

    results = defaultdict(list)
    results["A_pass"] = 0
    results["A_fail"] = 0
    results["B_total"] = 0
    results["B_exact"] = 0
    seen_pairs = set()

    per_lod_summary = {}

    for lod in lods:
        a_pass_before, a_fail_before = results["A_pass"], results["A_fail"]
        b_total_before, b_exact_before = results["B_total"], results["B_exact"]

        for face in faces:
            for edge_index in range(4):
                for x, y in boundary_tiles(face, lod, edge_index):
                    check_border_fidelity(grid, geom, face, lod, x, y, edge_index, results)
                    check_edge_continuity(grid, geom, face, lod, x, y, edge_index, results, seen_pairs)

        per_lod_summary[lod] = {
            "A_pass": results["A_pass"] - a_pass_before,
            "A_fail": results["A_fail"] - a_fail_before,
            "B_total": results["B_total"] - b_total_before,
            "B_exact": results["B_exact"] - b_exact_before,
        }

    print("=" * 78)
    print("Per-LOD summary")
    print("=" * 78)
    print(f"{'LOD':>4} | {'A pass':>8} {'A fail':>8} | {'B exact':>8} {'B total':>8}")
    for lod in lods:
        s = per_lod_summary[lod]
        print(f"{lod:>4} | {s['A_pass']:>8} {s['A_fail']:>8} | {s['B_exact']:>8} {s['B_total']:>8}")

    if results["A_fail"]:
        print()
        print("=" * 78)
        print(f"Check A FAILURES (border pixel != expected copy from neighbour): {results['A_fail']}")
        print("=" * 78)
        rows = sorted(results["A_failures"], key=lambda r: -r[8])
        for face, lod, x, y, ei, nface, nx, ny, max_diff, n_bad in rows[:40]:
            print(
                f"  face={face} lod={lod} xy=({x},{y}) edge={CARDINAL_NAMES[ei]:5s} "
                f"-> neighbour face={nface} xy=({nx},{ny})  max_diff={max_diff:.6g}  bad_px={n_bad}"
            )
        if len(rows) > 40:
            print(f"  ... and {len(rows) - 40} more")

    notable = [r for r in results["B_records"] if r[8] > args.threshold]
    print()
    print("=" * 78)
    print(f"Check B: real-edge continuity - {len(notable)} tile pairs above threshold ({args.threshold}) "
          f"out of {results['B_total']}")
    print("=" * 78)
    if notable:
        rows = sorted(notable, key=lambda r: -r[8])
        for face, lod, x, y, ei, nface, nx, ny, max_diff, mean_diff in rows[:40]:
            print(
                f"  face={face} lod={lod} xy=({x},{y}) edge={CARDINAL_NAMES[ei]:5s} "
                f"-> neighbour face={nface} xy=({nx},{ny})  max_diff={max_diff:.6g}  mean_diff={mean_diff:.6g}"
            )
        if len(rows) > 40:
            print(f"  ... and {len(rows) - 40} more")

    if results["A_shape_mismatch"] or results["B_shape_mismatch"]:
        print()
        print("!! shape mismatches encountered (bug in this script's geometry, or bad tile) !!")
        for r in results["A_shape_mismatch"]:
            print("  A shape:", r)
        for r in results["B_shape_mismatch"]:
            print("  B shape:", r)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
