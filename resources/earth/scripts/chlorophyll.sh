#!/usr/bin/env bash

WORK_DIR=$(git rev-parse --show-toplevel)
DATA_DIR=$WORK_DIR/resources/earth/data
THUMBS_DIR=$WORK_DIR/resources/earth/thumbnails
BUILD_DIR=$WORK_DIR/resources/earth/intermediates

cargo build -p texture-processor --release

mkdir -p $BUILD_DIR
mkdir -p $THUMBS_DIR

gdal_translate -unscale -ot Float32 \
    -a_srs EPSG:4326 \
    -r bilinear \
    -outsize 21600 10800 \
    NETCDF:"$WORK_DIR/resources/earth/data/chlorophyll.nc":pic \
    $WORK_DIR/resources/earth/data/chlorophyll_physical.tif

gdal_translate -ot Byte \
    -scale 0.00001 0.05 0 255 \
    -exponent 0.3 \
    $WORK_DIR/resources/earth/data/chlorophyll_physical.tif \
    $WORK_DIR/resources/earth/data/chlorophyll.tif

$WORK_DIR/target/release/texture-processor \
    --overwrite \
    scale-image \
    --width 1080 \
    --height 540 \
    --input $WORK_DIR/resources/earth/data/chlorophyll.tif \
    --output $WORK_DIR/resources/earth/thumbnails/chlorophyll.tif

$WORK_DIR/target/release/texture-processor \
    --overwrite \
    mul \
    --output $BUILD_DIR/chlorophyll.tif \
    $DATA_DIR/chlorophyll.tif \
    $BUILD_DIR/lakes-mask-inverted.tif

$WORK_DIR/target/release/texture-processor \
    --overwrite \
    scale-image \
    --width 1080 \
    --height 540 \
    --input $BUILD_DIR/chlorophyll.tif \
    --output $THUMBS_DIR/chlorophyll.tif
