#!/usr/bin/env bash

WORK_DIR=$(git rev-parse --show-toplevel)
DATA_DIR=$WORK_DIR/resources/earth/data
THUMBS_DIR=$WORK_DIR/resources/earth/thumbnails
BUILD_DIR=$WORK_DIR/resources/earth/intermediates

cargo build -p texture-processor --release

mkdir -p $BUILD_DIR
mkdir -p $THUMBS_DIR

$WORK_DIR/target/release/texture-processor \
    --overwrite \
    render-shapefile \
    --width 21600 \
    --height 10800 \
    --bbox -180 -90 180 90 \
    --input  $DATA_DIR/coastline/ne_10m_coastline.shp \
    --output $BUILD_DIR/coastline.tif

$WORK_DIR/target/release/texture-processor \
    --overwrite \
    scale-image \
    --width 1080 \
    --height 540 \
    --input $BUILD_DIR/coastline.tif \
    --output $THUMBS_DIR/coastline.tif

$WORK_DIR/target/release/texture-processor \
    --overwrite \
    flood-fill \
    --input $BUILD_DIR/coastline.tif \
    --output $BUILD_DIR/coastline-mask.tif \
    --start-point -48.8767 -123.3933


$WORK_DIR/target/release/texture-processor \
    --overwrite \
    scale-image \
    --width 1080 \
    --height 540 \
    --input $BUILD_DIR/coastline-mask.tif \
    --output $THUMBS_DIR/coastline-mask.tif

$WORK_DIR/target/release/texture-processor \
    --overwrite \
    render-shapefile \
    --width 21600 \
    --height 10800 \
    --bbox -180 -90 180 90 \
    --input $DATA_DIR/lakes/ne_10m_lakes.shp \
    --output $BUILD_DIR/lakes-mask.tif


$WORK_DIR/target/release/texture-processor \
    --overwrite \
    scale-image \
    --width 1080 \
    --height 540 \
    --input $BUILD_DIR/lakes-mask.tif \
    --output $THUMBS_DIR/lakes-mask.tif

$WORK_DIR/target/release/texture-processor \
    --overwrite \
    add \
    --output $BUILD_DIR/ocean-mask.tif \
    $BUILD_DIR/coastline-mask.tif \
    $BUILD_DIR/lakes-mask.tif


$WORK_DIR/target/release/texture-processor \
    --overwrite \
    scale-image \
    --width 1080 \
    --height 540 \
    --input $BUILD_DIR/ocean-mask.tif \
    --output $THUMBS_DIR/ocean-mask.tif

$WORK_DIR/target/release/texture-processor \
    --overwrite \
    invert \
    --input $BUILD_DIR/ocean-mask.tif \
    --output $BUILD_DIR/ocean-mask-inverted.tif

$WORK_DIR/target/release/texture-processor \
    --overwrite \
    scale-image \
    --width 1080 \
    --height 540 \
    --input $BUILD_DIR/ocean-mask-inverted.tif \
    --output $THUMBS_DIR/ocean-mask-inverted.tif

$WORK_DIR/target/release/texture-processor \
    --overwrite \
    invert \
    --input $BUILD_DIR/lakes-mask.tif \
    --output $BUILD_DIR/lakes-mask-inverted.tif

$WORK_DIR/target/release/texture-processor \
    --overwrite \
    scale-image \
    --width 1080 \
    --height 540 \
    --input $BUILD_DIR/lakes-mask-inverted.tif \
    --output $THUMBS_DIR/lakes-mask-inverted.tif
