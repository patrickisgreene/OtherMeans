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
    brightness \
    --brightness=0 \
    --input $DATA_DIR/depth.tif \
    --output $BUILD_DIR/depth-darkened.tif \

$WORK_DIR/target/release/texture-processor \
    --overwrite \
    scale-image \
    --width 1080 \
    --height 540 \
    --input $BUILD_DIR/depth-darkened.tif \
    --output $THUMBS_DIR/depth-processed.tif

$WORK_DIR/target/release/texture-processor \
    contrast \
    --contrast=0 \
    --input $BUILD_DIR/depth-darkened.tif \
    --output $BUILD_DIR/depth-processed.tif \

$WORK_DIR/target/release/texture-processor \
    --overwrite \
    scale-image \
    --width 1080 \
    --height 540 \
    --input $BUILD_DIR/depth-processed.tif \
    --output $THUMBS_DIR/depth-processed.tif
