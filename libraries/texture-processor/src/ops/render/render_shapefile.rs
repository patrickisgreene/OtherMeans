use clap::Parser;
use std::fs::File;
use std::path::PathBuf;

use image::GrayImage;
use shapefile::record::traits::HasXY;
use shapefile::{PolygonRing, Shape, ShapeReader};

use crate::utils::Progress;
use crate::{ProcessError, TextureFormat, cli::Cli};

#[derive(Parser, Clone)]
pub struct RenderShapefile {
    #[arg(long)]
    pub width: u32,
    #[arg(long)]
    pub height: u32,
    #[arg(short, long)]
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    #[arg(long, short, default_value_t = TextureFormat::default())]
    pub format: TextureFormat,
    /// Geographic bounding box as `<X_MIN> <Y_MIN> <X_MAX> <Y_MAX>` in
    /// degrees.  Defaults to the full equirectangular global extent
    /// (-180 -90 180 90).
    #[arg(long, num_args = 4, allow_hyphen_values = true,
          default_values_t = [-180.0, -90.0, 180.0, 90.0])]
    pub bbox: Vec<f64>,
}

/// Maps geographic (x, y) coordinates onto pixel space using the
/// supplied bounding box, flipping y since raster row 0 is north.
struct Projection {
    x_min: f64,
    y_max: f64,
    x_scale: f64,
    y_scale: f64,
}

impl Projection {
    fn new(
        x_min: f64,
        y_min: f64,
        x_max: f64,
        y_max: f64,
        width: u32,
        height: u32,
    ) -> Result<Self, ProcessError> {
        if x_max == x_min || y_max == y_min {
            return Err(ProcessError::InvalidInput(
                "bounding box is degenerate (zero width or height)".to_string(),
            ));
        }
        Ok(Self {
            x_min,
            y_max,
            x_scale: f64::from(width) / (x_max - x_min),
            y_scale: f64::from(height) / (y_max - y_min),
        })
    }

    fn project<P: HasXY>(&self, point: &P) -> (f64, f64) {
        (
            (point.x() - self.x_min) * self.x_scale,
            (self.y_max - point.y()) * self.y_scale,
        )
    }
}

pub fn render_shapefile(global: Cli, args: RenderShapefile) -> Result<(), ProcessError> {
    if args.output.exists() && !global.overwrite {
        println!(
            "Output file `{}` already exists!",
            args.output.to_string_lossy()
        );
        return Ok(());
    }

    let _progress = Progress::new(global.display_format, "render-shapefile", None);

    let reader = ShapeReader::from_path(&args.input)?;
    let bbox = &args.bbox;
    let projection = Projection::new(bbox[0], bbox[1], bbox[2], bbox[3], args.width, args.height)?;
    let shapes = reader.read()?;

    let mut img = GrayImage::new(args.width, args.height);

    for shape in &shapes {
        match shape {
            Shape::Polygon(polygon) => fill_rings(&mut img, polygon.rings(), &projection),
            Shape::PolygonM(polygon) => fill_rings(&mut img, polygon.rings(), &projection),
            Shape::PolygonZ(polygon) => fill_rings(&mut img, polygon.rings(), &projection),
            Shape::Polyline(polyline) => draw_parts(&mut img, polyline.parts(), &projection),
            Shape::PolylineM(polyline) => draw_parts(&mut img, polyline.parts(), &projection),
            Shape::PolylineZ(polyline) => draw_parts(&mut img, polyline.parts(), &projection),
            Shape::Point(point) => plot_point(&mut img, point, &projection),
            Shape::PointM(point) => plot_point(&mut img, point, &projection),
            Shape::PointZ(point) => plot_point(&mut img, point, &projection),
            Shape::Multipoint(multipoint) => {
                for point in multipoint.points() {
                    plot_point(&mut img, point, &projection);
                }
            }
            Shape::MultipointM(multipoint) => {
                for point in multipoint.points() {
                    plot_point(&mut img, point, &projection);
                }
            }
            Shape::MultipointZ(multipoint) => {
                for point in multipoint.points() {
                    plot_point(&mut img, point, &projection);
                }
            }
            Shape::NullShape | Shape::Multipatch(_) => {}
        }
    }

    let mut file = File::create(&args.output)?;
    image::DynamicImage::ImageLuma8(img).write_to(&mut file, args.format.into())?;
    Ok(())
}

fn set_pixel(img: &mut GrayImage, x: i64, y: i64) {
    if x >= 0 && y >= 0 && (x as u32) < img.width() && (y as u32) < img.height() {
        img.put_pixel(x as u32, y as u32, image::Luma([255]));
    }
}

fn plot_point<P: HasXY>(img: &mut GrayImage, point: &P, projection: &Projection) {
    let (x, y) = projection.project(point);
    set_pixel(img, x.round() as i64, y.round() as i64);
}

fn draw_parts<P: HasXY>(img: &mut GrayImage, parts: &[Vec<P>], projection: &Projection) {
    for part in parts {
        for pair in part.windows(2) {
            let a = projection.project(&pair[0]);
            let b = projection.project(&pair[1]);
            draw_line(img, a, b);
        }
    }
}

/// Simple DDA line rasterizer: steps in whichever axis has the larger span,
/// so both shallow and steep segments get a fully-connected pixel run.
fn draw_line(img: &mut GrayImage, (x0, y0): (f64, f64), (x1, y1): (f64, f64)) {
    let steps = (x1 - x0).abs().max((y1 - y0).abs()).ceil() as i64;
    if steps == 0 {
        set_pixel(img, x0.round() as i64, y0.round() as i64);
        return;
    }
    for step in 0..=steps {
        let t = step as f64 / steps as f64;
        let x = x0 + (x1 - x0) * t;
        let y = y0 + (y1 - y0) * t;
        set_pixel(img, x.round() as i64, y.round() as i64);
    }
}

/// Even-odd scanline fill across all rings of a polygon at once — holes
/// fall out naturally without needing to special-case inner vs outer rings.
fn fill_rings<P: HasXY>(img: &mut GrayImage, rings: &[PolygonRing<P>], projection: &Projection) {
    let edges: Vec<[(f64, f64); 2]> = rings
        .iter()
        .flat_map(|ring| {
            let points = ring.points();
            points
                .windows(2)
                .map(|pair| [projection.project(&pair[0]), projection.project(&pair[1])])
        })
        .collect();

    if edges.is_empty() {
        return;
    }

    for y_pixel in 0..img.height() {
        let y = f64::from(y_pixel) + 0.5;
        let mut intersections: Vec<f64> = edges
            .iter()
            .filter_map(|&[(x0, y0), (x1, y1)]| {
                if (y0 <= y && y1 > y) || (y1 <= y && y0 > y) {
                    Some(x0 + (y - y0) / (y1 - y0) * (x1 - x0))
                } else {
                    None
                }
            })
            .collect();
        intersections.sort_by(|a, b| a.partial_cmp(b).unwrap());

        for pair in intersections.as_chunks::<2>().0 {
            let start = pair[0].round().max(0.0) as u32;
            let end = pair[1].round().min(f64::from(img.width())) as u32;
            for x in start..end.min(img.width()) {
                img.put_pixel(x, y_pixel, image::Luma([255]));
            }
        }
    }
}
