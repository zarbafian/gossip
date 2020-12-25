use std::error::Error;
use gossip::{UpdateHandler, Update, GossipService};
use std::collections::HashMap;
use std::sync::{Mutex, Arc};

// noop handler
pub struct NoopUpdateHandler;
impl UpdateHandler for NoopUpdateHandler {
    fn on_update(&self, update: Update) {}
}
// text message handler
pub struct TextMessageHandler {id: String}
impl TextMessageHandler {
    pub fn new(id: String) -> Self { TextMessageHandler {id}}
}
impl UpdateHandler for TextMessageHandler {
    fn on_update(&self, update: Update) {
        log::info!("--------------------------");
        log::info!("[{}] Received message:", self.id);
        log::info!("{}", String::from_utf8(update.content().to_vec()).unwrap());
        log::info!("--------------------------");
    }
}
// handler storing messages in a shared map
pub struct MapUpdatingHandler {
    id: String,
    map: Arc<Mutex<HashMap<String, Vec<String>>>>,
}
impl MapUpdatingHandler {
    pub fn new(id: String, map: Arc<Mutex<HashMap<String, Vec<String>>>>) -> Self {
        MapUpdatingHandler {
            id,
            map,
        }
    }
}
impl UpdateHandler for MapUpdatingHandler {
    fn on_update(&self, update: Update) {
        self.map.lock().unwrap().entry(self.id.clone()).or_insert(Vec::new()).push(update.digest().clone());
    }
}

#[allow(dead_code)]
pub fn configure_logging(level: log::LevelFilter) -> Result<(), Box<dyn Error>>{

    use log4rs::encode::pattern::PatternEncoder;
    use log4rs::config::{Appender, Config, Root};
    use log4rs::append::console::ConsoleAppender;

    let console = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{h({l})} {T} - {m}{n}")))
        .build();

    let config = Config::builder()
        .appender(Appender::builder().build("console", Box::new(console)))
        .build(Root::builder()
            .appender("console")
            .build(level))?;

    log4rs::init_config(config)?;

    Ok(())
}
