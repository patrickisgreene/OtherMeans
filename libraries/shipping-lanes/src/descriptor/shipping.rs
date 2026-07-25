use serde::{Deserialize, Serialize};

/// A single road line (one part of one shapefile feature), as a sequence of
/// (longitude, latitude) points in degrees, in the original vertex order.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct ShippingLaneDescriptor {
    pub points: Vec<(f64, f64)>,
}
