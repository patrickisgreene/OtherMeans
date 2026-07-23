use std::str::FromStr;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct CountryParseError;

#[derive(Component, Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub struct CountryId(pub [char; 3]);

impl FromStr for CountryId {
    type Err = CountryParseError;

    fn from_str(s: &str) -> std::prelude::v1::Result<Self, Self::Err> {
        if s.len() == 3 {
            let chars: Vec<char> = s.chars().take(3).collect();
            Ok(CountryId([chars[0], chars[1], chars[2]]))
        } else {
            Err(CountryParseError)
        }
    }
}
