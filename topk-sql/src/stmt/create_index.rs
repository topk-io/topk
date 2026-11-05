use std::collections::HashMap;

use sqlparser::ast::{
    BinaryOperator, CreateIndex as SqlCreateIndex, Expr as SqlExpr, IndexType, Value as SqlValue,
};
use topk_rs::proto::v1::control::{
    FieldIndex, KeywordIndexType, MultiVectorDistanceMetric, MultiVectorQuantization,
    VectorDistanceMetric,
};

use crate::{
    parse_kwargs, sql_invalid, sql_unsupported, util::Kwargs, Error, SqlExprExt, Statement, Table,
};

pub fn index_name(collection: &str, field: &str) -> String {
    format!("{collection}_{field}_idx")
}

pub fn field_index(method: &str, opts: &HashMap<String, String>) -> Result<FieldIndex, Error> {
    let index = match method {
        "keyword_index" => {
            let kwargs = Kwargs(opts);
            let index_type = kwargs
                .optional::<KeywordIndexType>("type")?
                .unwrap_or(KeywordIndexType::Text);
            kwargs.done(&["type"])?;
            FieldIndex::keyword(index_type)
        }
        "semantic_index" => {
            sql_unsupported!(!opts.is_empty(), "semantic_index does not take options");
            FieldIndex::semantic()
        }
        "ngram_index" => {
            sql_unsupported!(!opts.is_empty(), "ngram_index does not take options");
            FieldIndex::ngram()
        }
        "vector_index" => {
            let (metric,) = parse_kwargs!(opts; metric: VectorDistanceMetric)?;
            FieldIndex::vector(metric)
        }
        "multi_vector_index" => {
            let (metric, quantization, width, top_k) = parse_kwargs!(
                opts;
                metric: MultiVectorDistanceMetric;
                quantization: MultiVectorQuantization, width: u32, top_k: u32
            )?;
            sql_invalid!(
                metric != MultiVectorDistanceMetric::Maxsim,
                "multi_vector_index metric must be 'maxsim'"
            );
            FieldIndex::multi_vector(metric, quantization, width, top_k)
        }
        _ => sql_unsupported!(
            "unknown index method `{method}`, expected: keyword_index | semantic_index | ngram_index | vector_index | multi_vector_index"
        ),
    };

    Ok(index)
}

pub fn option_value(expr: &SqlExpr) -> Option<String> {
    match expr.as_string() {
        Some(value) => Some(value),
        None => match expr {
            SqlExpr::Value(v) => match &v.value {
                SqlValue::Number(n, _) => Some(n.clone()),
                SqlValue::Boolean(b) => Some(b.to_string()),
                _ => None,
            },
            _ => None,
        },
    }
}

impl TryFrom<SqlCreateIndex> for Statement {
    type Error = Error;

    fn try_from(ci: SqlCreateIndex) -> Result<Statement, Error> {
        sql_unsupported!(ci.unique, "CREATE UNIQUE INDEX");
        sql_unsupported!(!ci.include.is_empty(), "CREATE INDEX … INCLUDE (…)");
        sql_unsupported!(ci.predicate.is_some(), "CREATE INDEX … WHERE …");
        sql_unsupported!(ci.nulls_distinct.is_some(), "CREATE INDEX … NULLS DISTINCT");
        sql_unsupported!(!ci.index_options.is_empty(), "CREATE INDEX index options");
        sql_unsupported!(
            !ci.alter_options.is_empty(),
            "CREATE INDEX … ALGORITHM/LOCK"
        );

        let table = Table::new(ci.table_name)?;
        sql_invalid!(
            !matches!(table, Table::Collection(_)),
            "CREATE INDEX requires a collection name"
        );

        sql_invalid!(
            ci.columns.len() != 1,
            "CREATE INDEX requires exactly one column"
        );
        let column = ci
            .columns
            .into_iter()
            .next()
            .ok_or_else(|| Error::Internal("CREATE INDEX column list is empty".to_string()))?;
        sql_unsupported!(
            column.operator_class.is_some(),
            "operator class; pass index options in WITH (…)"
        );
        sql_unsupported!(
            column.column.options.asc.is_some() || column.column.options.nulls_first.is_some(),
            "ASC/DESC and NULLS ordering on an index column"
        );
        let field = column.column.expr.as_ident().ok_or_else(|| {
            Error::Invalid("CREATE INDEX column must be an identifier".to_string())
        })?;

        let method = match ci.using {
            Some(IndexType::Custom(method)) => method.value.to_ascii_lowercase(),
            Some(method) => sql_unsupported!("index method `{method}`"),
            None => sql_invalid!(
                "CREATE INDEX requires USING <method>, e.g. USING vector_index (embedding) WITH (metric = 'cosine')"
            ),
        };

        let mut opts = HashMap::new();
        for option in ci.with {
            match option {
                SqlExpr::BinaryOp {
                    left,
                    op: BinaryOperator::Eq,
                    right,
                } => {
                    let key = match left.as_ident() {
                        Some(key) => key,
                        None => sql_invalid!("expected identifier in WITH (…)"),
                    };
                    let value = match option_value(&right) {
                        Some(value) => value,
                        None => sql_invalid!("expected a literal for option `{key}`"),
                    };
                    opts.insert(key, value);
                }
                other => sql_invalid!("expected key = value in WITH (…), got {other:?}"),
            }
        }

        let index = field_index(&method, &opts)?;

        if let Some(name) = ci.name {
            let expected = index_name(table.collection(), &field);
            let given = name
                .0
                .last()
                .and_then(|part| part.as_ident())
                .map(|ident| ident.value.clone());
            sql_invalid!(
                given.as_deref() != Some(expected.as_str()),
                "index name must be `{expected}`; TopK names an index after its column"
            );
        }

        Ok(Statement::CreateIndex {
            table,
            field,
            index,
            if_not_exists: ci.if_not_exists,
        })
    }
}
