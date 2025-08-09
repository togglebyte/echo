use std::collections::HashMap;

pub struct Context {
    data: HashMap<String, String>,
}
impl Context {
    pub(crate) fn new() -> Self {
        Self { data: HashMap::new() }
    }

    pub fn set(&mut self, key: String, value: String) {
        self.data.insert(key, value);
    }

    pub fn load(&self, key: impl AsRef<str>) -> Option<String> {
        self.data.get(key.as_ref()).cloned()
    }
}

