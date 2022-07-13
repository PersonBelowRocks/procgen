use common::{ConnectionId, GeneratorId, RequestId};

#[derive(Copy, Clone, Debug, Hash, PartialEq)]
pub struct RequestIdent {
    pub request_id: RequestId,
    pub client_id: ConnectionId,
}

impl RequestIdent {
    pub fn new(request_id: RequestId, client_id: ConnectionId) -> Self {
        Self {
            request_id,
            client_id,
        }
    }

    pub fn generation_ident(self, generator_id: GeneratorId) -> GenerationIdent {
        GenerationIdent::new(self, generator_id)
    }
}

impl From<RequestIdent> for RequestId {
    fn from(i: RequestIdent) -> Self {
        i.request_id
    }
}

impl From<GenerationIdent> for RequestId {
    fn from(i: GenerationIdent) -> Self {
        i.request_ident.into()
    }
}

#[derive(Copy, Clone, Debug, Hash, PartialEq)]
pub struct GenerationIdent {
    pub request_ident: RequestIdent,
    pub generator_id: GeneratorId,
}

impl GenerationIdent {
    pub fn new(request_ident: RequestIdent, generator_id: GeneratorId) -> Self {
        Self {
            request_ident,
            generator_id,
        }
    }
}

impl From<GenerationIdent> for GeneratorId {
    fn from(i: GenerationIdent) -> Self {
        i.generator_id
    }
}

impl From<RequestIdent> for ConnectionId {
    fn from(i: RequestIdent) -> Self {
        i.client_id
    }
}

impl From<GenerationIdent> for ConnectionId {
    fn from(i: GenerationIdent) -> Self {
        i.request_ident.into()
    }
}

impl From<GenerationIdent> for RequestIdent {
    fn from(i: GenerationIdent) -> Self {
        i.request_ident
    }
}
