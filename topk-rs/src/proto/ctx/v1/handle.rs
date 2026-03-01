#[derive(Clone, Debug)]
pub struct Handle(String);

impl From<Handle> for String {
    fn from(handle: Handle) -> Self {
        handle.0
    }
}

impl From<String> for Handle {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for Handle {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}
