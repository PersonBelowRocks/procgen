mod chunk;
pub use chunk::*;

pub mod generation;
pub mod packets;

extern crate downcast_rs as dc;
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
    ($t:ty) => {
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
    };
}

type IVec2 = na::Vector2<i32>;
type IVec3 = na::Vector3<i32>;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RequestId(pub u32);

impl_display_debug!(RequestId);
impl_from_u32_id!(RequestId);

#[derive(Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConnectionId(pub SocketAddrV4);

impl_display_debug!(ConnectionId);

#[derive(Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GeneratorId(pub u32);

impl_display_debug!(GeneratorId);
impl_from_u32_id!(GeneratorId);

#[derive(Default, Debug, PartialEq, Copy, Clone, Serialize, Deserialize)]
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
