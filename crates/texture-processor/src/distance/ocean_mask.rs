use std::path::Path;

use gdal::cpl::CslStringList;
use gdal::raster::{Buffer, RasterizeOptions};
use gdal::spatial_ref::SpatialRef;
use gdal::vector::{Geometry, LayerAccess};
use gdal::{Dataset, DriverManager};

use crate::ProcessError;

/// Rasterizes the ocean and lake polygons onto a raster matching the given grid and burns
/// both into a single binary water mask (255 = water, 0 = land).
///
/// Elevation-thresholded masks (e.g. "at or below sea level = ocean") misclassify large
/// stretches of flat, low-lying land: Florida and coastal Louisiana dip close enough to sea
/// level to read as open water or nonexistent lakes, while inland basins/deserts that sit
/// below sea level (e.g. depressions in the Sahara) read as fictitious ocean. Vector polygons
/// traced from an actual coastline/lake survey don't have that failure mode, so this is used
/// as the mask instead of thresholding topography.
///
/// Each geometry is repaired via `make_valid` before rasterizing: Natural Earth's 10m ocean
/// layer has self-intersecting ("bowtie") ring topology in some intricate coastal regions
/// (e.g. China's Bohai/Yellow Sea/Yangtze delta), and GDAL's scanline polygon fill assumes
/// simple (non-self-intersecting) rings - a self-intersecting ring can flip its inside/outside
/// accounting across a whole scanline span, producing large incorrectly-filled regions far
/// outside the true polygon boundary. `make_valid` falls back to the original geometry on
/// failure rather than dropping it, since a still-imperfect fill is better than a hole left by
/// discarding a whole coastal polygon.
pub fn rasterize_water_mask(
    ocean_path: &Path,
    lakes_path: &Path,
    width: usize,
    height: usize,
    geo_transform: [f64; 6],
    spatial_ref: &SpatialRef,
) -> Result<Vec<u8>, ProcessError> {
    let make_valid_opts = CslStringList::new();
    let mut geometries: Vec<Geometry> = Vec::new();
    for path in [ocean_path, lakes_path] {
        let vector_ds = Dataset::open(path)?;
        let mut layer = vector_ds.layer(0)?;
        geometries.extend(
            layer
                .features()
                .filter_map(|feature| feature.geometry().cloned())
                // A feature can carry an empty (not None) geometry - e.g. a null/degenerate
                // shape in the source data - which GDAL's rasterizer still happily "burns" at
                // its uninitialized coordinate, typically (0,0) ("null island"). Drop these
                // before rasterizing so they can't inject phantom burned pixels.
                .filter(|geom| !geom.is_empty())
                .map(|geom| geom.make_valid(&make_valid_opts).unwrap_or(geom)),
        );
    }
    let burn_values = vec![255.0; geometries.len()];

    let driver = DriverManager::get_driver_by_name("MEM")?;
    let mut raster_ds = driver.create_with_band_type::<u8, _>("", width, height, 1)?;
    raster_ds.set_geo_transform(&geo_transform)?;
    raster_ds.set_spatial_ref(spatial_ref)?;

    gdal::raster::rasterize(&mut raster_ds, &[1], &geometries, &burn_values, None)?;

    let data: Buffer<u8> = raster_ds.rasterband(1)?.read_band_as()?;
    Ok(data.data().to_vec())
}

/// Rasterizes a coastline LINE shapefile into a binary seed raster (255 = coastline pixel, 0 =
/// not) for `edt::compute_signed_shore_distance`, in place of `edt::shore_seeds`'s
/// mask-boundary-derived seeds.
///
/// A line has no inside/outside to get wrong, so this is immune to the self-intersecting-ring
/// fill corruption `rasterize_water_mask`'s `make_valid` repair guards against - using it as
/// the seed source decouples EDT seed *placement* accuracy from the water mask's fill quality
/// entirely (the mask is still needed for water/land *classification*, just not for seeding).
/// `ALL_TOUCHED` ensures every pixel a line segment passes through is burned, not just pixels
/// whose center happens to fall under it - without it, thin/diagonal coastline segments can
/// skip pixels and leave seed gaps.
pub fn rasterize_coastline_seeds(
    coastline_path: &Path,
    width: usize,
    height: usize,
    geo_transform: [f64; 6],
    spatial_ref: &SpatialRef,
) -> Result<Vec<u8>, ProcessError> {
    let vector_ds = Dataset::open(coastline_path)?;
    let mut layer = vector_ds.layer(0)?;
    // A feature can carry an empty (not None) geometry, which GDAL's rasterizer still burns at
    // its uninitialized coordinate (typically (0,0), "null island") - this is exactly what
    // produces stray, isolated seed points far from any real coastline: the EDT then radiates
    // visible concentric distance rings outward from each one, right in the middle of the ocean.
    let geometries: Vec<Geometry> = layer
        .features()
        .filter_map(|feature| feature.geometry().cloned())
        .filter(|geom| !geom.is_empty())
        .collect();
    let burn_values = vec![255.0; geometries.len()];

    let driver = DriverManager::get_driver_by_name("MEM")?;
    let mut raster_ds = driver.create_with_band_type::<u8, _>("", width, height, 1)?;
    raster_ds.set_geo_transform(&geo_transform)?;
    raster_ds.set_spatial_ref(spatial_ref)?;

    gdal::raster::rasterize(
        &mut raster_ds,
        &[1],
        &geometries,
        &burn_values,
        Some(RasterizeOptions {
            all_touched: true,
            ..Default::default()
        }),
    )?;

    let data: Buffer<u8> = raster_ds.rasterband(1)?.read_band_as()?;
    Ok(data.data().to_vec())
}
