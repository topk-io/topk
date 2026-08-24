mod common;

use std::collections::BTreeMap;

use assert_cmd::Command;
use serde_json::json;
use tempfile::tempdir;
use test_context::test_context;
use topk::commands::search::{Content, Image, SearchResult};
use topk_rs::proto::v1::{ctx::file::InputFile, data::Value};

use common::{CliTestContext, OutputJsonExt};

fn cmd() -> Command {
    Command::cargo_bin("topk").unwrap()
}

#[test_context(CliTestContext)]
#[tokio::test]
async fn search_returns_results(ctx: &mut CliTestContext) {
    let dataset = ctx.wrap("test");
    ctx.create_dataset(&dataset);

    let out = cmd()
        .args(["-o", "json", "search", "summarize", "--dataset", &dataset])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let _: Vec<SearchResult> = out.json().unwrap();
}

#[test_context(CliTestContext)]
#[tokio::test]
async fn search_json_output_saves_results_to_output_dir(ctx: &mut CliTestContext) {
    let dataset = ctx.wrap("json-output-dir");
    ctx.create_dataset(&dataset);

    let file = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/markdown.md");
    let out = cmd()
        .args([
            "-o",
            "json",
            "upload",
            file,
            "--dataset",
            &dataset,
            "-y",
            "--wait",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let dir = tempdir().unwrap();
    let out = cmd()
        .args([
            "-o",
            "json",
            "search",
            "Item one",
            "--dataset",
            &dataset,
            "--output-dir",
            dir.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let result: Vec<SearchResult> = out.json().unwrap();
    assert!(!result.is_empty(), "expected search results");

    let saved_files = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();

    assert_eq!(saved_files.len(), result.len());
    for (index, _) in result.iter().enumerate() {
        let ref_id = (index + 1).to_string();
        assert!(
            saved_files
                .iter()
                .any(|path| path.file_stem() == Some(ref_id.as_ref())),
            "missing saved file for ref {ref_id}"
        );
    }
}

#[test_context(CliTestContext)]
#[tokio::test]
async fn search_returns_metadata_fields(ctx: &mut CliTestContext) {
    let dataset = ctx.wrap("meta-fields");
    ctx.create_dataset(&dataset);

    let file = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/markdown.md");
    let input = InputFile::from_path(file).unwrap();
    let upload = ctx
        .client
        .dataset(&dataset)
        .upsert_file(
            "meta-fields-doc",
            input,
            [
                ("title", Value::string("My Test Document")),
                ("author", Value::string("Test Author")),
            ],
        )
        .await
        .unwrap();
    ctx.client
        .dataset(&dataset)
        .wait_for_handle(&upload, None)
        .await
        .unwrap();

    let out = cmd()
        .args([
            "-o",
            "json",
            "search",
            "test",
            "--dataset",
            &dataset,
            "--field",
            "title",
            "--field",
            "author",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "search failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let result: Vec<SearchResult> = out.json().unwrap();
    let doc = result
        .iter()
        .find(|r| r.doc_id == "meta-fields-doc")
        .expect("document not found in search results");

    assert_eq!(
        doc.metadata.get("title").and_then(|v| v.as_string()),
        Some("My Test Document")
    );
    assert_eq!(
        doc.metadata.get("author").and_then(|v| v.as_string()),
        Some("Test Author")
    );
}

#[test]
fn search_result_json_unwraps_metadata_values() {
    let result = topk_rs::proto::v1::ctx::SearchResult {
        doc_id: "doc1".to_string(),
        doc_type: "text/markdown".to_string(),
        doc_name: "doc1.md".to_string(),
        dataset: "sec-10k".to_string(),
        content_id: "chunk-1".to_string(),
        content: Some(topk_rs::proto::v1::ctx::Content {
            data: Some(topk_rs::proto::v1::ctx::content::Data::Chunk(
                topk_rs::proto::v1::ctx::Chunk {
                    text: "hello".to_string(),
                    doc_pages: vec![],
                },
            )),
        }),
        metadata: [
            ("ticker".to_string(), Value::string("AAPL")),
            ("cik".to_string(), Value::i64(320193)),
        ]
        .into_iter()
        .collect(),
    };

    let json_result = SearchResult::try_from(result).unwrap();

    assert_eq!(
        serde_json::to_value(json_result).unwrap(),
        json!({
            "doc_id": "doc1",
            "doc_type": "text/markdown",
            "doc_name": "doc1.md",
            "dataset": "sec-10k",
            "content_id": "chunk-1",
            "content": {
                "text": "hello",
                "doc_pages": []
            },
            "metadata": {
                "ticker": "AAPL",
                "cik": 320193
            }
        })
    );
}

#[test]
fn search_result_json_flattens_chunk_content() {
    let result = SearchResult {
        doc_id: "doc1".to_string(),
        doc_type: "application/pdf".to_string(),
        dataset: "sec-10k".to_string(),
        content_id: "chunk-1".to_string(),
        doc_name: "doc1.pdf".to_string(),
        content: Some(Content::Chunk {
            text: "hello".to_string(),
            doc_pages: vec![170],
        }),
        metadata: BTreeMap::new(),
    };

    assert_eq!(
        serde_json::to_value(result).unwrap(),
        json!({
            "doc_id": "doc1",
            "doc_type": "application/pdf",
            "dataset": "sec-10k",
            "content_id": "chunk-1",
            "doc_name": "doc1.pdf",
            "content": {
                "text": "hello",
                "doc_pages": [170]
            }
        })
    );
}

#[test]
fn search_result_json_encodes_image_bytes_as_base64() {
    let result = SearchResult {
        doc_id: "doc1".to_string(),
        doc_type: "image/png".to_string(),
        dataset: "images".to_string(),
        content_id: "img-1".to_string(),
        doc_name: "doc1.png".to_string(),
        content: Some(Content::Image(Image {
            mime_type: "image/png".to_string(),
            data: bytes::Bytes::from(vec![1, 2, 3]).into(),
        })),
        metadata: BTreeMap::new(),
    };

    assert_eq!(
        serde_json::to_value(result).unwrap(),
        json!({
            "doc_id": "doc1",
            "doc_type": "image/png",
            "dataset": "images",
            "content_id": "img-1",
            "doc_name": "doc1.png",
            "content": {
                "mime_type": "image/png",
                "data": "AQID"
            }
        })
    );
}
