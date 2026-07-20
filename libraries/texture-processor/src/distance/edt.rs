const INF: f32 = 1e20;

/// Labels 4-connected land components (mask < 128) with a 1-based id per pixel (0 = water),
/// wrapping at the antimeridian the same way `shore_seeds`/`edt_1d_wrapped` do, since this is a
/// global equirectangular raster where column 0 and column width-1 are the same meridian.
/// Returns the per-pixel label array together with each component's pixel area (`areas[i]` is
/// the area of label `i + 1`).
pub fn label_land_components(mask: &[u8], width: usize, height: usize) -> (Vec<u32>, Vec<u32>) {
    let mut labels = vec![0u32; width * height];
    let mut areas: Vec<u32> = Vec::new();
    let mut stack: Vec<(usize, usize)> = Vec::new();

    let is_land = |x: usize, y: usize| mask[y * width + x] < 128;

    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            if !is_land(x, y) || labels[idx] != 0 {
                continue;
            }

            areas.push(0);
            let label = areas.len() as u32;
            labels[idx] = label;
            stack.push((x, y));

            while let Some((cx, cy)) = stack.pop() {
                areas[(label - 1) as usize] += 1;

                for (dx, dy) in [(-1i64, 0i64), (1, 0), (0, -1), (0, 1)] {
                    let ny = cy as i64 + dy;
                    if ny < 0 || ny >= height as i64 {
                        continue;
                    }
                    let nx = (cx as i64 + dx).rem_euclid(width as i64) as usize;
                    let ny = ny as usize;
                    let nidx = ny * width + nx;

                    if is_land(nx, ny) && labels[nidx] == 0 {
                        labels[nidx] = label;
                        stack.push((nx, ny));
                    }
                }
            }
        }
    }

    (labels, areas)
}

/// Flips land pixels belonging to a component smaller than `min_area_px` to water (255) in
/// `mask`, so they can no longer seed or corroborate a coastline-distance seed - see
/// `distance::run`'s `water_for_seeding`, which uses this on a copy of the real water mask so
/// the real mask (used for the final land/water byte classification) is untouched and tiny
/// islands still show up as land wherever they survive mip-downsampling, just without the
/// disproportionate coastal-fade halo they'd otherwise radiate into open ocean around them.
pub fn erase_small_land_components(mask: &mut [u8], labels: &[u32], areas: &[u32], min_area_px: u32) {
    for (i, &label) in labels.iter().enumerate() {
        if label != 0 && areas[(label - 1) as usize] < min_area_px {
            mask[i] = 255;
        }
    }
}

/// Marks water pixels that touch at least one land pixel (4-connected) - the water-side of
/// the mask's coastline/lakeshore boundary. Pixels off the edge of the raster count as land,
/// so water that runs to the edge of the source grid is seeded there too.
///
/// This is the default/fallback seed source when no coastline line raster is available (see
/// `ocean_mask::rasterize_coastline_seeds`) - it derives seed pixels from the water mask's own
/// fill boundary, which inherits any polygon-rasterization topology errors (self-intersecting
/// rings etc.) baked into that mask. A coastline-line-rasterized seed set is immune to that
/// failure mode (no fill/winding ambiguity for a line) and should be preferred when available.
pub fn shore_seeds(mask: &[u8], width: usize, height: usize) -> Vec<u8> {
    // Longitude wraps at the antimeridian - column 0 and column width-1 are the same meridian
    // and must be treated as adjacent. Latitude does not wrap: the top and bottom rows are the
    // poles, which are genuine edges of the raster, not connected to each other.
    let is_water = |x: isize, y: isize| -> bool {
        if y < 0 || y >= height as isize {
            return false;
        }
        let x = x.rem_euclid(width as isize) as usize;
        mask[y as usize * width + x] >= 128
    };

    let mut seeds = vec![0u8; width * height];
    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            if mask[idx] < 128 {
                continue;
            }
            let (xi, yi) = (x as isize, y as isize);
            let touches_land = !is_water(xi - 1, yi)
                || !is_water(xi + 1, yi)
                || !is_water(xi, yi - 1)
                || !is_water(xi, yi + 1);
            if touches_land {
                seeds[idx] = 255;
            }
        }
    }
    seeds
}

/// Drops coastline-derived seed pixels (see `ocean_mask::rasterize_coastline_seeds`) that the
/// water mask itself doesn't corroborate - i.e. that sit farther than `max_distance_px` from
/// the mask's own boundary (`shore_seeds`). The coastline shapefile isn't guaranteed to agree
/// with the ocean/lake polygon shapefiles pixel-for-pixel: Natural Earth's coastline layer
/// includes at least one feature (`featurecla = "Null island"`) that's a cartographic reference
/// marker at (0,0), not real coastline, and separately contains small islands/atolls at higher
/// detail than the coarser ocean polygon layer resolves, so some real coastline features simply
/// have no nearby land in the mask. Either way, an unfiltered stray seed makes the EDT radiate a
/// visible, spurious concentric distance ring outward from the middle of open water - keeping
/// only seeds within a short distance of the mask's own boundary discards both failure modes
/// without needing to special-case them by name/attribute.
pub fn filter_seeds_near_mask_boundary(
    coastline_seeds: &[u8],
    mask: &[u8],
    width: usize,
    height: usize,
    max_distance_px: f32,
) -> Vec<u8> {
    let boundary = shore_seeds(mask, width, height);

    let mut field = vec![INF; width * height];
    for (i, &s) in boundary.iter().enumerate() {
        if s >= 128 {
            field[i] = 0.0;
        }
    }
    edt_2d(&mut field, width, height);

    coastline_seeds
        .iter()
        .zip(field.iter())
        .map(|(&s, &d)| if s >= 128 && d <= max_distance_px { 255 } else { 0 })
        .collect()
}

/// EDT distance from `seeds` (see `shore_seeds` or `ocean_mask::rasterize_coastline_seeds`),
/// masked to water pixels only. 0 = at the shoreline, higher = further from shore.
pub fn compute_shore_distance(mask: &[u8], seeds: &[u8], width: usize, height: usize) -> Vec<f32> {
    let mut field = vec![INF; width * height];
    for (i, &s) in seeds.iter().enumerate() {
        if s >= 128 {
            field[i] = 0.0;
        }
    }
    edt_2d(&mut field, width, height);

    // Zero out the land side - we only want water-pixel distances.
    for (i, &m) in mask.iter().enumerate() {
        if m < 128 {
            field[i] = 0.0;
        }
    }
    field
}

/// Normalizes shore distances to 0-255, clamping at `cap_px` so the gradient spans the
/// coastal band rather than the full width of an ocean. Land pixels are always 0.
pub fn normalize_distance(distance: &[f32], mask: &[u8], cap_px: f32) -> Vec<u8> {
    distance
        .iter()
        .zip(mask.iter())
        .map(|(&d, &o)| {
            if o < 128 {
                0
            } else {
                ((d / cap_px) * 255.0).min(255.0) as u8
            }
        })
        .collect()
}

/// Signed EDT distance to the coastline: positive on water (same magnitude as
/// `compute_shore_distance`), negative on land, ~0 exactly at the true shoreline on both sides -
/// unlike the one-sided `compute_shore_distance` (which is a flat 0 everywhere on land), this is
/// usable directly as a land/ocean discriminant (its sign) without a separate binary mask,
/// because it's a smooth, continuous field that stays well-behaved under bilinear/mip filtering.
/// Reuses `compute_shore_distance` on the complementary mask to get the land-side distance for
/// free, since "distance from shore, staying on land" is the same computation as
/// "distance from shore, staying on water" with land/water swapped. The same `seeds` are used
/// for both passes - the coastline itself has no land/water side, only the `mask`/inverted-`mask`
/// zero-out step afterward decides which side of it each pixel's distance belongs to.
pub fn compute_signed_shore_distance(mask: &[u8], seeds: &[u8], width: usize, height: usize) -> Vec<f32> {
    let water_distance = compute_shore_distance(mask, seeds, width, height);
    let inverted_mask: Vec<u8> = mask.iter().map(|&m| 255 - m).collect();
    let land_distance = compute_shore_distance(&inverted_mask, seeds, width, height);

    water_distance
        .iter()
        .zip(land_distance.iter())
        .map(|(&w, &l)| w - l)
        .collect()
}

/// Normalizes a signed shore distance (see `compute_signed_shore_distance`) to a Byte, clamping
/// magnitude at `cap_px` on both sides. 128 = the coastline itself; below 128 = land (0 = deep
/// inland, capped), above 128 = water (255 = deep ocean, capped).
pub fn normalize_signed_distance(signed: &[f32], cap_px: f32) -> Vec<u8> {
    signed
        .iter()
        .map(|&d| (128.0 + (d / cap_px).clamp(-1.0, 1.0) * 127.0) as u8)
        .collect()
}

/// 2D Euclidean distance transform (Felzenszwalb-Huttenlocher): separable row then column
/// passes, followed by sqrt. Operates in-place on a squared-distance field.
///
/// Rows (longitude) wrap at the antimeridian via `edt_1d_wrapped`; columns (latitude) use the
/// ordinary bounded `edt_1d` since the poles are genuine edges of the raster, not connected to
/// each other.
fn edt_2d(field: &mut [f32], width: usize, height: usize) {
    let mut row = vec![0.0f32; width];
    for y in 0..height {
        let start = y * width;
        row.copy_from_slice(&field[start..start + width]);
        edt_1d_wrapped(&mut row);
        field[start..start + width].copy_from_slice(&row);
    }

    let mut col = vec![0.0f32; height];
    for x in 0..width {
        for y in 0..height {
            col[y] = field[y * width + x];
        }
        edt_1d(&mut col);
        for y in 0..height {
            field[y * width + x] = col[y];
        }
    }

    for v in field.iter_mut() {
        *v = v.sqrt();
    }
}

/// Like `edt_1d`, but treats the array as cyclic (index 0 and index n-1 are adjacent) instead
/// of bounded - for rows of a global equirectangular raster, whose left and right edges are
/// both the antimeridian and should connect seamlessly rather than act as a hard boundary.
/// Without this, e.g. a coastal point at -179.9 degrees longitude can't "see" a coastline just
/// across the date line at +179.9 degrees, producing a sharp discontinuity in shore distance
/// right at the seam (confirmed directly: a real source raster read 255 at column 0 and 0 at
/// the last column of the very same row, despite both representing the same meridian).
///
/// Computed by tiling the row three times and running the ordinary (bounded) 1D EDT over the
/// whole 3n-length array, then keeping just the middle copy: on a cyclic domain of period n,
/// any point's true nearest seed is at most n/2 away, well within the one extra period
/// available on either side of the middle copy, so the tiled result already contains the exact
/// wrapped answer for every position in the middle third.
fn edt_1d_wrapped(f: &mut [f32]) {
    let n = f.len();
    if n <= 1 {
        return;
    }

    let mut tiled = Vec::with_capacity(n * 3);
    tiled.extend_from_slice(f);
    tiled.extend_from_slice(f);
    tiled.extend_from_slice(f);

    edt_1d(&mut tiled);

    f.copy_from_slice(&tiled[n..n * 2]);
}

/// 1D squared Euclidean distance transform (Felzenszwalb-Huttenlocher).
///
/// Computes `d[i] = min_j ((i - j)^2 + f[j])` for all i, writing the result back into the
/// slice. Entries that are not finite are skipped as parabola centres (they never contribute
/// to the lower envelope).
fn edt_1d(f: &mut [f32]) {
    let n = f.len();
    if n <= 1 {
        return;
    }

    let mut first = 0;
    while first < n && !f[first].is_finite() {
        first += 1;
    }
    if first == n {
        return;
    }

    let mut v: Vec<usize> = vec![0; n];
    let mut z: Vec<f32> = vec![0.0; n + 1];
    let mut k = 0;

    v[0] = first;
    z[0] = -INF;
    z[1] = INF;

    for q in (first + 1)..n {
        if !f[q].is_finite() {
            continue;
        }

        let mut s = ((f[q] + (q * q) as f32) - (f[v[k]] + (v[k] * v[k]) as f32))
            / (2.0 * (q - v[k]) as f32);

        while s <= z[k] {
            k -= 1;
            if k > 0 {
                s = ((f[q] + (q * q) as f32) - (f[v[k]] + (v[k] * v[k]) as f32))
                    / (2.0 * (q - v[k]) as f32);
            }
        }

        k += 1;
        v[k] = q;
        z[k] = s;
        z[k + 1] = INF;
    }

    k = 0;
    let mut d: Vec<f32> = vec![0.0; n];
    for q in 0..n {
        while z[k + 1] < q as f32 {
            k += 1;
        }
        let diff = q as f32 - v[k] as f32;
        d[q] = diff * diff + f[v[k]];
    }

    f.copy_from_slice(&d);
}
