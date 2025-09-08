use prost::Message;
use tonic::codec::{Codec, DecodeBuf, Decoder};
use tonic::{Code, Status};

#[derive(Clone, Default)]
pub(crate) struct ProstDecoder<T> {
    inner: tonic_prost::ProstDecoder<T>,
}

impl<T> Decoder for ProstDecoder<T>
where
    T: Default + prost::Message,
{
    type Item = T;
    type Error = Status;

    fn decode(&mut self, buf: &mut DecodeBuf<'_>) -> Result<Option<Self::Item>, Self::Error> {
        // By default, `prost` will return an INTERNAL_ERROR if the payload cannot be decoded.
        // This happens, for example, if the proto recursion limit is exceeded (eg. expr to deep).
        // Instead of returning an INTERNAL_ERROR, we want to remap this to INVALID_ARGUMENT instead.
        self.inner
            .decode(buf)
            .map_err(|status| Status::new(Code::InvalidArgument, status.message()))
    }

    fn buffer_settings(&self) -> tonic::codec::BufferSettings {
        self.inner.buffer_settings()
    }
}

#[derive(Clone, Default)]
pub(crate) struct ProstCodec<Req, Resp> {
    #[allow(dead_code)]
    inner: tonic_prost::ProstCodec<Req, Resp>,
}

impl<Req, Resp> Codec for ProstCodec<Req, Resp>
where
    Req: Default + Message + Send + 'static,
    Resp: Default + Message + Send + 'static,
{
    type Encode = Resp;
    type Decode = Req;
    type Encoder = tonic_prost::ProstEncoder<Resp>;
    type Decoder = ProstDecoder<Req>;

    fn encoder(&mut self) -> Self::Encoder {
        tonic_prost::ProstEncoder::<Resp>::default()
    }

    fn decoder(&mut self) -> Self::Decoder {
        ProstDecoder::<Req> {
            inner: tonic_prost::ProstDecoder::default(),
        }
    }
}
