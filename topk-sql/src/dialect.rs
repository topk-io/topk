use std::any::TypeId;

use sqlparser::ast::{CheckConstraint, ColumnOption, Expr as SqlExpr};
use sqlparser::dialect::{Dialect, PostgreSqlDialect, Precedence};
use sqlparser::keywords::Keyword;
use sqlparser::parser::{Parser, ParserError};

#[derive(Debug)]
pub struct TopKDialect {
    postgres: PostgreSqlDialect,
}

impl Default for TopKDialect {
    fn default() -> Self {
        Self {
            postgres: PostgreSqlDialect {},
        }
    }
}

impl Dialect for TopKDialect {
    // sqlparser gates syntax such as `UPDATE … FROM` and `ADD COLUMN IF NOT EXISTS` on the
    // dialect's `TypeId`; reporting Postgres' opens every branch it would take.
    fn dialect(&self) -> TypeId {
        self.postgres.dialect()
    }

    fn identifier_quote_style(&self, identifier: &str) -> Option<char> {
        self.postgres.identifier_quote_style(identifier)
    }

    fn is_delimited_identifier_start(&self, ch: char) -> bool {
        self.postgres.is_delimited_identifier_start(ch)
    }

    fn is_identifier_start(&self, ch: char) -> bool {
        self.postgres.is_identifier_start(ch)
    }

    fn is_identifier_part(&self, ch: char) -> bool {
        self.postgres.is_identifier_part(ch)
    }

    fn supports_unicode_string_literal(&self) -> bool {
        self.postgres.supports_unicode_string_literal()
    }

    fn is_custom_operator_part(&self, ch: char) -> bool {
        self.postgres.is_custom_operator_part(ch)
    }

    fn get_next_precedence(&self, parser: &Parser) -> Option<Result<u8, ParserError>> {
        self.postgres.get_next_precedence(parser)
    }

    fn supports_filter_during_aggregation(&self) -> bool {
        self.postgres.supports_filter_during_aggregation()
    }

    fn supports_group_by_expr(&self) -> bool {
        self.postgres.supports_group_by_expr()
    }

    fn prec_value(&self, prec: Precedence) -> u8 {
        self.postgres.prec_value(prec)
    }

    fn allow_extract_custom(&self) -> bool {
        self.postgres.allow_extract_custom()
    }

    fn allow_extract_single_quotes(&self) -> bool {
        self.postgres.allow_extract_single_quotes()
    }

    fn supports_create_index_with_clause(&self) -> bool {
        self.postgres.supports_create_index_with_clause()
    }

    fn supports_array_typedef_with_brackets(&self) -> bool {
        self.postgres.supports_array_typedef_with_brackets()
    }

    fn supports_explain_with_utility_options(&self) -> bool {
        self.postgres.supports_explain_with_utility_options()
    }

    fn parse_column_option(
        &self,
        parser: &mut Parser,
    ) -> Result<Option<Result<Option<ColumnOption>, ParserError>>, ParserError> {
        if !parser.parse_keyword(Keyword::INDEX) {
            return Ok(None);
        }
        let expr = parser.parse_expr()?;
        match expr {
            SqlExpr::Function(_) => Ok(Some(Ok(Some(ColumnOption::Check(CheckConstraint {
                name: None,
                expr: Box::new(expr),
                enforced: None,
            }))))),
            _ => Ok(Some(Err(ParserError::ParserError(
                "INDEX must be followed by a function call, e.g. INDEX vector_index(metric = 'cosine')"
                    .to_string(),
            )))),
        }
    }
}
