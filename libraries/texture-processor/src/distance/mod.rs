mod cli;
pub mod edt;
pub mod ocean_mask;

use std::path::Path;

use gdal::raster::Buffer;
use gdal::spatial_ref::SpatialRef;
use gdal::{Dataset, DriverManager};

use crate::cli::Cli;
use crate::utils::Progress;
use crate::{ProcessError, TextureFormat};
pub use cli::DistanceField;

const KM_PER_DEGREE_EQUATOR: f64 = 111.32;

/// How far a coastline-derived seed pixel (see `ocean_mask::rasterize_coastline_seeds`) may sit
/// from the water mask's own boundary before it's discarded as unreliable (see
/// `edt::filter_seeds_near_mask_boundary`) - generous enough to keep real coastline even where
/// the two datasets disagree slightly on exact placement (measured well under 5km in practice),
/// while safely rejecting things like Natural Earth's literal "Null island" reference point and
/// small islands missing from the coarser ocean polygon layer (measured at 35km+ away).
const COASTLINE_SEED_MAX_DRIFT_KM: f64 = 20.0;

pub fn run(cli: Cli, args: cli::DistanceField) -> Result<(), ProcessError> {
    if !cli.overwrite && args.out_path.exists() {
        println!("distance-map output already exists, skipping");
        return Ok(());
    }
    if let Some(parent) = args.out_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let topo_ds = Dataset::open(&args.topography_path)?;
    let (width, height) = topo_ds.raster_size();
    let geo_transform = topo_ds.geo_transform()?;
    let spatial_ref = topo_ds.spatial_ref().unwrap_or_else(|_| {
        SpatialRef::from_proj4("+proj=lonlat +ellps=WGS84 +datum=WGS84")
            .expect("WGS84 proj4 must parse")
    });

    let water = {
        let _progress = Progress::new(cli.display_format, "distance-water-mask", None);
        let water = ocean_mask::rasterize_water_mask(
            &args.ocean_path,
            &args.lakes_path,
            width,
            height,
            geo_transform,
            &spatial_ref,
        )?;

        if let Some(mask_out_path) = &args.mask_out_path {
            if let Some(parent) = mask_out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            write_raster(
                mask_out_path,
                water.clone(),
                width,
                height,
                args.format,
                geo_transform,
                &spatial_ref,
            )?;
        }

        water
    };

    let pixel_size_km = geo_transform[1].abs() * KM_PER_DEGREE_EQUATOR;

    // Seeding-only copy of the water mask with small islands erased (flipped to water) - see
    // `edt::erase_small_land_components`. `water` itself stays untouched so the final land/water
    // byte classification below still marks these islands as land wherever they survive.
    let water_for_seeding = {
        let _progress = Progress::new(cli.display_format, "distance-small-islands", None);
        let (labels, areas) = edt::label_land_components(&water, width, height);
        let min_area_px = (args.min_island_area_km2 / (pixel_size_km * pixel_size_km)).max(0.0) as u32;
        let mut masked = water.clone();
        edt::erase_small_land_components(&mut masked, &labels, &areas, min_area_px);
        masked
    };

    let seeds = {
        let _progress = Progress::new(cli.display_format, "distance-seeds", None);
        match &args.coastline_path {
            Some(coastline_path) => {
                let raw_seeds = ocean_mask::rasterize_coastline_seeds(
                    coastline_path,
                    width,
                    height,
                    geo_transform,
                    &spatial_ref,
                )?;
                let max_drift_px = (COASTLINE_SEED_MAX_DRIFT_KM / pixel_size_km) as f32;
                edt::filter_seeds_near_mask_boundary(&raw_seeds, &water_for_seeding, width, height, max_drift_px)
            }
            None => edt::shore_seeds(&water_for_seeding, width, height),
        }
    };

    {
        let _progress = Progress::new(cli.display_format, "distance-shore-distance", None);
        let distance = edt::compute_signed_shore_distance(&water, &seeds, width, height);

        let cap_px = (args.distance_cap_km / pixel_size_km) as f32;
        let normalized = edt::normalize_signed_distance(&distance, cap_px);

        write_raster(
            &args.out_path,
            normalized,
            width,
            height,
            args.format,
            geo_transform,
            &spatial_ref,
        )?;
    }

    Ok(())
}

/// Writes a single-band Byte raster. `Tiff` goes through GDAL's `GTiff` driver so the output
/// carries the same georeferencing (`geo_transform`/`spatial_ref`) as `--topography-path`;
/// `Png` has no standard mechanism for that metadata, so it's written as a plain grayscale PNG
/// via the `image` crate instead, matching every other command's PNG output.
pub fn write_raster(
    path: &Path,
    data: Vec<u8>,
    width: usize,
    height: usize,
    format: TextureFormat,
    geo_transform: [f64; 6],
    spatial_ref: &SpatialRef,
) -> Result<(), ProcessError> {
    match format {
        TextureFormat::Tiff => {
            let driver = DriverManager::get_driver_by_name("GTiff")?;
            let mut ds = driver.create_with_band_type::<u8, _>(path, width, height, 1)?;
            ds.set_geo_transform(&geo_transform)?;
            ds.set_spatial_ref(spatial_ref)?;
            let mut band = ds.rasterband(1)?;
            let mut buf = Buffer::<u8>::new((width, height), data);
            band.write((0, 0), (width, height), &mut buf)?;
        }
        TextureFormat::Png => {
            let image = image::GrayImage::from_raw(width as u32, height as u32, data)
                .expect("data.len() == width * height, since every write_raster caller builds it that way");
            let mut file = std::fs::File::create(path)?;
            image::DynamicImage::ImageLuma8(image).write_to(&mut file, TextureFormat::Png.into())?;
        }
    }
    Ok(())
}
