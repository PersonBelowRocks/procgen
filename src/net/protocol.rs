use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[non_exhaustive]
pub enum ProtocolVersion {
    V1,
}

impl Default for ProtocolVersion {
    fn default() -> Self {
        Self::V1
    }
}

pub trait Packet<'a> {
    const PACKET_ID: u16;

    fn to_bytes(&self, version: ProtocolVersion) -> Vec<u8>;
    fn from_bytes(bytes: &'a [u8], version: ProtocolVersion) -> Self;

    #[inline]
    fn id(_version: ProtocolVersion) -> u16 {
        Self::PACKET_ID
    }
}

pub trait BincodePacket<'a>
where
    Self: Serialize + Deserialize<'a>,
{
    const PACKET_ID: u16;

    #[inline]
    fn to_bytes(&self, _version: ProtocolVersion) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    #[inline]
    fn from_bytes(bytes: &'a [u8], _version: ProtocolVersion) -> Self {
        bincode::deserialize(bytes).unwrap()
    }
}

#[derive(Serialize, Deserialize)]
pub struct RegisterGenerator<'a> {
    name: &'a str,
}

impl<'a> BincodePacket<'a> for RegisterGenerator<'a> {
    const PACKET_ID: u16 = 0x01;
}
