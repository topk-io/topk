use crate::utils::ProjectTestContext;
use std::collections::HashMap;
use topk_rs::proto::v1::control::field_type_list::ListValueType;
use topk_rs::proto::v1::control::FieldIndex;
use topk_rs::proto::v1::control::KeywordIndexType;
use topk_rs::proto::v1::control::VectorDistanceMetric;
use topk_rs::proto::v1::data::Value;
use topk_rs::proto::v1::{
    control::{Collection, FieldSpec},
    data::Document,
};
use topk_rs::{doc, schema};

#[allow(dead_code)]
pub async fn setup(ctx: &mut ProjectTestContext) -> Collection {
    let collection = ctx
        .client
        .collections()
        .create(ctx.wrap("books"), schema())
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
        "summary" => FieldSpec::text(true, Some(KeywordIndexType::Text)),
        "summary_embedding" => FieldSpec::f32_vector(16, true, VectorDistanceMetric::Euclidean),
        "nullable_embedding" => FieldSpec::f32_vector(16, false, VectorDistanceMetric::Euclidean),
        "scalar_embedding" => FieldSpec::u8_vector(16, false, VectorDistanceMetric::Euclidean),
        "scalar_i8_embedding" => FieldSpec::i8_vector(16, false, VectorDistanceMetric::Euclidean),
        "binary_embedding" => FieldSpec::binary_vector(2, false, VectorDistanceMetric::Hamming),
        "sparse_f32_embedding" => FieldSpec::f32_sparse_vector(true, VectorDistanceMetric::DotProduct),
        "sparse_u8_embedding" => FieldSpec::u8_sparse_vector(false, VectorDistanceMetric::DotProduct),
        "tags" => FieldSpec::list(true, ListValueType::String).with_index(FieldIndex::keyword(KeywordIndexType::Text)),
    )
}

#[allow(dead_code)]
pub fn docs() -> Vec<Document> {
    // !!! IMPORTANT !!!
    // Do not change the values of existing fields.
    // If you need to test new behavior which is not already covered by existing fields, add a new field.
    vec![
        doc!(
            "_id" => "mockingbird",
            "title" => "To Kill a Mockingbird",
            "published_year" => 1960 as u32,
            "summary" => "A young girl confronts racial injustice in the Deep South through the eyes of her lawyer father.",
            "summary_embedding" => vec![1.0; 16],
            "nullable_embedding" => vec![1.0; 16],
            "scalar_embedding" => Value::list(vec![1u8; 16]),
            "binary_embedding" => Value::list(vec![0u8, 1]),
            "sparse_f32_embedding" => Value::f32_sparse_vector(vec![0, 1, 2], vec![1.0, 2.0, 3.0]),
            "sparse_u8_embedding" => Value::u8_sparse_vector(vec![0, 1, 2], vec![1, 2, 3]),
            "scalar_i8_embedding" => Value::list(vec![-100i8; 16]),
            "nullable_importance" => 2.0_f32,
            "tags" => Value::list(vec!["racism".to_string(), "injustice".to_string(), "girl".to_string(), "father".to_string(), "lawyer".to_string()]),
            "codes" => Value::list(vec!["ISBN 978-0-06-093546-7".to_string(), "ISBN 0-06-093546-4".to_string(), "LCCN 60007854".to_string(), "UPC 025192354670".to_string()]),
            "reprint_years" => Value::list(vec![1966u32, 1988, 1999, 2002, 2015]),
            "user_ratings" => Value::list(vec![9u32, 7, 10, 8])
        ),
        doc!(
            "_id" => "1984",
            "title" => "1984",
            "published_year" => 1949 as u32,
            "summary" => "A totalitarian regime uses surveillance and mind control to oppress its citizens.",
            "summary_embedding" => vec![2.0; 16],
            "nullable_embedding" => vec![2.0; 16],
            "scalar_embedding" => Value::list(vec![2u8; 16]),
            "scalar_i8_embedding" => Value::list(vec![-50i8; 16]),
            "binary_embedding" => Value::list(vec![0u8, 3]),
            "sparse_f32_embedding" => Value::f32_sparse_vector(vec![2,3,4], vec![1.0, 2.0, 3.0]),
            "sparse_u8_embedding" => Value::u8_sparse_vector(vec![2,3,4], vec![1, 2, 3]),
            "tags" => Value::list(vec!["dystopia".to_string(), "surveillance".to_string(), "totalitarianism".to_string(), "mind control".to_string(), "oppression".to_string()]),
            "codes" => Value::list(vec!["1984".to_string(), "ISBN 978-0-452-28423-4".to_string(), "ISBN 0-452-28423-6".to_string(), "OCLC 70775700".to_string(), "DOI 10.1000/182".to_string(), "EAN 9780452284234".to_string()]),
            "reprint_years" => Value::list(vec![1950u32, 1954, 1956, 1961, 1984, 1990, 2003]),
            "user_ratings" => Value::list(vec![5.0f32, 3.5, 4.5, 4.0, -1.0])
        ),
        doc!(
            "_id" => "pride",
            "title" => "Pride and Prejudice",
            "published_year" => 1813 as u32,
            "summary" => "A witty exploration of love, social class, and marriage in 19th-century England.",
            "summary_embedding" => vec![3.0; 16],
            "scalar_i8_embedding" => Value::list(vec![0i8; 16]),
            "sparse_f32_embedding" => Value::f32_sparse_vector(vec![3, 4, 5], vec![1.0, 2.0, 3.0]),
            "sparse_u8_embedding" => Value::u8_sparse_vector(vec![3, 4, 5], vec![1, 2, 3]),
            "tags" => Value::list(vec!["pride".to_string(), "love".to_string(), "romance".to_string(), "class".to_string(), "marriage".to_string(), "prejudice".to_string()]),
            "codes" => Value::list(vec!["ISBN 978-0-14-143951-8".to_string(), "OCLC 934546789".to_string()]),
            "reprint_years" => Value::list(vec![1966u32, 1972, 1985]),
            "user_ratings" => Value::list(vec!["romantic".to_string(), "classic".to_string(), "good".to_string()]),
        ),
        doc!(
            "_id" => "gatsby",
            "title" => "The Great Gatsby",
            "published_year" => 1925 as u32,
            "summary" => "A mysterious millionaire navigates love and wealth in the Roaring Twenties.",
            "summary_embedding" => vec![4.0; 16],
            "scalar_i8_embedding" => Value::list(vec![50i8; 16]),
            "sparse_f32_embedding" => Value::f32_sparse_vector(vec![4, 5, 6], vec![1.0, 2.0, 3.0]),
            "sparse_u8_embedding" => Value::u8_sparse_vector(vec![4, 5, 6], vec![1, 2, 3]),
            "tags" => Value::list(vec!["love".to_string(), "romance".to_string(), "wealth".to_string(), "marriage".to_string()]),
            "codes" => Value::list(vec!["ISBN 978-0-7432-7356-5".to_string(), "ISBN 0-7432-7356-7".to_string(), "OCLC 60393320".to_string(), "ASIN B000FC0SIS".to_string(), "UPC 074327356709".to_string(), "LCCN 2002114274".to_string()]),
            "reprint_years" => Value::list(vec![1953u32, 1996, 2004]),
            "user_ratings" => "good book",
        ),
        doc!(
            "_id" => "catcher",
            "title" => "The Catcher in the Rye",
            "published_year" => 1951 as u32,
            "summary" => "A rebellious teenager struggles with alienation and identity in mid-20th-century America.",
            "summary_embedding" => vec![5.0; 16],
            "nullable_embedding" => vec![5.0; 16],
            "scalar_embedding" => Value::list(vec![5u8; 16]),
            "scalar_i8_embedding" => Value::list(vec![100i8; 16]),
            "binary_embedding" => Value::list(vec![0u8, 7]),
            "sparse_f32_embedding" => Value::f32_sparse_vector(vec![5, 6, 7], vec![1.0, 2.0, 3.0]),
            "sparse_u8_embedding" => Value::u8_sparse_vector(vec![5, 6, 7], vec![1, 2, 3]),
            "tags" => Value::list(vec!["alienation".to_string(), "identity".to_string(), "rebellion".to_string(), "mid-20th-century".to_string(), "america".to_string()]),
            "codes" => Value::list(vec!["ISBN 978-0-316-76917-4".to_string(), "LCCN 51011564".to_string(), "OCLC 287628".to_string()]),
            "reprint_years" => Value::list(vec![1964u32, 1979, 1991, 2001, 2010]),
            "user_ratings" => Value::list(vec![4.5f32, 3.0, 3.5, 4.0, -1.0]),
        ),
        doc!(
            "_id" => "moby",
            "title" => "Moby-Dick",
            "published_year" => 1851 as u32,
            "summary" => "A sailor's obsessive quest to hunt a great white whale leads to tragic consequences.",
            "summary_embedding" => vec![6.0; 16],
            "sparse_f32_embedding" => Value::f32_sparse_vector(vec![6,7,8], vec![1.0, 2.0, 3.0]),
            "sparse_u8_embedding" => Value::u8_sparse_vector(vec![6,7,8], vec![1, 2, 3]),
            "nullable_importance" => 5.0_f32,
            "tags" => Value::list(vec!["whale".to_string(), "obsession".to_string(), "tragedy".to_string(), "sailing".to_string(), "ocean".to_string()]),
            "codes" => Value::list(Vec::<String>::new()),
            "reprint_years" => Value::list(Vec::<u32>::new()),
            "user_ratings" => Value::list(vec![-5i64, 2, -1, 1]),
        ),
        doc!(
            "_id" => "hobbit",
            "title" => "The Hobbit",
            "published_year" => 1937 as u32,
            "summary" => "A reluctant hobbit embarks on a quest to help a group of dwarves reclaim their mountain home.",
            "summary_embedding" => vec![7.0; 16],
            "sparse_f32_embedding" => Value::f32_sparse_vector(vec![7,8,9], vec![1.0, 2.0, 3.0]),
            "sparse_u8_embedding" => Value::u8_sparse_vector(vec![7,8,9], vec![1, 2, 3]),
            "tags" => Value::list(vec!["hobbit".to_string(), "dwarf".to_string(), "quest".to_string(), "home".to_string(), "adventure".to_string()]),
            "user_ratings" => Value::list(vec![5u32, 3, 4, 5, 2]),
        ),
        doc!(
            "_id" => "harry",
            "title" => "Harry Potter and the Sorcerer's Stone",
            "published_year" => 1997 as u32,
            "summary" => "A young wizard discovers his magical heritage and attends a school for witchcraft and wizardry.",
            "summary_embedding" => vec![8.0; 16],
            "nullable_embedding" => vec![8.0; 16],
            "scalar_embedding" => Value::list(vec![8u8; 16]),
            "binary_embedding" => Value::list(vec![0u8, 15]),
            "sparse_f32_embedding" => Value::f32_sparse_vector(vec![8,9,10], vec![1.0, 2.0, 3.0]),
            "sparse_u8_embedding" => Value::u8_sparse_vector(vec![8,9,10], vec![1, 2, 3]),
            "tags" => Value::list(vec!["wizard".to_string(), "magic".to_string(), "sorcerer".to_string(), "school".to_string(), "witchcraft".to_string()]),
            "codes" => Value::list(vec!["ISBN 978-0-439-70818-0".to_string(), "UPC 043970818909".to_string()]),
            "reprint_years" => Value::list(vec![1998u32, 1999, 2001, 2004, 2013, 2020]),
            "user_ratings" => 10_i64,
        ),
        doc!(
            "_id" => "lotr",
            "title" => "The Lord of the Rings: The Fellowship of the Ring",
            "published_year" => 1954 as u32,
            "summary" => "A group of unlikely heroes sets out to destroy a powerful, evil ring.",
            "summary_embedding" => vec![9.0; 16],
            "scalar_i8_embedding" => Value::list(vec![-100i8; 16]),
            "sparse_f32_embedding" => Value::f32_sparse_vector(vec![9,10,11], vec![1.0, 2.0, 3.0]),
            "sparse_u8_embedding" => Value::u8_sparse_vector(vec![9,10,11], vec![1, 2, 3]),
            "tags" => Value::list(vec!["lord of the rings".to_string(), "fellowship".to_string(), "magic".to_string(), "wizard".to_string(), "elves".to_string()]),
            "codes" => Value::list(vec!["ISBN 978-0-547-92821-0".to_string(), "ISBN 0-547-92821-2".to_string(), "OCLC 434394005".to_string(), "LCCN 2004558654".to_string(), "Barcode 0618346252".to_string()]),
            "user_ratings" => Value::list(vec!["epic".to_string(), "legendary".to_string(), "good".to_string()]),
        ),
        doc!(
            "_id" => "alchemist",
            "title" => "The Alchemist",
            "published_year" => 1988 as u32,
            "summary" => "A shepherd boy journeys to fulfill his destiny and discover the meaning of life.",
            "summary_embedding" => vec![10.0; 16],
            "sparse_f32_embedding" => Value::f32_sparse_vector(vec![10,11,12], vec![1.0, 2.0, 3.0]),
            "sparse_u8_embedding" => Value::u8_sparse_vector(vec![10,11,12], vec![1, 2, 3]),
            "tags" => Value::list(vec!["journey".to_string(), "destiny".to_string(), "meaning of life".to_string(), "alchemy".to_string(), "soul".to_string()]),
            "codes" => Value::list(vec!["ISBN 978-0-06-231500-7".to_string(), "ASIN 0062315005".to_string(), "OCLC 804616251".to_string(), "DOI 10.1234/alchemist".to_string(), "EAN 9780062315007".to_string(), "UPC 006231500719".to_string(), "LCCN 88675123".to_string()]),
            "reprint_years" => Value::list(vec![1993u32, 2005, 2014]),
            "user_ratings" => Value::list(vec![8u32, 10, 9, 7]),
        ),
    ]
}
