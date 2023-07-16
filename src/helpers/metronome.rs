use std::{
    thread::sleep,
    time::{Duration, Instant},
};

/// In charge of sleeping to keep the game timed correctly, regardless of how long processing each
/// tick takes.
pub struct Metronome {
    /// The approximate time the last sleep() call exited. Assuming the rest of the code runs fast
    /// enough to not trigger the min_sleep_time threshold, this is set incremented each tick by
    /// target_tick rather than being set to the actual measured time. This prevents drift.
    prev_tick_start: Instant,
    /// The preferred amount of total time each tick should take. sleep() checks how much time was
    /// used by processing and sleeps for the remainder of the tick.
    target_tick: f64,
    /// Sleep will always sleep for at least this much. If the game isn't performing well, it may
    /// make sense to slow the game down rather than use up the entire time budget. This is because
    /// clients should be able to mamke a roundtrip each tick.
    min_sleep: f64,
}

impl Default for Metronome {
    fn default() -> Self {
        Self {
            prev_tick_start: Instant::now(),
            target_tick: 0.0,
            min_sleep: 0.0,
        }
    }
}

impl Metronome {
    /// - target_tick: the time (in seconds) for each entire tick.
    /// - min_sleep: the minimum time (in seconds) each call to sleep() will sleep for. This is
    /// useful because giving clients enough time to do a roundtrip each tick may be more valuable
    /// than max perf.
    pub fn set_params(&mut self, target_tick: f64, min_sleep: f64) {
        assert!(target_tick >= 0.0);
        assert!(min_sleep >= 0.0);
        self.target_tick = target_tick;
        self.min_sleep = min_sleep;
    }

    /// Sleeps for the remainder of the tick. That is, sleeps for however long is required so that
    /// the time at return is target_tick greater than the time at the previous return. If the
    /// required sleep time is less than min_sleep then there is no drift. If the rest of the game
    /// has taken too long and the required sleep time would be less than min_sleep (or negative),
    /// it sleeps for min_sleep and drifts (doesn't try to make up the delay later).
    pub fn sleep(&mut self) {
        let elapsed = self.prev_tick_start.elapsed().as_secs_f64();
        let sleep_time = self.target_tick - elapsed;
        if sleep_time >= self.min_sleep {
            sleep(Duration::from_secs_f64(sleep_time));
            // doing it this way instead of taking current time prevents drift
            self.prev_tick_start += Duration::from_secs_f64(self.target_tick);
        } else {
            trace!(
                "tick took {:?} which is {:?} too long",
                Duration::from_secs_f64(elapsed),
                Duration::from_secs_f64(self.min_sleep - sleep_time)
            );
            if self.min_sleep > 0.0 {
                sleep(Duration::from_secs_f64(self.min_sleep))
            }
            self.prev_tick_start = Instant::now();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DURATION_EPSILON: f64 = 0.01;
    const SHORT_TIME: f64 = 0.2;

    fn assert_duration_eq(duration: Duration, expected: f64) {
        let error = (duration.as_secs_f64() - expected).abs();
        if error > DURATION_EPSILON {
            panic!("{:?} ≉ {:?}", duration, Duration::from_secs_f64(expected));
        }
    }

    #[test]
    fn sleeps_for_correct_time() {
        let mut m = Metronome::default();
        m.set_params(SHORT_TIME, 0.0);
        let start = Instant::now();
        m.sleep();
        assert_duration_eq(start.elapsed(), SHORT_TIME);
    }

    #[test]
    fn repeatedly_sleeps_for_correct_time() {
        let mut m = Metronome::default();
        m.set_params(SHORT_TIME, 0.0);
        let start = Instant::now();
        m.sleep();
        m.sleep();
        m.sleep();
        assert_duration_eq(start.elapsed(), SHORT_TIME * 3.0);
    }

    #[test]
    fn only_sleeps_for_remainder_of_time_budget() {
        let mut m = Metronome::default();
        m.set_params(SHORT_TIME, 0.0);
        sleep(Duration::from_secs_f64(SHORT_TIME * 0.6));
        let start = Instant::now();
        m.sleep();
        assert_duration_eq(start.elapsed(), SHORT_TIME * 0.4);
    }

    #[test]
    fn doesnt_sleep_when_over_budget() {
        let mut m = Metronome::default();
        m.set_params(SHORT_TIME, 0.0);
        m.sleep();
        sleep(Duration::from_secs_f64(SHORT_TIME * 1.5));
        let start = Instant::now();
        m.sleep();
        assert_duration_eq(start.elapsed(), 0.0);
    }

    #[test]
    fn accepts_drift_when_over_budget() {
        let mut m = Metronome::default();
        m.set_params(SHORT_TIME, 0.0);
        sleep(Duration::from_secs_f64(SHORT_TIME * 1.5));
        m.sleep();
        let start = Instant::now();
        m.sleep();
        assert_duration_eq(start.elapsed(), SHORT_TIME);
    }

    #[test]
    fn respects_min_sleep() {
        let mut m = Metronome::default();
        m.set_params(SHORT_TIME, SHORT_TIME * 0.7);
        sleep(Duration::from_secs_f64(SHORT_TIME * 0.6));
        let start = Instant::now();
        m.sleep();
        assert_duration_eq(start.elapsed(), SHORT_TIME * 0.7);
    }

    #[test]
    fn accepts_drift_when_min_sleep_hit() {
        let mut m = Metronome::default();
        m.set_params(SHORT_TIME, SHORT_TIME * 0.7);
        sleep(Duration::from_secs_f64(SHORT_TIME * 0.6));
        m.sleep();
        let start = Instant::now();
        m.sleep();
        assert_duration_eq(start.elapsed(), SHORT_TIME);
    }
}
