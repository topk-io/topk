use base64::{engine::general_purpose::STANDARD, Engine as _};
use bytes::Bytes;

#[derive(Debug, Clone)]
pub struct Base64Bytes(pub Bytes);

impl serde::Serialize for Base64Bytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&STANDARD.encode(self.0.as_ref()))
    }
}

impl<'de> serde::Deserialize<'de> for Base64Bytes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        STANDARD
            .decode(s)
            .map(Bytes::from)
            .map(Base64Bytes)
            .map_err(serde::de::Error::custom)
    }
}
