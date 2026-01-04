use std::time::Duration;

pub mod lib {
    pub mod fork;
    pub mod messages;
    pub mod thinker;
    pub mod transceiver;
    pub mod utils;
    pub mod visualizer;
}

pub const NETWORK_BUFFER_SIZE: usize = 1024;

pub const TICK_INTERVAL: Duration = Duration::from_millis(100);
pub const KEEP_ALIVE_TIMEOUT: Duration = Duration::from_secs(1);

pub const MIN_EATING_TIME: Duration = Duration::from_secs(5);
pub const MAX_EATING_TIME: Duration = Duration::from_secs(15);

pub const MIN_THINKING_TIME: Duration = Duration::from_secs(5);
pub const MAX_THINKING_TIME: Duration = Duration::from_secs(15);

pub fn init_logger() {
    env_logger::builder()
        .format_target(false)
        .filter_level(log::LevelFilter::Debug)
        .init();
}
