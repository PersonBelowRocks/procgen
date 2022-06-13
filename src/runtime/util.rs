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
pub(crate) struct RequestIdent {
    pub(crate) request_id: RequestId,
    pub(crate) client_id: ClientId,
}

impl RequestIdent {
    pub(crate) fn new(request_id: RequestId, client_id: ClientId) -> Self {
        Self {
            request_id,
            client_id,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub(crate) struct RequestId(u32);

impl_display_debug!(RequestId);
impl_from_u32_id!(RequestId);

impl From<RequestIdent> for RequestId {
    fn from(i: RequestIdent) -> Self {
        i.request_id
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub(crate) struct ClientId(u32);

impl_display_debug!(ClientId);
impl_from_u32_id!(ClientId);

impl From<RequestIdent> for ClientId {
    fn from(i: RequestIdent) -> Self {
        i.client_id
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub(crate) struct GeneratorId(u32);

impl_display_debug!(GeneratorId);
impl_from_u32_id!(GeneratorId);
