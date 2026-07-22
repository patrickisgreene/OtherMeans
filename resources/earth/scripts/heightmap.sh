#!/usr/bin/env bash

WORK_DIR=$(git rev-parse --show-toplevel)
DATA_DIR=$WORK_DIR/resources/earth/data
THUMBS_DIR=$WORK_DIR/resources/earth/thumbnails
BUILD_DIR=$WORK_DIR/resources/earth/intermediates
ASSETS_DIR=$WORK_DIR/assets/earth

cargo build -p texture-processor --release

mkdir -p $BUILD_DIR
mkdir -p $THUMBS_DIR
mkdir -p $ASSETS_DIR

# Ocean/lake pixels are forced to exactly 0.0 using the vector-derived ocean mask (real
# coastline + lakes shapefiles, see ocean-mask.sh) rather than trusting the raw topography
# byte's own value - GEBCO's land relief dips at or below its own "sea level" byte in real
# below-sea-level basins (Death Valley, the Caspian depression, etc.), which reading height
# directly (height <= 0 == ocean) would misread as fake inland lakes.
#
# Land pixels are rescaled off that same 0..255 byte into (EPSILON, 1.0], never touching
# exactly 0.0, so 0.0 stays an unambiguous "this is ocean" sentinel no matter how close to sea
# level a land pixel's real elevation is.
EPSILON=0.0000001

gdal_calc.py --quiet --overwrite \
    -A $DATA_DIR/height.tif \
    -B $BUILD_DIR/ocean-mask-inverted.tif \
    --type=Float32 \
    --calc="(($EPSILON) + (A / 255.0) * (1.0 - ($EPSILON))) * (B > 127)" \
    --outfile=$WORK_DIR/resources/earth/height.tif

$WORK_DIR/target/release/texture-processor \
    --overwrite \
    --no-memory-limit \
    scale-image \
    --width 1080 \
    --height 540 \
    --input $WORK_DIR/resources/earth/height.tif \
    --output $THUMBS_DIR/height.tif

# Coarse whole-globe copy for CPU-side sampling at runtime (e.g. buildings placement) - see
# libraries/buildings/src/height.rs. Values are still normalized (0..1, land only, 0.0 =
# ocean sentinel) same as the source; consumers must multiply by the terrain's runtime
# `height_scale` to get real-world metres, matching what the terrain mesh itself displaces by.
gdal_translate -ot Byte -scale 0 1 0 255 -r bilinear -outsize 5400 2700 \
    $WORK_DIR/resources/earth/height.tif \
    $ASSETS_DIR/height.tif
