use tonic::Status;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum InternalErrorCode {
    RequiredLsnGreaterThanManifestMaxLsn = 1000,
}

impl InternalErrorCode {
    /// Get the numeric code associated with the enum variant.
    pub fn code(&self) -> u32 {
        *self as u32
    }

    pub fn parse_status(e: &Status) -> anyhow::Result<InternalErrorCode> {
        let ddb_error_code = e
            .metadata()
            .get("x-topk-error-code")
            .ok_or(anyhow::anyhow!("x-topk-error-code not found"))?;
        let ddb_error_code = ddb_error_code.to_str()?;
        let ddb_error_code: u32 = ddb_error_code.parse()?;
        let code = InternalErrorCode::try_from(ddb_error_code)?;

        Ok(code)
    }
}

impl From<InternalErrorCode> for Status {
    fn from(error: InternalErrorCode) -> Self {
        let mut status = match error {
            InternalErrorCode::RequiredLsnGreaterThanManifestMaxLsn => {
                Status::failed_precondition("Lsn is greater than manifest max lsn")
            }
        };

        status
            .metadata_mut()
            .insert("x-topk-error-code", error.code().into());

        status
    }
}

impl TryFrom<u32> for InternalErrorCode {
    type Error = anyhow::Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1000 => Ok(InternalErrorCode::RequiredLsnGreaterThanManifestMaxLsn),
            _ => Err(anyhow::anyhow!("unknown internal error code: {}", value)),
        }
    }
}
