use std::collections::HashMap;

use serde::{Deserialize, Deserializer};
use serde_with::{serde_as, OneOrMany};
use topk_rs::json::Value;

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

#[derive(Deserialize)]
#[serde(try_from = "HashMap<String, V>")]
pub struct FieldClause<V> {
    pub field: FieldName,
    pub value: V,
}

impl<V> TryFrom<HashMap<String, V>> for FieldClause<V> {
    type Error = Error;

    fn try_from(map: HashMap<String, V>) -> Result<Self, Self::Error> {
        let mut iter = map.into_iter();
        let (field, value) = iter.next().ok_or_else(|| {
            Error::InvalidQuery("Expected a single \"field\": value clause".into())
        })?;
        if iter.next().is_some() {
            return Err(Error::InvalidQuery(
                "Expected exactly one field in clause".into(),
            ));
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
    pub fn value(&self) -> topk_rs::proto::v1::data::Value {
        match self {
            TermValue::Full { value, .. } => value.0.clone(),
            TermValue::Bare(value) => value.0.clone(),
        }
    }
}

#[derive(Deserialize)]
#[serde(try_from = "TermsQueryWire")]
pub struct TermsQuery {
    pub field: FieldName,
    pub values: topk_rs::proto::v1::data::Value,
    pub boost: Option<f32>,
}

#[derive(Deserialize)]
struct TermsQueryWire {
    #[serde(default)]
    boost: Option<f32>,

    #[serde(flatten)]
    fields: HashMap<String, Vec<serde_json::Value>>,
}

impl TryFrom<TermsQueryWire> for TermsQuery {
    type Error = Error;

    fn try_from(wire: TermsQueryWire) -> Result<Self, Self::Error> {
        let mut fields = wire.fields.into_iter();
        let (field, values) = fields
            .next()
            .ok_or_else(|| Error::InvalidQuery("Terms query missing a field".into()))?;
        if fields.next().is_some() {
            return Err(Error::InvalidQuery(
                "Terms query must have exactly one field".into(),
            ));
        }
        Ok(TermsQuery {
            field: FieldName::new(field),
            values: topk_rs::proto::v1::data::Value::try_from(values)
                .map_err(|e| Error::InvalidQuery(e.to_string()))?,
            boost: wire.boost,
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

// ES models a range as one bound per side, so a later `lt` overwrites an earlier `lte` in the
// same clause rather than intersecting with it.
pub struct RangeBounds {
    pub lower: Option<Bound>,
    pub upper: Option<Bound>,
    pub boost: Option<f32>,
    #[allow(dead_code)]
    pub format: Option<String>,

    // A zone-less bound is interpreted in this zone; a bound carrying its own offset ignores it.
    pub time_zone: Option<String>,
}

pub struct Bound {
    pub value: Value,
    pub inclusive: bool,
}

impl<'de> Deserialize<'de> for RangeBounds {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct BoundsVisitor;

        impl<'de> serde::de::Visitor<'de> for BoundsVisitor {
            type Value = RangeBounds;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("range bounds")
            }

            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<RangeBounds, A::Error> {
                let mut bounds = RangeBounds {
                    lower: None,
                    upper: None,
                    boost: None,
                    format: None,
                    time_zone: None,
                };

                while let Some(key) = map.next_key::<String>()? {
                    let bound = |value: Value, inclusive| {
                        match value.is_scalar() {
                            true => Ok(Bound { value, inclusive }),
                            false => Err(serde::de::Error::custom(format!(
                                "[range] query does not support a non-scalar value for [{key}]"
                            ))),
                        }
                    };
                    match key.as_str() {
                        "gte" => bounds.lower = Some(bound(map.next_value()?, true)?),
                        "gt" => bounds.lower = Some(bound(map.next_value()?, false)?),
                        "lte" => bounds.upper = Some(bound(map.next_value()?, true)?),
                        "lt" => bounds.upper = Some(bound(map.next_value()?, false)?),
                        "boost" => bounds.boost = map.next_value()?,
                        "format" => bounds.format = map.next_value()?,
                        "time_zone" => bounds.time_zone = map.next_value()?,
                        unknown => {
                            return Err(serde::de::Error::custom(format!(
                                "[range] query does not support [{unknown}]"
                            )))
                        }
                    }
                }

                Ok(bounds)
            }
        }

        deserializer.deserialize_map(BoundsVisitor)
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

impl From<&StringValue> for String {
    fn from(value: &StringValue) -> Self {
        match value {
            StringValue::Bare(s) => s.clone(),
            StringValue::Full(full) => full.value.clone(),
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
}

impl From<&RegexpValue> for String {
    fn from(value: &RegexpValue) -> Self {
        match value {
            RegexpValue::Bare(s) => s.clone(),
            RegexpValue::Full(full) => full.value.clone(),
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
