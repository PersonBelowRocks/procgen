//! This module contains code related to the generator server's runtime.

mod net;
mod server;

#[derive(Copy, Clone, Debug, Hash, PartialEq)]
pub struct RequestIdent {
    request_id: u32,
    caller_id: u32,
}

impl RequestIdent {
    pub fn new(request_id: u32, caller_id: u32) -> Self {
        Self {
            request_id,
            caller_id,
        }
    }
}
