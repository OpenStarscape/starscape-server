//! This should probably be replaced by some async tokio bullshit now that we're using that anyway

use super::*;
use ::mio::{event::Evented, Events, Poll, PollOpt, Ready, Registration, SetReadiness, Token};
use std::{
    sync::atomic::{AtomicBool, Ordering},
    thread::{spawn, JoinHandle},
};

const TOKEN: Token = Token(0);

fn poll_loop<F>(
    poll: Poll,
    _quit_registration: Registration,
    should_quit: Arc<AtomicBool>,
    mut process_event: F,
) where
    F: FnMut() -> Result<(), Box<dyn Error>>,
{
    let mut events = Events::with_capacity(256);
    loop {
        poll.poll(&mut events, None).unwrap();
        if should_quit.load(Ordering::Relaxed) {
            break;
        }
        for event in events.iter() {
            match event.token() {
                TOKEN => {
                    if let Err(e) = process_event() {
                        error!("processing Mio event: {}", e);
                    }
                }
                token => {
                    error!("invalid mio token {:?}", token);
                }
            }
        }
    }
}

struct MioPollThread {
    /// Once set to true next iteration of the poll loop will exit the thread
    should_quit: Arc<AtomicBool>,
    /// Allows us to interrupt the poll loop
    set_readiness_to_quit: SetReadiness,
    /// Option only so we can .take() it in the destructor
    join_handle: Option<JoinHandle<()>>,
}

impl Drop for MioPollThread {
    fn drop(&mut self) {
        self.should_quit.store(true, Ordering::Relaxed);
        self.set_readiness_to_quit
            .set_readiness(Ready::readable())
            .expect("failed to set rediness on Mio poll in order to exit loop and join thread");
        if let Err(e) = self.join_handle.take().unwrap().join() {
            error!("Mio thread panicked at some point: {:?}", e);
        }
    }
}

pub fn new_mio_poll_thread<F, T>(
    mut e: T,
    mut process_event: F,
) -> Result<Box<dyn Drop + Send>, Box<dyn Error>>
where
    T: Evented + Send + 'static,
    F: FnMut(&mut T) -> Result<(), Box<dyn Error>> + Send + 'static,
{
    let poll = Poll::new()?;
    poll.register(&e, TOKEN, Ready::readable(), PollOpt::edge())?;
    let (quit_registration, set_readiness_to_quit) = Registration::new2();
    poll.register(
        &quit_registration,
        TOKEN,
        Ready::readable(),
        PollOpt::edge(),
    )?;
    let should_quit = Arc::new(AtomicBool::new(false));
    let join_handle = {
        let should_quit = should_quit.clone();
        let process_event = move || process_event(&mut e);
        spawn(|| poll_loop(poll, quit_registration, should_quit, process_event))
    };
    Ok(Box::new(MioPollThread {
        should_quit,
        set_readiness_to_quit,
        join_handle: Some(join_handle),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use std::{thread, time::Duration};

    const SHORT_TIME: Duration = Duration::from_millis(20);

    #[test]
    fn can_start_and_stop_quickly() {
        run_with_timeout(|| {
            let (registration, _set_readiness) = Registration::new2();
            let _thread = new_mio_poll_thread(registration, |_| Ok(()));
        });
    }

    #[test]
    fn can_start_and_stop_with_pause() {
        run_with_timeout(|| {
            let (registration, _set_readiness) = Registration::new2();
            let _thread = new_mio_poll_thread(registration, |_| Ok(()));
            thread::sleep(SHORT_TIME);
        });
    }

    #[test]
    fn does_not_process_event_when_there_is_none() {
        let count = Arc::new(Mutex::new(0));
        let final_count = count.clone();
        run_with_timeout(|| {
            let (registration, _set_readiness) = Registration::new2();
            let _thread = new_mio_poll_thread(registration, move |_| {
                *count.lock().expect("failed to lock count") += 1;
                Ok(())
            });
            thread::sleep(SHORT_TIME);
        });
        assert_eq!(*final_count.lock().expect("failed to lock count"), 0);
    }

    #[test]
    fn can_process_event() {
        let count = Arc::new(Mutex::new(0));
        let final_count = count.clone();
        run_with_timeout(|| {
            let (registration, set_readiness) = Registration::new2();
            let _thread = new_mio_poll_thread(registration, move |_| {
                *count.lock().expect("failed to lock count") += 1;
                Ok(())
            });
            set_readiness
                .set_readiness(Ready::readable())
                .expect("set_readiness() failed");
            thread::sleep(SHORT_TIME);
        });
        assert_eq!(*final_count.lock().expect("failed to lock count"), 1);
    }

    #[test]
    fn can_process_several_events() {
        let count = Arc::new(Mutex::new(0));
        let final_count = count.clone();
        run_with_timeout(|| {
            let (registration, set_readiness) = Registration::new2();
            let _thread = new_mio_poll_thread(registration, move |_| {
                *count.lock().expect("failed to lock count") += 1;
                Ok(())
            });
            for _ in 0..3 {
                set_readiness
                    .set_readiness(Ready::readable())
                    .expect("set_readiness() failed");
                thread::sleep(SHORT_TIME);
            }
        });
        assert_eq!(*final_count.lock().expect("failed to lock count"), 3);
    }
}
