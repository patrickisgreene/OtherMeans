use std::path::PathBuf;

use clap::Parser;

use crate::TextureFormat;

#[derive(Parser, Debug, Clone)]
#[command(name = "distance-map", author, version, about)]
pub struct DistanceField {
    /// Reference raster (e.g. ETOPO2022 bed elevation) whose grid - size, geotransform, and
    /// spatial reference - the output distance map is aligned to. Its pixel values are not
    /// used; only its georeferencing matters.
    #[arg(long = "topography-path", required = true)]
    pub topography_path: PathBuf,

    /// Ocean polygon shapefile (.shp), e.g. Natural Earth ne_10m_ocean.
    #[arg(long = "ocean-path", required = true)]
    pub ocean_path: PathBuf,

    /// Lake polygon shapefile (.shp), e.g. Natural Earth ne_10m_lakes. Combined with the
    /// ocean polygons into one water mask, so lakes get correct shorelines too instead of
    /// being invisible to (or faked by) an elevation-based mask.
    #[arg(long = "lakes-path", required = true)]
    pub lakes_path: PathBuf,

    /// Optional coastline LINE shapefile (.shp), e.g. Natural Earth ne_10m_coastline. When
    /// given, seeds the distance transform from this rasterized line instead of the water
    /// mask's own fill boundary - a line has no inside/outside to get wrong, so this is immune
    /// to the self-intersecting-polygon fill corruption `--ocean-path`/`--lakes-path` can hit
    /// in intricate coastal regions (see `ocean_mask::rasterize_coastline_seeds`). Falls back
    /// to the mask-boundary-derived seeds (`edt::shore_seeds`) when omitted.
    #[arg(long = "coastline-path")]
    pub coastline_path: Option<PathBuf>,

    #[arg(long = "out-path", required = true)]
    pub out_path: PathBuf,

    /// Also write the raw binary water mask (255 = water, 0 = land) used to seed the
    /// distance transform, as its own single-band Byte GeoTIFF. Lets a downstream tool use
    /// the exact same land/ocean split this run computed, instead of re-deriving it.
    #[arg(long = "mask-out-path")]
    pub mask_out_path: Option<PathBuf>,

    /// Applies to both `--out-path` and `--mask-out-path`. `Tiff` (the default) writes a
    /// GeoTIFF via GDAL, carrying the same georeferencing (`geo_transform`/`spatial_ref`) as
    /// `--topography-path`; `Png` has no standard mechanism for that metadata, so it's written
    /// as a plain grayscale PNG instead, like every other command's PNG output.
    #[arg(long, short, default_value_t = TextureFormat::Tiff)]
    pub format: TextureFormat,

    /// Distances beyond this, on either side of the coastline, are clamped (255 = deep water,
    /// 0 = deep inland), so the byte gradient spans the coastal band rather than being wasted on
    /// visually-irrelevant open ocean/interior land.
    #[arg(long = "distance-cap-km", default_value_t = 278.0)]
    pub distance_cap_km: f64,

    /// Land components (islands) smaller than this are excluded from seeding the distance
    /// transform, so they don't leave a coastal-fade halo in open ocean out of proportion to
    /// their real size. Terrain LOD tiling mip-averages the height texture per tile, so an
    /// island much smaller than one coarse-LOD texel's footprint washes out of the visible
    /// terrain entirely there - but the distance field is computed once at full source
    /// resolution and simply resampled per LOD, so without this filter its "near coast" signal
    /// survives and drives shore-foam/color-fade for land that, at that zoom level, doesn't
    /// actually render. 200 km^2 excludes remote small islands (e.g. Ascension ~91 km^2, Saint
    /// Helena ~122 km^2) while keeping substantial ones (e.g. Malta ~316 km^2) that still read
    /// as real land even fairly zoomed out.
    #[arg(long = "min-island-area-km2", default_value_t = 200.0)]
    pub min_island_area_km2: f64,
}
