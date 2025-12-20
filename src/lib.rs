use std::time::Duration;

pub mod lib {
    pub mod fork;
    pub mod messages;
    pub mod thinker;
    pub mod transceiver;
    pub mod utils;
}

pub const TICK_INTERVAL: Duration = Duration::from_millis(50);
pub const NETWORK_BUFFER_SIZE: usize = 1024;

pub const MIN_EATING_TIME: Duration = Duration::from_secs(1);
pub const MAX_EATING_TIME: Duration = Duration::from_secs(3);

pub const MIN_THINKING_TIME: Duration = Duration::from_secs(1);
pub const MAX_THINKING_TIME: Duration = Duration::from_secs(3);
