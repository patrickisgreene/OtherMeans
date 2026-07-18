use texture_processor::distance::edt::{
    compute_shore_distance, compute_signed_shore_distance, filter_seeds_near_mask_boundary,
    normalize_distance, normalize_signed_distance, shore_seeds,
};

#[test]
fn shore_seeds_marks_only_water_touching_land() {
    // 3x3: a single water pixel at the center, land everywhere else.
    #[rustfmt::skip]
    let mask = vec![
        0,   0,   0,
        0, 255,   0,
        0,   0,   0,
    ];

    let seeds = shore_seeds(&mask, 3, 3);

    #[rustfmt::skip]
    assert_eq!(seeds, vec![
        0,   0,   0,
        0, 255,   0,
        0,   0,   0,
    ]);
}

#[test]
fn shore_seeds_wraps_columns_at_the_antimeridian_but_not_rows() {
    // 4x3, land only at (2, 1). Column 0's ordinary neighbors (x=1, and
    // wrapped x=3) are both water, so it must NOT be seeded — a broken
    // implementation that treated off-grid x as "land" instead of wrapping
    // would incorrectly seed it here. Rows 0 and 2 are the poles (their
    // up/down neighbor is genuinely off-grid), so every water pixel in them
    // is seeded regardless of the land island in the middle row.
    #[rustfmt::skip]
    let mask = vec![
        255, 255, 255, 255,
        255, 255,   0, 255,
        255, 255, 255, 255,
    ];

    let seeds = shore_seeds(&mask, 4, 3);

    #[rustfmt::skip]
    assert_eq!(seeds, vec![
        255, 255, 255, 255,
          0, 255,   0, 255,
        255, 255, 255, 255,
    ]);
}

#[test]
fn filter_seeds_near_mask_boundary_keeps_close_drops_far() {
    // 3x3: a single water pixel at the center surrounded by land, so the
    // mask's own boundary (`shore_seeds`) is exactly (1, 1).
    #[rustfmt::skip]
    let mask = vec![
        0,   0, 0,
        0, 255, 0,
        0,   0, 0,
    ];
    // Two coastline-derived candidates: one right at the true shore
    // (kept), one at the far corner - sqrt(2) px away, dropped at a 1px
    // cutoff (mirrors filtering out a stray point like Natural Earth's
    // "Null island").
    #[rustfmt::skip]
    let coastline_seeds = vec![
        255,   0, 0,
          0, 255, 0,
          0,   0, 0,
    ];

    let filtered = filter_seeds_near_mask_boundary(&coastline_seeds, &mask, 3, 3, 1.0);

    #[rustfmt::skip]
    assert_eq!(filtered, vec![
        0,   0, 0,
        0, 255, 0,
        0,   0, 0,
    ]);
}

#[test]
fn compute_shore_distance_wraps_at_antimeridian() {
    let mask = vec![255u8; 5]; // all water, width=5, height=1
    let mut seeds = vec![0u8; 5];
    seeds[0] = 255;

    let distance = compute_shore_distance(&mask, &seeds, 5, 1);

    // Column 0 wraps to column 4 (antimeridian), so distances count the
    // short way around the cycle: 0,1,2,2,1 - not 0,1,2,3,4.
    assert_eq!(distance, vec![0.0, 1.0, 2.0, 2.0, 1.0]);
}

#[test]
fn compute_shore_distance_zeroes_land_pixels() {
    let mask = vec![255, 0, 255]; // water, land, water
    let seeds = vec![255, 0, 0]; // seed at x=0

    let distance = compute_shore_distance(&mask, &seeds, 3, 1);

    // Raw (unmasked) distance from the seed would be [0, 1, 1] (x=2 is 1
    // step away via the antimeridian wrap), but x=1 is land so it's forced
    // to 0 regardless of its true distance from the seed.
    assert_eq!(distance, vec![0.0, 0.0, 1.0]);
}

#[test]
fn normalize_distance_clamps_at_cap_and_zeroes_land() {
    let distance = vec![0.0, 5.0, 20.0, 100.0];
    let mask = vec![255, 255, 255, 0]; // last pixel is land
    let cap_px = 10.0;

    let normalized = normalize_distance(&distance, &mask, cap_px);

    // 0/10*255=0; 5/10*255=127.5->127; 20/10*255=510->clamped to 255;
    // land pixel forced to 0 regardless of its (irrelevant) distance.
    assert_eq!(normalized, vec![0, 127, 255, 0]);
}

#[test]
fn compute_signed_shore_distance_is_positive_on_water_negative_on_land() {
    let mask = vec![255, 255, 255, 0, 0]; // water, water, water, land, land
    let seeds = vec![0, 0, 255, 0, 0]; // seed right at the shore

    let signed = compute_signed_shore_distance(&mask, &seeds, 5, 1);

    assert_eq!(signed, vec![2.0, 1.0, 0.0, -1.0, -2.0]);
}

#[test]
fn normalize_signed_distance_centers_shoreline_at_128() {
    let signed = vec![-20.0, -10.0, 0.0, 10.0, 20.0];
    let cap_px = 10.0;

    let normalized = normalize_signed_distance(&signed, cap_px);

    // d/cap clamped to [-1,1], scaled by 127 around a center of 128:
    // -20 -> clamp(-2)=-1 -> 1; -10 -> -1 -> 1; 0 -> 128; 10 -> 255; 20 -> clamp(2)=1 -> 255.
    assert_eq!(normalized, vec![1, 1, 128, 255, 255]);
}
