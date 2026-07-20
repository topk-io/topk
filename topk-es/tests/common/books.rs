use std::ops::Deref;

use serde_json::{json, Value};
use test_context::AsyncTestContext;

use super::TestScope;

pub struct BooksContext {
    scope: TestScope,
}

impl Deref for BooksContext {
    type Target = TestScope;

    fn deref(&self) -> &TestScope {
        &self.scope
    }
}

fn embedding(i: usize, dims: usize) -> Vec<f32> {
    let mut v = vec![0.0; dims];
    v[i] = 1.0;
    v
}

impl BooksContext {
    fn mapping() -> Value {
        json!({
            "mappings": {
                "properties": {
                    "title": { "type": "text" },
                    "author": { "type": "keyword" },
                    "published_year": { "type": "integer" },
                    "rating": { "type": "float" },
                    "genre": { "type": "keyword" },
                    "in_print": { "type": "boolean" },
                    "embedding": { "type": "dense_vector", "dims": 16, "similarity": "cosine" },
                    "token_embeddings": { "type": "rank_vectors", "dims": 4 }
                }
            }
        })
    }

    fn books() -> Vec<(&'static str, Value)> {
        vec![
            (
                "mockingbird",
                json!({
                    "title": "To Kill a Mockingbird",
                    "author": "Lee",
                    "published_year": 1960,
                    "rating": 4.3,
                    "genre": "fiction",
                    "in_print": true,
                    "tags": ["classic", "american", "fiction"],
                    "embedding": embedding(0, 16),
                    "token_embeddings": [[0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0]],
                }),
            ),
            (
                "nineteen_eighty_four",
                json!({
                    "title": "1984",
                    "author": "Orwell",
                    "published_year": 1949,
                    "rating": 4.2,
                    "genre": "dystopian",
                    "in_print": true,
                    "tags": ["dystopia", "classic", "political"],
                    "embedding": embedding(1, 16),
                    "token_embeddings": [[0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]],
                }),
            ),
            (
                "pride",
                json!({
                    "title": "Pride and Prejudice",
                    "author": "Austen",
                    "published_year": 1813,
                    "rating": 4.3,
                    "genre": "romance",
                    "in_print": true,
                    "tags": ["romance", "classic", "british"],
                    "embedding": embedding(2, 16),
                    "token_embeddings": [[0.0, 0.0, 0.0, 1.0], [0.0, 1.0, 0.0, 0.0]],
                }),
            ),
            (
                "gatsby",
                json!({
                    "title": "The Great Gatsby",
                    "author": "Fitzgerald",
                    "published_year": 1925,
                    "rating": 3.9,
                    "genre": "fiction",
                    "in_print": true,
                    "tags": ["classic", "american", "jazz"],
                    "embedding": embedding(3, 16),
                    "token_embeddings": [[0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0]],
                }),
            ),
            (
                "catcher",
                json!({
                    "title": "The Catcher in the Rye",
                    "author": "Salinger",
                    "published_year": 1951,
                    "rating": 3.8,
                    "genre": "fiction",
                    "in_print": false,
                    "tags": ["coming_of_age", "american", "fiction"],
                    "embedding": embedding(4, 16),
                    "token_embeddings": [[0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0]],
                }),
            ),
            (
                "hobbit",
                json!({
                    "title": "The Hobbit",
                    "author": "Tolkien",
                    "published_year": 1937,
                    "rating": 4.3,
                    "genre": "fantasy",
                    "in_print": true,
                    "tags": ["fantasy", "adventure", "tolkien"],
                    "embedding": embedding(5, 16),
                    "token_embeddings": [[0.5, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0]],
                }),
            ),
            (
                "lotr",
                json!({
                    "title": "The Lord of the Rings",
                    "author": "Tolkien",
                    "published_year": 1954,
                    "rating": 4.5,
                    "genre": "fantasy",
                    "in_print": true,
                    "tags": ["fantasy", "epic", "tolkien"],
                    "embedding": embedding(6, 16),
                    "token_embeddings": [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0]],
                }),
            ),
            (
                "harry",
                json!({
                    "title": "Harry Potter and the Sorcerer's Stone",
                    "author": "Rowling",
                    "published_year": 1997,
                    "rating": 4.5,
                    "genre": "fantasy",
                    "in_print": true,
                    "tags": ["fantasy", "magic", "children"],
                    "embedding": embedding(7, 16),
                    "token_embeddings": [[0.25, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0]],
                }),
            ),
            (
                "alchemist",
                json!({
                    "title": "The Alchemist",
                    "author": "Coelho",
                    "published_year": 1988,
                    "rating": 3.9,
                    "genre": "fiction",
                    "in_print": true,
                    "tags": ["philosophy", "fiction", "spiritual"],
                    "embedding": embedding(8, 16),
                    "token_embeddings": [[0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0]],
                }),
            ),
            (
                "moby",
                json!({
                    "title": "Moby Dick",
                    "author": "Melville",
                    "published_year": 1851,
                    "rating": 3.5,
                    "genre": "adventure",
                    "in_print": false,
                    "tags": ["classic", "adventure", "sea"],
                    "embedding": embedding(9, 16),
                    "token_embeddings": [[0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]],
                }),
            ),
        ]
    }
}

impl AsyncTestContext for BooksContext {
    async fn setup() -> Self {
        let scope = TestScope::setup().await;
        scope.create_with_mapping(Self::mapping()).await;

        let res = scope.index_docs(Self::books()).await;
        assert!(res.status.is_success());

        Self { scope }
    }

    async fn teardown(self) {
        self.scope.teardown().await;
    }
}
