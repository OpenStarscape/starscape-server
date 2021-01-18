/// Used to easily log and otherwise ignore an error
pub trait OrLog {
    fn or_log_warn(&self, context: &str);
    fn or_log_error(&self, context: &str);
}

impl<T, U> OrLog for Result<T, U>
where
    U: std::fmt::Display,
{
    fn or_log_warn(&self, context: &str) {
        if let Err(e) = self {
            warn!("{}: {}", context, e);
        }
    }

    fn or_log_error(&self, context: &str) {
        if let Err(e) = self {
            error!("{}: {}", context, e);
        }
    }
}
