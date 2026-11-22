use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use topk_rs::proto::v1::control::field_type::DataType;
use topk_rs::proto::v1::control::field_type_list::ListValueType;
use topk_rs::proto::v1::control::FieldSpec;
use topk_rs::proto::v1::data::{value::Value as Inner, Value};

use crate::import::error::Error;

/// The value of a `--set` field: a field, quoted text, `join(list, separator)` or `concat(a, b, …)`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum Expr {
    Field(String),
    Text(String),
    Join(Box<Expr>, Box<Expr>),
    Concat(Vec<Expr>),
}

impl Expr {
    /// One expression off the front of `source`, and what follows it.
    fn parse(source: &str) -> Result<(Self, &str), Error> {
        let source = source.trim_start();
        if let Some(quote @ ('"' | '\'')) = source.chars().next() {
            return match source[1..].split_once(quote) {
                Some((text, rest)) => Ok((Expr::Text(text.to_string()), rest)),
                None => Err(Error::InvalidArgument(format!(
                    "{source} has no closing {quote}"
                ))),
            };
        }
        let end = source
            .find(|c: char| !(c.is_alphanumeric() || matches!(c, '_' | '.')))
            .unwrap_or(source.len());
        let (name, rest) = source.split_at(end);
        if name.is_empty() {
            return Err(Error::InvalidArgument(match rest.chars().next() {
                Some(c) => format!("unexpected {c:?} at {rest:?}; text goes in quotes"),
                None => "the expression ends early".to_string(),
            }));
        }
        let mut rest = match rest.trim_start().strip_prefix('(') {
            Some(arguments) => arguments,
            None => return Ok((Expr::Field(name.to_string()), rest)),
        };
        let mut arguments = Vec::new();
        loop {
            let (argument, after) = Expr::parse(rest)?;
            arguments.push(argument);
            let after = after.trim_start();
            rest = match (after.strip_prefix(','), after.strip_prefix(')')) {
                (Some(next), _) => next,
                (_, Some(done)) => return Ok((Expr::call(name, arguments)?, done)),
                _ => {
                    return Err(Error::InvalidArgument(format!(
                        "expected , or ) in {name}(…) at {after:?}"
                    )))
                }
            };
        }
    }

    fn call(name: &str, arguments: Vec<Expr>) -> Result<Self, Error> {
        match name {
            "join" => match <[Expr; 2]>::try_from(arguments) {
                Ok([list, separator]) => Ok(Expr::Join(Box::new(list), Box::new(separator))),
                Err(arguments) => Err(Error::InvalidArgument(format!(
                    "join takes a list and a separator, not {} arguments",
                    arguments.len()
                ))),
            },
            "concat" => Ok(Expr::Concat(arguments)),
            _ => Err(Error::InvalidArgument(format!(
                "unknown function {name}; the functions are join(list, separator) and concat(a, b, …)"
            ))),
        }
    }

    pub fn fields(&self) -> Vec<&str> {
        match self {
            Expr::Field(name) => vec![name],
            Expr::Text(_) => Vec::new(),
            Expr::Join(list, separator) => [list.fields(), separator.fields()].concat(),
            Expr::Concat(parts) => parts.iter().flat_map(Expr::fields).collect(),
        }
    }

    /// The type of the value, checked against the types of the fields it reads where
    /// `field` knows them.
    pub fn spec<'a>(
        &self,
        field: &impl Fn(&str) -> Option<&'a FieldSpec>,
    ) -> Result<FieldSpec, Error> {
        let scalar = |part: &Expr, ints: bool| {
            let spec = part.spec(field)?;
            match spec.data_type.and_then(|ty| ty.data_type) {
                Some(DataType::Text(_)) => Ok(()),
                Some(DataType::Integer(_)) if ints => Ok(()),
                _ => Err(Error::InvalidArgument(format!(
                    "{self} takes text{} here; {part} is not",
                    if ints { " and int" } else { "" }
                ))),
            }
        };
        match self {
            Expr::Field(name) => Ok(
                field(name).map_or(FieldSpec::text(false), |spec| FieldSpec {
                    required: false,
                    ..spec.clone()
                }),
            ),
            Expr::Text(_) => Ok(FieldSpec::text(false)),
            Expr::Join(list, separator) => {
                let known = match list.as_ref() {
                    Expr::Field(name) => field(name),
                    _ => None,
                };
                let listed =
                    match known.and_then(|spec| spec.data_type.as_ref()?.data_type.as_ref()) {
                        Some(DataType::List(list)) => list.value_type() == ListValueType::String,
                        Some(_) => false,
                        None => matches!(list.as_ref(), Expr::Field(_)),
                    };
                if !listed {
                    return Err(Error::InvalidArgument(format!(
                        "{self} takes a field holding a text list; {list} is not one"
                    )));
                }
                scalar(separator, false)?;
                Ok(FieldSpec::text(false))
            }
            Expr::Concat(parts) => {
                for part in parts {
                    scalar(part, true)?;
                }
                Ok(FieldSpec::text(false))
            }
        }
    }

    /// `None` when the value is null or empty: the document then has no such field.
    pub fn eval<'a>(
        &self,
        field: &impl Fn(&str) -> Option<&'a Value>,
    ) -> Result<Option<Value>, Error> {
        match self {
            Expr::Field(name) => Ok(field(name)
                .filter(|value| value.as_null().is_none())
                .cloned()),
            Expr::Text(text) => Ok(Some(Value::string(text.clone()))),
            Expr::Join(list, separator) => {
                let (list, separator) = match (list.eval(field)?, separator.eval(field)?) {
                    (Some(list), Some(separator)) => (list, separator),
                    _ => return Ok(None),
                };
                let joined = match (list.as_string_list(), separator.as_string()) {
                    (Some(items), Some(separator)) => items.join(separator),
                    _ => {
                        return Err(Error::InvalidArgument(format!(
                            "{self} takes a text list and a text separator"
                        )))
                    }
                };
                Ok((!joined.is_empty()).then(|| Value::string(joined)))
            }
            Expr::Concat(parts) => {
                // Text alone is a value; with fields, at least one has to be there.
                let mut found = parts.iter().all(|part| matches!(part, Expr::Text(_)));
                let mut text = String::new();
                for part in parts {
                    let value = match part.eval(field)? {
                        Some(value) => value,
                        None => continue,
                    };
                    found |= !matches!(part, Expr::Text(_));
                    match value.value {
                        Some(Inner::String(part)) => text.push_str(&part),
                        Some(Inner::I32(n)) => text.push_str(&n.to_string()),
                        Some(Inner::I64(n)) => text.push_str(&n.to_string()),
                        Some(Inner::U32(n)) => text.push_str(&n.to_string()),
                        Some(Inner::U64(n)) => text.push_str(&n.to_string()),
                        _ => {
                            return Err(Error::InvalidArgument(format!(
                                "{self} takes text and int; {part} is neither"
                            )))
                        }
                    }
                }
                Ok((found && !text.is_empty()).then(|| Value::string(text)))
            }
        }
    }
}

impl FromStr for Expr {
    type Err = Error;

    fn from_str(source: &str) -> Result<Self, Error> {
        let (expr, rest) = Expr::parse(source)?;
        match rest.trim_start() {
            "" => Ok(expr),
            rest => Err(Error::InvalidArgument(format!(
                "unexpected {rest:?} after {expr}"
            ))),
        }
    }
}

impl TryFrom<String> for Expr {
    type Error = Error;

    fn try_from(source: String) -> Result<Self, Error> {
        source.parse()
    }
}

impl From<Expr> for String {
    fn from(expr: Expr) -> String {
        expr.to_string()
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Field(name) => f.write_str(name),
            Expr::Text(text) if text.contains('"') => write!(f, "'{text}'"),
            Expr::Text(text) => write!(f, "\"{text}\""),
            Expr::Join(list, separator) => write!(f, "join({list}, {separator})"),
            Expr::Concat(parts) => {
                f.write_str("concat(")?;
                for (index, part) in parts.iter().enumerate() {
                    if index > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{part}")?;
                }
                f.write_str(")")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    fn field(name: &str) -> Expr {
        Expr::Field(name.into())
    }

    fn text(text: &str) -> Expr {
        Expr::Text(text.into())
    }

    #[rstest]
    #[case::field("paper", field("paper"))]
    #[case::struct_field("image.path", field("image.path"))]
    #[case::join(
        r#"join(authors, "; ")"#,
        Expr::Join(Box::new(field("authors")), Box::new(text("; ")))
    )]
    #[case::single_quotes("concat(paper, '-p', page)", Expr::Concat(vec![field("paper"), text("-p"), field("page")]))]
    #[case::nested(
        r#" concat( title , ": ", join(authors,", ") ) "#,
        Expr::Concat(vec![field("title"), text(": "), Expr::Join(Box::new(field("authors")), Box::new(text(", ")))])
    )]
    #[case::quote_in_text(r#"concat(a, '"')"#, Expr::Concat(vec![field("a"), text("\"")]))]
    fn parses_and_prints_back(#[case] source: &str, #[case] expected: Expr) {
        let parsed: Expr = source.parse().unwrap();
        assert_eq!(parsed, expected);
        assert_eq!(parsed.to_string().parse::<Expr>().unwrap(), expected);
    }

    #[rstest]
    #[case::unknown_function(r#"joinn(authors, "; ")"#, "unknown function joinn")]
    #[case::join_arity("join(authors)", "not 1 arguments")]
    #[case::unquoted_text("concat(paper, -p, page)", "text goes in quotes")]
    #[case::sql_operator("paper || page", "unexpected \"|| page\"")]
    #[case::trailing(r#"join(authors, "; ") x"#, "unexpected \"x\"")]
    #[case::open_quote(r#"concat(paper, "-p)"#, "no closing \"")]
    #[case::open_call("concat(paper", "expected , or )")]
    #[case::empty_call("concat()", "unexpected ')'")]
    #[case::empty("", "ends early")]
    fn rejects(#[case] source: &str, #[case] message: &str) {
        let error = source.parse::<Expr>().unwrap_err().to_string();
        assert!(error.contains(message), "{error}");
    }

    fn row() -> Vec<(String, Value)> {
        vec![
            ("paper".into(), Value::string("2401.00001")),
            ("page".into(), Value::i64(3)),
            ("title".into(), Value::string("Sector Rotation")),
            ("abstract".into(), Value::null()),
            (
                "authors".into(),
                Value::list(vec!["Runjia Yang".to_string(), "Beining Shi".to_string()]),
            ),
            ("nobody".into(), Value::list(Vec::<String>::new())),
            ("ink".into(), Value::f64(0.5)),
        ]
    }

    #[rstest]
    #[case::copy("paper", Some("2401.00001"))]
    #[case::null_field("abstract", None)]
    #[case::join(r#"join(authors, "; ")"#, Some("Runjia Yang; Beining Shi"))]
    #[case::empty_list(r#"join(nobody, "; ")"#, None)]
    #[case::missing_list(r#"join(editors, "; ")"#, None)]
    #[case::text_and_int(r#"concat(paper, "-p", page)"#, Some("2401.00001-p3"))]
    #[case::null_part_skipped(r#"concat(title, ". ", abstract)"#, Some("Sector Rotation. "))]
    #[case::no_field_present(r#"concat(abstract, ". ", summary)"#, None)]
    #[case::text_only(r#"concat("a", "b")"#, Some("ab"))]
    #[case::nested(
        r#"concat(title, ": ", join(authors, ", "))"#,
        Some("Sector Rotation: Runjia Yang, Beining Shi")
    )]
    fn evaluates(#[case] source: &str, #[case] expected: Option<&str>) {
        let row = row();
        let value = source
            .parse::<Expr>()
            .unwrap()
            .eval(&|name| {
                row.iter()
                    .find(|(key, _)| key == name)
                    .map(|(_, value)| value)
            })
            .unwrap();
        assert_eq!(value.as_ref().and_then(Value::as_string), expected);
    }

    #[rstest]
    #[case::join_of_text(r#"join(title, "; ")"#)]
    #[case::concat_of_list("concat(paper, authors)")]
    #[case::concat_of_float("concat(paper, ink)")]
    fn wrong_types_fail_the_row_and_the_plan(#[case] source: &str) {
        let row = row();
        let specs = [
            ("paper", FieldSpec::text(false)),
            ("title", FieldSpec::text(false)),
            ("ink", FieldSpec::float(false)),
            ("authors", FieldSpec::list(false, ListValueType::String)),
        ];
        let expr: Expr = source.parse().unwrap();
        assert!(expr
            .eval(&|name| row
                .iter()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value))
            .is_err());
        assert!(expr
            .spec(&|name| specs
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, spec)| spec))
            .is_err());
    }

    #[test]
    fn a_copy_keeps_its_type() {
        let authors = FieldSpec::list(true, ListValueType::String);
        let spec = field("authors").spec(&|_| Some(&authors)).unwrap();
        assert_eq!(spec, FieldSpec::list(false, ListValueType::String));
        let joined: Expr = r#"join(authors, "; ")"#.parse().unwrap();
        assert_eq!(
            joined.spec(&|_| Some(&authors)).unwrap(),
            FieldSpec::text(false)
        );
    }
}
