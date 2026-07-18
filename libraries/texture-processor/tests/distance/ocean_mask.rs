use std::path::Path;

use gdal::spatial_ref::SpatialRef;
use shapefile::{Point, Polygon, PolygonRing, Polyline, ShapeWriter};
use texture_processor::distance::ocean_mask::{rasterize_coastline_seeds, rasterize_water_mask};

use crate::temp_dir;

const WIDTH: usize = 10;
const HEIGHT: usize = 10;
// Origin at (lon 0, lat 10), 1 degree/pixel, north-up: pixel (col, row)
// covers lon [col, col+1), lat [10-row-1, 10-row).
const GEO_TRANSFORM: [f64; 6] = [0.0, 1.0, 0.0, 10.0, 0.0, -1.0];

fn wgs84() -> SpatialRef {
    SpatialRef::from_proj4("+proj=lonlat +ellps=WGS84 +datum=WGS84").unwrap()
}

fn rect_ring(x0: f64, y0: f64, x1: f64, y1: f64) -> PolygonRing<Point> {
    PolygonRing::Outer(vec![
        Point::new(x0, y0),
        Point::new(x0, y1),
        Point::new(x1, y1),
        Point::new(x1, y0),
        Point::new(x0, y0),
    ])
}

fn write_polygon(path: &Path, rings: Vec<PolygonRing<Point>>) {
    let writer = ShapeWriter::from_path(path).unwrap();
    writer.write_shapes(&[Polygon::with_rings(rings)]).unwrap();
}

#[test]
fn rasterize_water_mask_burns_ocean_and_lakes() {
    let dir = temp_dir("rasterize_water_mask_burns_ocean_and_lakes");
    let ocean_path = dir.join("ocean.shp");
    let lakes_path = dir.join("lakes.shp");

    // Ocean: left half of the raster (lon 0..5, full latitude range).
    write_polygon(&ocean_path, vec![rect_ring(0.0, 0.0, 5.0, 10.0)]);
    // Lake: a small inland square at lon 7..8, lat 4..6.
    write_polygon(&lakes_path, vec![rect_ring(7.0, 4.0, 8.0, 6.0)]);

    let spatial_ref = wgs84();
    let mask = rasterize_water_mask(
        &ocean_path,
        &lakes_path,
        WIDTH,
        HEIGHT,
        GEO_TRANSFORM,
        &spatial_ref,
    )
    .unwrap();

    for row in 0..HEIGHT {
        for col in 0..WIDTH {
            let expected = if col < 5 || (col == 7 && (row == 4 || row == 5)) {
                255
            } else {
                0
            };
            assert_eq!(mask[row * WIDTH + col], expected, "col={col} row={row}");
        }
    }

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn rasterize_coastline_seeds_burns_all_touched_line_pixels() {
    let dir = temp_dir("rasterize_coastline_seeds_burns_all_touched_line_pixels");
    let coastline_path = dir.join("coastline.shp");

    // A vertical line through the center of column 5 (x=5.5), full height.
    let writer = ShapeWriter::from_path(&coastline_path).unwrap();
    let line = Polyline::new(vec![Point::new(5.5, 0.0), Point::new(5.5, 10.0)]);
    writer.write_shapes(&[line]).unwrap();

    let spatial_ref = wgs84();
    let seeds =
        rasterize_coastline_seeds(&coastline_path, WIDTH, HEIGHT, GEO_TRANSFORM, &spatial_ref)
            .unwrap();

    for row in 0..HEIGHT {
        for col in 0..WIDTH {
            let expected = if col == 5 { 255 } else { 0 };
            assert_eq!(seeds[row * WIDTH + col], expected, "col={col} row={row}");
        }
    }

    std::fs::remove_dir_all(&dir).unwrap();
}
