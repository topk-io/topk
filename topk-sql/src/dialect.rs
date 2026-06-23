use sqlparser::ast::helpers::attached_token::AttachedToken;
use sqlparser::ast::{
    CheckConstraint, ColumnOption, Expr as SqlExpr, Statement as SqlStatement, Update,
    UpdateTableFromKind,
};
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
        false
    }

    fn supports_array_typedef_with_brackets(&self) -> bool {
        self.postgres.supports_array_typedef_with_brackets()
    }

    fn supports_explain_with_utility_options(&self) -> bool {
        self.postgres.supports_explain_with_utility_options()
    }

    fn parse_statement(&self, parser: &mut Parser) -> Option<Result<SqlStatement, ParserError>> {
        if parser.parse_keyword(Keyword::UPDATE) {
            return Some(parse_update_statement(parser));
        }

        self.postgres.parse_statement(parser)
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

// sqlparser's parse_update gates UPDATE … FROM behind a dialect_of! TypeId check that
// only matches built-in dialects. Pulled out here so parse_statement can intercept UPDATE
// before the main dispatch and handle FROM correctly.
fn parse_update_statement(parser: &mut Parser) -> Result<SqlStatement, ParserError> {
    let table = parser.parse_table_and_joins()?;
    let assignments = parser
        .expect_keyword(Keyword::SET)
        .and_then(|_| parser.parse_comma_separated(Parser::parse_assignment))?;
    let from = parser
        .parse_keyword(Keyword::FROM)
        .then(|| parser.parse_table_and_joins())
        .transpose()?
        .map(|t| UpdateTableFromKind::AfterSet(vec![t]));
    let selection = parser
        .parse_keyword(Keyword::WHERE)
        .then(|| parser.parse_expr())
        .transpose()?;
    let returning = parser
        .parse_keyword(Keyword::RETURNING)
        .then(|| parser.parse_comma_separated(Parser::parse_select_item))
        .transpose()?;

    Ok(SqlStatement::Update(Update {
        update_token: AttachedToken::empty(),
        optimizer_hint: None,
        table,
        assignments,
        from,
        selection,
        returning,
        or: None,
        limit: None,
    }))
}
