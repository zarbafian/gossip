use std::error::Error;
use gossip::{UpdateHandler, Update};

pub struct TextMessageListener {id: String}
impl TextMessageListener {
    pub fn new(id: String) -> Self {TextMessageListener{id}}
}
impl UpdateHandler for TextMessageListener {
    fn on_update(&self, update: Update) {
        log::info!("--------------------------");
        log::info!("[{}] Received message:", self.id);
        log::info!("{}", String::from_utf8(update.content().to_vec()).unwrap());
        log::info!("--------------------------");
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
