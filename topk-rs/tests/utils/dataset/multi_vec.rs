use std::collections::HashMap;

use topk_rs::proto::v1::control::field_type_matrix::MatrixValueType;
use topk_rs::proto::v1::control::{FieldIndex, KeywordIndexType, MultiVectorDistanceMetric};
use topk_rs::proto::v1::data::Value;
use topk_rs::proto::v1::{
    control::{Collection, FieldSpec},
    data::Document,
};
use topk_rs::{doc, schema};

use crate::utils::ProjectTestContext;

#[allow(dead_code)]
pub async fn setup(ctx: &mut ProjectTestContext) -> Collection {
    let collection = ctx
        .client
        .collections()
        .create(ctx.wrap("multi_vec"), schema())
        .await
        .expect("could not create collection");

    let mut lsn = "".to_string();
    let docs = docs();
    for chunk in docs.chunks(4) {
        lsn = ctx
            .client
            .collection(&collection.name)
            .upsert(chunk.to_vec())
            .await
            .expect("upsert failed");
    }

    let count = ctx
        .client
        .collection(&collection.name)
        .count(Some(lsn), None)
        .await
        .expect("could not query");

    assert_eq!(count, docs.len() as u64);

    collection
}

#[allow(dead_code)]
pub fn schema() -> HashMap<String, FieldSpec> {
    schema!(
        "title" => FieldSpec::text(true, Some(KeywordIndexType::Text)),
        "published_year" => FieldSpec::integer(true),
        "token_embeddings" => FieldSpec::matrix(false, 7, MatrixValueType::F32)
            .with_index(FieldIndex::multi_vector(MultiVectorDistanceMetric::Maxsim))
    )
}

#[allow(dead_code)]
pub fn docs() -> Vec<Document> {
    // !!! IMPORTANT !!!
    // Do not change the values of existing fields.
    // If you need to test new behavior which is not already covered by existing fields, add a new field.
    vec![
        doc!(
            "_id" => "doc_0",
            "title" => "To Kill a Mockingbird",
            "published_year" => 1960 as u32,
            "token_embeddings" => Value::matrix(2, 7, vec![
                0.9719, 0.132, 0.5612, -1.1843, -0.2115, 0.1455, -1.6471,
                -0.1054, 1.6053, -0.0901, 0.5288, -0.6347, 0.9521, -0.8853
            ]),
        ),
        doc!(
            "_id" => "doc_1",
            "title" => "1984",
            "published_year" => 1949 as u32,
            "token_embeddings" => Value::matrix(2, 7, vec![
                0.4364, -0.4954, 0.3665, 1.5041, -1.4773, -0.701, -0.9732,
                -1.2239, 1.7501, 0.4089, 2.0643, -1.3925, 0.4711, -0.6247
            ]),
        ),
        doc!(
            "_id" => "doc_2",
            "title" => "Pride and Prejudice",
            "published_year" => 1813 as u32,
            "token_embeddings" => Value::matrix(1, 7, vec![
                -2.6447, 0.3202, -0.5956, 0.6756, 1.0693, -1.0891, 1.0181
            ]),
        ),
        doc!(
            "_id" => "doc_3",
            "title" => "The Great Gatsby",
            "published_year" => 1925 as u32,
            "token_embeddings" => Value::matrix(3, 7, vec![
                0.1643, -0.2945, 1.3312, -0.3341, -0.3304, -0.029, -0.4426,
                -0.0975, -0.3696, -0.4106, -0.451, 0.4149, 0.8296, 0.3084,
                0.68, -0.182, -0.2652, -0.9707, -0.3433, 0.9671, -1.9293,
            ]),
        ),
        doc!(
            "_id" => "doc_4",
            "title" => "The Catcher in the Rye",
            "published_year" => 1951 as u32,
            "token_embeddings" => Value::matrix(2, 7, vec![
                0.8748, 0.9163, 1.5845, -1.303, 1.7739, 0.9365, 1.2679,
                -0.6695, 0.5488, -1.0841, 0.3331, 0.5206, -1.2897, 0.6149,
            ]),
        ),
        doc!(
            "_id" => "doc_5",
            "title" => "Moby-Dick",
            "published_year" => 1851 as u32,
            "token_embeddings" => Value::matrix(3, 7, vec![
                -0.6367, -0.5482, -1.2782, 1.0357, 1.044, -1.7687, 0.1703,
                -1.379, 0.0448, -0.7917, -1.693, -0.6001, 0.0598, 1.5035,
                1.968, -0.8128, 0.7871, -1.2036, -0.6445, -0.0684, 0.3407,
            ]),
        ),
        doc!(
            "_id" => "doc_6",
            "title" => "The Hobbit",
            "published_year" => 1937 as u32,
            "token_embeddings" => Value::matrix(3, 7, vec![
                -0.4733, 0.5792, 0.1226, 0.4607, -0.3138, -0.2211, -0.1725,
                1.0828, -0.9416, 0.0848, 1.5135, 1.0625, 0.5481, 0.1558,
                0.71, -1.3281, 0.5986, -2.2235, -0.1252, -0.5943, 0.6521,
            ]),
        ),
        doc!(
            "_id" => "doc_7",
            "title" => "Harry Potter and the Sorcerer's Stone",
            "published_year" => 1997 as u32,
            "token_embeddings" => Value::matrix(3, 7, vec![
                -0.4046, -0.1552, 2.632, -0.5471, -0.1942, -0.731, -1.1103,
                0.5813, 0.247, 0.0275, 0.0063, -2.4539, -0.2918, 1.1274,
                1.0666, 0.5535, 1.184, 0.5897, 1.2976, 1.2298, 2.6738,
            ]),
        ),
        doc!(
            "_id" => "doc_8",
            "title" => "The Lord of the Rings: The Fellowship of the Ring",
            "published_year" => 1954 as u32,
            "token_embeddings" => Value::matrix(4, 7, vec![
                -0.2822, -0.4862, 2.0163, -1.4105, 2.1853, 0.583, 0.7119,
                -1.7254, 0.3599, 0.2296, 0.1091, -0.6483, 0.3901, -0.9539,
                -0.5296, -0.3046, 1.5027, 0.7712, -1.071, 0.7371, 0.1228,
                1.7048, 0.182, 0.3116, 0.7806, 0.2414, -0.7322, -0.1204,
            ]),
        ),
        doc!(
            "_id" => "doc_9",
            "title" => "The Alchemist",
            "published_year" => 1988 as u32,
        ),
    ]
}
