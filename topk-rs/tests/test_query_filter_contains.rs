use test_context::test_context;
use topk_rs::proto::v1::data::Value;
use topk_rs::query::{field, filter, not, select};
use topk_rs::{doc, Error};

mod utils;
use utils::dataset;
use utils::ProjectTestContext;

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_string_contains_literal(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    // find books with "ob" substring in their ID
    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(field("_id").contains("ob")).topk(field("published_year"), 100, false),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(result, ["moby", "hobbit"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_string_contains_literal_no_match(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    // search for books with non-existent text in ID
    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(field("_id").contains("rubbish")).topk(field("published_year"), 100, false),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(result, Vec::<String>::new());
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_string_contains_literal_empty(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    // empty substring matches all books
    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(field("_id").contains("")).topk(field("published_year"), 100, false),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(
        result,
        [
            "gatsby",
            "catcher",
            "moby",
            "mockingbird",
            "alchemist",
            "harry",
            "lotr",
            "pride",
            "1984",
            "hobbit"
        ]
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_string_contains_literal_with_keyword_index(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    // search for books with the substring "to h" in their summary
    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(field("summary").contains("to h")).topk(field("published_year"), 100, false),
            None,
            None,
        )
        .await
        .expect("could not query");

    // "to hunt", "to help"
    assert_doc_ids!(result, ["moby", "hobbit"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_string_contains_field(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    // find books where the id is a substring of the title
    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(field("title").contains(field("_id"))).topk(field("published_year"), 100, false),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(result, ["1984"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_string_in_field(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    // find books where the id is a substring of the title
    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(field("_id").in_(field("title"))).topk(field("published_year"), 100, false),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(result, ["1984"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_string_contains_field_self(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    // verify that title is always a substring of itself
    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(not(field("title").contains(field("title")))).topk(
                field("published_year"),
                100,
                false,
            ),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(result, vec![]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_list_match_any_with_keyword_index(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    // find books tagged with "love" using the keyword index
    let results = ctx
        .client
        .collection(&collection.name)
        .query(
            select([
                ("_id", field("_id")),
                ("title", field("title")),
                ("tags", field("tags")),
            ])
            .filter(field("tags").match_any("love"))
            .topk(field("published_year"), 100, true),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(
        results,
        vec![
            doc!("_id" => "pride", "title" => "Pride and Prejudice", "tags" => Value::list(vec!["pride".to_string(), "love".to_string(), "romance".to_string(), "class".to_string(), "marriage".to_string(), "prejudice".to_string()])),
            doc!("_id" => "gatsby", "title" => "The Great Gatsby", "tags" => Value::list(vec!["love".to_string(), "romance".to_string(), "wealth".to_string(), "marriage".to_string()])),
        ]
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_list_match_any_all_without_keyword_index(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    // verify match_any and match_all fail on list field without a keyword index
    for filter_expr in vec![
        field("codes").match_any("ISBN 0-547-92821-2"),
        field("codes").match_all("ISBN 0-547-92821-2"),
    ] {
        let err = ctx
            .client
            .collection(&collection.name)
            .query(
                select([
                    ("_id", field("_id")),
                    ("title", field("title")),
                    ("codes", field("codes")),
                ])
                .filter(filter_expr)
                .topk(field("published_year"), 100, true),
                None,
                None,
            )
            .await
            .expect_err("should have failed");

        assert!(matches!(err, Error::InvalidArgument(_)));
    }
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_list_contains_with_keyword_index(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    // find books with "love" in their tags list optimized using the keyword index
    let results = ctx
        .client
        .collection(&collection.name)
        .query(
            select([
                ("_id", field("_id")),
                ("title", field("title")),
                ("tags", field("tags")),
            ])
            .filter(field("tags").contains("love"))
            .topk(field("published_year"), 100, true),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(
        results,
        vec![
            doc!("_id" => "pride", "title" => "Pride and Prejudice", "tags" => Value::list(vec!["pride".to_string(), "love".to_string(), "romance".to_string(), "class".to_string(), "marriage".to_string(), "prejudice".to_string()])),
            doc!("_id" => "gatsby", "title" => "The Great Gatsby", "tags" => Value::list(vec!["love".to_string(), "romance".to_string(), "wealth".to_string(), "marriage".to_string()])),
        ]
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_list_contains_literal(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    // find books with specific ISBN code
    let results = ctx
        .client
        .collection(&collection.name)
        .query(
            select([
                ("_id", field("_id")),
                ("title", field("title")),
                ("codes", field("codes")),
            ])
            .filter(field("codes").contains("ISBN 0-547-92821-2"))
            .topk(field("published_year"), 100, true),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(
        results,
        vec![
            doc!("_id" => "lotr", "title" => "The Lord of the Rings: The Fellowship of the Ring", "codes" => Value::list(vec!["ISBN 978-0-547-92821-0".to_string(), "ISBN 0-547-92821-2".to_string(), "OCLC 434394005".to_string(), "LCCN 2004558654".to_string(), "Barcode 0618346252".to_string()])),
        ]
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_list_contains_int_literal(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    // books that were reprinted in 1999
    let results = ctx
        .client
        .collection(&collection.name)
        .query(
            select([
                ("_id", field("_id")),
                ("title", field("title")),
                ("reprint_years", field("reprint_years")),
            ])
            .filter(field("reprint_years").contains(1999u32))
            .topk(field("published_year"), 100, true),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(results, ["mockingbird", "harry"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_list_contains_int_literal_different_type(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    // books that were reprinted in 1999
    let results = ctx
        .client
        .collection(&collection.name)
        .query(
            select([
                ("_id", field("_id")),
                ("title", field("title")),
                ("reprint_years", field("reprint_years")),
            ])
            .filter(field("reprint_years").contains(1999i32))
            .topk(field("published_year"), 100, true),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(results, ["mockingbird", "harry"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_list_contains_int_field(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    // books which were reprinted one year after they were published
    let results = ctx
        .client
        .collection(&collection.name)
        .query(
            select([
                ("_id", field("_id")),
                ("title", field("title")),
                ("reprint_years", field("reprint_years")),
            ])
            .filter(field("reprint_years").contains(field("published_year").add(1)))
            .topk(field("published_year"), 100, true),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(results, ["harry", "1984"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_list_in_int_field(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    // books which were reprinted one year after they were published
    let results = ctx
        .client
        .collection(&collection.name)
        .query(
            select([
                ("_id", field("_id")),
                ("title", field("title")),
                ("reprint_years", field("reprint_years")),
            ])
            .filter(field("published_year").add(1).in_(field("reprint_years")))
            .topk(field("published_year"), 100, true),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(results, ["harry", "1984"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_list_contains_string_field_with_keyword_index(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    // books which have a tag that is the same as the book's id
    let results = ctx
        .client
        .collection(&collection.name)
        .query(
            select([
                ("_id", field("_id")),
                ("title", field("title")),
                ("reprint_years", field("reprint_years")),
            ])
            .filter(field("tags").contains(field("_id")))
            .topk(field("published_year"), 100, true),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(results, ["pride", "hobbit"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_list_in_string_field_with_keyword_index(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    // books which have a tag that is the same as the book's id
    let results = ctx
        .client
        .collection(&collection.name)
        .query(
            select([
                ("_id", field("_id")),
                ("title", field("title")),
                ("reprint_years", field("reprint_years")),
            ])
            .filter(field("_id").in_(field("tags")))
            .topk(field("published_year"), 100, true),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(results, ["pride", "hobbit"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_list_contains_string_field_without_keyword_index(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    // books which have a code that is the same as the book's id
    let results = ctx
        .client
        .collection(&collection.name)
        .query(
            select([
                ("_id", field("_id")),
                ("title", field("title")),
                ("codes", field("codes")),
            ])
            .filter(field("codes").contains(field("_id")))
            .topk(field("published_year"), 100, true),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(results, ["1984"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_list_contains_invalid_types(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    // verify that invalid type combinations fail validation
    for filter_expr in vec![
        field("codes").contains(Value::list(vec![
            "ISBN 978-0-547-92821-0".to_string(),
            "ISBN 0-547-92821-2".to_string(),
        ])),
        field("codes").contains(978),
        field("codes").contains(Value::list(vec![978])),
        field("codes").contains(true),
        field("codes").contains(field("published_year")),
        field("reprint_years").contains(field("title")),
        field("published_year").contains(field("reprint_years")),
    ] {
        let err = ctx
            .client
            .collection(&collection.name)
            .query(
                select([
                    ("_id", field("_id")),
                    ("title", field("title")),
                    ("codes", field("codes")),
                ])
                .filter(filter_expr)
                .topk(field("published_year"), 100, true),
                None,
                None,
            )
            .await
            .expect_err("should have failed");

        assert!(matches!(err, Error::InvalidArgument(_)));
    }
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_string_in(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    // find books where the id is a substring of the title
    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(field("_id").in_("harryhobbitlotr")).topk(field("published_year"), 100, false),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(result, ["harry", "hobbit", "lotr"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_in_list_literal_int(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("_id", field("_id")), ("title", field("title"))])
                .filter(field("published_year").in_(Value::list(vec![1999u32, 1988, 1997])))
                .topk(field("published_year"), 100, true),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(result, ["alchemist", "harry"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_in_list_literal_string(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("_id", field("_id")), ("title", field("title"))])
                .filter(field("title").in_(Value::list(vec![
                    "The Great Gatsby".to_string(),
                    "The Catcher in the Rye".to_string(),
                    "The Lord of the Rings: NOT THIS ONE".to_string(),
                    "The".to_string(),
                    "something 123".to_string(),
                ])))
                .topk(field("published_year"), 100, true),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(result, ["gatsby", "catcher"]);
}
