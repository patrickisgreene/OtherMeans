use clap::Parser;
use std::fs::File;
use std::path::PathBuf;

use image::{GrayImage, Luma};

use crate::utils::Progress;
use crate::{ProcessError, TextureFormat, cli::Cli};

/// Pixels at or above this value are treated as a line/wall that the flood
/// fill cannot cross — matches the bilevel line rasters `render-shapefile`
/// produces (white lines on a black background).
const WALL_THRESHOLD: u8 = 128;

#[derive(Parser, Clone)]
pub struct FloodFill {
    #[arg(short, long)]
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    #[arg(long, short, default_value_t = TextureFormat::default())]
    pub format: TextureFormat,
    /// A seed point to flood-fill from, given in degrees as
    /// `--start-point <LAT> <LON>`. Repeat the flag once per disconnected
    /// region (e.g. one per ocean/sea/lake) — a single seed only reaches
    /// whatever is connected to it through open (non-line) pixels.
    #[arg(long = "start-point", num_args = 2, allow_hyphen_values = true)]
    pub start_points: Vec<f64>,
}

pub fn flood_fill(global: Cli, args: FloodFill) -> Result<(), ProcessError> {
    if args.output.exists() && !global.overwrite {
        println!(
            "Output file `{}` already exists!",
            args.output.to_string_lossy()
        );
        return Ok(());
    }

    let _progress = Progress::new(global.display_format, "flood-fill", None);

    let mut reader = image::ImageReader::open(&args.input)?.with_guessed_format()?;
    if global.no_memory_limit {
        reader.no_limits();
    }
    let source = reader.decode()?.into_luma8();
    let (width, height) = source.dimensions();

    let mut filled = vec![false; (width as usize) * (height as usize)];

    for pair in args.start_points.chunks_exact(2) {
        let (lat, lon) = (pair[0], pair[1]);
        if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
            return Err(ProcessError::InvalidInput(format!(
                "start point ({lat}, {lon}) is out of range (lat must be -90..=90, lon -180..=180)"
            )));
        }

        let x = (((lon + 180.0) / 360.0) * f64::from(width))
            .floor()
            .clamp(0.0, f64::from(width) - 1.0) as u32;
        let y = (((90.0 - lat) / 180.0) * f64::from(height))
            .floor()
            .clamp(0.0, f64::from(height) - 1.0) as u32;

        if source.get_pixel(x, y).0[0] >= WALL_THRESHOLD {
            return Err(ProcessError::InvalidInput(format!(
                "start point ({lat}, {lon}) lands on a line pixel, not an open region"
            )));
        }

        flood_from(&source, &mut filled, width, height, x, y);
    }

    let mut out = GrayImage::new(width, height);
    for (i, is_filled) in filled.iter().enumerate() {
        if *is_filled {
            out.put_pixel((i as u32) % width, (i as u32) / width, Luma([255]));
        }
    }

    let mut file = File::create(&args.output)?;
    image::DynamicImage::ImageLuma8(out).write_to(&mut file, args.format.into())?;
    Ok(())
}

/// Iterative 8-connected flood fill with corner-cutting disallowed: a
/// diagonal step is blocked when both of the orthogonal cells between the
/// source and destination are walls. Without this, a single-pixel-wide
/// diagonal line (exactly what `render-shapefile`'s line rasterizer draws)
/// has no orthogonally-touching wall pixels along it, so plain 4- or
/// 8-connected fill leaks straight through the gap between them.
fn flood_from(source: &GrayImage, filled: &mut [bool], width: u32, height: u32, start_x: u32, start_y: u32) {
    let is_wall = |x: u32, y: u32| source.get_pixel(x, y).0[0] >= WALL_THRESHOLD;
    let index = |x: u32, y: u32| (y as usize) * (width as usize) + (x as usize);

    if filled[index(start_x, start_y)] {
        return;
    }

    let mut stack = vec![(start_x, start_y)];
    filled[index(start_x, start_y)] = true;

    while let Some((x, y)) = stack.pop() {
        for (dx, dy) in [
            (-1, -1),
            (0, -1),
            (1, -1),
            (-1, 0),
            (1, 0),
            (-1, 1),
            (0, 1),
            (1, 1),
        ] {
            let (nx, ny) = (x as i64 + dx, y as i64 + dy);
            if nx < 0 || ny < 0 || nx >= i64::from(width) || ny >= i64::from(height) {
                continue;
            }
            let (nx, ny) = (nx as u32, ny as u32);

            if filled[index(nx, ny)] || is_wall(nx, ny) {
                continue;
            }
            if dx != 0 && dy != 0 && is_wall(x, ny) && is_wall(nx, y) {
                continue;
            }

            filled[index(nx, ny)] = true;
            stack.push((nx, ny));
        }
    }
}
