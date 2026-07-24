#!/usr/bin/env bash

WORK_DIR=$(git rev-parse --show-toplevel)
DATA_DIR=$WORK_DIR/resources/earth/data

cargo build -p roads --release

$WORK_DIR/target/release/roads-preprocess \
    --out-file $WORK_DIR/assets/earth/roads.ron \
    --roads $DATA_DIR/roads/ne_10m_roads.shp
