use sqlparser::ast::{AlterColumnOperation, AlterTable as SqlAlterTable, AlterTableOperation};
use topk_rs::proto::v1::control::{FieldSpec, FieldType};

use crate::{sql_invalid, sql_unsupported, Error, FromSql, Statement, Table};

#[derive(Debug, Clone, PartialEq)]
pub enum AlterOp {
    AddColumn {
        name: String,
        spec: FieldSpec,
        if_not_exists: bool,
    },
    DropColumn {
        name: String,
        if_exists: bool,
    },
    SetRequired {
        name: String,
        required: bool,
    },
    SetType {
        name: String,
        data_type: FieldType,
    },
}

impl TryFrom<SqlAlterTable> for Statement {
    type Error = Error;

    fn try_from(alter: SqlAlterTable) -> Result<Statement, Error> {
        sql_unsupported!(alter.if_exists, "ALTER TABLE IF EXISTS");
        sql_unsupported!(alter.only, "ALTER TABLE ONLY");
        sql_unsupported!(alter.on_cluster.is_some(), "ALTER TABLE … ON CLUSTER");

        let table = Table::new(alter.name)?;
        sql_invalid!(
            !matches!(table, Table::Collection(_)),
            "ALTER TABLE requires a collection name"
        );

        let mut ops = vec![];
        for op in alter.operations {
            match op {
                AlterTableOperation::AddColumn {
                    if_not_exists,
                    column_def,
                    column_position,
                    ..
                } => {
                    sql_unsupported!(column_position.is_some(), "ADD COLUMN … FIRST/AFTER");
                    ops.push(AlterOp::AddColumn {
                        name: column_def.name.value.clone(),
                        spec: FieldSpec::from_sql(column_def)?,
                        if_not_exists,
                    });
                }
                AlterTableOperation::DropColumn {
                    column_names,
                    if_exists,
                    drop_behavior,
                    ..
                } => {
                    sql_unsupported!(drop_behavior.is_some(), "DROP COLUMN … CASCADE/RESTRICT");
                    for name in column_names {
                        ops.push(AlterOp::DropColumn {
                            name: name.value,
                            if_exists,
                        });
                    }
                }
                AlterTableOperation::AlterColumn { column_name, op } => match op {
                    AlterColumnOperation::SetNotNull => ops.push(AlterOp::SetRequired {
                        name: column_name.value,
                        required: true,
                    }),
                    AlterColumnOperation::DropNotNull => ops.push(AlterOp::SetRequired {
                        name: column_name.value,
                        required: false,
                    }),
                    AlterColumnOperation::SetDataType {
                        data_type, using, ..
                    } => {
                        sql_unsupported!(using.is_some(), "ALTER COLUMN … TYPE … USING …");
                        ops.push(AlterOp::SetType {
                            name: column_name.value,
                            data_type: FieldType::from_sql(data_type)?,
                        });
                    }
                    AlterColumnOperation::SetDefault { .. } => {
                        sql_unsupported!("ALTER COLUMN … SET DEFAULT")
                    }
                    AlterColumnOperation::DropDefault => {
                        sql_unsupported!("ALTER COLUMN … DROP DEFAULT")
                    }
                    AlterColumnOperation::AddGenerated { .. } => {
                        sql_unsupported!("ALTER COLUMN … ADD GENERATED")
                    }
                },
                AlterTableOperation::RenameColumn { .. } => {
                    sql_unsupported!("ALTER TABLE … RENAME COLUMN")
                }
                AlterTableOperation::RenameTable { .. } => {
                    sql_unsupported!("ALTER TABLE … RENAME TO")
                }
                AlterTableOperation::ChangeColumn { .. } => {
                    sql_unsupported!("ALTER TABLE … CHANGE COLUMN, use ALTER COLUMN")
                }
                AlterTableOperation::ModifyColumn { .. } => {
                    sql_unsupported!("ALTER TABLE … MODIFY COLUMN, use ALTER COLUMN")
                }
                other => sql_unsupported!("ALTER TABLE operation: {other}"),
            }
        }

        sql_invalid!(ops.is_empty(), "ALTER TABLE requires at least one action");

        Ok(Statement::AlterTable { table, ops })
    }
}
