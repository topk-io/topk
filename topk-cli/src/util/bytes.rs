use base64::{engine::general_purpose::STANDARD, Engine as _};
use bytes::Bytes;

#[derive(Debug, Clone)]
pub struct Base64(Bytes);

impl From<Bytes> for Base64 {
    fn from(b: Bytes) -> Self {
        Self(b)
    }
}

impl AsRef<[u8]> for Base64 {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl serde::Serialize for Base64 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&STANDARD.encode(self.0.as_ref()))
    }
}

impl<'de> serde::Deserialize<'de> for Base64 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        STANDARD
            .decode(s)
            .map(Bytes::from)
            .map(Base64::from)
            .map_err(serde::de::Error::custom)
    }
}
