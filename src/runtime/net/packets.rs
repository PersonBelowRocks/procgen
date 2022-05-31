use serde::de::DeserializeOwned;

pub(crate) trait Packet: dc::Downcast {}

dc::impl_downcast!(Packet);

#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct GenerateChunk;
