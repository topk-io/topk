use serde::{de::DeserializeOwned, Serialize};

#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ValidationErrorBag<T: Serialize>(Vec<T>);

impl<T: Serialize + DeserializeOwned> ValidationErrorBag<T> {
    pub fn new(errors: Vec<T>) -> Self {
        Self(errors)
    }

    pub fn from(errors: impl IntoIterator<Item = T>) -> Self {
        Self(errors.into_iter().collect())
    }

    pub fn empty() -> Self {
        Self(vec![])
    }

    pub fn push(&mut self, error: T) {
        self.0.push(error);
    }

    pub fn extend(&mut self, errors: impl IntoIterator<Item = T>) {
        self.0.extend(errors.into_iter());
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}
