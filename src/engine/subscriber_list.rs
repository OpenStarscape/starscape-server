use super::*;

/// Returned by Subscriptionlist::subscribe(), used instead of a raw bool for code readablity
pub struct SubscribeReport {
    pub was_empty: bool,
}

/// Returned by Subscriptionlist::unsubscribe(), used instead of a raw bool for code readablity
pub struct UnsubscribeReport {
    pub is_now_empty: bool,
}

/// A list of subscribers, with methods for adding and removing subscribers. Conceptually this is a
/// set of Weaks.
///
/// You can't hash or compare a weak so we use a map. The  keys are pointers obtained with
/// thin_ptr(). Raw pointers should be sync but aren't (see https://internals.rust-lang.org/t/8818),
/// so we cast them to usize.
///
/// Since most lists will contain 0 or 1 elements and iteration speed is
/// most important, we use a Vec instead of a map. We store the thin pointers instead of getting
/// them each time because getting required upgrading to an Arc
pub struct SubscriberList(pub Vec<(usize, Weak<dyn Subscriber>)>);

impl SubscriberList {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn add(&mut self, subscriber: &Arc<dyn Subscriber>) -> RequestResult<SubscribeReport> {
        let subscriber_ptr = subscriber.thin_ptr() as usize;
        let subscriber = Arc::downgrade(subscriber);
        if self
            .0
            .iter()
            .any(|(ptr, _subscriber)| *ptr == subscriber_ptr)
        {
            Err(InternalError("subscriber subscribed multiple times".into()))
        } else {
            let was_empty = self.0.is_empty();
            self.0.push((subscriber_ptr, subscriber));
            Ok(SubscribeReport { was_empty })
        }
    }

    pub fn remove(
        &mut self,
        subscriber: &Weak<dyn Subscriber>,
    ) -> RequestResult<UnsubscribeReport> {
        let subscriber_ptr = subscriber.thin_ptr() as usize;
        match self
            .0
            .iter()
            .position(|(ptr, _subscriber)| *ptr == subscriber_ptr)
        {
            None => Err(InternalError(
                "unsubscribed subscriber not already subscribed".into(),
            )),
            Some(i) => {
                self.0.swap_remove(i);
                let is_now_empty = self.0.is_empty();
                Ok(UnsubscribeReport { is_now_empty })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (SubscriberList, Vec<Arc<dyn Subscriber>>) {
        (
            SubscriberList::new(),
            (0..3).map(|_| MockSubscriber::new().get()).collect(),
        )
    }

    #[test]
    fn subscribing_same_subscriber_twice_errors() {
        let (mut list, subscribers) = setup();
        list.add(&subscribers[0]).expect("subscribing failed");
        assert!(list.add(&subscribers[0]).is_err());
    }

    #[test]
    fn unsubscribing_when_not_subscribed_errors() {
        let (mut list, subscribers) = setup();
        assert!(list.remove(&Arc::downgrade(&subscribers[0])).is_err());
        list.add(&subscribers[0]).expect("subscribing failed");
        assert!(list.remove(&Arc::downgrade(&subscribers[1])).is_err());
    }

    #[test]
    fn first_subscriber_reports_was_empty() {
        let (mut list, subscribers) = setup();
        let report = list.add(&subscribers[0]).expect("subscribing failed");
        assert_eq!(report.was_empty, true);
    }

    #[test]
    fn subsequent_subscribers_do_not_report_was_empty() {
        let (mut list, subscribers) = setup();
        list.add(&subscribers[0]).expect("subscribing failed");
        let report = list.add(&subscribers[1]).expect("subscribing failed");
        assert_eq!(report.was_empty, false);
        let report = list.add(&subscribers[2]).expect("subscribing failed");
        assert_eq!(report.was_empty, false);
    }

    #[test]
    fn adding_removing_and_adding_new_subscriber_reports_was_empty() {
        let (mut list, subscribers) = setup();
        list.add(&subscribers[0]).expect("subscribing failed");
        list.remove(&Arc::downgrade(&subscribers[0]))
            .expect("unsubscribing failed");
        let report = list.add(&subscribers[1]).expect("subscribing failed");
        assert_eq!(report.was_empty, true);
    }

    #[test]
    fn removing_only_subscriber_reports_emtpy() {
        let (mut list, subscribers) = setup();
        list.add(&subscribers[0]).expect("subscribing failed");
        let report = list
            .remove(&Arc::downgrade(&subscribers[0]))
            .expect("unsubscribing failed");
        assert_eq!(report.is_now_empty, true);
    }

    #[test]
    fn removing_one_of_two_subscribers_does_not_report_empty() {
        let (mut list, subscribers) = setup();
        list.add(&subscribers[0]).expect("subscribing failed");
        list.add(&subscribers[1]).expect("subscribing failed");
        let report = list
            .remove(&Arc::downgrade(&subscribers[0]))
            .expect("unsubscribing failed");
        assert_eq!(report.is_now_empty, false);
    }
}
