use std::collections::HashMap;
use std::error::Error;
use crate::config::UpdateExpirationValue;
use crate::UpdateExpirationMode;

/// A generic update for sending data as binary content
pub struct Update {
    /// Message content
    content: Vec<u8>,
    /// Content digest
    digest: String,
}

impl Update {
    /// Creates a new update with specified content
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

/// A decorator for handling operations around updates
pub struct UpdateDecorator {
    /// Active updates
    active_updates: HashMap<String, (Update, UpdateExpirationValue)>,
    /// Removed/expired updates
    removed_updates: Vec<String>,
    /// Strategy for expiring updates
    expiration_mode: UpdateExpirationMode,
    /// Number of digests of expired updates that are kept
    max_expired_size: usize,
    /// Margin for cleanup of expired updates
    max_expired_margin: f64,
}
impl UpdateDecorator {
    pub fn new(expiration_mode: UpdateExpirationMode) -> Self {
        Self{
            active_updates: HashMap::new(),
            removed_updates: Vec::new(),
            expiration_mode,
            max_expired_size: 10000,
            max_expired_margin: 0.5
        }
    }
    pub fn active_count(&self) -> usize {
        self.active_updates.len()
    }

    pub fn active_headers(&self) -> Vec<String> {
        self.active_updates.iter().map(|(header, _)| header.to_owned()).collect()
    }

    pub fn is_new(&self, digest: &String) -> bool {
        !self.active_updates.contains_key(digest) && !self.removed_updates.contains(&digest)
    }

    pub fn is_expired(&self, digest: &String) -> bool {
        self.removed_updates.contains(&digest)
    }

    pub fn is_active(&self, digest: &String) -> bool {
        self.active_updates.contains_key(digest)
    }

    pub fn get_update(&self, digest: &str) -> Option<&Update> {
        self.active_updates.get(digest).map_or(None, |(update, _)| Some(update))
    }

    pub fn insert_update(&mut self, update: Update) -> Result<(), Box<dyn Error>> {
        if let None = self.active_updates.insert(update.digest().to_owned(), (update, UpdateExpirationValue::new(self.expiration_mode.clone()))) {
            Ok(())
        }
        else {
            Err("Update already existed")?
        }
    }

    pub fn clear(&mut self) {
        self.active_updates.clear();
        self.removed_updates.clear();
    }

    pub fn active_headers_for_push(&mut self) -> Vec<String> {
        let mut headers = Vec::new();
        self.active_updates.iter_mut()
            .for_each(|(digest, (_, expiration))| {
                expiration.increase_push_count();
                headers.push(digest.clone());
            });
        headers
    }

    pub fn clear_expired(&mut self) {
        match self.expiration_mode {
            UpdateExpirationMode::None => (),
            UpdateExpirationMode::MostRecent(size, margin) => {
                let max_size = size + (size as f64 * margin) as usize;
                if self.active_updates.len() > max_size {
                    let removal_count = self.active_updates.len() - max_size;

                    let mut removal_keys: Vec<(String, std::time::Instant)> = Vec::new();
                    for(digest, (_, expiration_value)) in &self.active_updates {
                        match expiration_value {
                            UpdateExpirationValue::MostRecent(created) => removal_keys.push((digest.to_owned(), created.clone())),
                            _ => (),
                        }
                    }
                    // sort from oldest to more recent
                    removal_keys.sort_by_key(|(_, created)| *created);
                    for i in 0..removal_count {
                        self.active_updates.remove(&removal_keys[i].0);
                        self.removed_updates.push(removal_keys[i].0.clone());
                    }
                }
            },
            UpdateExpirationMode::PushCount(_) | UpdateExpirationMode::DurationMillis(_) => {
                let expired_keys: Vec<String> = self.active_updates.iter()
                    .filter(|(_, (_, expiration_value))| expiration_value.has_expired())
                    .map(|(digest, (_, _))| digest.to_owned())
                    .collect();
                for key in expired_keys {
                    self.active_updates.remove(&key);
                    self.removed_updates.push(key.clone());
                }
            }
        }

        let margin_size = (self.max_expired_size as f64 * self.max_expired_margin) as usize;
        let max_expired = self.max_expired_size + margin_size;
        if self.removed_updates.len() > max_expired && margin_size > 0 {
            self.removed_updates.drain(0..margin_size);
        }
    }
}