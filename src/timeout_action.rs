use std::time::{Duration, Instant};

pub type Action = fn() -> ();

pub struct TimeoutAction {
    start: Instant,
    timeout: Duration,
    action: Action,
}

impl TimeoutAction {
    pub fn new(timeout: Duration, action: Action) -> TimeoutAction {
        TimeoutAction {
            start: Instant::now(),
            timeout: timeout,
            action: action,
        }
    }
}

impl Drop for TimeoutAction {
    fn drop(&mut self) {
        if self.start.elapsed() > self.timeout {
            (self.action)();
        }
    }
}
