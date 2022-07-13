use std::marker::PhantomData;

use crate::{BlockId, IVec2};

// TODO: rename this to something with parameters (instead of args) for consistency
#[derive(Debug, Copy, Clone)]
pub struct GenerationArgs {
    pub pos: IVec2,
}

#[allow(dead_code)]
#[derive(Copy, Clone)]
pub struct FactoryParameters<'a> {
    pub max_height: i32,
    pub min_height: i32,
    pub default: BlockId,

    // this is here because in the future we're gonna want more complex and lengthy parameters, in which case we don't want to copy them.
    // it's always a hassle to convert code with plenty of copying into non-copy code after it's written, so we'll write it like this from the start.
    pub _future_noncopy_params: PhantomData<&'a [u8]>,
    // TODO: this should have a seed field too for RNG
}
