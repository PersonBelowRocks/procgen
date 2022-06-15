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

impl From<GenerationIdent> for RequestIdent {
    fn from(i: GenerationIdent) -> Self {
        i.request_ident
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct RequestId(u32);

impl_display_debug!(RequestId);
impl_from_u32_id!(RequestId);

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

#[derive(Copy, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ConnectionId(pub SocketAddrV4);

impl_display_debug!(ConnectionId);

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

#[derive(Copy, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct GeneratorId(u32);

impl_display_debug!(GeneratorId);
impl_from_u32_id!(GeneratorId);

impl From<GenerationIdent> for GeneratorId {
    fn from(i: GenerationIdent) -> Self {
        i.generator_id
    }
}
