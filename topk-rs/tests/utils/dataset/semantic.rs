use crate::utils::ProjectTestContext;
use std::collections::HashMap;
use topk_protos::v1::{
    control::{Collection, FieldSpec},
    data::Document,
};
use topk_protos::{doc, schema};

#[allow(dead_code)]
pub async fn setup(ctx: &mut ProjectTestContext) -> Collection {
    let collection = ctx
        .client
        .collections()
        .create(ctx.wrap("semantic"), schema())
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
        "title" => FieldSpec::semantic(true, Some("dummy".into()), None),
        "summary" => FieldSpec::semantic(false, Some("dummy".into()), None),
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
        ),
        doc!(
            "_id" => "1984",
            "title" => "1984",
            "published_year" => 1949 as u32,
            "summary" => "A totalitarian regime uses surveillance and mind control to oppress its citizens.",
        ),
        doc!(
            "_id" => "pride",
            "title" => "Pride and Prejudice",
            "published_year" => 1813 as u32,
            "summary" => "A witty exploration of love, social class, and marriage in 19th-century England.",
        ),
        doc!(
            "_id" => "gatsby",
            "title" => "The Great Gatsby",
            "published_year" => 1925 as u32,
            "summary" => "A mysterious millionaire navigates love and wealth in the Roaring Twenties.",
        ),
        doc!(
            "_id" => "catcher",
            "title" => "The Catcher in the Rye",
            "published_year" => 1951 as u32,
            "summary" => "A rebellious teenager struggles with alienation and identity in mid-20th-century America.",
        ),
        doc!(
            "_id" => "moby",
            "title" => "Moby-Dick",
            "published_year" => 1851 as u32,
            "summary" => "A sailor's obsessive quest to hunt a great white whale leads to tragic consequences.",
        ),
        doc!(
            "_id" => "hobbit",
            "title" => "The Hobbit",
            "published_year" => 1937 as u32,
            "summary" => "A reluctant hobbit embarks on a quest to help a group of dwarves reclaim their mountain home.",
        ),
        doc!(
            "_id" => "harry",
            "title" => "Harry Potter and the Sorcerer's Stone",
            "published_year" => 1997 as u32,
            "summary" => "A young wizard discovers his magical heritage and attends a school for witchcraft and wizardry.",
        ),
        doc!(
            "_id" => "lotr",
            "title" => "The Lord of the Rings: The Fellowship of the Ring",
            "published_year" => 1954 as u32,
            "summary" => "A group of unlikely heroes sets out to destroy a powerful, evil ring.",
        ),
        doc!(
            "_id" => "alchemist",
            "title" => "The Alchemist",
            "published_year" => 1988 as u32,
            "summary" => "A shepherd boy journeys to fulfill his destiny and discover the meaning of life.",
        ),
    ]
}
