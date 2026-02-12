//! Shared schema types used by all database drivers

/// Database object
#[derive(Clone, Debug)]
pub struct DatabaseObject {
    pub name: String,
    pub object_type: ObjectType,
    pub schema: String,
}

/// Object type
#[derive(Clone, Debug, PartialEq)]
pub enum ObjectType {
    Database,
    Schema,
    Table,
    View,
    StoredProcedure,
    Function,
    Column,
    Index,
}

impl std::fmt::Display for ObjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObjectType::Database => write!(f, "Database"),
            ObjectType::Schema => write!(f, "Schema"),
            ObjectType::Table => write!(f, "Table"),
            ObjectType::View => write!(f, "View"),
            ObjectType::StoredProcedure => write!(f, "Procedure"),
            ObjectType::Function => write!(f, "Function"),
            ObjectType::Column => write!(f, "Column"),
            ObjectType::Index => write!(f, "Index"),
        }
    }
}

/// Column definition
#[derive(Clone, Debug)]
pub struct ColumnDef {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub is_primary_key: bool,
    pub is_identity: bool,
    pub max_length: Option<i32>,
    pub precision: Option<i32>,
    pub scale: Option<i32>,
}

/// Table definition
#[derive(Clone, Debug)]
pub struct TableDef {
    pub schema: String,
    pub name: String,
    pub columns: Vec<ColumnDef>,
    pub row_count: Option<i64>,
}
