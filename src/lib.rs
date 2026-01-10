use core::f64;
use std::{sync::LazyLock, time::Duration};

use rand::Rng;

pub mod lib {
    pub mod config;
    pub mod fork;
    pub mod messages;
    pub mod thinker;
    pub mod transceiver;
    pub mod utils;
    pub mod visualizer;
}

pub const NETWORK_BUFFER_SIZE: usize = 1024;
pub const DROP_MESSAGE_PERCENTAGE: f64 = 0.95;

pub const TICK_INTERVAL: Duration = Duration::from_millis(250);
pub const KEEP_ALIVE_TIMEOUT: Duration = Duration::from_secs(2);
pub const KEEP_TOKEN_ALIVE_TIMEOUT: Duration = Duration::from_secs(3);

pub const MIN_EATING_TIME: Duration = Duration::from_secs(3);
pub const MAX_EATING_TIME: Duration = Duration::from_secs(7);

pub const MIN_THINKING_TIME: Duration = Duration::from_secs(5);
pub const MAX_THINKING_TIME: Duration = Duration::from_secs(10);

pub const MIN_CRASH_DURATION: Duration = Duration::from_secs(5);
pub const MAX_CRASH_DURATION: Duration = Duration::from_secs(10);
pub const PERMANET_CRASH_PERCENTAGE: f64 = 0.05;

const NODE_SURVIVAL_TIMESPAN: Duration = Duration::from_secs(30);
const NODE_SURVIVAL_PERCANTAGE: f64 = 0.1;
pub static CRASH_PROBABILITY_PER_TICK: LazyLock<f64> = LazyLock::new(|| {
    let tick_amount = NODE_SURVIVAL_TIMESPAN.div_duration_f64(TICK_INTERVAL);
    let survival_percentage = NODE_SURVIVAL_PERCANTAGE.powf(1.0 / tick_amount);
    1.0 - survival_percentage
});

pub fn init_logger() {
    env_logger::builder()
        .format_target(false)
        .filter_level(log::LevelFilter::Debug)
        .init();
}

pub enum CrashStatus {
    Continue,
    Crash,
    PermanentCrash,
}

pub fn should_crash() -> CrashStatus {
    match rand::rng().random_bool(*CRASH_PROBABILITY_PER_TICK) {
        true => match rand::rng().random_bool(PERMANET_CRASH_PERCENTAGE) {
            true => CrashStatus::PermanentCrash,
            false => CrashStatus::Crash,
        },
        false => CrashStatus::Continue,
    }
}
