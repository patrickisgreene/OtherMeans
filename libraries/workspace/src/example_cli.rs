use bevy::log::{DEFAULT_FILTER, Level};
use clap::Parser;

#[derive(Parser)]
#[command(version, about)]
pub struct ExampleCli {
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Log filter to append to the default log filter.
    #[arg(short = 'l', long, default_value = "")]
    pub log_filter: String,

    ///Remove Bevys default log filter.
    #[arg(short = 'r', long)]
    pub remove_default_log_filter: bool,
}

impl ExampleCli {
    pub fn get() -> ExampleCli {
        ExampleCli::parse()
    }

    pub fn log_level(&self) -> Level {
        match self.verbose {
            0 => Level::INFO, // default
            1 => Level::DEBUG,
            2.. => Level::TRACE, // 2 or more → Trace
        }
    }

    pub fn log_filter(&self) -> String {
        if self.remove_default_log_filter {
            self.log_filter.clone()
        } else {
            format!("{},{}", DEFAULT_FILTER, self.log_filter)
        }
    }
}
