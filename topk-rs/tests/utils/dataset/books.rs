use crate::utils::ProjectTestContext;
use std::collections::HashMap;
use topk_protos::v1::{
    control::{Collection, FieldSpec, KeywordIndexType, VectorDistanceMetric},
    data::{Document, Value},
};
use topk_protos::{doc, schema};

#[allow(dead_code)]
pub async fn setup(ctx: &mut ProjectTestContext) -> Collection {
    let collection = ctx
        .client
        .collections()
        .create(ctx.wrap("books"), schema())
        .await
        .expect("could not create collection");

    let lsn = ctx
        .client
        .collection(&collection.name)
        .upsert(docs())
        .await
        .expect("upsert failed");

    let _ = ctx
        .client
        .collection(&collection.name)
        .count(Some(lsn), None)
        .await
        .expect("could not query");

    collection
}

#[allow(dead_code)]
pub fn schema() -> HashMap<String, FieldSpec> {
    schema!(
        "title" => FieldSpec::text(true, Some(KeywordIndexType::Text)),
        "published_year" => FieldSpec::integer(true),
        "summary" => FieldSpec::text(true, Some(KeywordIndexType::Text)),
        "summary_embedding" => FieldSpec::f32_vector(16, true, VectorDistanceMetric::Euclidean),
        "nullable_embedding" => FieldSpec::f32_vector(16, false, VectorDistanceMetric::Euclidean),
        "scalar_embedding" => FieldSpec::u8_vector(16, false, VectorDistanceMetric::Euclidean),
        "binary_embedding" => FieldSpec::binary_vector(2, false, VectorDistanceMetric::Hamming),
    )
}

#[allow(dead_code)]
pub fn docs() -> Vec<Document> {
    vec![
        doc!(
            "_id" => "mockingbird",
            "title" => "To Kill a Mockingbird",
            "published_year" => 1960 as u32,
            "summary" => "A young girl confronts racial injustice in the Deep South through the eyes of her lawyer father.",
            "summary_embedding" => vec![1.0; 16],
            "nullable_embedding" => vec![1.0; 16],
            "scalar_embedding" => Value::byte_vector(vec![1; 16]),
            "binary_embedding" => Value::byte_vector(vec![0, 1]),
        ),
        doc!(
            "_id" => "1984",
            "title" => "1984",
            "published_year" => 1949 as u32,
            "summary" => "A totalitarian regime uses surveillance and mind control to oppress its citizens.",
            "summary_embedding" => vec![2.0; 16],
            "nullable_embedding" => vec![2.0; 16],
            "scalar_embedding" => Value::byte_vector(vec![2; 16]),
            "binary_embedding" => Value::byte_vector(vec![0, 3]),
        ),
        doc!(
            "_id" => "pride",
            "title" => "Pride and Prejudice",
            "published_year" => 1813 as u32,
            "summary" => "A witty exploration of love, social class, and marriage in 19th-century England.",
            "summary_embedding" => vec![3.0; 16],
        ),
        doc!(
            "_id" => "gatsby",
            "title" => "The Great Gatsby",
            "published_year" => 1925 as u32,
            "summary" => "A mysterious millionaire navigates love and wealth in the Roaring Twenties.",
            "summary_embedding" => vec![4.0; 16],
        ),
        doc!(
            "_id" => "catcher",
            "title" => "The Catcher in the Rye",
            "published_year" => 1951 as u32,
            "summary" => "A rebellious teenager struggles with alienation and identity in mid-20th-century America.",
            "summary_embedding" => vec![5.0; 16],
            "nullable_embedding" => vec![5.0; 16],
            "scalar_embedding" => Value::byte_vector(vec![5; 16]),
            "binary_embedding" => Value::byte_vector(vec![0, 7]),
        ),
        doc!(
            "_id" => "moby",
            "title" => "Moby-Dick",
            "published_year" => 1851 as u32,
            "summary" => "A sailor's obsessive quest to hunt a great white whale leads to tragic consequences.",
            "summary_embedding" => vec![6.0; 16],
        ),
        doc!(
            "_id" => "hobbit",
            "title" => "The Hobbit",
            "published_year" => 1937 as u32,
            "summary" => "A reluctant hobbit embarks on a quest to help a group of dwarves reclaim their mountain home.",
            "summary_embedding" => vec![7.0; 16],
        ),
        doc!(
            "_id" => "harry",
            "title" => "Harry Potter and the Sorcerer's Stone",
            "published_year" => 1997 as u32,
            "summary" => "A young wizard discovers his magical heritage and attends a school for witchcraft and wizardry.",
            "summary_embedding" => vec![8.0; 16],
            "nullable_embedding" => vec![8.0; 16],
            "scalar_embedding" => Value::byte_vector(vec![8; 16]),
            "binary_embedding" => Value::byte_vector(vec![0, 15]),
        ),
        doc!(
            "_id" => "lotr",
            "title" => "The Lord of the Rings: The Fellowship of the Ring",
            "published_year" => 1954 as u32,
            "summary" => "A group of unlikely heroes sets out to destroy a powerful, evil ring.",
            "summary_embedding" => vec![9.0; 16],
        ),
        doc!(
            "_id" => "alchemist",
            "title" => "The Alchemist",
            "published_year" => 1988 as u32,
            "summary" => "A shepherd boy journeys to fulfill his destiny and discover the meaning of life.",
            "summary_embedding" => vec![10.0; 16],
        ),
    ]
}
