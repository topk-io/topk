use std::collections::{BTreeMap, HashSet};

use futures::TryStreamExt;
use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
use topk_rs::proto::v1::control::{
    field_index, field_type::DataType, field_type_list::ListValueType,
    field_type_matrix::MatrixValueType, FieldIndex, FieldSpec, FieldType, KeywordIndexType,
    MultiVectorDistanceMetric, MultiVectorQuantization, VectorDistanceMetric,
};
use tracing::{debug, info};

use crate::import::ddl::Schema;
use crate::import::error::Error;
use crate::import::source::{ReadEvent, Source, SourceLocation, Table};

#[derive(Clone, Deserialize, PartialEq, Serialize)]
pub struct Plan {
    pub collection: String,
    pub target: Target,
}

impl Plan {
    /// Selects fields against the destination; true when the collection does not exist yet.
    pub async fn resolve(
        &mut self,
        source: &Source,
        client: &topk_rs::Client,
        fields: &[FieldSelection],
        renames: &[Rename],
        exclude: &[String],
    ) -> Result<bool, Error> {
        let existing = match client.collections().get(&self.collection).await {
            Ok(collection) => Some(collection.schema),
            Err(topk_rs::Error::CollectionNotFound) => None,
            Err(error) => return Err(Error::from(error)),
        };
        debug!(collection = %self.collection, existing = existing.is_some(), "resolving collection fields");
        self.target
            .select(source, fields, renames, exclude, existing.as_ref())
            .await?;
        if let Some(schema) = &existing {
            self.target.validate_schema(&self.collection, schema)?;
        }
        info!(create = existing.is_none(), "import plan resolved");
        Ok(existing.is_none())
    }

    pub fn validate(&self) -> Result<(), Error> {
        collection_name(&self.collection)?;
        self.target.validate()
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
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
                    ) && match ty {
                        DataType::Text(_) => true,
                        DataType::List(list) => list.value_type == ListValueType::String as i32,
                        _ => false,
                    }
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
    pub from: SourceLocation,
    pub id: String,
    pub filter: Option<String>,
    pub partition: Option<String>,
    pub limit: Option<u64>,
    pub fields: IndexMap<String, Field>,
    /// Source columns left out with `--exclude`; any other unplanned key passes through.
    pub exclude: Vec<String>,
}

impl Target {
    pub fn new(table: &Table, id: Option<&str>) -> Result<Self, Error> {
        let id = id
            .or(table.key.as_deref())
            .or_else(|| {
                table
                    .columns
                    .iter()
                    .find(|(name, _)| matches!(name.to_ascii_lowercase().as_str(), "_id" | "id"))
                    .map(|(name, _)| name.as_str())
            })
            .ok_or_else(|| Error::MissingId {
                from: table.from.clone(),
                columns: table.columns.iter().map(|(name, _)| name.clone()).collect(),
            })?;

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
        for (name, field) in table.columns.iter().filter(|(name, _)| name.as_str() != id) {
            let mut field = Field::new(field.clone());
            let stripped = name.trim_start_matches('_');
            if name.starts_with('_')
                && !stripped.is_empty()
                && !taken.contains(stripped)
                && !fields.contains_key(stripped)
            {
                field.from = Some(name.clone());
                fields.insert(stripped.to_string(), field);
            } else {
                fields.insert(name.clone(), field);
            }
        }
        Ok(Self {
            fields,
            exclude: Vec::new(),
            from: table.from.clone(),
            id: id.to_string(),
            filter: None,
            partition: None,
            limit: None,
        })
    }

    /// Every column imports; `--rename`, then `--exclude`, then `-f` adjust the set.
    pub async fn select(
        &mut self,
        source: &Source,
        fields: &[FieldSelection],
        renames: &[Rename],
        exclude: &[String],
        existing: Option<&Schema>,
    ) -> Result<(), Error> {
        let typed = source.discover().await?.complete;
        if typed {
            let columns: Vec<&str> = source
                .discover()
                .await?
                .columns
                .iter()
                .map(|(name, _)| name.as_str())
                .collect();
            for (what, column) in [
                ("id", Some(&self.id)),
                ("partition", self.partition.as_ref()),
            ] {
                if let Some(column) = column.filter(|column| !columns.contains(&column.as_str())) {
                    return Err(Error::InvalidArgument(format!(
                        "{what} column {column:?} is not in {:?} — available: {}",
                        self.from.to_string(),
                        columns.join(", ")
                    )));
                }
            }
        }
        let mut renamed = HashSet::new();
        for rename in renames {
            if !renamed.insert(rename.source.as_str()) {
                return Err(Error::InvalidArgument(format!(
                    "column {:?} is renamed twice",
                    rename.source
                )));
            }
            let old = self
                .fields
                .iter()
                .find(|(name, field)| field.source(name) == rename.source)
                .map(|(name, _)| name.clone());
            if old.as_ref() == Some(&rename.target) {
                continue;
            }
            if self.fields.contains_key(&rename.target) {
                return Err(Error::InvalidArgument(format!(
                    "field {:?} already exists",
                    rename.target
                )));
            }
            let mut field = match old {
                Some(old) => self.fields.shift_remove(&old),
                None => (!typed).then(Field::default),
            }
            .ok_or_else(|| {
                Error::InvalidArgument(format!("source field {:?} was not found", rename.source))
            })?;
            field.from = Some(rename.source.clone());
            self.fields.insert(rename.target.clone(), field);
        }
        for excluded in exclude {
            if fields.iter().any(|arg| arg.name == *excluded) {
                return Err(Error::InvalidArgument(format!(
                    "field {excluded:?} is both excluded and set with -f"
                )));
            }
            let index = self
                .fields
                .iter()
                .position(|(name, field)| name == excluded || field.source(name) == excluded);
            let column = match index.and_then(|index| self.fields.shift_remove_index(index)) {
                Some((name, field)) => field.source(&name).to_string(),
                None if !typed => excluded.clone(),
                None => {
                    return Err(Error::InvalidArgument(format!(
                        "excluded field {excluded:?} was not found"
                    )))
                }
            };
            self.exclude.push(column);
        }
        if let Some(schema) = existing {
            let read: HashSet<String> = self
                .source_columns()
                .into_iter()
                .chain(self.exclude.iter().map(String::as_str))
                .map(str::to_string)
                .collect();
            for (name, spec) in schema.iter().collect::<BTreeMap<_, _>>() {
                match self.fields.get_mut(name) {
                    Some(field)
                        if fields
                            .iter()
                            .any(|arg| arg.name == *name && arg.ty.is_some()) =>
                    {
                        field.schema.required = spec.required
                    }
                    Some(field) => field.schema = spec.clone(),
                    None if !typed && !read.contains(name) => {
                        self.fields.insert(name.clone(), Field::new(spec.clone()));
                    }
                    None => {}
                }
            }
        }
        let mut selected = HashSet::new();
        for arg in fields {
            if !selected.insert(arg.name.as_str()) {
                return Err(Error::InvalidArgument(format!(
                    "field {:?} selected twice",
                    arg.name
                )));
            }
            if arg.name == self.id {
                return Err(Error::InvalidArgument(format!(
                    "{:?} is the document id and imports as _id; pick another id column with --id to also import it as a field",
                    arg.name
                )));
            }
            // CONVENTION: -f on a field discovery did not see starts as text; a type choice replaces it
            let mut field = self
                .fields
                .get(&arg.name)
                .cloned()
                .or_else(|| (!typed).then(|| Field::new(FieldSpec::text(false))))
                .ok_or_else(|| {
                    Error::InvalidArgument(format!("field {:?} was not found", arg.name))
                })?;
            let dimension = if arg.samples_dimension(&field) {
                let mut sample = self.capped(100);
                sample.fields.retain(|name, _| name == &arg.name);
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
        let unnamed: Vec<&str> = source
            .discover()
            .await?
            .unnamed
            .iter()
            .map(String::as_str)
            .filter(|name| self.fields.contains_key(*name))
            .collect();
        if let Some(first) = unnamed.first() {
            return Err(Error::InvalidArgument(format!(
                "{}: {} {} no name in the header (a blank header cell, a trailing comma, or rows with more fields than the header) — rename with `--rename {first}=<name>`, or drop the values with `--exclude {}`",
                self.from,
                unnamed.join(", "),
                if unnamed.len() == 1 { "has" } else { "have" },
                unnamed.join(",")
            )));
        }
        let untyped: Vec<&str> = self
            .fields
            .iter()
            .filter(|(_, field)| field.schema.data_type.is_none())
            .map(|(name, _)| name.as_str())
            .collect();
        if !untyped.is_empty() {
            return Err(Error::InvalidArgument(format!(
                "{} {} no TopK type: leave {} out with `--exclude {}`, or set a type with -f <field>=<type> where one applies",
                untyped.join(", "),
                if untyped.len() == 1 { "has" } else { "have" },
                if untyped.len() == 1 { "it" } else { "them" },
                untyped.join(",")
            )));
        }
        self.validate()
    }

    pub fn validate(&self) -> Result<(), Error> {
        if self.from.value().trim().is_empty() {
            return Err(Error::InvalidArgument(
                "`from` is empty — name the table, index, collection or file path to read".into(),
            ));
        }
        if self.id.trim().is_empty() {
            return Err(Error::InvalidArgument("`id` cannot be empty".into()));
        }
        for (field_name, field) in &self.fields {
            field.validate()?;
            if field_name.is_empty() || field_name.starts_with('_') {
                return Err(Error::InvalidArgument(format!("{field_name:?}: field names cannot be empty or start with `_` — rename the column with `--rename {field_name}=<name>` or leave it out with `--exclude {field_name}`")));
            }
            for (dot, _) in field_name.match_indices('.') {
                let parent = &field_name[..dot];
                if matches!(
                    self.fields
                        .get(parent)
                        .and_then(|field| field.schema.data_type.as_ref())
                        .and_then(|ty| ty.data_type.as_ref()),
                    Some(DataType::Struct(_))
                ) {
                    return Err(Error::InvalidArgument(format!(
                        "field {field_name:?} conflicts with struct field {parent:?}; rename one with --rename"
                    )));
                }
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
    pub fn field_type(&self) -> Option<FieldType> {
        Some(match self {
            Self::Text => FieldType::text(),
            Self::Int => FieldType::integer(),
            Self::Float => FieldType::float(),
            Self::Bool => FieldType::boolean(),
            Self::Bytes => FieldType::bytes(),
            Self::Image => FieldType::image(),
            Self::Timestamp => FieldType::timestamp(),
            Self::TextList => FieldType::list(ListValueType::String),
            Self::IntList => FieldType::list(ListValueType::Integer),
            Self::FloatList => FieldType::list(ListValueType::Float),
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
        if let Some(ty) = self.ty.as_ref().and_then(Type::field_type) {
            field.schema.data_type = Some(ty);
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
                    field.schema.data_type = Some(FieldType::f32_vector(dim));
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
                    field.schema.data_type = Some(FieldType::matrix(cols, MatrixValueType::F32));
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
                    field.schema.data_type = Some(FieldType::f32_sparse_vector());
                }
            }
            _ => {}
        }
        if self.ty.is_some() || self.index.is_some() {
            field.schema.index = self.index.clone();
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct Rename {
    pub source: String,
    pub target: String,
}

pub fn collection_name(name: &str) -> Result<String, Error> {
    let mut chars = name.chars();
    let valid = matches!(chars.next(), Some(c) if c.is_ascii_alphanumeric())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
        && name.len() <= 255;
    match valid {
        true => Ok(name.to_string()),
        false => Err(Error::InvalidArgument(format!("{name:?}: collection names start with a letter or digit, then letters, digits, `_`, `.` or `-` (max 255 characters)"))),
    }
}
