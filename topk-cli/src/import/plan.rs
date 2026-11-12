use std::collections::HashSet;

use futures::{StreamExt, TryStreamExt};
use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
use topk_rs::proto::v1::control::{
    field_index, field_type::DataType, field_type_list::ListValueType,
    field_type_matrix::MatrixValueType, FieldIndex, FieldSpec, KeywordIndexType,
    MultiVectorDistanceMetric, MultiVectorQuantization, VectorDistanceMetric,
};

use crate::import::ddl::Schema;
use crate::import::error::Error;
use crate::import::source::{ReadEvent, Source, Table};

#[derive(Clone, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Plan {
    pub collections: IndexMap<String, Target>,
}

impl Plan {
    pub async fn resolve(
        &mut self,
        source: &Source,
        client: &topk_rs::Client,
        fields: &[FieldSelection],
        renames: &[Rename],
    ) -> Result<Vec<String>, Error> {
        let to_create: Vec<String> = futures::stream::iter(self.collections.iter_mut().map(
            |(name, target)| async move {
                let existing = match client.collections().get(name).await {
                    Ok(collection) => Some(collection.schema),
                    Err(topk_rs::Error::CollectionNotFound) => None,
                    Err(error) => return Err(Error::from(error)),
                };
                target
                    .select(source, fields, renames, existing.as_ref())
                    .await?;
                if let Some(schema) = &existing {
                    target.validate_schema(name, schema)?;
                }
                Ok(existing.is_none().then(|| name.clone()))
            },
        ))
        .buffered(8)
        .try_filter_map(|name| async { Ok(name) })
        .try_collect()
        .await?;
        self.validate()?;
        Ok(to_create)
    }

    pub fn validate(&self) -> Result<(), Error> {
        for (name, target) in self.collections.iter() {
            collection_name(name)?;
            target.validate()?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Field {
    pub from: Option<String>,
    pub schema: FieldSpec,
}

impl Field {
    pub fn new(schema: FieldSpec) -> Self {
        Self { from: None, schema }
    }

    pub fn source<'a>(&'a self, name: &'a str) -> &'a str {
        self.from.as_deref().unwrap_or(name)
    }

    pub fn validate(&self) -> Result<(), Error> {
        Self::validate_schema(&self.schema)
    }
}

impl Field {
    fn validate_schema(schema: &FieldSpec) -> Result<(), Error> {
        let ty = schema
            .data_type
            .as_ref()
            .and_then(|ty| ty.data_type.as_ref())
            .ok_or_else(|| Error::InvalidArgument("field has no data type".into()))?;
        match ty {
            DataType::List(list)
                if !matches!(
                    ListValueType::try_from(list.value_type),
                    Ok(ListValueType::String | ListValueType::Integer | ListValueType::Float)
                ) =>
            {
                return Err(Error::InvalidArgument(
                    "unsupported list element type".into(),
                ))
            }
            DataType::Matrix(matrix)
                if !matches!(
                    MatrixValueType::try_from(matrix.value_type),
                    Ok(MatrixValueType::F32
                        | MatrixValueType::F16
                        | MatrixValueType::F8
                        | MatrixValueType::U8
                        | MatrixValueType::I8)
                ) =>
            {
                return Err(Error::InvalidArgument(
                    "unsupported matrix element type".into(),
                ))
            }
            DataType::Struct(nested) => {
                for schema in nested.fields.values() {
                    Self::validate_schema(schema)?;
                }
            }
            _ => {}
        }
        if let Some(index) = &schema.index {
            let valid = match &index.index {
                Some(field_index::Index::KeywordIndex(index)) => {
                    matches!(
                        KeywordIndexType::try_from(index.index_type),
                        Ok(KeywordIndexType::Text | KeywordIndexType::Exact)
                    ) && matches!(ty, DataType::Text(_))
                }
                Some(field_index::Index::SemanticIndex(_)) => {
                    matches!(ty, DataType::Text(_) | DataType::Image(_))
                }
                Some(field_index::Index::NgramIndex(_)) => matches!(ty, DataType::Text(_)),
                Some(field_index::Index::VectorIndex(index)) => {
                    matches!(
                        VectorDistanceMetric::try_from(index.metric),
                        Ok(VectorDistanceMetric::Cosine
                            | VectorDistanceMetric::Euclidean
                            | VectorDistanceMetric::DotProduct
                            | VectorDistanceMetric::Hamming)
                    ) && matches!(
                        ty,
                        DataType::F32Vector(_)
                            | DataType::F16Vector(_)
                            | DataType::F8Vector(_)
                            | DataType::U8Vector(_)
                            | DataType::I8Vector(_)
                            | DataType::BinaryVector(_)
                            | DataType::F32SparseVector(_)
                            | DataType::F16SparseVector(_)
                            | DataType::F8SparseVector(_)
                            | DataType::U8SparseVector(_)
                            | DataType::I8SparseVector(_)
                    )
                }
                Some(field_index::Index::MultiVectorIndex(index)) => {
                    index.metric == MultiVectorDistanceMetric::Maxsim as i32
                        && index
                            .quantization
                            .is_none_or(|q| MultiVectorQuantization::try_from(q).is_ok())
                        && matches!(ty, DataType::Matrix(_))
                }
                None => false,
            };
            if !valid {
                return Err(Error::InvalidArgument(
                    "incompatible index for field type".into(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Deserialize, PartialEq, Serialize)]
pub struct Target {
    pub from: String,
    pub id: String,
    pub filter: Option<String>,
    pub partition: Option<String>,
    pub limit: Option<u64>,
    pub fields: IndexMap<String, Field>,
}

impl Target {
    pub async fn select(
        &mut self,
        source: &Source,
        fields: &[FieldSelection],
        renames: &[Rename],
        existing: Option<&Schema>,
    ) -> Result<(), Error> {
        for rename in renames {
            let old = self
                .fields
                .iter()
                .find(|(name, field)| field.source(name) == rename.source)
                .map(|(name, _)| name.clone())
                .ok_or_else(|| {
                    Error::InvalidArgument(format!(
                        "source field {:?} was not found",
                        rename.source
                    ))
                })?;
            if old != rename.target {
                if self.fields.contains_key(&rename.target) {
                    return Err(Error::InvalidArgument(format!(
                        "field {:?} already exists",
                        rename.target
                    )));
                }
                let mut field = self.fields.shift_remove(&old).ok_or_else(|| {
                    Error::InvalidArgument(format!(
                        "source field {:?} was not found",
                        rename.source
                    ))
                })?;
                field.from = Some(rename.source.clone());
                self.fields.insert(rename.target.clone(), field);
            }
        }
        if let Some(schema) = existing {
            for (name, field) in &mut self.fields {
                if let Some(existing) = schema.get(name) {
                    if fields
                        .iter()
                        .any(|arg| arg.name == *name && arg.ty.is_some())
                    {
                        field.schema.required = existing.required;
                    } else {
                        field.schema = existing.clone();
                    }
                }
            }
        }
        if !fields.is_empty() {
            let mut available = std::mem::take(&mut self.fields);
            for arg in fields {
                if self.fields.contains_key(&arg.name) {
                    return Err(Error::InvalidArgument(format!(
                        "field {:?} selected twice",
                        arg.name
                    )));
                }
                let mut field = available.shift_remove(&arg.name).ok_or_else(|| {
                    Error::InvalidArgument(format!("field {:?} was not found", arg.name))
                })?;
                let dimension = if arg.samples_dimension(&field) {
                    let mut sample = self.capped(100);
                    sample.fields = IndexMap::from([(arg.name.clone(), field.clone())]);
                    let mut stream = source.scan(&sample, None).await?;
                    let mut dimension = None;
                    'sample: while let Some(event) = stream.try_next().await? {
                        if let ReadEvent::Chunk(chunk) = event {
                            for row in chunk.rows {
                                let row = row?;
                                let value =
                                    row.iter().find(|(name, _)| name == field.source(&arg.name));
                                let size = value
                                    .and_then(|(_, value)| crate::import::floats(value))
                                    .map(|values| values.len());
                                if let Some(size) = size.filter(|size| *size > 0) {
                                    let size = u32::try_from(size).map_err(|_| {
                                        Error::InvalidArgument(format!(
                                            "{}: vector dimension exceeds u32",
                                            arg.name
                                        ))
                                    })?;
                                    dimension = Some(size);
                                    break 'sample;
                                }
                            }
                        }
                    }
                    dimension
                } else {
                    None
                };
                arg.apply(&mut field, dimension)?;
                self.fields.insert(arg.name.clone(), field);
            }
        }
        self.validate()
    }

    pub fn validate(&self) -> Result<(), Error> {
        if self.from.trim().is_empty() {
            return Err(Error::InvalidArgument(
                "`from` is empty — name the table, index, collection or file path to read".into(),
            ));
        }
        if self.id.trim().is_empty() {
            return Err(Error::InvalidArgument("`id` cannot be empty".into()));
        }
        if self.fields.is_empty() {
            return Err(Error::InvalidArgument(
                "select at least one field — only selected fields are imported".into(),
            ));
        }
        for (field_name, field) in &self.fields {
            field.validate()?;
            if field_name.is_empty() || field_name.starts_with('_') {
                return Err(Error::InvalidArgument(format!("{field_name:?}: field names cannot be empty or start with `_` — read the column under another name with `from = {field_name:?}`")));
            }
        }
        Ok(())
    }

    pub fn capped(&self, limit: u64) -> Self {
        Self {
            limit: Some(self.limit.map_or(limit, |existing| existing.min(limit))),
            ..self.clone()
        }
    }

    pub(super) fn source_columns(&self) -> IndexSet<&str> {
        std::iter::once(self.id.as_str())
            .chain(self.fields.iter().map(|(name, field)| field.source(name)))
            .chain(self.partition.as_deref())
            .collect()
    }
    pub fn from_table(table: &Table, id: Option<&str>) -> Result<Target, Error> {
        let id = id
            .or(table.key.as_deref())
            .or_else(|| {
                table
                    .columns
                    .iter()
                    .find(|(name, _)| matches!(name.to_ascii_lowercase().as_str(), "_id" | "id"))
                    .map(|(name, _)| name.as_str())
            })
            .ok_or_else(|| Error::MissingId(table.from.clone()))?;

        // Field names may not start with `_`, but sources use it for bookkeeping
        // columns (`_lang`, `_n_chars`); strip it and keep the source column in
        // `from`. A name that strips to nothing or onto another column is left
        // for validation to reject.
        let mut fields: IndexMap<String, Field> = IndexMap::new();
        let taken: HashSet<&str> = table
            .columns
            .iter()
            .map(|(name, _)| name.as_str())
            .collect();
        for (name, field) in table
            .columns
            .iter()
            .filter(|(name, _)| name.as_str() != id)
            .filter_map(|(name, field)| field.as_ref().map(|field| (name, field)))
        {
            let stripped = name.trim_start_matches('_');
            if name.starts_with('_')
                && !stripped.is_empty()
                && !taken.contains(stripped)
                && !fields.contains_key(stripped)
            {
                let mut field = Field::new(field.clone());
                field.from = Some(name.clone());
                fields.insert(stripped.to_string(), field);
            } else {
                fields.insert(name.clone(), Field::new(field.clone()));
            }
        }

        Ok(Target {
            fields,
            from: table.from.clone(),
            id: id.to_string(),
            filter: None,
            partition: None,
            limit: None,
        })
    }
}

#[derive(Clone, Debug)]
pub struct FieldSelection {
    pub name: String,
    pub ty: Option<Type>,
    pub index: Option<FieldIndex>,
}

#[derive(Clone, Debug)]
pub enum Type {
    Text,
    Int,
    Float,
    Bool,
    Bytes,
    Image,
    Timestamp,
    TextList,
    IntList,
    FloatList,
    Vector(Option<u32>),
    Matrix(Option<u32>),
    Sparse,
}

impl Type {
    pub fn field_spec(&self, required: bool) -> Option<FieldSpec> {
        Some(match self {
            Self::Text => FieldSpec::text(required),
            Self::Int => FieldSpec::integer(required),
            Self::Float => FieldSpec::float(required),
            Self::Bool => FieldSpec::boolean(required),
            Self::Bytes => FieldSpec::bytes(required),
            Self::Image => FieldSpec::image(required),
            Self::Timestamp => FieldSpec::timestamp(required),
            Self::TextList => FieldSpec::list(required, ListValueType::String),
            Self::IntList => FieldSpec::list(required, ListValueType::Integer),
            Self::FloatList => FieldSpec::list(required, ListValueType::Float),
            Self::Vector(_) | Self::Matrix(_) | Self::Sparse => return None,
        })
    }
}

impl FieldSelection {
    pub fn samples_dimension(&self, field: &Field) -> bool {
        matches!(&self.ty, Some(Type::Vector(None)))
            && !matches!(
                field
                    .schema
                    .data_type
                    .as_ref()
                    .and_then(|ty| ty.data_type.as_ref()),
                Some(
                    DataType::F32Vector(_)
                        | DataType::F16Vector(_)
                        | DataType::F8Vector(_)
                        | DataType::U8Vector(_)
                        | DataType::I8Vector(_)
                        | DataType::BinaryVector(_)
                )
            )
    }

    pub fn apply(&self, field: &mut Field, sampled_dimension: Option<u32>) -> Result<(), Error> {
        if let Some(schema) = self
            .ty
            .as_ref()
            .and_then(|ty| ty.field_spec(field.schema.required))
        {
            field.schema = schema;
        }
        match &self.ty {
            Some(Type::Vector(dim)) => {
                let native_dim = match field
                    .schema
                    .data_type
                    .as_ref()
                    .and_then(|ty| ty.data_type.as_ref())
                {
                    Some(DataType::F32Vector(v)) => Some(v.dimension),
                    Some(DataType::F16Vector(v)) => Some(v.dimension),
                    Some(DataType::F8Vector(v)) => Some(v.dimension),
                    Some(DataType::U8Vector(v)) => Some(v.dimension),
                    Some(DataType::I8Vector(v)) => Some(v.dimension),
                    Some(DataType::BinaryVector(v)) => Some(v.dimension),
                    _ => None,
                };
                let dim = dim.or(sampled_dimension).or(native_dim).ok_or_else(|| {
                    Error::InvalidArgument(format!(
                        "{}: vector dimension cannot be inferred; use vector:<dimension>",
                        self.name
                    ))
                })?;
                if let Some(source) = native_dim.filter(|source| *source != dim) {
                    return Err(Error::InvalidArgument(format!(
                        "{}: vector dimension {dim} differs from source dimension {source}",
                        self.name
                    )));
                }
                if native_dim.is_none() {
                    field.schema = FieldSpec::f32_vector(dim, field.schema.required);
                }
            }
            Some(Type::Matrix(cols)) => {
                let native_cols = match field
                    .schema
                    .data_type
                    .as_ref()
                    .and_then(|ty| ty.data_type.as_ref())
                {
                    Some(DataType::Matrix(m)) => Some(m.dimension),
                    _ => None,
                };
                let cols = cols.or(native_cols).ok_or_else(|| {
                    Error::InvalidArgument(format!(
                        "{}: matrix column count cannot be inferred; use matrix:<columns>",
                        self.name
                    ))
                })?;
                if let Some(source) = native_cols.filter(|source| *source != cols) {
                    return Err(Error::InvalidArgument(format!(
                        "{}: matrix columns {cols} differ from source columns {source}",
                        self.name
                    )));
                }
                if native_cols.is_none() {
                    field.schema =
                        FieldSpec::matrix(field.schema.required, cols, MatrixValueType::F32);
                }
            }
            Some(Type::Sparse) => {
                if !matches!(
                    field
                        .schema
                        .data_type
                        .as_ref()
                        .and_then(|ty| ty.data_type.as_ref()),
                    Some(
                        DataType::F32SparseVector(_)
                            | DataType::F16SparseVector(_)
                            | DataType::F8SparseVector(_)
                            | DataType::U8SparseVector(_)
                            | DataType::I8SparseVector(_)
                    )
                ) {
                    field.schema = FieldSpec::f32_sparse_vector(field.schema.required);
                }
            }
            _ => {}
        }
        if self.ty.is_some() || self.index.is_some() {
            field.schema.index = self.index.clone();
        }
        field.validate()
    }
}

#[derive(Clone, Debug)]
pub struct Rename {
    pub source: String,
    pub target: String,
}

fn collection_name(name: &str) -> Result<(), Error> {
    let mut chars = name.chars();
    let valid = matches!(chars.next(), Some(c) if c.is_ascii_alphanumeric())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
        && name.len() <= 255;
    match valid {
        true => Ok(()),
        false => Err(Error::InvalidArgument(format!("{name:?}: collection names start with a letter or digit, then letters, digits, `_`, `.` or `-` (max 255 characters)"))),
    }
}

/// Source object hint → collection name; None when no valid name derives.
pub(super) fn collection_key(hint: &str) -> Option<String> {
    let key: String = hint
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect();
    collection_name(&key).ok().map(|()| key)
}
