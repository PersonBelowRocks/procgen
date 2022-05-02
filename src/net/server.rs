use super::protocol::ProtocolVersion;

pub struct Server {
    version: ProtocolVersion,
    compression_threshold: Option<usize>,
}

impl Server {
    pub fn new() -> Self {
        Self {
            version: Default::default(),
            compression_threshold: None,
        }
    }

    pub fn with_version(mut self, version: ProtocolVersion) -> Self {
        self.version = version;
        self
    }

    pub fn with_compression_threshold(mut self, threshold: usize) -> Self {
        self.compression_threshold = Some(threshold);
        self
    }
}
