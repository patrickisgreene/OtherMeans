#!/usr/bin/env bash

WORK_DIR=$(git rev-parse --show-toplevel)
DATA_DIR=$WORK_DIR/resources/earth/data

cargo build -p shipping-lanes --release

$WORK_DIR/target/release/shipping-lanes-preprocessor \
    --out-file $WORK_DIR/assets/earth/shipping.ron \
    --roads $DATA_DIR/shipping/Shipping-Lanes-main/data/Shipping-Lanes-v1/Shipping-Lanes-v1.shp
