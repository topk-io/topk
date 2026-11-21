use std::ops::Range;

use super::Error;

#[derive(Debug, Clone, PartialEq)]
pub struct Clause {
    pub span: Range<usize>,
    pub negated: bool,
    pub field: Option<String>,
    pub value: Value,
    pub boost: Option<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Word(String),
    Phrase(String),
    /// The pattern and its span in the input, without the slashes.
    Regex(String, Range<usize>),
    Any(Vec<String>),
    Range {
        lo: Option<Bound>,
        hi: Option<Bound>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Bound {
    pub value: String,
    pub inclusive: bool,
}

/// Groups joined by implicit AND; the clauses of a group are joined by `OR`.
pub fn parse(input: &str) -> Result<Vec<Vec<Clause>>, Error> {
    let mut p = Parser { s: input, pos: 0 };
    let mut groups: Vec<Vec<Clause>> = vec![];
    let mut or = false;
    loop {
        p.skip_ws();
        if p.eof() {
            break;
        }
        let clause = p.clause()?;
        match groups.last_mut() {
            Some(group) if or => group.push(clause),
            _ => groups.push(vec![clause]),
        }
        p.skip_ws();
        or = p.keyword("OR");
        if or {
            p.skip_ws();
            if p.eof() {
                return Err(Error::new(p.pos, "OR needs a clause on both sides"));
            }
        }
    }
    Ok(groups)
}

struct Parser<'a> {
    s: &'a str,
    pos: usize,
}

impl Parser<'_> {
    fn clause(&mut self) -> Result<Clause, Error> {
        let start = self.pos;
        let negated = self.eat('-');
        let field = self.field();
        let value = match (&field, self.peek()) {
            (Some(_), _) => self.value()?,
            (None, Some(q @ ('"' | '\''))) => Value::Phrase(self.quoted(q)?),
            (None, Some('/')) => {
                return Err(Error::new(self.pos, "a regex needs a field").hint("e.g. grep:/…/"))
            }
            (None, _) => Value::Word(self.word()),
        };
        let boost = match field.is_some() && self.eat('^') {
            true => {
                let at = self.pos;
                let word = self.word();
                Some(
                    word.parse::<f32>()
                        .map_err(|_| Error::new(at, format!("invalid boost `{word}`")))?,
                )
            }
            false => None,
        };
        if let Value::Word(w) = &value {
            if w.is_empty() {
                return Err(Error::new(self.pos, "expected a value"));
            }
        }
        Ok(Clause {
            span: start..self.pos,
            negated,
            field,
            value,
            boost,
        })
    }

    /// `name:` followed by a value; restores the position if the input is not a field prefix.
    fn field(&mut self) -> Option<String> {
        let start = self.pos;
        let len = self
            .rest()
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '.'))?;
        let rest = &self.rest()[len..];
        let after = rest.chars().nth(1);
        if len == 0
            || !rest.starts_with(':')
            || rest.starts_with("://")
            || after.is_none_or(char::is_whitespace)
        {
            return None;
        }
        self.pos += len + 1;
        Some(self.s[start..start + len].to_string())
    }

    fn value(&mut self) -> Result<Value, Error> {
        match self.peek() {
            Some(q @ ('"' | '\'')) => Ok(Value::Phrase(self.quoted(q)?)),
            Some('/') => {
                let start = self.pos + 1;
                let pattern = self.delimited('/')?;
                Ok(Value::Regex(pattern, start..self.pos - 1))
            }
            Some('(') => {
                self.pos += 1;
                let mut values = vec![];
                loop {
                    self.skip_ws();
                    values.push(match self.peek() {
                        Some(q @ ('"' | '\'')) => self.quoted(q)?,
                        _ => self.word(),
                    });
                    self.skip_ws();
                    if self.eat(')') {
                        break;
                    }
                    if !self.keyword("OR") {
                        return Err(Error::new(self.pos, "expected OR or `)`"));
                    }
                }
                if values.iter().any(String::is_empty) {
                    return Err(Error::new(self.pos, "empty value in group"));
                }
                Ok(Value::Any(values))
            }
            Some('>' | '<') => {
                let greater = self.eat('>') || !self.eat('<');
                let inclusive = self.eat('=');
                let bound = Some(Bound {
                    value: self.word(),
                    inclusive,
                });
                Ok(match greater {
                    true => Value::Range {
                        lo: bound,
                        hi: None,
                    },
                    false => Value::Range {
                        lo: None,
                        hi: bound,
                    },
                })
            }
            _ => {
                let word = self.word();
                Ok(match word.split_once("..") {
                    Some((lo, hi)) => {
                        let bound = |v: &str| {
                            (!v.is_empty()).then(|| Bound {
                                value: v.to_string(),
                                inclusive: true,
                            })
                        };
                        Value::Range {
                            lo: bound(lo),
                            hi: bound(hi),
                        }
                    }
                    None => Value::Word(word),
                })
            }
        }
    }

    fn quoted(&mut self, q: char) -> Result<String, Error> {
        let text = self.delimited(q)?;
        Ok(text.replace(&format!("\\{q}"), &q.to_string()).replace("\\\\", "\\"))
    }

    /// Reads `<d>…<d>`, where `\<d>` does not close it; returns the text between the delimiters.
    fn delimited(&mut self, d: char) -> Result<String, Error> {
        let (s, start) = (self.s, self.pos);
        self.pos += 1;
        let mut escaped = false;
        for (i, c) in s[self.pos..].char_indices() {
            match (escaped, c) {
                (false, '\\') => escaped = true,
                (false, c) if c == d => {
                    let text = self.s[start + 1..self.pos + i].to_string();
                    self.pos += i + 1;
                    return Ok(text);
                }
                _ => escaped = false,
            }
        }
        Err(Error::new(start, format!("unterminated `{d}`")))
    }

    fn word(&mut self) -> String {
        let len = self
            .rest()
            .find(|c: char| c.is_whitespace() || matches!(c, '(' | ')' | '"' | '^'))
            .unwrap_or(self.rest().len());
        let word = self.s[self.pos..self.pos + len].to_string();
        self.pos += len;
        word
    }

    fn keyword(&mut self, kw: &str) -> bool {
        let rest = self.rest();
        let ok = rest.starts_with(kw)
            && rest[kw.len()..]
                .chars()
                .next()
                .is_none_or(char::is_whitespace);
        if ok {
            self.pos += kw.len();
        }
        ok
    }

    fn eat(&mut self, c: char) -> bool {
        let ok = self.peek() == Some(c);
        if ok {
            self.pos += c.len_utf8();
        }
        ok
    }

    fn skip_ws(&mut self) {
        self.pos += self.rest().len() - self.rest().trim_start().len();
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn rest(&self) -> &str {
        &self.s[self.pos..]
    }

    fn eof(&self) -> bool {
        self.pos >= self.s.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    fn one(input: &str) -> Clause {
        let mut groups = parse(input).unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 1);
        groups.remove(0).remove(0)
    }

    #[rstest]
    #[case::bare_word("diffusion", None, Value::Word("diffusion".into()))]
    #[case::bare_phrase("\"diffusion model\"", None, Value::Phrase("diffusion model".into()))]
    #[case::field_phrase("image:\"robot arm\"", Some("image"), Value::Phrase("robot arm".into()))]
    #[case::literal_with_regex_chars("title:\"*?(sdf\"", Some("title"), Value::Phrase("*?(sdf".into()))]
    #[case::escaped_quote("text:\"a \\\"b\\\"\"", Some("text"), Value::Phrase("a \"b\"".into()))]
    #[case::regex("grep:/github\\.com\\/x/", Some("grep"), Value::Regex("github\\.com\\/x".into(), 6..20))]
    #[case::group("cat:(cs.LG OR cs.CV)", Some("cat"), Value::Any(vec!["cs.LG".into(), "cs.CV".into()]))]
    #[case::open_range("year:2023..", Some("year"), Value::Range { lo: Some(Bound { value: "2023".into(), inclusive: true }), hi: None })]
    #[case::closed_range("year:2020..2023", Some("year"), Value::Range { lo: Some(Bound { value: "2020".into(), inclusive: true }), hi: Some(Bound { value: "2023".into(), inclusive: true }) })]
    #[case::greater("created:>2024-06", Some("created"), Value::Range { lo: Some(Bound { value: "2024-06".into(), inclusive: false }), hi: None })]
    #[case::at_most("page:<=3", Some("page"), Value::Range { lo: None, hi: Some(Bound { value: "3".into(), inclusive: true }) })]
    #[case::dotted_word("cites:1706.03762", Some("cites"), Value::Word("1706.03762".into()))]
    #[case::url_is_a_word("http://x.org", None, Value::Word("http://x.org".into()))]
    #[case::single_quoted("image:'robot arm'", Some("image"), Value::Phrase("robot arm".into()))]
    #[case::bare_single_quoted("'diffusion model'", None, Value::Phrase("diffusion model".into()))]
    #[case::escaped_single_quote("text:'don\\'t stop'", Some("text"), Value::Phrase("don't stop".into()))]
    #[case::double_quote_in_single("title:'say \"hi\"'", Some("title"), Value::Phrase("say \"hi\"".into()))]
    #[case::apostrophe_in_word("don't", None, Value::Word("don't".into()))]
    #[case::single_quoted_group("author:('van der berg' OR lecun)", Some("author"), Value::Any(vec!["van der berg".into(), "lecun".into()]))]
    fn test_clause(#[case] input: &str, #[case] field: Option<&str>, #[case] value: Value) {
        let clause = one(input);
        assert_eq!(clause.field.as_deref(), field);
        assert_eq!(clause.value, value);
        assert_eq!(clause.span, 0..input.len());
    }

    #[test]
    fn test_negation_and_boost() {
        let clause = one("-cat:cs.CV");
        assert!(clause.negated);
        assert_eq!(one("text:\"capsules\"^2").boost, Some(2.0));
    }

    #[test]
    fn test_groups() {
        let groups =
            parse("\"diffusion model\" -cat:cs.CV title:/survey/ OR author:hinton").unwrap();
        let sizes: Vec<usize> = groups.iter().map(Vec::len).collect();
        assert_eq!(sizes, vec![1, 1, 2]);
        assert_eq!(groups[1][0].span, 18..28);
    }

    #[rstest]
    #[case::unterminated_quote("text:\"abc", 5, "unterminated `\"`")]
    #[case::unterminated_single_quote("text:'abc", 5, "unterminated `'`")]
    #[case::unterminated_regex("grep:/abc", 5, "unterminated `/`")]
    #[case::bare_regex("/abc/", 0, "a regex needs a field")]
    #[case::dangling_or("cat:x OR", 8, "OR needs a clause on both sides")]
    #[case::bad_group("cat:(a b)", 7, "expected OR or `)`")]
    #[case::bad_boost("text:x^y", 7, "invalid boost `y`")]
    fn test_parse_error(#[case] input: &str, #[case] offset: usize, #[case] message: &str) {
        let err = parse(input).unwrap_err();
        assert_eq!((err.offset, err.message.as_str()), (offset, message));
    }
}
