#!/usr/bin/env bash

WORK_DIR=$(git rev-parse --show-toplevel)
EXAMPLE_DIR=$WORK_DIR/libraries/terrain/example

HEIGHT_URL=https://planetarymaps.usgs.gov/mosaic/Mars_MGS_MOLA_DEM_mosaic_global_463m.tif
COLOR_URL=https://assets.science.nasa.gov/content/dam/science/cds/3d/resources/image/mars/Mars.tif

mkdir -p $EXAMPLE_DIR/preprocess

wget -nc -O $EXAMPLE_DIR/preprocess/height.tif $HEIGHT_URL
wget -nc -O $EXAMPLE_DIR/preprocess/color.tif $COLOR_URL

cargo build --release

# The reprojection is a normalized angular/UV cube-sphere warp, not a real physical
# Mars<->Earth coordinate conversion, so PROJ's (correct, by default) refusal to transform
# between two different celestial bodies' ellipsoids doesn't apply here - this is the
# intended use of the override.
export PROJ_IGNORE_CELESTIAL_BODY=YES

# color.tif (unlike height.tif) carries no CRS/geotransform at all - gdalinfo shows plain
# pixel corners (0,0)-(1440,720), not lon/lat. It's a simple full-globe equirectangular map,
# so a plain -180..180/-90..90 extent is correct; IAU_2015:49900 is Mars' geographic CRS
# (same 3396190m radius as height.tif's projected CRS, recognized directly by this GDAL/PROJ
# build).
gdal_edit.py -a_srs IAU_2015:49900 -a_ullr -180 90 180 -90 $EXAMPLE_DIR/preprocess/color.tif

# height.tif is Int16 (real elevation in metres, -8201..21241, NoData=-32768), not Byte -
# terrain-preprocess's --data-type controls the actual on-disk pixel type it writes into the
# generated tiles, completely independent of --format; a mismatch here means the runtime
# reinterprets the tile bytes as the wrong type (the exact corruption bug hit earlier with the
# Moon's height data). Upcast to Float32 first (lossless - every Int16 value is exactly
# representable) so --data-type source (auto-detected as Float32) lines up with --format r32f,
# storing real metres directly with no lossy quantization or offset reconstruction needed.
rm -f $EXAMPLE_DIR/preprocess/height-float32.tif
gdal_translate -ot Float32 $EXAMPLE_DIR/preprocess/height.tif $EXAMPLE_DIR/preprocess/height-float32.tif

$WORK_DIR/target/release/terrain-preprocess \
    --src-path $EXAMPLE_DIR/preprocess/height-float32.tif \
    --terrain-path $EXAMPLE_DIR/assets/terrain/ \
    --overwrite \
    --lod-count 4 \
    --fill-radius 0.0 \
    --no-data source \
    --attachment-label height \
    --texture-size 512 \
    --border-size 4 \
    --mip-level-count 4 \
    --format r32f \
    --shape mars

$WORK_DIR/target/release/terrain-preprocess \
    --src-path $EXAMPLE_DIR/preprocess/color.tif \
    --terrain-path $EXAMPLE_DIR/assets/terrain/ \
    --overwrite \
    --lod-count 4 \
    --fill-radius 0.0 \
    --no-data source \
    --data-type Byte \
    --attachment-label albedo \
    --texture-size 512 \
    --border-size 4 \
    --mip-level-count 4 \
    --format rgb8u \
    --shape mars
