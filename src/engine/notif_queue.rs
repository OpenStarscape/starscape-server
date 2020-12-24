use super::*;

pub type Notification = Weak<dyn Subscriber>;

/// A queue of pending notifications, there is currently one per state
#[derive(Clone)]
pub struct NotifQueue(Arc<Mutex<Vec<Notification>>>);

impl NotifQueue {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(Vec::new())))
    }

    pub fn extend<T: IntoIterator<Item = Notification>>(&self, iter: T) {
        self.0
            .lock()
            .expect("failed to lock NotifQueue")
            .extend(iter);
    }

    /// Swaps the internal buffer with another. This is useful because two buffers can be swapped
    /// and forth without deallocating either.
    pub fn swap_buffer(&self, other: &mut Vec<Notification>) {
        // This doesn't deallocate the memory
        other.clear();
        std::mem::swap(
            &mut *self.0.lock().expect("failed to lock NotifQueue"),
            other,
        );
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.0.lock().expect("failed to lock NotifQueue").len()
    }

    #[cfg(test)]
    pub fn contains(&self, subscriber: &Arc<dyn Subscriber>) -> bool {
        let subscriber = Arc::downgrade(&subscriber).thin_ptr();
        self.0
            .lock()
            .expect("failed to lock NotifQueue")
            .iter()
            .any(|i| i.thin_ptr() == subscriber)
    }
}

impl PartialEq for NotifQueue {
    fn eq(&self, other: &Self) -> bool {
        self.0.thin_ptr() == other.0.thin_ptr()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockSubscriber;

    impl Subscriber for MockSubscriber {
        fn notify(&self, _: &State, _: &dyn OutboundMessageHandler) -> Result<(), Box<dyn Error>> {
            Ok(())
        }
    }

    fn notif() -> Notification {
        Arc::downgrade(&Arc::new(MockSubscriber)) as Notification
    }

    #[test]
    fn can_extend() {
        let notif_queue = NotifQueue::new();
        notif_queue.extend(vec![notif(), notif(), notif()]);
        assert_eq!(notif_queue.len(), 3);
    }

    #[test]
    fn can_swap_buffers() {
        let notif_queue = NotifQueue::new();
        let mut buf = vec![];
        notif_queue.extend(vec![notif(), notif(), notif()]);
        notif_queue.swap_buffer(&mut buf);
        assert_eq!(buf.len(), 3);
    }

    #[test]
    fn clears_on_buffer_swap() {
        let notif_queue = NotifQueue::new();
        let mut buf = vec![notif(), notif(), notif()];
        notif_queue.swap_buffer(&mut buf);
        assert_eq!(notif_queue.len(), 0);
    }

    #[test]
    fn capacity_remains_after_buf_swap() {
        let notif_queue = NotifQueue::new();
        let mut buf = vec![notif(), notif(), notif()];
        notif_queue.swap_buffer(&mut buf);
        notif_queue.swap_buffer(&mut buf);
        assert_eq!(buf.len(), 0);
        assert!(buf.capacity() > 0);
        buf.shrink_to_fit();
        assert_eq!(buf.capacity(), 0);
    }
}
