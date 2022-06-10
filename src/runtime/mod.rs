//! This module contains code related to the generator server's runtime. This includes the ECS world(s), ECS schedule(s), and async runtime for netcode.

// mod components;
// mod events;
mod net;
// mod resources;
mod server;
// mod systems;

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
