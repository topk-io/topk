use std::collections::HashMap;

use sqlparser::ast::{
    ArrayElemTypeDef, BinaryOperator, ColumnDef, ColumnOption, CreateIndex as SqlCreateIndex,
    CreateTable as SqlCreateTable, DataType, Expr as SqlExpr,
};
use topk_rs::proto::v1::control::{
    FieldIndex, FieldSpec, FieldType, KeywordIndexType, MultiVectorDistanceMetric,
    MultiVectorQuantization, VectorDistanceMetric, field_type_list::ListValueType,
    field_type_matrix::MatrixValueType,
};

use crate::{
    Error, FromSql, Index, SqlExprExt, Statement, Table, parse_args, parse_kwargs, sql_invalid,
    sql_unsupported,
};

impl TryFrom<SqlCreateTable> for Statement {
    type Error = Error;

    fn try_from(ct: SqlCreateTable) -> Result<Statement, Error> {
        sql_unsupported!(ct.query.is_some(), "CREATE TABLE … AS SELECT");

        let table = Table::new(ct.name)?;
        sql_invalid!(
            !matches!(table, Table::Collection(_)),
            "CREATE TABLE requires a collection name"
        );

        let schema = ct
            .columns
            .into_iter()
            .map(|column| {
                let name = column.name.to_string();
                FieldSpec::from_sql(column).map(|spec| (name, spec))
            })
            .collect::<Result<_, Error>>()?;

        Ok(Statement::CreateTable {
            table,
            schema,
            if_not_exists: ct.if_not_exists,
        })
    }
}

impl FromSql<SqlCreateIndex> for Index {
    fn from_sql(idx: SqlCreateIndex) -> Result<Self, Error> {
        sql_unsupported!(idx.unique, "CREATE UNIQUE INDEX");
        sql_unsupported!(idx.concurrently, "CREATE INDEX CONCURRENTLY");
        sql_unsupported!(idx.if_not_exists, "CREATE INDEX IF NOT EXISTS");
        sql_unsupported!(!idx.include.is_empty(), "INDEX … INCLUDE");
        sql_unsupported!(idx.predicate.is_some(), "partial index (WHERE clause)");
        sql_invalid!(
            idx.columns.len() != 1,
            "CREATE INDEX must reference exactly one column"
        );

        // Parse table name.
        let table = Table::new(idx.table_name)?;
        sql_invalid!(
            !matches!(table, Table::Collection(_)),
            "CREATE INDEX requires a collection name"
        );

        // Parse index method.
        let method = idx
            .using
            .as_ref()
            .map(|using| using.value.to_ascii_lowercase());
        let method = match method {
            Some(method) => method,
            None => sql_invalid!("CREATE INDEX requires USING <index method>"),
        };

        // Parse field name.
        let field = match &idx.columns[0].expr {
            SqlExpr::Identifier(ident) => ident.value.clone(),
            e => sql_invalid!("expected column name in CREATE INDEX, got {e:?}"),
        };

        // Parse WITH options.
        let opts = idx
            .with
            .into_iter()
            .map(|expr| match expr {
                SqlExpr::BinaryOp {
                    left,
                    op: BinaryOperator::Eq,
                    right,
                } => {
                    let key = match left.as_ident() {
                        Some(key) => key,
                        None => sql_invalid!("expected identifier in WITH option"),
                    };
                    let value = match right.as_string() {
                        Some(value) => value,
                        None => sql_invalid!("expected string in WITH option"),
                    };

                    Ok((key, value))
                }
                e => sql_invalid!("expected key = value in WITH clause, got {e:?}"),
            })
            .collect::<Result<HashMap<String, String>, Error>>()?;

        let index = match method.as_str() {
            "keyword_index" => {
                sql_unsupported!(!opts.is_empty(), "keyword_index does not take WITH options");
                FieldIndex::keyword(KeywordIndexType::Text)
            }
            "semantic_index" => {
                sql_unsupported!(
                    !opts.is_empty(),
                    "semantic_index does not take WITH options"
                );
                FieldIndex::semantic()
            }
            "vector_index" => {
                let (metric,) = parse_kwargs!(&opts; metric: VectorDistanceMetric)?;
                FieldIndex::vector(metric)
            }
            "multi_vector_index" => {
                let (metric, quantization, width, top_k) = parse_kwargs!(
                    &opts;
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
                "unknown index method `{method}`, expected: keyword_index | semantic_index | vector_index | multi_vector_index"
            ),
        };

        Ok(Index {
            table,
            field,
            index,
        })
    }
}

impl FromSql<SqlCreateIndex> for Statement {
    fn from_sql(idx: SqlCreateIndex) -> Result<Self, Error> {
        Ok(Statement::CreateIndex {
            index: Index::from_sql(idx)?,
        })
    }
}

impl FromSql<ColumnDef> for FieldSpec {
    fn from_sql(column: ColumnDef) -> Result<Self, Error> {
        let data_type = FieldType::from_sql(column.data_type)?;

        let mut required = false;
        for option in column.options {
            match option.option {
                ColumnOption::Null => {}
                ColumnOption::NotNull => required = true,
                ColumnOption::Default(_) => sql_unsupported!("DEFAULT constraint"),
                ColumnOption::Unique { .. } => sql_unsupported!("UNIQUE constraint"),
                ColumnOption::Check(_) => sql_unsupported!("CHECK constraint"),
                ColumnOption::ForeignKey { .. } => sql_unsupported!("FOREIGN KEY constraint"),
                other => sql_unsupported!("column constraint: {other:?}"),
            }
        }

        Ok(FieldSpec {
            data_type: Some(data_type),
            required,
            index: None,
        })
    }
}

impl FromSql<DataType> for FieldType {
    fn from_sql(data_type: DataType) -> Result<Self, Error> {
        use sqlparser::ast::DataType::*;

        match data_type {
            // Native scalar types
            Boolean => Ok(FieldType::boolean()),
            Integer(_) | BigInt(_) | SmallInt(_) => Ok(FieldType::integer()),
            Float(_) | Float4 | Float8 | Real | DoublePrecision => Ok(FieldType::float()),
            Text | Varchar(_) => Ok(FieldType::text()),
            Bytea => Ok(FieldType::bytes()),

            // Native array types → list
            Array(ArrayElemTypeDef::SquareBracket(inner, _)) => match *inner {
                Text | Varchar(_) => Ok(FieldType::list(ListValueType::String)),
                Integer(_) | BigInt(_) | SmallInt(_) => Ok(FieldType::list(ListValueType::Integer)),
                Float(_) | Float4 | Float8 | Real | DoublePrecision => {
                    Ok(FieldType::list(ListValueType::Float))
                }
                dt => sql_unsupported!("list element type: {dt}"),
            },

            // JSON/JSONB → opaque struct
            JSON | JSONB => Ok(FieldType::r#struct(std::iter::empty::<(
                std::string::String,
                FieldSpec,
            )>())),

            // Custom TopK types
            Custom(name, args) => {
                let fn_name = name.to_string().to_ascii_lowercase();
                match fn_name.as_str() {
                    // Dense vectors
                    "f32_vector" => {
                        let (dim,) = parse_args!(&args; u32)?;
                        Ok(FieldType::f32_vector(dim))
                    }
                    "f16_vector" => {
                        let (dim,) = parse_args!(&args; u32)?;
                        Ok(FieldType::f16_vector(dim))
                    }
                    "f8_vector" => {
                        let (dim,) = parse_args!(&args; u32)?;
                        Ok(FieldType::f8_vector(dim))
                    }
                    "u8_vector" => {
                        let (dim,) = parse_args!(&args; u32)?;
                        Ok(FieldType::u8_vector(dim))
                    }
                    "i8_vector" => {
                        let (dim,) = parse_args!(&args; u32)?;
                        Ok(FieldType::i8_vector(dim))
                    }
                    "binary_vector" => {
                        let (dim,) = parse_args!(&args; u32)?;
                        Ok(FieldType::binary_vector(dim))
                    }

                    // Sparse vectors
                    "f32_sparse_vector" => Ok(FieldType::f32_sparse_vector()),
                    "f16_sparse_vector" => Ok(FieldType::f16_sparse_vector()),
                    "f8_sparse_vector" => Ok(FieldType::f8_sparse_vector()),
                    "u8_sparse_vector" => Ok(FieldType::u8_sparse_vector()),
                    "i8_sparse_vector" => Ok(FieldType::i8_sparse_vector()),

                    // Matrix types
                    "f32_matrix" => {
                        let (dim,) = parse_args!(&args; u32)?;
                        Ok(FieldType::matrix(dim, MatrixValueType::F32))
                    }
                    "f16_matrix" => {
                        let (dim,) = parse_args!(&args; u32)?;
                        Ok(FieldType::matrix(dim, MatrixValueType::F16))
                    }
                    "f8_matrix" => {
                        let (dim,) = parse_args!(&args; u32)?;
                        Ok(FieldType::matrix(dim, MatrixValueType::F8))
                    }
                    "u8_matrix" => {
                        let (dim,) = parse_args!(&args; u32)?;
                        Ok(FieldType::matrix(dim, MatrixValueType::U8))
                    }
                    "i8_matrix" => {
                        let (dim,) = parse_args!(&args; u32)?;
                        Ok(FieldType::matrix(dim, MatrixValueType::I8))
                    }

                    _ => sql_unsupported!("data type: {name}"),
                }
            }

            dt => sql_unsupported!("data type: {dt}"),
        }
    }
}
