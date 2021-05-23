use super::*;
use tokio::runtime::Runtime;

pub fn run_with_tokio<F>(f: F)
where
    F: FnOnce() + Send + 'static,
{
    run_with_timeout(|| {
        let mut rt = Runtime::new().unwrap();
        rt.block_on(async {
            f();
        });
    });
}
