use std::{collections::HashMap, str::FromStr};

use tonic::{metadata::AsciiMetadataValue, service::Interceptor, Status};

use crate::Error;

#[derive(Clone)]
pub struct AppendHeadersInterceptor {
    /// Headers
    headers: HashMap<&'static str, AsciiMetadataValue>,
}

impl AppendHeadersInterceptor {
    pub fn new(
        headers: impl IntoIterator<Item = (&'static str, impl AsRef<str>)>,
    ) -> Result<Self, Error> {
        Ok(Self {
            headers: headers
                .into_iter()
                .map(|(key, value)| {
                    let value = AsciiMetadataValue::from_str(value.as_ref()).map_err(|e| {
                        Error::Input(anyhow::anyhow!("invalid header value: {e:?}"))
                    })?;
                    Ok((key, value))
                })
                .collect::<Result<_, Error>>()?,
        })
    }
}

impl Interceptor for AppendHeadersInterceptor {
    fn call(&mut self, mut request: tonic::Request<()>) -> Result<tonic::Request<()>, Status> {
        for (key, value) in self.headers.iter() {
            request.metadata_mut().insert(*key, value.clone());
        }
        Ok(request)
    }
}
