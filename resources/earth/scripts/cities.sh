#!/usr/bin/env bash

WORK_DIR=$(git rev-parse --show-toplevel)
DATA_DIR=$WORK_DIR/resources/earth/data

cargo build -p cities --release

target/release/cities-preprocess \
    --out-file $WORK_DIR/assets/earth/cities.ron \
    --populated-places $DATA_DIR/places/ne_10m_populated_places.shp
