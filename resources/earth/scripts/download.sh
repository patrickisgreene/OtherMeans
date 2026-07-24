#!/usr/bin/env bash

WORK_DIR=$(git rev-parse --show-toplevel)

TOPOGRAPHY_URL=https://assets.science.nasa.gov/content/dam/science/esd/eo/images/bmng/topography/gebco_08_rev_elev_21600x10800.tif
BATHYOMETRY_URL=https://assets.science.nasa.gov/content/dam/science/esd/eo/images/bmng/bathymetry/gebco_08_rev_bath_21600x10800.tif
APRIL_COLOR_URL=https://assets.science.nasa.gov/content/dam/science/esd/eo/images/bmng/bmng-base/april/world.200404.3x21600x10800_geo.tif
COASTLINE_URL=https://naturalearth.s3.amazonaws.com/10m_physical/ne_10m_coastline.zip
LAKES_URL=https://naturalearth.s3.amazonaws.com/10m_physical/ne_10m_lakes.zip
OCEAN_URL=https://naturalearth.s3.amazonaws.com/10m_physical/ne_10m_ocean.zip
CHLOROPHYLL_URL=https://coastwatch.pfeg.noaa.gov/erddap/files/erdMPIC8day_R2022NRT/AQUA_MODIS.20220226_20220305.L3m.8D.PIC.pic.4km.NRT.nc
PLACES_URL=https://naturalearth.s3.amazonaws.com/10m_cultural/ne_10m_populated_places.zip
POPULATION_URL=https://data.worldpop.org/GIS/Population/Global_2015_2030/R2025A/2020/0_Mosaicked/v1/1km/constrained/global_pop_2020_CN_1km_R2025A_v1.tif
ROADS_URL=https://naturalearth.s3.amazonaws.com/10m_cultural/ne_10m_roads.zip

mkdir -p $WORK_DIR/resources/earth/data

wget -nc -O $WORK_DIR/resources/earth/data/height.tif $TOPOGRAPHY_URL
wget -nc -O $WORK_DIR/resources/earth/data/depth.tif $BATHYOMETRY_URL
wget -nc -O $WORK_DIR/resources/earth/data/color.tif $APRIL_COLOR_URL
wget -nc -O $WORK_DIR/resources/earth/data/coastline.zip $COASTLINE_URL
wget -nc -O $WORK_DIR/resources/earth/data/lakes.zip $LAKES_URL
wget -nc -O $WORK_DIR/resources/earth/data/ocean.zip $OCEAN_URL
wget -nc -O $WORK_DIR/resources/earth/data/chlorophyll.nc $CHLOROPHYLL_URL
wget -nc -O $WORK_DIR/resources/earth/data/places.zip $PLACES_URL
wget -nc -O $WORK_DIR/resources/earth/data/population.tif $POPULATION_URL
wget -nc -O $WORK_DIR/resources/earth/data/roads.zip $ROADS_URL

unzip -n $WORK_DIR/resources/earth/data/coastline.zip \
      -d $WORK_DIR/resources/earth/data/coastline

unzip -n $WORK_DIR/resources/earth/data/lakes.zip \
      -d $WORK_DIR/resources/earth/data/lakes

unzip -n $WORK_DIR/resources/earth/data/ocean.zip \
      -d $WORK_DIR/resources/earth/data/ocean

unzip -n $WORK_DIR/resources/earth/data/places.zip \
      -d $WORK_DIR/resources/earth/data/places

unzip -n $WORK_DIR/resources/earth/data/roads.zip \
      -d $WORK_DIR/resources/earth/data/roads
