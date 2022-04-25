#[derive(Debug, PartialEq, Copy, Clone)]
pub struct BlockId(u32);

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
