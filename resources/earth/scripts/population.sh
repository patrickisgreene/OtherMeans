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

gdal_translate -ot Byte \
    -scale 0 1000 0 255 \
    -exponent 0.3 \
    -r bilinear \
    -outsize 5400 2700 \
    $DATA_DIR/population.tif \
    $BUILD_DIR/population-processed.tif

$WORK_DIR/target/release/texture-processor \
    --overwrite \
    scale-image \
    --width 1080 \
    --height 540 \
    --input $BUILD_DIR/population-processed.tif \
    --output $THUMBS_DIR/population-processed.tif

cp $BUILD_DIR/population-processed.tif $ASSETS_DIR/population.tif
