use std::error::Error;

pub fn configure_logging(level: log::LevelFilter) -> Result<(), Box<dyn Error>>{

    use log4rs::encode::pattern::PatternEncoder;
    use log4rs::config::{Appender, Config, Root};
    use log4rs::append::console::ConsoleAppender;

    let console = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new("[{l}] {T} - {m}{n}")))
        .build();

    let config = Config::builder()
        .appender(Appender::builder().build("console", Box::new(console)))
        .build(Root::builder()
            .appender("console")
            .build(level))?;

    log4rs::init_config(config)?;

    Ok(())
}