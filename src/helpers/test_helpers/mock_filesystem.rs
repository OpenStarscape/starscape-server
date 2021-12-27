use super::*;

#[derive(Debug)]
struct MockFilesystemInner {
    map: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct MockFilesystem(Arc<Mutex<MockFilesystemInner>>);

impl MockFilesystem {
    pub fn new() -> Self {
        MockFilesystem(Arc::new(Mutex::new(MockFilesystemInner {
            map: HashMap::new(),
        })))
    }

    pub fn add_file(self, path: &str, contents: &str) -> Self {
        self.0
            .lock()
            .unwrap()
            .map
            .insert(path.to_string(), contents.to_string());
        self
    }

    pub fn boxed(&self) -> Filesystem {
        Box::new(self.clone())
    }
}

impl FilesystemTrait for MockFilesystem {
    fn is_file(&self, path: &str) -> bool {
        let fs = self.0.lock().unwrap();
        fs.map.get(path).is_some()
    }

    fn read_to_string(&self, path: &str) -> std::io::Result<String> {
        let fs = self.0.lock().unwrap();
        match fs.map.get(path) {
            Some(contents) => Ok(contents.clone()),
            None => Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "file not found",
            )),
        }
    }
}
