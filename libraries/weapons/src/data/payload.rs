use serde::{Deserialize, Serialize};

use super::{Domain, Explosive};

#[derive(Debug, Default, PartialEq, Clone, Hash, Eq, Serialize, Deserialize)]
pub enum Payload {
    #[default]
    None,
    Explosive {
        weight: u32,
        explosive: Vec<Explosive>,
    },
    Capacity {
        weight: u32,
        domains: Vec<Domain>,
    },
}
