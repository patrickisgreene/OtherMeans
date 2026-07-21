#!/usr/bin/env bash

WORK_DIR=$(git rev-parse --show-toplevel)
DATA_DIR=$WORK_DIR/resources/earth/data
THUMBS_DIR=$WORK_DIR/resources/earth/thumbnails
BUILD_DIR=$WORK_DIR/resources/earth/intermediates

cargo build -p texture-processor --release

mkdir -p $BUILD_DIR
mkdir -p $THUMBS_DIR

# Land and water are baked as separate terrain attachments (see preprocess.sh) instead of being
# merged into one texture, so there's no hard per-pixel cut for resampling/mipmapping to blend
# across at the coastline - each texture's own mip chain only ever contains its own kind of data.
cp $DATA_DIR/color.tif $WORK_DIR/resources/earth/land.tif

$WORK_DIR/target/release/texture-processor \
    --overwrite \
    rgba-concat \
    --red $BUILD_DIR/depth-processed.tif \
    --green $BUILD_DIR/chlorophyll.tif \
    --blue $BUILD_DIR/distance.tif \
    --output $WORK_DIR/resources/earth/water.tif

$WORK_DIR/target/release/texture-processor \
    --overwrite \
    --no-memory-limit \
    scale-image \
    --width 1080 \
    --height 540 \
    --input $WORK_DIR/resources/earth/land.tif \
    --output $THUMBS_DIR/land.tif

$WORK_DIR/target/release/texture-processor \
    --overwrite \
    --no-memory-limit \
    scale-image \
    --width 1080 \
    --height 540 \
    --input $WORK_DIR/resources/earth/water.tif \
    --output $THUMBS_DIR/water.tif
