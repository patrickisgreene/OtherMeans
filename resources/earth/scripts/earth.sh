#!/usr/bin/env bash

WORK_DIR=$(git rev-parse --show-toplevel)
DATA_DIR=$WORK_DIR/resources/earth/data
THUMBS_DIR=$WORK_DIR/resources/earth/thumbnails
BUILD_DIR=$WORK_DIR/resources/earth/intermediates

cargo build -p texture-processor --release

mkdir -p $BUILD_DIR
mkdir -p $THUMBS_DIR

# earth.tif packs (depth, chlorophyll, distance) directly into ocean pixels' r/g/b (see
# mask-merge below), read back as real color by anything that samples the attachment - the
# shader already zeroes r/g out on land via ocean_blend, so it never matters there, but
# terrain-preprocess's bilinear GDALWarp resampling blends a handful of destination pixels'
# worth of neighboring source pixels together regardless of what the shader does at render
# time. Right at the coast that means real land color gets blended with an unrelated-looking
# (depth, chlorophyll, ~128) byte triple, which reads as a bright magenta/pink fringe hugging
# every coastline. distance.tif (byte 128 = coastline, increasing seaward - see
# distance-field.sh) is already exactly the signal needed to fade depth/chlorophyll to 0 within
# COASTAL_FADE_BYTES of the coast, so any warp-time bleed there lands on a harmless dark/blue
# tint instead - full-strength data survives everywhere the debug views (render_mode 5/6)
# actually care about it, well out from the coast.
COASTAL_FADE_BYTES=20

gdal_calc.py --quiet --overwrite \
    -A $BUILD_DIR/depth-processed.tif \
    -B $BUILD_DIR/distance.tif \
    --type=Byte \
    --calc="(A * numpy.clip((B.astype(float) - 128.0) / $COASTAL_FADE_BYTES, 0.0, 1.0)).astype(numpy.uint8)" \
    --outfile=$BUILD_DIR/depth-faded.tif

gdal_calc.py --quiet --overwrite \
    -A $BUILD_DIR/chlorophyll.tif \
    -B $BUILD_DIR/distance.tif \
    --type=Byte \
    --calc="(A * numpy.clip((B.astype(float) - 128.0) / $COASTAL_FADE_BYTES, 0.0, 1.0)).astype(numpy.uint8)" \
    --outfile=$BUILD_DIR/chlorophyll-faded.tif

$WORK_DIR/target/release/texture-processor \
    --overwrite \
    rgba-concat \
    --red $BUILD_DIR/depth-faded.tif \
    --green $BUILD_DIR/chlorophyll-faded.tif \
    --blue $BUILD_DIR/distance.tif \
    --output $BUILD_DIR/water.tif

$WORK_DIR/target/release/texture-processor \
    --overwrite \
    --no-memory-limit \
    scale-image \
    --width 1080 \
    --height 540 \
    --input $BUILD_DIR/water.tif \
    --output $THUMBS_DIR/water.tif

$WORK_DIR/target/release/texture-processor \
    --overwrite \
    mask-merge \
    --mask $BUILD_DIR/ocean-mask.tif \
    --output $WORK_DIR/resources/earth/earth.tif \
    $DATA_DIR/color.tif \
    $BUILD_DIR/water.tif

$WORK_DIR/target/release/texture-processor \
    --overwrite \
    --no-memory-limit \
    scale-image \
    --width 1080 \
    --height 540 \
    --input $WORK_DIR/resources/earth/earth.tif \
    --output $THUMBS_DIR/earth.tif
