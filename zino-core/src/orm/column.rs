use crate::{
    extension::JsonObjectExt,
    model::{Column, EncodeColumn},
};

/// Extension trait for [`Column`](crate::model::Column).
pub(super) trait ColumnExt {
    /// Returns the field definition.
    fn field_definition(&self, primary_key_name: &str) -> String;

    /// Returns the type annotation.
    fn type_annotation(&self) -> &'static str;
}

impl<'a> ColumnExt for Column<'a> {
    fn field_definition(&self, primary_key_name: &str) -> String {
        let column_name = self
            .extra()
            .get_str("column_name")
            .unwrap_or_else(|| self.name());
        let column_type = self.column_type();
        let mut definition = format!("{column_name} {column_type}");
        if column_name == primary_key_name {
            definition += " PRIMARY KEY";
        }
        if let Some(value) = self.default_value() {
            if self.auto_increment() {
                definition += if cfg!(any(
                    feature = "orm-mariadb",
                    feature = "orm-mysql",
                    feature = "orm-tidb"
                )) {
                    " AUTO_INCREMENT"
                } else {
                    // PostgreSQL does not support `AUTO INCREMENT` and SQLite does not need it.
                    ""
                };
            } else if self.auto_random() {
                // Only TiDB supports this feature.
                definition += if cfg!(feature = "orm-tidb") {
                    " AUTO_RANDOM"
                } else {
                    ""
                };
            } else {
                let value = self.format_value(value);
                if cfg!(feature = "orm-sqlite") && value.contains('(') {
                    definition = format!("{definition} DEFAULT ({value})");
                } else {
                    definition = format!("{definition} DEFAULT {value}");
                }
            }
        } else if self.is_not_null() {
            definition += " NOT NULL";
        }
        definition
    }

    fn type_annotation(&self) -> &'static str {
        if cfg!(feature = "orm-postgres") {
            match self.column_type() {
                "UUID" => "::UUID",
                "BIGINT" | "BIGSERIAL" => "::BIGINT",
                "INT" | "SERIAL" => "::INT",
                "SMALLINT" | "SMALLSERIAL" => "::SMALLINT",
                _ => "::TEXT",
            }
        } else {
            ""
        }
    }
}
