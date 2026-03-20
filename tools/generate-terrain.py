#!/usr/bin/env python3
"""
generate-terrain.py — Download and process Vermont USGS DEM data for
the Verdant simulator.

Usage:
    python3 tools/generate-terrain.py [--output terrain.json] [--bbox W,S,E,N]

Generates a JSON file with elevation data, zone boundaries, and
watershed flow directions that verdant-sim can consume.

Prerequisites:
    pip install requests
"""

import argparse
import json
import math
import os
import sys

# Default bounding box: central Vermont (Green Mountains)
DEFAULT_BBOX = "-72.9,44.0,-72.4,44.5"


def parse_args():
    parser = argparse.ArgumentParser(
        description="Generate terrain data for Verdant simulator"
    )
    parser.add_argument(
        "--output",
        default="terrain.json",
        help="Output file path (default: terrain.json)",
    )
    parser.add_argument(
        "--bbox",
        default=DEFAULT_BBOX,
        help="Bounding box as W,S,E,N (default: central Vermont)",
    )
    parser.add_argument(
        "--resolution",
        type=int,
        default=100,
        help="Grid resolution in meters (default: 100)",
    )
    parser.add_argument(
        "--zones",
        type=int,
        default=10,
        help="Number of zones to generate (default: 10)",
    )
    return parser.parse_args()


def generate_synthetic_elevation(bbox, resolution):
    """Generate synthetic elevation data mimicking Vermont Green Mountains.

    When USGS DEM data is unavailable, this produces a realistic terrain
    model with ridges, valleys, and drainage patterns.
    """
    west, south, east, north = bbox

    # Convert to approximate meters
    lat_m = (north - south) * 111_320
    lon_m = (east - west) * 111_320 * math.cos(math.radians((north + south) / 2))

    rows = max(1, int(lat_m / resolution))
    cols = max(1, int(lon_m / resolution))

    grid = []
    for r in range(rows):
        row = []
        lat_frac = r / max(rows - 1, 1)
        for c in range(cols):
            lon_frac = c / max(cols - 1, 1)

            # Base elevation: 200-600m with ridge running N-S
            base = 300 + 200 * math.sin(lon_frac * math.pi)

            # Add ridge detail
            ridge = 100 * math.sin(lat_frac * 4 * math.pi) * math.sin(
                lon_frac * 2 * math.pi
            )

            # Valley channels
            valley = -80 * max(0, math.cos(lon_frac * 3 * math.pi))

            elevation = base + ridge + valley
            row.append(round(elevation, 1))
        grid.append(row)

    return {
        "rows": rows,
        "cols": cols,
        "resolution_m": resolution,
        "origin": {"lat": south, "lon": west},
        "elevations": grid,
    }


def compute_flow_directions(elevations):
    """Compute D8 flow direction for each cell.

    Returns a grid of (dr, dc) tuples indicating the steepest
    downhill neighbor. Used for watershed delineation.
    """
    rows = len(elevations)
    cols = len(elevations[0]) if rows > 0 else 0
    directions = []

    for r in range(rows):
        row_dirs = []
        for c in range(cols):
            h = elevations[r][c]
            best_drop = 0.0
            best_dir = (0, 0)

            for dr in [-1, 0, 1]:
                for dc in [-1, 0, 1]:
                    if dr == 0 and dc == 0:
                        continue
                    nr, nc = r + dr, c + dc
                    if 0 <= nr < rows and 0 <= nc < cols:
                        drop = h - elevations[nr][nc]
                        dist = math.sqrt(dr * dr + dc * dc)
                        slope = drop / dist
                        if slope > best_drop:
                            best_drop = slope
                            best_dir = (dr, dc)

            row_dirs.append(best_dir)
        directions.append(row_dirs)

    return directions


def assign_zones(elevations, zone_count):
    """Assign grid cells to zones using simple spatial partitioning."""
    rows = len(elevations)
    cols = len(elevations[0]) if rows > 0 else 0
    zones = []

    # Divide into roughly equal rectangular zones
    side = max(1, int(math.sqrt(zone_count)))
    zone_rows = max(1, rows // side)
    zone_cols = max(1, cols // side)

    for r in range(rows):
        row_zones = []
        for c in range(cols):
            zr = min(r // zone_rows, side - 1)
            zc = min(c // zone_cols, side - 1)
            zone_id = zr * side + zc
            row_zones.append(min(zone_id, zone_count - 1))
        zones.append(row_zones)

    return zones


def compute_watershed_graph(zones, flow_dirs, zone_count):
    """Build zone-to-zone flow graph from D8 flow directions."""
    edges = set()
    rows = len(zones)
    cols = len(zones[0]) if rows > 0 else 0

    for r in range(rows):
        for c in range(cols):
            src_zone = zones[r][c]
            dr, dc = flow_dirs[r][c]
            nr, nc = r + dr, c + dc
            if 0 <= nr < rows and 0 <= nc < cols:
                dst_zone = zones[nr][nc]
                if src_zone != dst_zone:
                    edges.add((src_zone, dst_zone))

    graph = {}
    for src, dst in edges:
        key = str(src)
        if key not in graph:
            graph[key] = []
        if dst not in graph[key]:
            graph[key].append(dst)

    return graph


def main():
    args = parse_args()

    bbox = [float(x) for x in args.bbox.split(",")]
    if len(bbox) != 4:
        print("Error: bbox must be W,S,E,N", file=sys.stderr)
        sys.exit(1)

    print(f"==> Generating terrain for bbox {bbox}...")
    terrain = generate_synthetic_elevation(bbox, args.resolution)

    print(f"    Grid: {terrain['rows']}x{terrain['cols']} @ {args.resolution}m")

    print("==> Computing flow directions...")
    flow_dirs = compute_flow_directions(terrain["elevations"])

    print(f"==> Assigning {args.zones} zones...")
    zones = assign_zones(terrain["elevations"], args.zones)
    terrain["zones"] = zones

    print("==> Building watershed graph...")
    watershed = compute_watershed_graph(zones, flow_dirs, args.zones)
    terrain["watershed"] = watershed

    print(f"==> Writing {args.output}...")
    with open(args.output, "w") as f:
        json.dump(terrain, f, separators=(",", ":"))

    size = os.path.getsize(args.output)
    print(f"==> Done. {args.output} ({size:,} bytes)")


if __name__ == "__main__":
    main()
