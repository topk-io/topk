use axum::response::{IntoResponse, Response};
use http::header::{HeaderValue, CONTENT_TYPE};
use http::StatusCode;
use serde::Serialize;

const CONTENT_TYPE_JSON: HeaderValue = HeaderValue::from_static("application/json");

/// A JSON response body, serialized with `sonic_rs` — the counterpart to the
/// `sonic_rs`-backed request extractors, and a drop-in for `axum::Json` on the
/// response side.
pub struct Json<T>(pub T);

impl<T: Serialize> IntoResponse for Json<T> {
    fn into_response(self) -> Response {
        match sonic_rs::to_vec(&self.0) {
            Ok(body) => ([(CONTENT_TYPE, CONTENT_TYPE_JSON)], body).into_response(),
            // Serialization only fails on values JSON cannot express (a NaN
            // score, say). The fallback is written by hand because rendering the
            // error as JSON is exactly what just failed.
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(CONTENT_TYPE, CONTENT_TYPE_JSON)],
                format!(
                    r#"{{"error":{{"type":"internal_server_error","reason":{}}},"status":500}}"#,
                    escape(&e.to_string())
                ),
            )
                .into_response(),
        }
    }
}

fn escape(reason: &str) -> String {
    sonic_rs::to_string(reason).unwrap_or_else(|_| "\"Internal error\"".to_string())
}

#[cfg(test)]
mod tests {
    use topk_rs::proto::v1::data::Value as TopkValue;

    use crate::api::{DocId, Hit, IndexName, SearchResponse, Source};
    use crate::Error;

    use super::*;

    fn index() -> IndexName {
        IndexName::try_from("books".to_string()).unwrap()
    }

    fn hit(id: &str, score: f32) -> Hit {
        Hit {
            id: DocId::try_from(id.to_string()).unwrap(),
            score: Some(score),
            sort: None,
            source: Some(Source(
                TopkValue::r#struct([("title", TopkValue::string("a"))]).into(),
            )),
        }
    }

    // `_index` is written per hit even though it is stored once, and `Hit`'s
    // fields are flattened alongside it.
    #[test]
    fn search_response_carries_the_index_on_every_hit() {
        let response =
            SearchResponse::new(&index(), vec![hit("1", 1.5), hit("2", 0.5)], None, &[2]);

        assert_eq!(
            sonic_rs::to_string(&response).unwrap(),
            r#"{"took":1,"timed_out":false,"_shards":{"total":1,"successful":1,"failed":0},"hits":{"total":{"value":2,"relation":"eq"},"max_score":1.5,"hits":[{"_index":"books","_id":"1","_score":1.5,"_source":{"title":"a"}},{"_index":"books","_id":"2","_score":0.5,"_source":{"title":"a"}}]}}"#
        );
    }

    #[test]
    fn empty_response_reports_a_null_max_score() {
        let response = SearchResponse::new(&index(), vec![], None, &[]);
        let body = sonic_rs::to_string(&response).unwrap();

        assert!(body.contains(r#""max_score":null"#), "{body}");
        assert!(body.contains(r#""hits":[]"#), "{body}");
    }

    #[test]
    fn error_responses_serialize() {
        let response = Error::IndexNotFound("no such index [books]".into()).into_response();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            response
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|value| value.to_str().ok()),
            Some("application/json")
        );
    }
}
