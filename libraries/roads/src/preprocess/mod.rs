mod cli;

pub use cli::Cli;

use crate::descriptor::{RoadDescriptor, RoadNetwork};
use shapefile::record::traits::HasXY;
use shapefile::{Shape, ShapeReader};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("failed to read shapefile")]
    Shapefile(#[from] shapefile::Error),
    #[error("failed to serialize RON output")]
    Ron(#[from] ron::Error),
    #[error("failed to write output file")]
    Io(#[from] std::io::Error),
}

fn push_parts<P: HasXY>(roads: &mut Vec<RoadDescriptor>, parts: &[Vec<P>]) {
    for part in parts {
        if part.len() < 2 {
            continue;
        }

        roads.push(RoadDescriptor {
            points: part.iter().map(|point| (point.x(), point.y())).collect(),
        });
    }
}

pub fn run_preprocessor() -> Result<(), Error> {
    let args = Cli::default();

    let mut reader = ShapeReader::from_path(&args.roads)?;

    let mut roads = Vec::new();
    for result in reader.iter_shapes() {
        let shape = result?;

        match shape {
            Shape::Polyline(polyline) => push_parts(&mut roads, polyline.parts()),
            Shape::PolylineM(polyline) => push_parts(&mut roads, polyline.parts()),
            Shape::PolylineZ(polyline) => push_parts(&mut roads, polyline.parts()),
            _ => continue,
        }
    }

    let network = RoadNetwork(roads);
    let encoded = ron::ser::to_string_pretty(&network, ron::ser::PrettyConfig::default())?;
    std::fs::write(&args.out_file, encoded)?;

    Ok(())
}
