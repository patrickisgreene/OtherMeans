#!/usr/bin/env bash

WORK_DIR=$(git rev-parse --show-toplevel)
EARTH_DIR=$WORK_DIR/resources/earth/
SCRIPT_DIR=$EARTH_DIR/scripts

$SCRIPT_DIR/download.sh
$SCRIPT_DIR/ocean-mask.sh
$SCRIPT_DIR/chlorophyll.sh
$SCRIPT_DIR/bathyometry.sh
$SCRIPT_DIR/distance-field.sh
$SCRIPT_DIR/earth.sh
$SCRIPT_DIR/heightmap.sh
$SCRIPT_DIR/cities.sh
$SCRIPT_DIR/population.sh

rm -f $EARTH_DIR/height-float32.tif
gdal_translate -ot Float32 $EARTH_DIR/height.tif $EARTH_DIR/height-float32.tif

gdal_edit.py -a_srs IAU_2015:39900 -a_ullr -180 90 180 -90 $EARTH_DIR/height-float32.tif
gdal_edit.py -a_srs IAU_2015:39900 -a_ullr -180 90 180 -90 $EARTH_DIR/land.tif
gdal_edit.py -a_srs IAU_2015:39900 -a_ullr -180 90 180 -90 $EARTH_DIR/water.tif

$WORK_DIR/target/release/terrain-preprocess \
    --src-path $WORK_DIR/resources/earth/height-float32.tif \
    --terrain-path $WORK_DIR/assets/earth/ \
    --overwrite \
    --lod-count 6 \
    --fill-radius 16.0 \
    --no-data source \
    --attachment-label height \
    --texture-size 512 \
    --border-size 4 \
    --mip-level-count 4 \
    --format r32f \
    --shape earth

$WORK_DIR/target/release/terrain-preprocess \
    --src-path $WORK_DIR/resources/earth/land.tif \
    --terrain-path $WORK_DIR/assets/earth/ \
    --overwrite \
    --lod-count 6 \
    --fill-radius 16.0 \
    --no-data source \
    --data-type Byte \
    --attachment-label earth \
    --texture-size 512 \
    --border-size 4 \
    --mip-level-count 4 \
    --format rgb8u \
    --shape earth

$WORK_DIR/target/release/terrain-preprocess \
    --src-path $WORK_DIR/resources/earth/water.tif \
    --terrain-path $WORK_DIR/assets/earth/ \
    --overwrite \
    --lod-count 6 \
    --fill-radius 16.0 \
    --no-data source \
    --data-type Byte \
    --attachment-label water \
    --texture-size 256 \
    --border-size 4 \
    --mip-level-count 4 \
    --format rgb8u \
    --shape earth
