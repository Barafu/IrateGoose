use anyhow::Result;
use log4rs::append::Append;
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::pattern::PatternEncoder;
use log4rs::filter::threshold::ThresholdFilter;
use std::sync::{Arc, Mutex};

/// A custom log4rs appender that stores log lines in a shared buffer.
#[derive(Debug)]
pub struct MemoryAppender {
    buffer: Arc<Mutex<Vec<String>>>,
}

impl MemoryAppender {
    pub fn new(buffer: Arc<Mutex<Vec<String>>>) -> Self {
        Self { buffer }
    }
}

impl Append for MemoryAppender {
    fn append(&self, record: &log::Record) -> Result<()> {
        // Only store logs from our own crate
        if record
            .module_path()
            .map(|p| p.starts_with("irate_goose"))
            .unwrap_or(false)
        {
            let formatted = format!("{}", record.args());
            if let Ok(mut guard) = self.buffer.lock() {
                guard.push(formatted);
            }
        }
        Ok(())
    }

    fn flush(&self) {}
}

/// Initializes log4rs with a console appender and a memory appender.
/// The memory appender writes into the provided buffer.
pub fn init_logging(buffer: Arc<Mutex<Vec<String>>>) -> Result<()> {
    // Determine console log level from environment variable, default to Warn
    let console_level = std::env::var("RUST_LOG")
        .ok()
        .and_then(|s| s.parse::<log::LevelFilter>().ok())
        .unwrap_or(log::LevelFilter::Warn);

    // Console appender with default pattern
    let console = log4rs::append::console::ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{d} {l} {t} - {m}{n}")))
        .build();

    let console_appender = Appender::builder()
        .filter(Box::new(ThresholdFilter::new(console_level)))
        .build("console", Box::new(console));

    // Memory appender using the shared buffer
    let memory = MemoryAppender::new(buffer);
    let memory_appender = Appender::builder().build("memory", Box::new(memory));

    let config = Config::builder()
        .appender(console_appender)
        .appender(memory_appender)
        .build(
            Root::builder()
                .appender("console")
                .appender("memory")
                .build(log::LevelFilter::Info),
        )?;

    log4rs::init_config(config)?;
    Ok(())
}
