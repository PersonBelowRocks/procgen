mod volumes;
pub use volumes::*;

pub mod packets;

extern crate downcast_rs as dc;
extern crate hashbrown as hb;
extern crate nalgebra as na;
extern crate thiserror as te;
extern crate volume as vol;

use serde::{Deserialize, Serialize};
use std::net::SocketAddrV4;

macro_rules! impl_display_debug {
    ($t:ty) => {
        impl std::fmt::Display for $t {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl std::fmt::Debug for $t {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

macro_rules! impl_from_u32_id {
    ($name:literal, $t:ty) => {
        impl From<u32> for $t {
            fn from(n: u32) -> Self {
                Self(n)
            }
        }

        impl From<$t> for u32 {
            fn from(id: $t) -> Self {
                id.0
            }
        }

        impl std::fmt::Display for $t {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                <Self as std::fmt::Debug>::fmt(self, f)
            }
        }

        impl std::fmt::Debug for $t {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}.{:x}", $name, self.0)
            }
        }
    };
}

type IVec3 = na::Vector3<i64>;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RequestId(pub u32);

impl_from_u32_id!("request", RequestId);

#[derive(Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConnectionId(pub SocketAddrV4);

impl_display_debug!(ConnectionId);

#[derive(Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GeneratorId(pub u32);

impl_from_u32_id!("generator", GeneratorId);

#[derive(Default, Debug, PartialEq, Eq, Copy, Clone, Serialize, Deserialize)]
pub struct BlockId(pub u32);

impl BlockId {
    pub const fn new(id: u32) -> Self {
        Self(id)
    }
}

impl From<u32> for BlockId {
    fn from(val: u32) -> Self {
        Self(val)
    }
}

impl From<BlockId> for u32 {
    fn from(val: BlockId) -> Self {
        val.0
    }
}

#[derive(te::Error, Debug)]
pub enum ParameterError {
    #[error("No parameter with name {0}")]
    DoesntExist(String),
    #[error("Error parsing parameter with name {0}, raw data: {1}, details: {2}")]
    ParseError(String, String, String),
}

#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Parameters {
    pub generator_name: String, // TODO: cache hashmap or something in here
}

impl Parameters {
    pub fn generator_name(&self) -> &str {
        &self.generator_name
    }
}
