use test_context::test_context;
use topk_rs::Error;

mod utils;
use utils::ProjectTestContext;

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_list_datasets(ctx: &mut ProjectTestContext) {
    let d = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"))
        .await
        .expect("could not create dataset")
        .dataset
        .expect("invalid dataset proto")
        .name;

    let response = ctx
        .client
        .datasets()
        .list()
        .await
        .expect("could not list datasets")
        .datasets
        .iter()
        .map(|d| d.name.clone())
        .collect::<Vec<String>>();

    assert!(response.iter().any(|dd| dd == &d));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_create_dataset(ctx: &mut ProjectTestContext) {
    let d = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"))
        .await
        .expect("could not create dataset")
        .dataset
        .expect("invalid dataset proto")
        .name;

    let datasets = ctx
        .client
        .datasets()
        .list()
        .await
        .expect("could not list datasets")
        .datasets
        .iter()
        .map(|d| d.name.clone())
        .collect::<Vec<String>>();

    assert!(datasets.iter().any(|dd| dd == &d));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_create_duplicate_dataset(ctx: &mut ProjectTestContext) {
    ctx.client
        .datasets()
        .create(ctx.wrap("test"))
        .await
        .expect("could not create dataset");

    let err = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"))
        .await
        .expect_err("should not be able to create duplicate dataset");

    assert!(matches!(err, Error::DatasetAlreadyExists));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_delete_non_existent_dataset(ctx: &mut ProjectTestContext) {
    let err = ctx
        .client
        .datasets()
        .delete(ctx.wrap("test"))
        .await
        .expect_err("should not be able to delete non-existent dataset");

    assert!(matches!(err, Error::DatasetNotFound));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_delete_dataset(ctx: &mut ProjectTestContext) {
    let d = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"))
        .await
        .expect("could not create dataset")
        .dataset
        .expect("invalid dataset proto")
        .name;

    ctx.client
        .datasets()
        .delete(ctx.wrap("test"))
        .await
        .expect("could not delete dataset");

    let datasets = ctx
        .client
        .datasets()
        .list()
        .await
        .expect("could not list datasets")
        .datasets
        .iter()
        .map(|d| d.name.clone())
        .collect::<Vec<String>>();

    assert!(!datasets.iter().any(|dd| *dd == d));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_get_dataset(ctx: &mut ProjectTestContext) {
    // Test getting non-existent dataset
    let err = ctx
        .client
        .datasets()
        .get(ctx.wrap("test"))
        .await
        .expect_err("should not be able to get non-existent dataset");

    assert!(matches!(err, Error::DatasetNotFound));

    // Create dataset
    let d = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"))
        .await
        .expect("could not create dataset")
        .dataset
        .expect("invalid dataset proto")
        .name;

    // Get dataset
    let dataset = ctx
        .client
        .datasets()
        .get(ctx.wrap("test"))
        .await
        .expect("could not get dataset")
        .dataset
        .expect("invalid dataset proto")
        .name;

    assert_eq!(dataset, d);
}
