use crate::import::common::{refused, Ctx};
use futures::TryStreamExt;
use std::collections::HashMap;
use test_macros::rstest_ctx;
use topk::import::source::Source;
use topk::import::state::Run;
use topk::import::{Import, WriteOptions};
use topk_rs::doc;
use topk_rs::proto::v1::control::{
    field_type_matrix::MatrixValueType, FieldIndex, FieldSpec, MultiVectorDistanceMetric,
    MultiVectorQuantization, VectorDistanceMetric,
};
use topk_rs::proto::v1::data::{ConsistencyLevel, Value};

#[rstest_ctx(Ctx)]
async fn copies_native_values(ctx: &Ctx) {
    let name = ctx.collection("native");
    let dest = ctx.collection("copy");
    let f16 = [1., 2.].map(half::f16::from_f64).to_vec();
    let f8 = [1., 2.].map(float8::F8E4M3::from_f64).to_vec();
    let mut schema = HashMap::new();
    let mut document = doc!("_id" => "one");
    for (name, ty, value) in [
        (
            "f32",
            FieldSpec::f32_vector(2, false)
                .with_index(FieldIndex::vector(VectorDistanceMetric::Euclidean)),
            Value::list(vec![0.125_f32, -2.75]),
        ),
        (
            "f16",
            FieldSpec::f16_vector(2, false),
            Value::list(f16.clone()),
        ),
        (
            "f8",
            FieldSpec::f8_vector(2, false),
            Value::list(f8.clone()),
        ),
        (
            "u8",
            FieldSpec::u8_vector(2, false),
            Value::list(vec![1_u8, 2]),
        ),
        (
            "i8",
            FieldSpec::i8_vector(2, false),
            Value::list(vec![-1_i8, 2]),
        ),
        (
            "s32",
            FieldSpec::f32_sparse_vector(false),
            Value::sparse_vector(vec![1, 3], vec![1_f32, 2.]),
        ),
        (
            "s16",
            FieldSpec::f16_sparse_vector(false),
            Value::sparse_vector(vec![1, 3], f16.clone()),
        ),
        (
            "s8",
            FieldSpec::f8_sparse_vector(false),
            Value::sparse_vector(vec![1, 3], f8.clone()),
        ),
        (
            "su8",
            FieldSpec::u8_sparse_vector(false),
            Value::sparse_vector(vec![1, 3], vec![1_u8, 2]),
        ),
        (
            "si8",
            FieldSpec::i8_sparse_vector(false),
            Value::sparse_vector(vec![1, 3], vec![-1_i8, 2]),
        ),
        (
            "quantized_matrix",
            FieldSpec::matrix(false, 7, MatrixValueType::F32).with_index(FieldIndex::multi_vector(
                MultiVectorDistanceMetric::Maxsim,
                Some(MultiVectorQuantization::Scalar),
                None,
                None,
            )),
            Value::matrix(
                7,
                vec![
                    0.125_f32, -0.37, 2.75, 0.91, -1.23, 0.04, 3.1, -2.6, 0.33, 1.07, -0.19, 0.73,
                    2.01, -0.81,
                ],
            ),
        ),
        (
            "m16",
            FieldSpec::matrix(false, 2, MatrixValueType::F16),
            Value::matrix(2, f16.repeat(2)),
        ),
        (
            "m8",
            FieldSpec::matrix(false, 2, MatrixValueType::F8),
            Value::matrix(2, f8.repeat(2)),
        ),
        (
            "mu8",
            FieldSpec::matrix(false, 2, MatrixValueType::U8),
            Value::matrix(2, vec![1_u8, 2, 3, 4]),
        ),
        (
            "mi8",
            FieldSpec::matrix(false, 2, MatrixValueType::I8),
            Value::matrix(2, vec![1_i8, 2, 3, 4]),
        ),
    ] {
        schema.insert(name.into(), ty);
        document.fields.insert(name.into(), value);
    }
    ctx.client
        .collections()
        .create(&name, schema.clone(), None)
        .await
        .unwrap();
    ctx.client
        .collection(&name)
        .upsert(vec![document.clone()])
        .await
        .unwrap();
    let schema = ctx.client.collections().get(&name).await.unwrap().schema;
    let source = Source::topk(
        (*ctx.client).clone(),
        format!("topk://{name}"),
        name,
        schema.clone(),
    );
    let spec = source
        .catalog()
        .await
        .unwrap()
        .discover(&[], Some(&dest), None)
        .unwrap();
    let directory = ctx.cli.path("state");
    let run = Run::start(&directory, ctx.state(&source, spec))
        .await
        .unwrap();
    let import = Import::prepare(source, (*ctx.client).clone(), run)
        .await
        .unwrap();
    import
        .execute(WriteOptions {
            continue_on_error: false,
            concurrency: 1,
            batch_bytes: bytesize::ByteSize(1_000_000),
        })
        .try_collect::<Vec<_>>()
        .await
        .unwrap();
    let copied = ctx
        .client
        .collection(&dest)
        .get(["one"], None, None, Some(ConsistencyLevel::Strong))
        .await
        .unwrap();
    assert_eq!(copied["one"], document.fields);
    assert_eq!(
        ctx.client.collections().get(&dest).await.unwrap().schema,
        schema
    );
}

#[rstest_ctx(Ctx)]
async fn rejects_partitioned_copy(ctx: &Ctx) {
    let name = ctx.collection("partitioned");
    ctx.client
        .collections()
        .create(&name, HashMap::new(), None)
        .await
        .unwrap();
    ctx.client
        .collection(&name)
        .partition("p1")
        .upsert(vec![doc!("_id" => "one")])
        .await
        .unwrap();
    let source = Source::topk(
        (*ctx.client).clone(),
        format!("topk://{name}"),
        name,
        HashMap::new(),
    );
    assert!(refused(source.catalog().await).contains("named partitions"));
}
