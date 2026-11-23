use std::collections::HashMap;

use sqlparser::ast::{
    ArrayElemTypeDef, BinaryOperator, CheckConstraint, ColumnDef, ColumnOption,
    CreateTable as SqlCreateTable, DataType, Expr as SqlExpr, Function, FunctionArg,
    FunctionArgExpr, FunctionArguments,
};
use topk_rs::proto::v1::control::{
    field_index::Index, field_type::DataType as FieldDataType, field_type_list::ListValueType,
    field_type_matrix::MatrixValueType, FieldIndex, FieldSpec, FieldType, KeywordIndexType,
    MultiVectorDistanceMetric, MultiVectorQuantization, VectorDistanceMetric,
};

use crate::{
    parse_args, parse_kwargs, sql_invalid, sql_unsupported, util::Kwargs, Error, FromSql,
    SqlExprExt, Statement, Table, ToSql,
};

impl TryFrom<SqlCreateTable> for Statement {
    type Error = Error;

    fn try_from(ct: SqlCreateTable) -> Result<Statement, Error> {
        sql_unsupported!(ct.query.is_some(), "CREATE … AS SELECT");
        sql_unsupported!(!ct.constraints.is_empty(), "table constraint");

        let table = Table::new(ct.name)?;
        sql_invalid!(
            !matches!(table, Table::Collection(_)),
            "CREATE requires a collection name"
        );

        let mut schema = HashMap::new();
        for column in ct.columns {
            let name = column.name.to_string();
            let spec = FieldSpec::from_sql(column)?;
            sql_invalid!(
                schema.insert(name.clone(), spec).is_some(),
                "duplicate column `{name}`"
            );
        }

        Ok(Statement::CreateTable {
            table,
            schema,
            if_not_exists: ct.if_not_exists,
        })
    }
}

impl FromSql<ColumnDef> for FieldSpec {
    fn from_sql(column: ColumnDef) -> Result<Self, Error> {
        let data_type = FieldType::from_sql(column.data_type)?;

        let mut required = false;
        let mut index = None;
        for option in column.options {
            match option.option {
                ColumnOption::Null => {}
                ColumnOption::NotNull => required = true,
                ColumnOption::Check(CheckConstraint { expr, .. }) => match *expr {
                    SqlExpr::Function(func) => {
                        sql_invalid!(index.is_some(), "column has multiple INDEX definitions");
                        index = Some(FieldIndex::from_sql(func)?);
                    }
                    _ => sql_unsupported!("CHECK constraint"),
                },
                ColumnOption::Default(_) => sql_unsupported!("DEFAULT constraint"),
                ColumnOption::Unique { .. } => sql_unsupported!("UNIQUE constraint"),
                ColumnOption::ForeignKey { .. } => sql_unsupported!("FOREIGN KEY constraint"),
                other => sql_unsupported!("column constraint: {other}"),
            }
        }

        Ok(FieldSpec {
            data_type: Some(data_type),
            required,
            index,
        })
    }
}

impl FromSql<DataType> for FieldType {
    fn from_sql(data_type: DataType) -> Result<Self, Error> {
        use sqlparser::ast::DataType::*;

        match data_type {
            // Native scalar types
            Boolean => Ok(FieldType::boolean()),
            Integer(_) | BigInt(_) | SmallInt(_) | Int(_) | Int2(_) | Int4(_) | Int8(_) => {
                Ok(FieldType::integer())
            }
            Float(_) | Float4 | Float8 | Real | DoublePrecision => Ok(FieldType::float()),
            Text | Varchar(_) => Ok(FieldType::text()),
            Bytea => Ok(FieldType::bytes()),
            Timestamp(_, sqlparser::ast::TimezoneInfo::None) => Ok(FieldType::timestamp()),
            Timestamp(_, _) => sql_unsupported!("TIMESTAMPTZ"),

            // Native array types → list
            Array(ArrayElemTypeDef::SquareBracket(inner, _)) => match *inner {
                Text | Varchar(_) => Ok(FieldType::list(ListValueType::String)),
                Integer(_) | BigInt(_) | SmallInt(_) | Int(_) | Int2(_) | Int4(_) | Int8(_) => {
                    Ok(FieldType::list(ListValueType::Integer))
                }
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

                    "image" => Ok(FieldType::image()),

                    _ => sql_unsupported!("data type: {name}"),
                }
            }

            dt => sql_unsupported!("data type: {dt}"),
        }
    }
}

impl FromSql<Function> for FieldIndex {
    fn from_sql(func: Function) -> Result<Self, Error> {
        let method = func.name.to_string().to_ascii_lowercase();

        let opts: HashMap<String, String> = match func.args {
            FunctionArguments::None => HashMap::new(),
            FunctionArguments::List(list) => list
                .args
                .iter()
                .map(|arg| match arg {
                    FunctionArg::Unnamed(FunctionArgExpr::Expr(SqlExpr::BinaryOp {
                        left,
                        op: BinaryOperator::Eq,
                        right,
                    })) => {
                        let key = match left.as_ident() {
                            Some(key) => key,
                            None => sql_invalid!("expected identifier in INDEX option"),
                        };
                        let value = match right.as_string() {
                            Some(value) => value,
                            None => sql_invalid!("expected string in INDEX option"),
                        };
                        Ok((key, value))
                    }
                    other => {
                        sql_invalid!("expected key = 'value' in INDEX options, got {other}")
                    }
                })
                .collect::<Result<_, Error>>()?,
            other => {
                sql_invalid!("expected function argument list for index options, got {other}")
            }
        };

        let index = match method.as_str() {
            "keyword_index" => {
                let kwargs = Kwargs(&opts);
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
                "unknown index method `{method}`, expected: keyword_index | semantic_index | ngram_index | vector_index | multi_vector_index"
            ),
        };

        Ok(index)
    }
}

impl ToSql for FieldType {
    fn to_sql(&self) -> String {
        // a field with no declared type holds any value, which the wire carries as JSON
        let Some(data_type) = &self.data_type else {
            return "JSONB".to_string();
        };

        match data_type {
            FieldDataType::Text(_) => "TEXT".to_string(),
            FieldDataType::Integer(_) => "INTEGER".to_string(),
            FieldDataType::Float(_) => "FLOAT".to_string(),
            FieldDataType::Boolean(_) => "BOOLEAN".to_string(),
            FieldDataType::Bytes(_) => "BYTEA".to_string(),
            FieldDataType::Timestamp(_) => "TIMESTAMP".to_string(),
            FieldDataType::Image(_) => "image".to_string(),
            FieldDataType::Struct(_) => "JSONB".to_string(),
            FieldDataType::List(list) => match list.value_type() {
                ListValueType::String => "TEXT[]",
                ListValueType::Integer => "INTEGER[]",
                ListValueType::Float => "FLOAT[]",
                ListValueType::Unspecified => "JSONB",
            }
            .to_string(),
            FieldDataType::F32Vector(v) => format!("f32_vector({})", v.dimension),
            FieldDataType::F16Vector(v) => format!("f16_vector({})", v.dimension),
            FieldDataType::F8Vector(v) => format!("f8_vector({})", v.dimension),
            FieldDataType::U8Vector(v) => format!("u8_vector({})", v.dimension),
            FieldDataType::I8Vector(v) => format!("i8_vector({})", v.dimension),
            FieldDataType::BinaryVector(v) => format!("binary_vector({})", v.dimension),
            FieldDataType::F32SparseVector(_) => "f32_sparse_vector".to_string(),
            FieldDataType::F16SparseVector(_) => "f16_sparse_vector".to_string(),
            FieldDataType::F8SparseVector(_) => "f8_sparse_vector".to_string(),
            FieldDataType::U8SparseVector(_) => "u8_sparse_vector".to_string(),
            FieldDataType::I8SparseVector(_) => "i8_sparse_vector".to_string(),
            FieldDataType::Matrix(matrix) => {
                let value_type = match matrix.value_type() {
                    MatrixValueType::F32 | MatrixValueType::Unspecified => "f32",
                    MatrixValueType::F16 => "f16",
                    MatrixValueType::F8 => "f8",
                    MatrixValueType::U8 => "u8",
                    MatrixValueType::I8 => "i8",
                };
                format!("{value_type}_matrix({})", matrix.dimension)
            }
        }
    }
}

impl ToSql for Index {
    fn to_sql(&self) -> String {
        match self {
            Index::KeywordIndex(keyword) => match keyword.index_type() {
                KeywordIndexType::Exact => "keyword_index(type = 'exact')".to_string(),
                KeywordIndexType::Text | KeywordIndexType::Unspecified => {
                    "keyword_index()".to_string()
                }
            },
            Index::SemanticIndex(_) => "semantic_index()".to_string(),
            Index::NgramIndex(_) => "ngram_index()".to_string(),
            Index::VectorIndex(vector) => format!("vector_index(metric = '{}')", vector.metric()),
            Index::MultiVectorIndex(multi) => {
                let mut options = vec![format!("metric = '{}'", multi.metric())];
                if multi.quantization.is_some() {
                    options.push(format!("quantization = '{}'", multi.quantization()));
                }
                if let Some(width) = multi.width {
                    options.push(format!("width = '{width}'"));
                }
                if let Some(top_k) = multi.top_k {
                    options.push(format!("top_k = '{top_k}'"));
                }
                format!("multi_vector_index({})", options.join(", "))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{convert_sql, parse_sql, Statement, ToSql};

    // Every type and index `CREATE TABLE` takes, written back as SQL, reads back the same.
    #[test]
    fn to_sql_reads_back() {
        let sql = "CREATE TABLE books (
            title TEXT NOT NULL INDEX keyword_index(),
            genre TEXT INDEX keyword_index(type = 'exact'),
            body TEXT INDEX semantic_index(),
            code TEXT INDEX ngram_index(),
            year INTEGER,
            price FLOAT,
            in_stock BOOLEAN,
            raw BYTEA,
            published TIMESTAMP,
            cover image INDEX semantic_index(),
            tags TEXT[],
            counts INTEGER[],
            scores FLOAT[],
            metadata JSONB,
            f32 f32_vector(3) INDEX vector_index(metric = 'cosine'),
            f16 f16_vector(3) INDEX vector_index(metric = 'euclidean'),
            f8 f8_vector(3) INDEX vector_index(metric = 'dot_product'),
            u8 u8_vector(3),
            i8 i8_vector(3),
            bits binary_vector(8) INDEX vector_index(metric = 'hamming'),
            sparse f32_sparse_vector INDEX vector_index(metric = 'dot_product'),
            sparse16 f16_sparse_vector,
            sparse8 f8_sparse_vector,
            sparse_u8 u8_sparse_vector,
            sparse_i8 i8_sparse_vector,
            colbert f32_matrix(4) INDEX multi_vector_index(metric = 'maxsim'),
            colbert16 f16_matrix(4) INDEX multi_vector_index(metric = 'maxsim', quantization = '1bit', width = '16', top_k = '100'),
            colbert8 f8_matrix(4),
            colbert_u8 u8_matrix(4),
            colbert_i8 i8_matrix(4)
        )";
        let Statement::CreateTable { schema, .. } =
            convert_sql(parse_sql(sql).unwrap()).unwrap().remove(0).0
        else {
            panic!("not a CREATE TABLE");
        };

        let columns = schema
            .iter()
            .map(|(name, spec)| {
                let data_type = spec.data_type.as_ref().unwrap().to_sql();
                let not_null = if spec.required { " NOT NULL" } else { "" };
                let index = spec
                    .index
                    .as_ref()
                    .and_then(|index| index.index.as_ref())
                    .map(|index| format!(" INDEX {}", index.to_sql()))
                    .unwrap_or_default();
                format!("{name} {data_type}{not_null}{index}")
            })
            .collect::<Vec<_>>();
        let Statement::CreateTable { schema: read, .. } = convert_sql(
            parse_sql(&format!("CREATE TABLE books ({})", columns.join(", "))).unwrap(),
        )
        .unwrap()
        .remove(0)
        .0
        else {
            panic!("not a CREATE TABLE");
        };

        assert_eq!(read, schema);
    }
}
