use super::*;

struct MockRequestHandlerInner {
    should_return: RequestResult<()>,
    requests: Vec<Request>,
}

struct MockSub(GenericId, String);

impl Subscription for MockSub {
    fn finalize(self: Box<Self>, handler: &dyn RequestHandler) -> RequestResult<()> {
        let handler: &MockRequestHandler = (handler.as_ref() as &dyn Any).downcast_ref().unwrap();
        let mut lock = handler.0.lock().unwrap();
        lock.requests
            .push(Request::unsubscribe(self.0, self.1.to_string()));
        lock.should_return.clone()
    }
}

#[derive(Clone)]
pub struct MockRequestHandler(Arc<Mutex<MockRequestHandlerInner>>);

impl MockRequestHandler {
    /// If should_return is Err, always returns it. else returns default value.
    pub fn new(should_return: RequestResult<()>) -> Self {
        Self(Arc::new(Mutex::new(MockRequestHandlerInner {
            should_return,
            requests: Vec::new(),
        })))
    }

    pub fn assert_requests_eq(&self, expected: Vec<Request>) {
        assert_eq!(self.0.lock().unwrap().requests, expected);
    }
}

impl AsRef<dyn Any> for MockRequestHandler {
    fn as_ref(&self) -> &dyn Any {
        self
    }
}

impl RequestHandler for MockRequestHandler {
    fn fire_action(
        &mut self,
        _: ConnectionKey,
        e: GenericId,
        n: &str,
        v: Value,
    ) -> RequestResult<()> {
        let mut lock = self.0.lock().unwrap();
        lock.requests.push(Request::action(e, n.to_string(), v));
        lock.should_return.clone()
    }

    fn set_property(
        &mut self,
        _: ConnectionKey,
        e: GenericId,
        n: &str,
        v: Value,
    ) -> RequestResult<()> {
        let mut lock = self.0.lock().unwrap();
        lock.requests.push(Request::set(e, n.to_string(), v));
        lock.should_return.clone()
    }

    fn get_property(&self, _: ConnectionKey, e: GenericId, n: &str) -> RequestResult<Value> {
        let mut lock = self.0.lock().unwrap();
        lock.requests.push(Request::get(e, n.to_string()));
        lock.should_return
            .clone()
            .map(|()| Value::Text("MockRequestHandler get response value".to_string()))
    }

    fn subscribe(
        &self,
        _: ConnectionKey,
        e: GenericId,
        n: Option<&str>,
    ) -> RequestResult<Box<dyn Subscription>> {
        let mut lock = self.0.lock().unwrap();
        lock.requests.push(Request::subscribe(
            e,
            n.unwrap_or("<destroyed signal>").to_string(),
        ));
        lock.should_return.clone().map(|()| {
            Box::new(MockSub(e, n.unwrap_or("<destroyed signal>").to_string()))
                as Box<dyn Subscription>
        })
    }
}
