use super::*;

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(1);

/// Try to run the given function, or panic if it takes too long
/// Originally from https://github.com/rust-lang/rfcs/issues/2798#issuecomment-552949300
pub fn run_with_specific_timeout<T, F>(d: Duration, f: F) -> T
where
    T: Send + 'static,
    F: FnOnce() -> T,
    F: Send + 'static,
{
    let (done_tx, done_rx) = channel();
    let handle = thread::spawn(move || {
        let val = f();
        done_tx.send(()).expect("unable to send completion signal");
        val
    });

    match done_rx.recv_timeout(d) {
        Ok(()) => match handle.join() {
            Ok(result) => result,
            Err(e) => panic!(
                "thread panicked but channel was not disconnected: {}",
                attempt_any_to_string(&*e)
            ),
        },
        Err(Disconnected) => match handle.join() {
            Ok(_) => panic!("thread did not panic but channel was disconnected"),
            Err(e) => panic!("thread panicked: {}", attempt_any_to_string(&*e)),
        },
        Err(Timeout) => panic!("thread timed out"),
    }
}

/// Try to run the given function, or panic if it takes longer than DEFAULT_TIMEOUT
pub fn run_with_timeout<T, F>(f: F) -> T
where
    T: Send + 'static,
    F: FnOnce() -> T,
    F: Send + 'static,
{
    run_with_specific_timeout(DEFAULT_TIMEOUT, f)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::{Arc, Mutex},
        thread,
    };

    #[test]
    fn runs_fn() {
        let value_a = Arc::new(Mutex::new(7));
        let value_b = value_a.clone();
        run_with_timeout(move || {
            thread::sleep(Duration::from_millis(20));
            *value_a.lock().expect("failed to lock mutex") = 5;
            thread::sleep(Duration::from_millis(20));
        });
        assert_eq!(*value_b.lock().expect("failed to lock mutex"), 5);
    }

    #[test]
    fn returns_value() {
        let result = run_with_timeout(move || {
            thread::sleep(Duration::from_millis(20));
            12
        });
        assert_eq!(result, 12);
    }

    #[test]
    fn does_not_time_out_if_quick() {
        run_with_specific_timeout(Duration::from_millis(50), move || {
            thread::sleep(Duration::from_millis(10));
        });
    }

    #[test]
    #[should_panic(expected = "timed out")]
    fn times_out() {
        run_with_specific_timeout(Duration::from_millis(50), move || {
            thread::sleep(Duration::from_secs(5));
        });
        unreachable!();
    }

    #[test]
    #[should_panic(expected = "this is fine")]
    fn shows_str_panic_message() {
        run_with_timeout(move || {
            panic!("this is fine");
        });
        unreachable!();
    }
}
