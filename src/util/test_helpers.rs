use std::sync::mpsc::channel;
use std::{sync::mpsc, thread, time::Duration};

const TIMEOUT: Duration = Duration::from_secs(2);

/// Originally from https://github.com/rust-lang/rfcs/issues/2798#issuecomment-552949300
pub fn run_with_timeout<T, F>(f: F) -> T
where
    T: Send + 'static,
    F: FnOnce() -> T,
    F: Send + 'static,
{
    let (done_tx, done_rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        let val = f();
        done_tx.send(()).expect("unable to send completion signal");
        val
    });

    match done_rx.recv_timeout(TIMEOUT) {
        Err(mpsc::RecvTimeoutError::Timeout) => panic!("thread timed out"),
        Err(mpsc::RecvTimeoutError::Disconnected) | Ok(()) => match handle.join() {
            Ok(result) => result,
            Err(e) => {
                if let Some(e) = e.downcast_ref::<&'static str>() {
                    panic!("Got an error: {}", e);
                } else if let Some(e) = e.downcast_ref::<String>() {
					panic!("Got an error: {}", e);
				} else {
                    panic!("Got an unknown error: {:?}", e);
                }
            }
        },
    }
}