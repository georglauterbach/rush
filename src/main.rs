use log::{
    Level,
    Metadata,
    Record,
};
use rush::prelude::*;

struct SimpleLogger;

static LOGGER: SimpleLogger = SimpleLogger;

impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool { metadata.level() <= Level::Info }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            println!("{} - {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    log::set_logger(&LOGGER).expect("Setting the logger failed but was expected to succeed");
    log::set_max_level(log::LevelFilter::Trace);

    let file = rush::fs::File::new("lol");
    file.overwrite("WTF")?;
    file.move_to("haha")?;

    rush::fs::File::new("abc").create_on_fs()?;
    rush::fs::Directory::new("jo/roll").create_on_fs_recursive()?;

    Ok(())
}
