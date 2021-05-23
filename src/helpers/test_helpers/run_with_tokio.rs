use super::*;
use tokio::runtime::Runtime;

pub fn run_with_tokio<F>(f: F)
where
    F: FnOnce() + Send + 'static,
{
    // This doesn't pass the "times_out" test for some reason (nor work in practice)
    /*
    let mut rt = Runtime::new().unwrap();
    if let Err(_) =
        rt.block_on(async { tokio::time::timeout(Duration::from_secs(1), async { f() }).await })
    {
        panic!("timed out");
    }
    */

    run_with_timeout(|| {
        let mut rt = Runtime::new().unwrap();
        rt.block_on(async {
            f();
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    #[should_panic(expected = "timed out")]
    fn times_out() {
        run_with_tokio(|| {
            thread::sleep(Duration::from_secs(5));
        });
    }
}
