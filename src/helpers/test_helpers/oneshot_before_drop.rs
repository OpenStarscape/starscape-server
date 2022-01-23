/// Must be fired exactly once before being dropped
pub struct OneshotBeforeDrop {
    fired: std::sync::Mutex<bool>,
}

impl OneshotBeforeDrop {
    pub fn new() -> Self {
        Self {
            fired: std::sync::Mutex::new(false),
        }
    }

    pub fn fire(&self) {
        let mut fired = self.fired.lock().unwrap();
        if *fired {
            panic!("OneshotBeforeDrop fired twice");
        }
        *fired = true;
    }
}

impl Drop for OneshotBeforeDrop {
    fn drop(&mut self) {
        if !std::thread::panicking() && !*self.fired.lock().unwrap() {
            panic!("OneshotBeforeDrop never fired");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn works_if_fired_once() {
        let a = OneshotBeforeDrop::new();
        a.fire();
    }

    #[test]
    #[should_panic(expected = "never fired")]
    fn panics_if_not_fired() {
        let _a = OneshotBeforeDrop::new();
    }

    #[test]
    #[should_panic(expected = "fired twice")]
    fn panics_if_fired_multiple_times() {
        let a = OneshotBeforeDrop::new();
        a.fire();
        a.fire();
    }

    #[test]
    #[should_panic(expected = "foo")]
    fn doesnt_freak_out_when_dropped_within_panic() {
        let _a = OneshotBeforeDrop::new();
        panic!("foo")
    }
}
