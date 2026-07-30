use std::fmt;
use std::marker::PhantomData;

use serde::de::{Error as DeError, IgnoredAny, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_with::{serde_as, OneOrMany};
use topk_rs::json::Value;
use topk_rs::proto::v1::data::{value, Value as TopkValue};

use super::DocId;
use crate::value::ValueExt;
use crate::Error;

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Query {
    MatchAll(MatchAllQuery),
    Match(FieldClause<MatchValue>),
    MultiMatch(MultiMatch),
    Term(FieldClause<TermValue>),
    Terms(TermsQuery),
    Ids(IdsQuery),
    Prefix(FieldClause<StringValue>),
    Regexp(FieldClause<RegexpValue>),
    Range(FieldClause<RangeBounds>),
    Exists(ExistsQuery),
    Bool(BoolQuery),
    Semantic(SemanticQuery),
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct MatchAllQuery {
    #[serde(default)]
    pub boost: Option<f32>,
}

#[serde_as]
#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct BoolQuery {
    #[serde_as(as = "OneOrMany<_>")]
    #[serde(default)]
    pub must: Vec<Query>,

    #[serde_as(as = "OneOrMany<_>")]
    #[serde(default)]
    pub filter: Vec<GateQuery>,

    #[serde_as(as = "OneOrMany<_>")]
    #[serde(default)]
    pub must_not: Vec<GateQuery>,

    #[serde_as(as = "OneOrMany<_>")]
    #[serde(default)]
    pub should: Vec<Query>,

    #[serde(default)]
    pub boost: Option<f32>,
}

impl BoolQuery {
    pub fn is_empty(&self) -> bool {
        self.must.is_empty()
            && self.filter.is_empty()
            && self.must_not.is_empty()
            && self.should.is_empty()
    }
}

#[derive(Deserialize)]
#[serde(try_from = "Query")]
pub struct GateQuery(pub Query);

impl TryFrom<Query> for GateQuery {
    type Error = Error;

    fn try_from(query: Query) -> Result<Self, Self::Error> {
        fn semantic(query: &Query) -> bool {
            match query {
                Query::Semantic(_) => true,
                Query::Bool(b) => b.must.iter().chain(&b.should).any(semantic),
                _ => false,
            }
        }

        match semantic(&query) {
            false => Ok(GateQuery(query)),
            true => Err(Error::InvalidQuery(
                "\"semantic\" is a scoring clause; it is only valid in a query, \"must\", or \"should\" position".into(),
            )),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticQuery {
    pub field: FieldName,
    pub query: String,

    #[serde(default)]
    pub boost: Option<f32>,
}

/// A `{"field": value}` clause. Read straight off the map — a single-entry
/// `HashMap` never gets built.
pub struct FieldClause<V> {
    pub field: FieldName,
    pub value: V,
}

impl<'de, V: Deserialize<'de>> Deserialize<'de> for FieldClause<V> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(FieldClauseVisitor(PhantomData))
    }
}

struct FieldClauseVisitor<V>(PhantomData<V>);

impl<'de, V: Deserialize<'de>> Visitor<'de> for FieldClauseVisitor<V> {
    type Value = FieldClause<V>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a single \"field\": value clause")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<FieldClause<V>, A::Error> {
        let Some((field, value)) = map.next_entry::<String, V>()? else {
            return Err(DeError::custom("Expected a single \"field\": value clause"));
        };

        if map.next_key::<IgnoredAny>()?.is_some() {
            return Err(DeError::custom("Expected exactly one field in clause"));
        }

        Ok(FieldClause {
            field: FieldName::new(field),
            value,
        })
    }
}

#[derive(Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MatchOperator {
    #[serde(alias = "OR")]
    #[default]
    Or,
    #[serde(alias = "AND")]
    And,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum MatchValue {
    Bare(String),
    Full(MatchValueFull),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatchValueFull {
    pub query: String,

    #[serde(default)]
    pub operator: MatchOperator,

    #[serde(default)]
    pub boost: Option<f32>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MultiMatch {
    pub query: String,

    pub fields: Vec<BoostedField>,

    #[serde(default)]
    pub operator: MatchOperator,

    #[serde(default)]
    pub boost: Option<f32>,
}

#[derive(Deserialize)]
#[serde(try_from = "String")]
pub struct BoostedField {
    pub name: FieldName,
    pub boost: f32,
}

impl TryFrom<String> for BoostedField {
    type Error = Error;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.split_once('^') {
            None => Ok(BoostedField {
                name: FieldName::new(s),
                boost: 1.0,
            }),
            Some((name, boost)) => {
                let boost: f32 = boost
                    .parse()
                    .map_err(|_| Error::InvalidQuery(format!("Invalid field boost in \"{s}\"")))?;
                Ok(BoostedField {
                    name: FieldName::new(name),
                    boost,
                })
            }
        }
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum TermValue {
    Full {
        value: Value,

        #[serde(default)]
        boost: Option<f32>,
    },

    Bare(Value),
}

impl TermValue {
    pub fn into_parts(self) -> (TopkValue, Option<f32>) {
        match self {
            TermValue::Full { value, boost } => (value.into_inner(), boost),
            TermValue::Bare(value) => (value.into_inner(), None),
        }
    }
}

pub struct TermsQuery {
    pub field: FieldName,
    pub values: TopkValue,
    pub boost: Option<f32>,
}

impl<'de> Deserialize<'de> for TermsQuery {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(TermsQueryVisitor)
    }
}

struct TermsQueryVisitor;

impl<'de> Visitor<'de> for TermsQueryVisitor {
    type Value = TermsQuery;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a \"field\": [values] clause with an optional \"boost\"")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<TermsQuery, A::Error> {
        let mut terms: Option<(FieldName, TopkValue)> = None;
        let mut boost = None;

        while let Some(key) = map.next_key::<String>()? {
            if key == "boost" {
                boost = Some(map.next_value()?);
                continue;
            }

            if terms.is_some() {
                return Err(DeError::custom("Terms query must have exactly one field"));
            }

            let values = map.next_value::<Value>()?.into_inner();
            if !matches!(values.value, Some(value::Value::List(_))) {
                return Err(DeError::custom("Terms query values must be an array"));
            }

            terms = Some((FieldName::new(key), values));
        }

        let (field, values) =
            terms.ok_or_else(|| DeError::custom("Terms query missing a field"))?;

        Ok(TermsQuery {
            field,
            values,
            boost,
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdsQuery {
    pub values: Vec<DocId>,

    #[serde(default)]
    pub boost: Option<f32>,
}

#[derive(Deserialize)]
#[serde(remote = "Self", deny_unknown_fields)]
pub struct RangeBounds {
    #[serde(default)]
    pub gte: Option<Value>,

    #[serde(default)]
    pub gt: Option<Value>,

    #[serde(default)]
    pub lte: Option<Value>,

    #[serde(default)]
    pub lt: Option<Value>,

    #[serde(default)]
    pub boost: Option<f32>,

    #[serde(default)]
    pub format: Option<String>,
}

impl<'de> Deserialize<'de> for RangeBounds {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let bounds = Self::deserialize(deserializer)?;

        for (name, bound) in [
            ("gte", &bounds.gte),
            ("gt", &bounds.gt),
            ("lte", &bounds.lte),
            ("lt", &bounds.lt),
        ] {
            if bound.as_ref().is_some_and(|v| !v.is_scalar()) {
                return Err(serde::de::Error::custom(format!(
                    "[range] query does not support a non-scalar value for [{name}]"
                )));
            }
        }

        Ok(bounds)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExistsQuery {
    pub field: FieldName,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum StringValue {
    Bare(String),
    Full(StringValueFull),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StringValueFull {
    value: String,
}

impl StringValue {
    pub fn into_string(self) -> String {
        match self {
            StringValue::Bare(s) => s,
            StringValue::Full(full) => full.value,
        }
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum RegexpValue {
    Bare(String),
    Full(RegexpValueFull),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegexpValueFull {
    value: String,

    #[serde(default)]
    case_insensitive: Option<bool>,
}

impl RegexpValue {
    pub fn case_insensitive(&self) -> bool {
        match self {
            RegexpValue::Bare(_) => false,
            RegexpValue::Full(full) => full.case_insensitive.unwrap_or(false),
        }
    }

    pub fn into_string(self) -> String {
        match self {
            RegexpValue::Bare(s) => s,
            RegexpValue::Full(full) => full.value,
        }
    }
}

#[repr(transparent)]
#[derive(Clone, Debug, Deserialize)]
pub struct FieldName(String);

impl FieldName {
    pub fn new(name: impl Into<String>) -> Self {
        FieldName(name.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.strip_suffix(".keyword").unwrap_or(&self.0)
    }
}

impl From<FieldName> for String {
    fn from(name: FieldName) -> Self {
        name.as_str().to_string()
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[test]
    fn field_clause_reads_the_single_entry() {
        let clause: FieldClause<StringValue> = sonic_rs::from_str(r#"{"title": "a"}"#).unwrap();

        assert_eq!(clause.field.as_str(), "title");
        assert_eq!(clause.value.into_string(), "a");
    }

    #[rstest]
    #[case::empty("{}")]
    #[case::two_fields(r#"{"title": "a", "genre": "b"}"#)]
    fn field_clause_requires_exactly_one_entry(#[case] body: &str) {
        assert!(sonic_rs::from_str::<FieldClause<StringValue>>(body).is_err());
    }

    #[test]
    fn terms_query_reads_field_values_and_boost() {
        let query: TermsQuery =
            sonic_rs::from_str(r#"{"genre": ["a", "b"], "boost": 2.0}"#).unwrap();

        assert_eq!(query.field.as_str(), "genre");
        assert_eq!(query.values, TopkValue::list(vec!["a", "b"]));
        assert_eq!(query.boost, Some(2.0));
    }

    #[rstest]
    #[case::no_field(r#"{"boost": 2.0}"#)]
    #[case::two_fields(r#"{"a": [1], "b": [2]}"#)]
    #[case::scalar_value(r#"{"a": "b"}"#)]
    fn terms_query_rejected(#[case] body: &str) {
        assert!(sonic_rs::from_str::<TermsQuery>(body).is_err());
    }

    #[test]
    fn term_value_carries_its_boost() {
        let bare: TermValue = sonic_rs::from_str("1").unwrap();
        assert_eq!(bare.into_parts(), (TopkValue::i64(1), None));

        let full: TermValue = sonic_rs::from_str(r#"{"value": "a", "boost": 3.0}"#).unwrap();
        assert_eq!(full.into_parts(), (TopkValue::string("a"), Some(3.0)));
    }

    // A sparse vector is an object, and `term` must still read it as a value
    // rather than as the `{value, boost}` form.
    #[test]
    fn term_value_accepts_an_object_value() {
        let bare: TermValue = sonic_rs::from_str(r#"{"0": 1.5}"#).unwrap();

        assert_eq!(
            bare.into_parts(),
            (TopkValue::f32_sparse_vector(vec![0], vec![1.5]), None)
        );
    }

    #[rstest]
    #[case::list(r#"{"gte": [1, 2]}"#)]
    #[case::object(r#"{"lt": {"a": 1}}"#)]
    fn range_bounds_reject_non_scalars(#[case] body: &str) {
        assert!(sonic_rs::from_str::<RangeBounds>(body).is_err());
    }

    #[test]
    fn range_bounds_read_scalars() {
        let bounds: RangeBounds = sonic_rs::from_str(r#"{"gte": 1, "lt": "b"}"#).unwrap();

        assert_eq!(bounds.gte.map(|v| v.into_inner()), Some(TopkValue::i64(1)));
        assert_eq!(
            bounds.lt.map(|v| v.into_inner()),
            Some(TopkValue::string("b"))
        );
    }
}
