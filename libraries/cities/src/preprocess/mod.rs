mod cli;

pub use cli::Cli;

use crate::descriptor::{CitiesDatabase, CityDescriptor, Industries};
use shapefile::Shape;
use shapefile::dbase::{FieldValue, Record};
use smol_str::SmolStr;
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

fn field_string(record: &Record, name: &str) -> SmolStr {
    match record.get(name) {
        Some(FieldValue::Character(Some(value))) => SmolStr::new(value),
        _ => SmolStr::default(),
    }
}

fn field_number(record: &Record, name: &str) -> f64 {
    match record.get(name) {
        Some(FieldValue::Numeric(Some(value))) => *value,
        _ => 0.0,
    }
}

pub fn run_preprocessor() -> Result<(), Error> {
    let args = Cli::default();

    let mut reader = shapefile::Reader::from_path(&args.populated_places)?;

    let mut cities = Vec::new();
    for result in reader.iter_shapes_and_records() {
        let (shape, record) = result?;

        let (lon, lat) = match shape {
            Shape::Point(point) => (point.x, point.y),
            _ => continue,
        };

        cities.push(CityDescriptor {
            id: field_number(&record, "NE_ID") as u32,
            lat,
            lon,
            name: field_string(&record, "NAME"),
            country: field_string(&record, "ADM0NAME"),
            scale_rank: field_number(&record, "SCALERANK") as u8,
            label_rank: field_number(&record, "LABELRANK") as u8,
            population: field_number(&record, "POP_MAX") as u32,
            industries: Industries::empty(),
        });
    }

    let database = CitiesDatabase(cities);
    let encoded = ron::ser::to_string_pretty(&database, ron::ser::PrettyConfig::default())?;
    std::fs::write(&args.out_file, encoded)?;

    Ok(())
}
