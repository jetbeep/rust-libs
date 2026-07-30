#[cfg(feature = "platform-zephyr")]
use alloc::string::ToString;
use alloc::vec::Vec;
#[cfg(feature = "platform-zephyr")]
use core::cell::UnsafeCell;

use futures::channel::oneshot;

use crate::error::Error;
use crate::proto::DeviceSettings;

#[cfg(feature = "platform-zephyr")]
const CACHE_DIR: &str = "/lfs1/sys";
#[cfg(feature = "platform-zephyr")]
const CACHE_PATH: &str = "/lfs1/sys/device_settings.jkv";
#[cfg(feature = "platform-zephyr")]
const CACHE_TEMP_PATH: &str = "/lfs1/sys/device_settings.jkv.tmp";
#[cfg(feature = "platform-zephyr")]
const EEXIST: i32 = 17;
#[cfg(feature = "platform-zephyr")]
const EINVAL: i32 = 22;
#[cfg(feature = "platform-zephyr")]
const EIO: i32 = 5;

type Waiter = oneshot::Sender<Result<DeviceSettings, Error>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ExpectedCrc {
    Unknown,
    Missing,
    Value(u32),
}

enum Action {
    None,
    Load(u64),
    Ready(DeviceSettings, Vec<Waiter>),
    Refresh(u64),
}

struct State {
    fs_loaded: bool,
    cached: Option<DeviceSettings>,
    expected: ExpectedCrc,
    valid: bool,
    refresh_in_flight: bool,
    retry_blocked: bool,
    epoch: u64,
    generation: u32,
    waiters: Vec<Waiter>,
}

impl State {
    const fn new() -> Self {
        Self {
            fs_loaded: false,
            cached: None,
            expected: ExpectedCrc::Unknown,
            valid: false,
            refresh_in_flight: false,
            retry_blocked: false,
            epoch: 0,
            generation: 0,
            waiters: Vec::new(),
        }
    }

    fn next_action(&mut self) -> Action {
        if !self.fs_loaded || self.refresh_in_flight || self.retry_blocked {
            return Action::None;
        }

        match self.expected {
            ExpectedCrc::Unknown => Action::None,
            ExpectedCrc::Value(expected)
                if self
                    .cached
                    .as_ref()
                    .is_some_and(|settings| settings.crc32_hash == expected) =>
            {
                self.valid = true;
                let settings = self.cached.as_ref().unwrap().clone();
                Action::Ready(settings, core::mem::take(&mut self.waiters))
            }
            ExpectedCrc::Missing | ExpectedCrc::Value(_) => {
                self.valid = false;
                self.refresh_in_flight = true;
                Action::Refresh(self.epoch)
            }
        }
    }

    fn begin(&mut self, expected: ExpectedCrc) -> Action {
        self.epoch = self.epoch.wrapping_add(1);
        self.generation = crate::generation::current();
        self.expected = expected;
        self.fs_loaded = false;
        self.valid = false;
        self.refresh_in_flight = false;
        self.retry_blocked = false;
        Action::Load(self.epoch)
    }

    fn response_matches(&self, refresh_epoch: u64, settings: &DeviceSettings) -> bool {
        if self.epoch != refresh_epoch {
            return false;
        }

        match self.expected {
            ExpectedCrc::Missing => true,
            ExpectedCrc::Value(expected) => settings.crc32_hash == expected,
            ExpectedCrc::Unknown => false,
        }
    }
}

#[cfg(feature = "platform-zephyr")]
struct StateCell(UnsafeCell<State>);

// All access is serialized on the Rust workqueue.
#[cfg(feature = "platform-zephyr")]
unsafe impl Sync for StateCell {}

#[cfg(feature = "platform-zephyr")]
static STATE: StateCell = StateCell(UnsafeCell::new(State::new()));

#[cfg(feature = "platform-zephyr")]
fn state() -> &'static mut State {
    unsafe { &mut *STATE.0.get() }
}

#[cfg(feature = "platform-zephyr")]
pub(crate) fn start() {
    let action = state().begin(ExpectedCrc::Unknown);
    drive(action);
}

#[cfg(feature = "platform-zephyr")]
fn load_cache(load_epoch: u64) {
    crate::executor::run(async move {
        if let Err(error) = crate::fs::mkdir(CACHE_DIR).await {
            if error.code != -EEXIST {
                log::warn!(
                    "device settings cache: failed to create {}: {}",
                    CACHE_DIR,
                    error
                );
            }
        }

        let cached = match crate::jkv_file::read::<DeviceSettings>(CACHE_PATH).await {
            Ok(settings) => {
                log::info!(
                    "device settings cache: loaded CRC 0x{:08x}",
                    settings.crc32_hash
                );
                Some(settings)
            }
            Err(error) => {
                log::info!("device settings cache: no usable cache: {}", error);
                None
            }
        };

        complete_load(load_epoch, cached);
    });
}

#[cfg(feature = "platform-zephyr")]
pub(crate) fn set_expected_crc(expected: Option<u32>) {
    let action = state().begin(expected.map_or(ExpectedCrc::Missing, ExpectedCrc::Value));
    drive(action);
}

#[cfg(feature = "platform-zephyr")]
pub(crate) async fn get() -> Result<DeviceSettings, Error> {
    let receiver = {
        let cache = state();
        let restart = if cache.generation != crate::generation::current() {
            cache.generation = crate::generation::current();
            (!cache.valid).then(|| cache.begin(cache.expected))
        } else {
            None
        };
        if cache.valid {
            if let Some(settings) = cache.cached.as_ref() {
                return Ok(settings.clone());
            }
        }

        let (sender, receiver) = oneshot::channel();
        cache.waiters.push(sender);
        cache.retry_blocked = false;
        drive(restart.unwrap_or_else(|| cache.next_action()));
        receiver
    };

    receiver.await.unwrap_or_else(|_| {
        Err(Error {
            code: -EIO,
            message: "device settings cache waiter was cancelled".to_string(),
        })
    })
}

#[cfg(feature = "platform-zephyr")]
fn drive(action: Action) {
    match action {
        Action::None => {}
        Action::Load(epoch) => load_cache(epoch),
        Action::Ready(settings, waiters) => {
            for waiter in waiters {
                let _ = waiter.send(Ok(settings.clone()));
            }
        }
        Action::Refresh(epoch) => {
            crate::executor::run(async move {
                let result = crate::bus::fetch_device_settings().await;
                match result {
                    Ok(settings) => {
                        if let Err(error) = persist(&settings).await {
                            log::warn!(
                                "device settings cache: failed to persist settings: {}",
                                error
                            );
                        }
                        complete_refresh(epoch, Ok(settings));
                    }
                    Err(error) => complete_refresh(epoch, Err(error)),
                }
            });
        }
    }
}

#[cfg(feature = "platform-zephyr")]
async fn persist(settings: &DeviceSettings) -> Result<(), Error> {
    crate::jkv_file::write(CACHE_TEMP_PATH, settings).await?;
    crate::fs::rename(CACHE_TEMP_PATH, CACHE_PATH).await
}

#[cfg(feature = "platform-zephyr")]
fn complete_load(load_epoch: u64, cached: Option<DeviceSettings>) {
    let cache = state();
    if cache.epoch != load_epoch {
        return;
    }

    cache.cached = cached;
    cache.fs_loaded = true;
    cache.retry_blocked = false;
    drive(cache.next_action());
}

#[cfg(feature = "platform-zephyr")]
fn complete_refresh(refresh_epoch: u64, result: Result<DeviceSettings, Error>) {
    let cache = state();
    if cache.epoch != refresh_epoch {
        return;
    }
    cache.refresh_in_flight = false;

    match result {
        Ok(settings) => {
            let response_matches = cache.response_matches(refresh_epoch, &settings);
            cache.cached = Some(settings);

            if response_matches {
                cache.valid = true;
                cache.retry_blocked = false;
            } else {
                cache.valid = false;
                cache.retry_blocked = true;
                let error = Error {
                    code: -EINVAL,
                    message: "polled device settings CRC does not match init_screen".to_string(),
                };
                notify_error(core::mem::take(&mut cache.waiters), error);
            }
        }
        Err(error) => {
            cache.valid = false;
            cache.retry_blocked = true;
            notify_error(core::mem::take(&mut cache.waiters), error);
        }
    }

    drive(cache.next_action());
}

#[cfg(feature = "platform-zephyr")]
fn notify_error(waiters: Vec<Waiter>, error: Error) {
    for waiter in waiters {
        let _ = waiter.send(Err(error.clone()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(crc32_hash: u32) -> DeviceSettings {
        DeviceSettings {
            crc32_hash,
            ..Default::default()
        }
    }

    #[test]
    fn matching_loaded_cache_is_ready_without_refresh() {
        let mut state = State::new();
        state.fs_loaded = true;
        state.cached = Some(settings(0x1234));
        state.expected = ExpectedCrc::Value(0x1234);

        match state.next_action() {
            Action::Ready(settings, waiters) => {
                assert_eq!(settings.crc32_hash, 0x1234);
                assert!(waiters.is_empty());
            }
            _ => panic!("matching cache was not marked ready"),
        }
        assert!(state.valid);
        assert!(!state.refresh_in_flight);
    }

    #[test]
    fn mismatch_starts_exactly_one_refresh() {
        let mut state = State::new();
        state.fs_loaded = true;
        state.cached = Some(settings(1));
        state.expected = ExpectedCrc::Value(2);
        state.epoch = 7;

        assert!(matches!(state.next_action(), Action::Refresh(7)));
        assert!(state.refresh_in_flight);
        assert!(matches!(state.next_action(), Action::None));
    }

    #[test]
    fn concurrent_waiters_are_released_together_from_ram() {
        let mut state = State::new();
        state.fs_loaded = true;
        state.cached = Some(settings(9));
        state.expected = ExpectedCrc::Value(9);
        let (first, _) = oneshot::channel();
        let (second, _) = oneshot::channel();
        state.waiters.push(first);
        state.waiters.push(second);

        match state.next_action() {
            Action::Ready(_, waiters) => assert_eq!(waiters.len(), 2),
            _ => panic!("matching cache did not release waiters"),
        }
        assert!(state.waiters.is_empty());
    }

    #[test]
    fn absent_crc_forces_refresh_even_when_cache_exists() {
        let mut state = State::new();
        state.fs_loaded = true;
        state.cached = Some(settings(1));
        state.expected = ExpectedCrc::Missing;

        assert!(matches!(state.next_action(), Action::Refresh(0)));
    }

    #[test]
    fn cache_waits_until_filesystem_and_crc_are_known() {
        let mut state = State::new();
        assert!(matches!(state.next_action(), Action::None));

        state.fs_loaded = true;
        assert!(matches!(state.next_action(), Action::None));
    }

    #[test]
    fn failed_refresh_remains_blocked_until_retry_is_requested() {
        let mut state = State::new();
        state.fs_loaded = true;
        state.expected = ExpectedCrc::Value(2);
        state.retry_blocked = true;

        assert!(matches!(state.next_action(), Action::None));
        state.retry_blocked = false;
        assert!(matches!(state.next_action(), Action::Refresh(0)));
    }

    #[test]
    fn newer_missing_crc_init_rejects_older_refresh_response() {
        let mut state = State::new();
        state.expected = ExpectedCrc::Missing;
        state.epoch = 2;

        assert!(!state.response_matches(1, &settings(7)));
        assert!(state.response_matches(2, &settings(7)));
    }

    #[test]
    fn restart_invalidates_an_abandoned_operation() {
        let mut state = State::new();
        state.expected = ExpectedCrc::Value(7);
        state.epoch = 2;
        state.refresh_in_flight = true;

        assert!(matches!(state.begin(ExpectedCrc::Value(7)), Action::Load(3)));
        assert!(!state.fs_loaded);
        assert!(!state.refresh_in_flight);
        assert!(!state.response_matches(2, &settings(7)));
    }
}
