/// A generic message for sending data as binary content
pub struct Update {
    /// Message content
    content: Vec<u8>,
    /// Content digest
    digest: String,
}

impl Update {
    /// Creates a new message with generic content
    ///
    /// # Arguments
    ///
    /// * `content` - Message content
    /// * `digest` - Content digest
    pub fn new(content: Vec<u8>) -> Self {
        let digest = blake3::hash(&content).to_hex().to_string();
        Update {
            content,
            digest,
        }
    }

    pub fn content(&self) -> &Vec<u8> {
        &self.content
    }

    pub fn digest(&self) -> &String {
        &self.digest
    }
}

/// Trait for receiving updates from the gossip protocol.
///
/// See: [Update]
pub trait UpdateHandler {
    /// Method called every time a new update is available for the application layer
    ///
    /// # Arguments
    ///
    /// * `update` - The update that has been received
    fn on_update(&self, update: Update);
}
