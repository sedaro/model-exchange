use log::{SetLoggerError, LevelFilter, Record, Level, Metadata};
use colored::Colorize;

struct SimpleLogger;
impl log::Log for SimpleLogger {
  fn enabled(&self, metadata: &Metadata) -> bool {
    metadata.level() <= Level::Info
  }
  fn log(&self, record: &Record) {
    if self.enabled(record.metadata()) {
      match record.level() {
        Level::Info => println!("{}", record.args()),
        level => {
          let level = match level {
            Level::Warn => Level::Warn.to_string().yellow(),
            Level::Error => Level::Error.to_string().red(),
            _ => record.level().to_string().normal(),
          };
          println!("{}: {}", level, record.args())
        },
      }
    }
  }
  fn flush(&self) {}
}

static LOGGER: SimpleLogger = SimpleLogger;

pub fn init_logger() -> Result<(), SetLoggerError> {
  log::set_logger(&LOGGER).map(|()| log::set_max_level(LevelFilter::Info))
}