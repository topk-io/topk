use std::{hint::black_box, time::Instant};

use bytes::Bytes;
use flatbuffers::{FlatBufferBuilder, ForwardsUOffset, WIPOffset};
use prost::Message;
use rand::{thread_rng, Rng};
use topk_rs::{
    doc,
    flatbuffers::v1::document::{
        Bool as BoolFB, BoolArgs, Bytes as BytesFB, BytesArgs, Document as DocumentFB,
        DocumentArgs, F32Args, F64Args, I32Args, I64Args, String as StringFB, StringArgs, U32Args,
        U64Args, Value as ValueFB, ValueArgs, ValueUnion, F32 as F32FB, F64 as F64FB, I32 as I32FB,
        I64 as I64FB, U32 as U32FB, U64 as U64FB,
    },
    proto::v1::data::{value as value_pb, Document, Value},
};

fn main() {
    let docs = (0..1000)
        .map(|i| {
            doc!(
                "_id" => format!("{i}"),
                "name" => "John Doe",
                "age" => thread_rng().gen::<u32>(),
                "is_active" => thread_rng().gen::<bool>(),
                "created_at" => "2021-01-01",
                "updated_at" => "2021-01-01",
            )
        })
        .collect::<Vec<_>>();

    let mut pb_size = 0;
    let s = Instant::now();
    for doc in &docs {
        pb_size += black_box(doc.encode_to_vec()).len();
    }
    println!("pb time = {:?}", s.elapsed());

    let mut fb_size = 0;
    let s = Instant::now();
    for doc in &docs {
        fb_size += black_box(serialize_document(doc)).len();
    }
    println!("fb time = {:?}", s.elapsed());
    println!("fb len = {}, pb len = {}", fb_size, pb_size);
    drop(docs);
}

fn serialize_document(doc: &Document) -> Bytes {
    let mut fbb = flatbuffers::FlatBufferBuilder::new();

    let len = doc.fields.len();
    let (fields, values): (Vec<_>, Vec<_>) = doc.fields.iter().unzip();

    // Serialize fields
    fbb.start_vector::<ForwardsUOffset<&str>>(len);
    for field in fields {
        let field = fbb.create_string(&field);
        fbb.push(field);
    }
    let fields = fbb.end_vector::<ForwardsUOffset<&str>>(len);

    // Serialize values
    fbb.start_vector::<ForwardsUOffset<ValueFB>>(len);
    for value in values {
        let value = serialize_value(&mut fbb, value);
        fbb.push(value);
    }
    let values = fbb.end_vector::<ForwardsUOffset<ValueFB>>(len);

    let doc = DocumentFB::create(
        &mut fbb,
        &DocumentArgs {
            fields: Some(fields),
            values: Some(values),
        },
    );

    fbb.finish(doc, None);
    Bytes::copy_from_slice(fbb.finished_data())
}

fn serialize_value<'b>(fbb: &mut FlatBufferBuilder<'b>, value: &Value) -> WIPOffset<ValueFB<'b>> {
    macro_rules! create_value_fb {
        ($fbb:expr, $value_type:expr, $value:expr) => {
            ValueFB::create(
                $fbb,
                &ValueArgs {
                    value: Some($value.as_union_value()),
                    value_type: $value_type,
                },
            )
        };
    }

    match &value.value {
        Some(value_pb::Value::Bool(value)) => {
            let value = BoolFB::create(fbb, &BoolArgs { value: *value });
            create_value_fb!(fbb, ValueUnion::Bool, value)
        }
        Some(value_pb::Value::U32(value)) => {
            let value = U32FB::create(fbb, &U32Args { value: *value });
            create_value_fb!(fbb, ValueUnion::U32, value)
        }
        Some(value_pb::Value::U64(value)) => {
            let value = U64FB::create(fbb, &U64Args { value: *value });
            create_value_fb!(fbb, ValueUnion::U64, value)
        }
        Some(value_pb::Value::I32(value)) => {
            let value = I32FB::create(fbb, &I32Args { value: *value });
            create_value_fb!(fbb, ValueUnion::I32, value)
        }
        Some(value_pb::Value::I64(value)) => {
            let value = I64FB::create(fbb, &I64Args { value: *value });
            create_value_fb!(fbb, ValueUnion::I64, value)
        }
        Some(value_pb::Value::F32(value)) => {
            let value = F32FB::create(fbb, &F32Args { value: *value });
            create_value_fb!(fbb, ValueUnion::F32, value)
        }
        Some(value_pb::Value::F64(value)) => {
            let value = F64FB::create(fbb, &F64Args { value: *value });
            create_value_fb!(fbb, ValueUnion::F64, value)
        }
        Some(value_pb::Value::Binary(value)) => {
            let value = fbb.create_vector(value);
            let value = BytesFB::create(fbb, &BytesArgs { value: Some(value) });
            create_value_fb!(fbb, ValueUnion::Bytes, value)
        }
        Some(value_pb::Value::String(value)) => {
            let value = fbb.create_string(value);
            let value = StringFB::create(fbb, &StringArgs { value: Some(value) });
            create_value_fb!(fbb, ValueUnion::String, value)
        }
        _ => todo!(),
    }
}
