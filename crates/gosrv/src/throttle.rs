use std::{
    hint::spin_loop,
    thread::sleep,
    time::{Duration, Instant},
};

pub struct Throttle {
    interval: Duration,
    last: Instant,
    spin_threshold: Option<Duration>,
}

impl Throttle {
    /// Create a new throttle.
    ///
    /// `interval`: desired time between steps.
    /// `spin_threshold`: time before the deadline where we switch from sleep to spin.
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            last: Instant::now() - interval,
            spin_threshold: None,
        }
    }

    /// Sleep or spin until the interval has elapsed.
    pub fn throttle(&mut self) {
        let target = self.last + self.interval;
        let now = Instant::now();

        if now < target {
            let sleep_time = target - now;

            if let Some(spin_threshold) = self.spin_threshold {
                if sleep_time > spin_threshold {
                    sleep(sleep_time - spin_threshold);
                }

                // spin-wait until exact deadline
                while Instant::now() < target {
                    spin_loop();
                }
            } else {
                sleep(sleep_time);
            }
        }

        self.last = Instant::now();
    }
}
