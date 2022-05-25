use crate::{chunk::Chunk, generation::GenerationArgs};

use super::RequestIdent;

pub struct FinishedGenerateChunkEvent {
    chunk: Chunk,
    request: RequestIdent,
}

impl FinishedGenerateChunkEvent {
    pub fn new(chunk: Chunk, request: RequestIdent) -> Self {
        Self { chunk, request }
    }
}

pub struct RequestGenerateChunkEvent {
    name: String,
    request: RequestIdent,
    args: GenerationArgs,
}

impl RequestGenerateChunkEvent {
    pub fn new(name: String, request: RequestIdent, args: GenerationArgs) -> Self {
        Self {
            name,
            request,
            args,
        }
    }
}
