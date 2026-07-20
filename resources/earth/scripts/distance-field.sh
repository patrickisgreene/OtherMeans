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
    distance-field \
    --topography-path $DATA_DIR/height.tif \
    --ocean-path $DATA_DIR/ocean/ne_10m_ocean.shp \
    --lakes-path $DATA_DIR/lakes/ne_10m_lakes.shp \
    --coastline-path $DATA_DIR/coastline/ne_10m_coastline.shp \
    --out-path $BUILD_DIR/distance.tif

$WORK_DIR/target/release/texture-processor \
    --overwrite \
    scale-image \
    --width 1080 \
    --height 540 \
    --input $BUILD_DIR/distance.tif \
    --output $THUMBS_DIR/distance.tif
