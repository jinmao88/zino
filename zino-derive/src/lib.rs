#![doc = include_str!("../README.md")]
#![doc(html_favicon_url = "https://photino.github.io/zino-docs-zh/assets/zino-logo.png")]
#![doc(html_logo_url = "https://photino.github.io/zino-docs-zh/assets/zino-logo.svg")]
#![forbid(unsafe_code)]
#![feature(let_chains)]

use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields};

mod parser;

#[doc = include_str!("../docs/schema.md")]
#[proc_macro_derive(Schema, attributes(schema))]
pub fn derive_schema(item: TokenStream) -> TokenStream {
    // Integer types
    const INTEGER_TYPES: [&str; 10] = [
        "u64", "i64", "u32", "i32", "u16", "i16", "u8", "i8", "usize", "isize",
    ];

    // Special attributes
    const SPECIAL_ATTRIBUTES: [&str; 8] = [
        "ignore",
        "type_name",
        "not_null",
        "default_value",
        "index_type",
        "reference",
        "comment",
        "constructor",
    ];

    // Reserved fields
    const RESERVED_FIELDS: [&str; 4] = ["created_at", "updated_at", "version", "edition"];

    // Input
    let input = parse_macro_input!(item as DeriveInput);

    // Model name
    let name = input.ident;
    let mut model_name = name.to_string();

    // Parsing struct attributes
    let mut reader_name = String::from("main");
    let mut writer_name = String::from("main");
    let mut table_name = None;
    let mut model_comment = None;
    for attr in input.attrs.iter() {
        for (key, value) in parser::parse_schema_attr(attr).into_iter() {
            if let Some(value) = value {
                match key.as_str() {
                    "model_name" => {
                        model_name = value;
                    }
                    "reader_name" => {
                        reader_name = value;
                    }
                    "writer_name" => {
                        writer_name = value;
                    }
                    "table_name" => {
                        table_name = Some(value);
                    }
                    "comment" => {
                        model_comment = Some(value);
                    }
                    _ => (),
                }
            }
        }
    }

    // Parsing field attributes
    let mut primary_key_type = String::from("Uuid");
    let mut primary_key_name = String::from("id");
    let mut primary_key_value = None;
    let mut primary_key_column = None;
    let mut columns = Vec::new();
    let mut column_fields = Vec::new();
    let mut read_only_fields = Vec::new();
    let mut write_only_fields = Vec::new();
    if let Data::Struct(data) = input.data
        && let Fields::Named(fields) = data.fields
    {
        for field in fields.named.into_iter() {
            let mut type_name = parser::get_type_name(&field.ty);
            if let Some(ident) = field.ident
                && !type_name.is_empty()
            {
                let name = ident.to_string();
                let mut ignore = false;
                let mut not_null = false;
                let mut column_type = None;
                let mut default_value = None;
                let mut index_type = None;
                let mut reference = None;
                let mut comment = None;
                let mut extra_attributes = Vec::new();
                'inner: for attr in field.attrs.iter() {
                    let arguments = parser::parse_schema_attr(attr);
                    for (key, value) in arguments.into_iter() {
                        let key = key.as_str();
                        if !SPECIAL_ATTRIBUTES.contains(&key) {
                            let attribute_setter = if let Some(value) = value.as_ref() {
                                if let Ok(value) = value.parse::<i64>() {
                                    quote! { column.set_extra_attribute(#key, #value); }
                                } else if let Ok(value) = value.parse::<bool>() {
                                    quote! { column.set_extra_attribute(#key, #value); }
                                } else {
                                    quote! { column.set_extra_attribute(#key, #value); }
                                }
                            } else {
                                quote! { column.set_extra_attribute(#key, true); }
                            };
                            extra_attributes.push(attribute_setter);
                        }
                        if RESERVED_FIELDS.contains(&name.as_str()) {
                            extra_attributes.push(quote! {
                                column.set_extra_attribute("reserved", true);
                            });
                        }
                        match key {
                            "ignore" => {
                                ignore = true;
                                break 'inner;
                            }
                            "type_name" => {
                                if let Some(value) = value {
                                    type_name = value;
                                }
                            }
                            "column_type" => {
                                column_type = value;
                            }
                            "length" if type_name == "String" => {
                                if let Some(value) = value {
                                    column_type = Some(format!("CHAR({value})"));
                                }
                            }
                            "max_length" if type_name == "String" => {
                                if let Some(value) = value {
                                    column_type = Some(format!("VARCHAR({value})"));
                                }
                            }
                            "not_null" => {
                                not_null = true;
                            }
                            "default_value" => {
                                default_value = value;
                            }
                            "auto_increment" => {
                                default_value = Some("auto_increment".to_owned());
                            }
                            "auto_random" => {
                                default_value = Some("auto_random".to_owned());
                            }
                            "index_type" => {
                                index_type = value;
                            }
                            "reference" => {
                                reference = value;
                            }
                            "comment" => {
                                comment = value;
                            }
                            "primary_key" => {
                                primary_key_name = name.clone();
                            }
                            "read_only" => {
                                read_only_fields.push(quote! { #name });
                            }
                            "write_only" => {
                                write_only_fields.push(quote! { #name });
                            }
                            _ => (),
                        }
                    }
                }
                if ignore {
                    continue;
                }
                if primary_key_name == name {
                    primary_key_type = type_name.clone();
                    not_null = true;
                    extra_attributes.push(quote! {
                        column.set_extra_attribute("primary_key", true);
                    });
                } else if parser::check_option_type(&type_name) {
                    not_null = false;
                } else if INTEGER_TYPES.contains(&type_name.as_str()) {
                    default_value = default_value.or_else(|| Some("0".to_owned()));
                } else if let Some(value) = column_type {
                    extra_attributes.push(quote! {
                        column.set_extra_attribute("column_type", #value);
                    });
                }
                let quote_value = if let Some(value) = default_value {
                    if value.contains("::") {
                        if let Some((type_name, type_fn)) = value.split_once("::") {
                            let type_name_ident = format_ident!("{}", type_name);
                            let type_fn_ident = format_ident!("{}", type_fn);
                            extra_attributes.push(quote! {
                                let value = <#type_name_ident>::#type_fn_ident();
                                column.set_extra_attribute("default", value);
                            });
                            quote! { Some(<#type_name_ident>::#type_fn_ident().into()) }
                        } else {
                            extra_attributes.push(quote! {
                                column.set_extra_attribute("default", #value);
                            });
                            quote! { Some(#value) }
                        }
                    } else {
                        extra_attributes.push(quote! {
                            column.set_extra_attribute("default", #value);
                        });
                        quote! { Some(#value) }
                    }
                } else {
                    quote! { None }
                };
                let quote_index = if let Some(index) = index_type {
                    quote! { Some(#index) }
                } else {
                    quote! { None }
                };
                let quote_reference = if let Some(ref model_name) = reference {
                    let model_ident = format_ident!("{}", model_name);
                    quote! {{
                        let table_name = <#model_ident>::table_name();
                        let column_name = <#model_ident>::PRIMARY_KEY_NAME;
                        Some(zino_core::model::Reference::new(table_name, column_name))
                    }}
                } else {
                    quote! { None }
                };
                let quote_comment = if let Some(comment) = comment {
                    quote! { Some(#comment) }
                } else {
                    quote! { None }
                };
                let column = quote! {{
                    let mut column = zino_core::model::Column::new(#name, #type_name, #not_null);
                    if let Some(default_value) = #quote_value {
                        column.set_default_value(default_value);
                    }
                    if let Some(index_type) = #quote_index {
                        column.set_index_type(index_type);
                    }
                    if let Some(reference) = #quote_reference {
                        column.set_reference(reference);
                    }
                    if let Some(comment) = #quote_comment {
                        column.set_comment(comment);
                    }
                    #(#extra_attributes)*
                    column
                }};
                if primary_key_name == name {
                    let primary_key = if primary_key_type == "Uuid" {
                        quote! { self.primary_key().to_string() }
                    } else {
                        quote! { self.primary_key().clone() }
                    };
                    primary_key_value = Some(primary_key);
                    primary_key_column = Some(column.clone());
                }
                columns.push(column);
                column_fields.push(quote! { #name });
            }
        }
    }

    // Output
    let model_name_snake = model_name.to_case(Case::Snake);
    let model_name_upper_snake = model_name.to_case(Case::UpperSnake);
    let schema_primary_key_type = format_ident!("{}", primary_key_type);
    let schema_primary_key = format_ident!("{}", primary_key_name);
    let schema_primary_key_column = format_ident!("{}_PRIMARY_KEY_COLUMN", model_name_upper_snake);
    let schema_columns = format_ident!("{}_COLUMNS", model_name_upper_snake);
    let schema_fields = format_ident!("{}_FIELDS", model_name_upper_snake);
    let schema_read_only_fields = format_ident!("{}_READ_ONLY_FIELDS", model_name_upper_snake);
    let schema_write_only_fields = format_ident!("{}_WRITE_ONLY_FIELDS", model_name_upper_snake);
    let schema_reader = format_ident!("{}_READER", model_name_upper_snake);
    let schema_writer = format_ident!("{}_WRITER", model_name_upper_snake);
    let avro_schema = format_ident!("{}_AVRO_SCHEMA", model_name_upper_snake);
    let num_columns = columns.len();
    let num_read_only_fields = read_only_fields.len();
    let num_write_only_fields = write_only_fields.len();
    let quote_table_name = if let Some(table_name) = table_name {
        quote! { Some(#table_name) }
    } else {
        quote! { None }
    };
    let quote_model_comment = if let Some(comment) = model_comment {
        quote! { Some(#comment) }
    } else {
        quote! { None }
    };
    let output = quote! {
        use zino_core::{
            error::Error as ZinoError,
            model::{schema, Column},
            orm::{self, ConnectionPool, Schema},
        };

        static #avro_schema: std::sync::LazyLock<schema::Schema> = std::sync::LazyLock::new(|| {
            let mut fields = #schema_columns.iter().enumerate()
                .map(|(index, col)| {
                    let mut field = col.record_field();
                    field.position = index;
                    field
                })
                .collect::<Vec<_>>();
            let record_schema = schema::RecordSchema {
                name: schema::Name {
                    name: #model_name.to_owned(),
                    namespace: Some(<#name>::model_namespace().to_owned()),
                },
                aliases: None,
                doc: #quote_model_comment,
                fields,
                lookup: std::collections::BTreeMap::new(),
                attributes: std::collections::BTreeMap::new(),
            };
            schema::Schema::Record(record_schema)
        });
        static #schema_primary_key_column: std::sync::LazyLock<Column> =
            std::sync::LazyLock::new(|| #primary_key_column);
        static #schema_columns: std::sync::LazyLock<[Column; #num_columns]> =
            std::sync::LazyLock::new(|| [#(#columns),*]);
        static #schema_fields: std::sync::LazyLock<[&str; #num_columns]> =
            std::sync::LazyLock::new(|| [#(#column_fields),*]);
        static #schema_read_only_fields: std::sync::LazyLock<[&str; #num_read_only_fields]> =
            std::sync::LazyLock::new(|| [#(#read_only_fields),*]);
        static #schema_write_only_fields: std::sync::LazyLock<[&str; #num_write_only_fields]> =
            std::sync::LazyLock::new(|| [#(#write_only_fields),*]);
        static #schema_reader: std::sync::OnceLock<&ConnectionPool> = std::sync::OnceLock::new();
        static #schema_writer: std::sync::OnceLock<&ConnectionPool> = std::sync::OnceLock::new();

        impl Schema for #name {
            type PrimaryKey = #schema_primary_key_type;

            const MODEL_NAME: &'static str = #model_name_snake;
            const PRIMARY_KEY_NAME: &'static str = #primary_key_name;
            const READER_NAME: &'static str = #reader_name;
            const WRITER_NAME: &'static str = #writer_name;
            const TABLE_NAME: Option<&'static str> = #quote_table_name;

            #[inline]
            fn primary_key(&self) -> &Self::PrimaryKey {
                &self.#schema_primary_key
            }

            #[inline]
            fn primary_key_value(&self) -> zino_core::JsonValue {
                #primary_key_value.into()
            }

            #[inline]
            fn primary_key_column() -> &'static Column<'static> {
                std::sync::LazyLock::force(&#schema_primary_key_column)
            }

            #[inline]
            fn schema() -> &'static schema::Schema {
                std::sync::LazyLock::force(&#avro_schema)
            }

            #[inline]
            fn columns() -> &'static [Column<'static>] {
                #schema_columns.as_slice()
            }

            #[inline]
            fn fields() -> &'static [&'static str] {
                #schema_fields.as_slice()
            }

            #[inline]
            fn read_only_fields() -> &'static [&'static str] {
                #schema_read_only_fields.as_slice()
            }

            #[inline]
            fn write_only_fields() -> &'static [&'static str] {
                #schema_write_only_fields.as_slice()
            }

            async fn acquire_reader() -> Result<&'static ConnectionPool, ZinoError> {
                use zino_core::{bail, warn};
                if let Some(connection_pool) = #schema_reader.get() {
                    Ok(*connection_pool)
                } else {
                    let model_name = Self::MODEL_NAME;
                    let connection_pool = Self::init_reader()?;
                    if let Err(err) = Self::create_table().await {
                        connection_pool.store_availability(false);
                        bail!(
                            "503 Service Unavailable: fail to acquire reader for the model `{}`: {}",
                            model_name,
                            err
                        );
                    }
                    if let Err(err) = Self::synchronize_schema().await {
                        connection_pool.store_availability(false);
                        bail!(
                            "503 Service Unavailable: fail to acquire reader for the model `{}`: {}",
                            model_name,
                            err
                        );
                    }
                    if let Err(err) = Self::create_indexes().await {
                        connection_pool.store_availability(false);
                        bail!(
                            "503 Service Unavailable: fail to acquire reader for the model `{}`: {}",
                            model_name,
                            err
                        );
                    }
                    #schema_reader.set(connection_pool).map_err(|_| {
                        warn!(
                            "503 Service Unavailable: fail to acquire reader for the model `{}`",
                            model_name
                        )
                    })?;
                    Ok(connection_pool)
                }
            }

            async fn acquire_writer() -> Result<&'static ConnectionPool, ZinoError> {
                use zino_core::{bail, warn};
                if let Some(connection_pool) = #schema_writer.get() {
                    Ok(*connection_pool)
                } else {
                    let model_name = Self::MODEL_NAME;
                    let connection_pool = Self::init_writer()?;
                    if let Err(err) = Self::create_table().await {
                        connection_pool.store_availability(false);
                        bail!(
                            "503 Service Unavailable: fail to acquire writer for the model `{}`: {}",
                            model_name,
                            err
                        );
                    }
                    if let Err(err) = Self::synchronize_schema().await {
                        bail!(
                            "503 Service Unavailable: fail to acquire writer for the model `{}`: {}",
                            model_name,
                            err
                        );
                    }
                    if let Err(err) = Self::create_indexes().await {
                        bail!(
                            "503 Service Unavailable: fail to acquire writer for the model `{}`: {}",
                            model_name,
                            err
                        );
                    }
                    #schema_writer.set(connection_pool).map_err(|_| {
                        warn!(
                            "503 Service Unavailable: fail to acquire writer for the model `{}`",
                            model_name
                        )
                    })?;
                    Ok(connection_pool)
                }
            }
        }

        impl PartialEq for #name {
            #[inline]
            fn eq(&self, other: &Self) -> bool {
                self.#schema_primary_key == other.#schema_primary_key
            }
        }

        impl Eq for #name {}
    };

    TokenStream::from(output)
}

#[doc = include_str!("../docs/model_accessor.md")]
#[proc_macro_derive(ModelAccessor, attributes(schema))]
pub fn derive_model_accessor(item: TokenStream) -> TokenStream {
    // Input
    let input = parse_macro_input!(item as DeriveInput);

    // Parsing struct attributes
    let mut composite_constraints = Vec::new();
    for attr in input.attrs.iter() {
        for (key, value) in parser::parse_schema_attr(attr).into_iter() {
            if let Some(value) = value
                && key == "unique_on"
            {
                let mut fields = Vec::new();
                let column_values = value
                    .trim_start_matches('(')
                    .trim_end_matches(')')
                    .split(',')
                    .map(|s| {
                        let field = s.trim();
                        let field_ident = format_ident!("{}", field);
                        fields.push(field);
                        quote! {
                            (#field, self.#field_ident.to_string().into())
                        }
                    })
                    .collect::<Vec<_>>();
                let composite_field = fields.join("_");
                composite_constraints.push(quote! {
                    let columns = [#(#column_values),*];
                    if !self.is_unique_on(columns).await? {
                        validation.record(#composite_field, "the composite values should be unique");
                    }
                });
            }
        }
    }

    // Parsing field attributes
    let name = input.ident;
    let mut column_methods = Vec::new();
    let mut snapshot_fields = Vec::new();
    let mut snapshot_entries = Vec::new();
    let mut field_constraints = Vec::new();
    let mut populated_queries = Vec::new();
    let mut populated_one_queries = Vec::new();
    let mut primary_key_type = String::from("Uuid");
    let mut primary_key_name = String::from("id");
    let mut user_id_type = String::new();
    if let Data::Struct(data) = input.data
        && let Fields::Named(fields) = data.fields
    {
        let mut model_references: Vec<(String, Vec<String>)> = Vec::new();
        for field in fields.named.into_iter() {
            let type_name = parser::get_type_name(&field.ty);
            if let Some(ident) = field.ident
                && !type_name.is_empty()
            {
                let name = ident.to_string();
                let mut field_alias = None;
                for attr in field.attrs.iter() {
                    let arguments = parser::parse_schema_attr(attr);
                    let is_readable = arguments.iter().all(|arg| arg.0 != "write_only");
                    for (key, value) in arguments.into_iter() {
                        match key.as_str() {
                            "alias" => {
                                field_alias = value;
                            }
                            "primary_key" => {
                                primary_key_name = name.clone();
                            }
                            "snapshot" => {
                                let field = name.clone();
                                let field_ident = format_ident!("{}", field);
                                if type_name == "Uuid" {
                                    snapshot_entries.push(quote! {
                                        snapshot.upsert(#field, self.#field_ident.to_string());
                                    });
                                } else if type_name == "Option<Uuid>" {
                                    snapshot_entries.push(quote! {
                                        let snapshot_value = self.#field_ident
                                            .map(|v| v.to_string());
                                        snapshot.upsert(#field, snapshot_value);
                                    });
                                } else if type_name == "Vec<Uuid>" {
                                    snapshot_entries.push(quote! {
                                        let snapshot_value = self.#field_ident.iter()
                                            .map(|v| v.to_string())
                                            .collect::<Vec<_>>();
                                        snapshot.upsert(#field, snapshot_value);
                                    });
                                } else {
                                    snapshot_entries.push(quote! {
                                        snapshot.upsert(#field, self.#field_ident.clone());
                                    });
                                }
                                snapshot_fields.push(field);
                            }
                            "reference" => {
                                if let Some(value) = value {
                                    let model_ident = format_ident!("{}", value);
                                    if type_name == "Uuid" {
                                        field_constraints.push(quote! {
                                            let values = vec![self.#ident.to_string()];
                                            let data = <#model_ident>::filter(values).await?;
                                            if data.len() != 1 {
                                                validation.record(#name, "it is a nonexistent value");
                                            }
                                        });
                                    } else if type_name == "Option<Uuid>"
                                        || type_name == "Option<String>"
                                    {
                                        field_constraints.push(quote! {
                                            if let Some(value) = self.#ident {
                                                let values = vec![value.to_string()];
                                                let data = <#model_ident>::filter(values).await?;
                                                if data.len() != 1 {
                                                    validation.record(#name, "it is a nonexistent value");
                                                }
                                            }
                                        });
                                    } else if type_name == "Vec<Uuid>" || type_name == "Vec<String>"
                                    {
                                        field_constraints.push(quote! {
                                            let values = self.#ident
                                                .iter()
                                                .map(|v| v.to_string())
                                                .collect::<Vec<_>>();
                                            let length = values.len();
                                            if length > 0 {
                                                let data = <#model_ident>::filter(values).await?;
                                                if data.len() != length {
                                                    validation.record(#name, "there are nonexistent values");
                                                }
                                            }
                                        });
                                    } else if parser::check_vec_type(&type_name) {
                                        field_constraints.push(quote! {
                                            let values = self.#ident.clone();
                                            let length = values.len();
                                            if length > 0 {
                                                let data = <#model_ident>::filter(values).await?;
                                                if data.len() != length {
                                                    validation.record(#name, "there are nonexistent values");
                                                }
                                            }
                                        });
                                    } else if parser::check_option_type(&type_name) {
                                        field_constraints.push(quote! {
                                            if let Some(value) = self.#ident {
                                                let values = vec![value.clone()];
                                                let data = <#model_ident>::filter(values).await?;
                                                if data.len() != 1 {
                                                    validation.record(#name, "it is a nonexistent value");
                                                }
                                            }
                                        });
                                    } else {
                                        field_constraints.push(quote! {
                                            let values = vec![self.#ident.clone()];
                                            let data = <#model_ident>::filter(values).await?;
                                            if data.len() != 1 {
                                                validation.record(#name, "it is a nonexistent value");
                                            }
                                        });
                                    }
                                    match model_references.iter_mut().find(|r| r.0 == value) {
                                        Some(r) => r.1.push(name.clone()),
                                        None => model_references.push((value, vec![name.clone()])),
                                    }
                                }
                            }
                            "unique" => {
                                if type_name == "Uuid" {
                                    field_constraints.push(quote! {
                                        let value = self.#ident;
                                        if !value.is_nil() {
                                            let columns = [(#name, value.to_string().into())];
                                            if !self.is_unique_on(columns).await? {
                                                let message = format!("the value `{value}` is not unique");
                                                validation.record(#name, message);
                                            }
                                        }
                                    });
                                } else if type_name == "String" {
                                    field_constraints.push(quote! {
                                        let value = self.#ident.as_str();
                                        if !value.is_empty() {
                                            let columns = [(#name, value.into())];
                                            if !self.is_unique_on(columns).await? {
                                                let message = format!("the value `{value}` is not unique");
                                                validation.record(#name, message);
                                            }
                                        }
                                    });
                                } else if type_name == "Option<String>" {
                                    field_constraints.push(quote! {
                                        if let Some(value) = self.#ident.as_deref() && !value.is_empty() {
                                            let columns = [(#name, value.into())];
                                            if !self.is_unique_on(columns).await? {
                                                let message = format!("the value `{value}` is not unique");
                                                validation.record(#name, message);
                                            }
                                        }
                                    });
                                } else if type_name == "Option<Uuid>" {
                                    field_constraints.push(quote! {
                                        if let Some(value) = self.#ident && !value.is_nil() {
                                            let columns = [(#name, value.to_string().into())];
                                            if !self.is_unique_on(columns).await? {
                                                let message = format!("the value `{value}` is not unique");
                                                validation.record(#name, message);
                                            }
                                        }
                                    });
                                } else if parser::check_option_type(&type_name) {
                                    field_constraints.push(quote! {
                                        if let Some(value) = self.#ident {
                                            let columns = [(#name, value.into())];
                                            if !self.is_unique_on(columns).await? {
                                                let message = format!("the value `{value}` is not unique");
                                                validation.record(#name, message);
                                            }
                                        }
                                    });
                                } else {
                                    field_constraints.push(quote! {
                                        let value = self.#ident;
                                        let columns = [(#name, value.into())];
                                        if !self.is_unique_on(columns).await? {
                                            let message = format!("the value `{value}` is not unique");
                                            validation.record(#name, message);
                                        }
                                    });
                                }
                            }
                            "not_null" if is_readable => {
                                if type_name == "String" {
                                    field_constraints.push(quote! {
                                        if self.#ident.is_empty() {
                                            validation.record(#name, "it should be nonempty");
                                        }
                                    });
                                } else if type_name == "Uuid" {
                                    field_constraints.push(quote! {
                                        if self.#ident.is_nil() {
                                            validation.record(#name, "it should not be nil");
                                        }
                                    });
                                }
                            }
                            "nonempty" if is_readable => {
                                if parser::check_vec_type(&type_name)
                                    || matches!(type_name.as_str(), "String" | "Map")
                                {
                                    field_constraints.push(quote! {
                                        if self.#ident.is_empty() {
                                            validation.record(#name, "it should be nonempty");
                                        }
                                    });
                                }
                            }
                            "format" if type_name == "String" => {
                                field_constraints.push(quote! {
                                    if !self.#ident.is_empty() {
                                        validation.validate_format(#name, self.#ident.as_str(), #value);
                                    }
                                });
                            }
                            "length" => {
                                let length = value
                                    .and_then(|s| s.parse::<usize>().ok())
                                    .unwrap_or_default();
                                if type_name == "String" {
                                    field_constraints.push(quote! {
                                        let length = #length;
                                        if self.#ident.len() != length {
                                            let message = format!("the length should be {length}");
                                            validation.record(#name, message);
                                        }
                                    });
                                } else if type_name == "Option<String>" {
                                    field_constraints.push(quote! {
                                        let length = #length;
                                        if let Some(ref s) = self.#ident && s.len() != length {
                                            let message = format!("the length should be {length}");
                                            validation.record(#name, message);
                                        }
                                    });
                                }
                            }
                            "max_length" => {
                                let length = value
                                    .and_then(|s| s.parse::<usize>().ok())
                                    .unwrap_or_default();
                                if type_name == "String" {
                                    field_constraints.push(quote! {
                                        let length = #length;
                                        if self.#ident.len() > length {
                                            let message = format!("the length should be at most {length}");
                                            validation.record(#name, message);
                                        }
                                    });
                                } else if type_name == "Option<String>" {
                                    field_constraints.push(quote! {
                                        let length = #length;
                                        if let Some(ref s) = self.#ident && s.len() > length {
                                            let message = format!("the length should be at most {length}");
                                            validation.record(#name, message);
                                        }
                                    });
                                }
                            }
                            "min_length" => {
                                let length = value
                                    .and_then(|s| s.parse::<usize>().ok())
                                    .unwrap_or_default();
                                if type_name == "String" {
                                    field_constraints.push(quote! {
                                        let length = #length;
                                        if self.#ident.len() < length {
                                            let message = format!("the length should be at least {length}");
                                            validation.record(#name, message);
                                        }
                                    });
                                } else if type_name == "Option<String>" {
                                    field_constraints.push(quote! {
                                        let length = #length;
                                        if let Some(ref s) = self.#ident && s.len() < length {
                                            let message = format!("the length should be at least {length}");
                                            validation.record(#name, message);
                                        }
                                    });
                                }
                            }
                            "max_items" => {
                                if let Some(length) = value.and_then(|s| s.parse::<usize>().ok())
                                    && parser::check_vec_type(&type_name)
                                {
                                    field_constraints.push(quote! {
                                        let length = #length;
                                        if self.#ident.len() > length {
                                            let message = format!("the length should be at most {length}");
                                            validation.record(#name, message);
                                        }
                                    });
                                }
                            }
                            "min_items" => {
                                if let Some(length) = value.and_then(|s| s.parse::<usize>().ok())
                                    && parser::check_vec_type(&type_name)
                                {
                                    field_constraints.push(quote! {
                                        let length = #length;
                                        if self.#ident.len() < length {
                                            let message = format!("the length should be at least {length}");
                                            validation.record(#name, message);
                                        }
                                    });
                                }
                            }
                            "unique_items" => {
                                if parser::check_vec_type(&type_name) {
                                    field_constraints.push(quote! {
                                        let slice = self.#ident.as_slice();
                                        for index in 1..slice.len() {
                                            if slice[index..].contains(&slice[index - 1]) {
                                                let message = format!("array items should be unique");
                                                validation.record(#name, message);
                                                break;
                                            }
                                        }
                                    });
                                }
                            }
                            _ => (),
                        }
                    }
                }
                if primary_key_name == name {
                    primary_key_type = type_name;
                } else {
                    let name_ident = format_ident!("{}", name);
                    let mut snapshot_field = None;
                    match field_alias.as_deref().unwrap_or(name.as_str()) {
                        "name" | "status" => {
                            let method = quote! {
                                #[inline]
                                fn #name_ident(&self) -> &str {
                                    self.#name_ident.as_ref()
                                }
                            };
                            column_methods.push(method);
                            snapshot_field = Some(name.as_str());
                        }
                        "namespace" | "visibility" | "description" => {
                            let method = quote! {
                                #[inline]
                                fn #name_ident(&self) -> &str {
                                    self.#name_ident.as_ref()
                                }
                            };
                            column_methods.push(method);
                        }
                        "content" | "extra" if type_name == "Map" => {
                            let method = quote! {
                                #[inline]
                                fn #name_ident(&self) -> Option<&Map> {
                                    let map = &self.#name_ident;
                                    (!map.is_empty()).then_some(map)
                                }
                            };
                            column_methods.push(method);
                        }
                        "owner_id" | "maintainer_id" => {
                            let user_type_opt = type_name.strip_prefix("Option");
                            let user_type = if let Some(user_type) = user_type_opt {
                                user_type.trim_matches(|c| c == '<' || c == '>').to_owned()
                            } else {
                                type_name.clone()
                            };
                            let user_type_ident = format_ident!("{}", user_type);
                            let method = if user_type_opt.is_some() {
                                quote! {
                                    #[inline]
                                    fn #name_ident(&self) -> Option<&#user_type_ident> {
                                        self.#name_ident.as_ref()
                                    }
                                }
                            } else {
                                quote! {
                                    #[inline]
                                    fn #name_ident(&self) -> Option<&#user_type_ident> {
                                        let id = &self.#name_ident;
                                        (id != &#user_type_ident::default()).then_some(id)
                                    }
                                }
                            };
                            column_methods.push(method);
                            user_id_type = user_type;
                        }
                        "created_at" if type_name == "DateTime" => {
                            let method = quote! {
                                #[inline]
                                fn #name_ident(&self) -> DateTime {
                                    self.#name_ident
                                }
                            };
                            column_methods.push(method);
                        }
                        "updated_at" if type_name == "DateTime" => {
                            let method = quote! {
                                #[inline]
                                fn #name_ident(&self) -> DateTime {
                                    self.#name_ident
                                }
                            };
                            column_methods.push(method);
                            snapshot_field = Some("updated_at");
                        }
                        "version" if type_name == "u64" => {
                            let method = quote! {
                                #[inline]
                                fn #name_ident(&self) -> u64 {
                                    self.#name_ident
                                }
                            };
                            column_methods.push(method);
                            snapshot_field = Some("version");
                        }
                        "edition" if type_name == "u32" => {
                            let method = quote! {
                                #[inline]
                                fn #name_ident(&self) -> u32 {
                                    self.#name_ident
                                }
                            };
                            column_methods.push(method);
                        }
                        _ => (),
                    }
                    if let Some(field) = snapshot_field {
                        let field_ident = format_ident!("{}", field);
                        snapshot_entries.push(quote! {
                            snapshot.upsert(#field, self.#field_ident.clone());
                        });
                        snapshot_fields.push(field.to_owned());
                    }
                }
            }
        }
        if model_references.is_empty() {
            populated_queries.push(quote! {
                let mut models = Self::find::<Map>(query).await?;
                for model in models.iter_mut() {
                    Self::after_decode(model).await?;
                    translate_enabled.then(|| Self::translate_model(model));
                }
            });
            populated_one_queries.push(quote! {
                let mut model = Self::find_by_id::<Map>(id)
                    .await?
                    .ok_or_else(|| zino_core::warn!("404 Not Found: cannot find the model `{}`", id))?;
                Self::after_decode(&mut model).await?;
                Self::translate_model(&mut model);
            });
        } else {
            populated_queries.push(quote! {
                let mut models = Self::find::<Map>(query).await?;
                for model in models.iter_mut() {
                    Self::after_decode(model).await?;
                    translate_enabled.then(|| Self::translate_model(model));
                }
            });
            populated_one_queries.push(quote! {
                let mut model = Self::find_by_id::<Map>(id)
                    .await?
                    .ok_or_else(|| zino_core::warn!("404 Not Found: cannot find the model `{}`", id))?;
                Self::after_decode(&mut model).await?;
                Self::translate_model(&mut model);
            });
            for (model, ref_fields) in model_references.into_iter() {
                let model_ident = format_ident!("{}", model);
                let populated_query = quote! {
                    let mut query = #model_ident::default_snapshot_query();
                    query.add_filter("translate", translate_enabled);
                    #model_ident::populate(&mut query, &mut models, [#(#ref_fields),*]).await?;
                };
                let populated_one_query = quote! {
                    let mut query = #model_ident::default_query();
                    query.add_filter("translate", true);
                    #model_ident::populate_one(&mut query, &mut model, [#(#ref_fields),*]).await?;
                };
                populated_queries.push(populated_query);
                populated_one_queries.push(populated_one_query);
            }
        }
        populated_queries.push(quote! { Ok(models) });
        populated_one_queries.push(quote! { Ok(model) });
    }
    if user_id_type.is_empty() {
        user_id_type = primary_key_type.clone();
    }

    // Output
    let model_primary_key_type = format_ident!("{}", primary_key_type);
    let model_primary_key = format_ident!("{}", primary_key_name);
    let model_user_id_type = format_ident!("{}", user_id_type);
    let output = quote! {
        use zino_core::{
            model::Query,
            orm::{ModelAccessor, ModelHelper as _},
            validation::Validation as ZinoValidation,
            Map as ZinoMap,
        };

        impl ModelAccessor<#model_primary_key_type, #model_user_id_type> for #name {
            #[inline]
            fn id(&self) -> &#model_primary_key_type {
                &self.#model_primary_key
            }

            #(#column_methods)*

            fn snapshot(&self) -> Map {
                let mut snapshot = Map::new();
                snapshot.upsert(Self::PRIMARY_KEY_NAME, self.primary_key_value());
                #(#snapshot_entries)*
                snapshot
            }

            fn default_snapshot_query() -> Query {
                let mut query = Self::default_query();
                let fields = [
                    Self::PRIMARY_KEY_NAME,
                    #(#snapshot_fields),*
                ];
                query.allow_fields(&fields);
                query
            }

            async fn check_constraints(&self) -> Result<ZinoValidation, ZinoError> {
                let mut validation = ZinoValidation::new();
                if self.id() == &<#model_primary_key_type>::default()
                    && !Self::primary_key_column().auto_increment()
                {
                    validation.record(Self::PRIMARY_KEY_NAME, "should not be a default value");
                }
                #(#composite_constraints)*
                #(#field_constraints)*
                Ok(validation)
            }

            async fn fetch(query: &Query) -> Result<Vec<ZinoMap>, ZinoError> {
                let translate_enabled = query.translate_enabled();
                #(#populated_queries)*
            }

            async fn fetch_by_id(id: &#model_primary_key_type) -> Result<ZinoMap, ZinoError> {
                #(#populated_one_queries)*
            }
        }
    };

    TokenStream::from(output)
}

#[doc = include_str!("../docs/decode_row.md")]
#[proc_macro_derive(DecodeRow, attributes(schema))]
pub fn derive_decode_row(item: TokenStream) -> TokenStream {
    /// Integer types
    const UNSIGNED_INTEGER_TYPES: [&str; 5] = ["u64", "u32", "u16", "u8", "usize"];

    // Input
    let input = parse_macro_input!(item as DeriveInput);

    // Parsing field attributes
    let name = input.ident;
    let mut decode_model_fields = Vec::new();
    if let Data::Struct(data) = input.data
        && let Fields::Named(fields) = data.fields
    {
        for field in fields.named.into_iter() {
            let type_name = parser::get_type_name(&field.ty);
            if let Some(ident) = field.ident
                && !type_name.is_empty()
            {
                let name = ident.to_string();
                let mut ignore = false;
                'inner: for attr in field.attrs.iter() {
                    let arguments = parser::parse_schema_attr(attr);
                    for (key, _value) in arguments.iter() {
                        if key == "ignore" || key == "write_only" {
                            ignore = true;
                            break 'inner;
                        }
                    }
                }
                if ignore {
                    continue;
                }
                if type_name == "Map" {
                    decode_model_fields.push(quote! {
                        if let JsonValue::Object(map) = orm::decode(row, #name)? {
                            model.#ident = map;
                        }
                    });
                } else if parser::check_vec_type(&type_name) {
                    decode_model_fields.push(quote! {
                        model.#ident = orm::decode_array(row, #name)?;
                    });
                } else if UNSIGNED_INTEGER_TYPES.contains(&type_name.as_str()) {
                    let integer_type_ident = format_ident!("{}", type_name.replace('u', "i"));
                    decode_model_fields.push(quote! {
                        let value = orm::decode::<#integer_type_ident>(row, #name)?;
                        model.#ident = value.try_into()?;
                    });
                } else {
                    decode_model_fields.push(quote! {
                        model.#ident = orm::decode(row, #name)?;
                    });
                }
            }
        }
    }

    // Output
    let output = quote! {
        impl zino_core::model::DecodeRow<zino_core::orm::DatabaseRow> for #name {
            type Error = zino_core::error::Error;

            fn decode_row(row: &zino_core::orm::DatabaseRow) -> Result<Self, Self::Error> {
                use zino_core::{extension::JsonValueExt, orm, JsonValue};

                let mut model = Self::default();
                #(#decode_model_fields)*
                Ok(model)
            }
        }
    };

    TokenStream::from(output)
}

#[doc = include_str!("../docs/model_hooks.md")]
#[proc_macro_derive(ModelHooks)]
pub fn derive_model_hooks(item: TokenStream) -> TokenStream {
    // Input
    let input = parse_macro_input!(item as DeriveInput);

    // Parsing field attributes
    let name = input.ident;

    // Output
    let output = quote! {
        impl zino_core::model::ModelHooks for #name {
            type Data = ();
            type Extension = ();
        }
    };

    TokenStream::from(output)
}

#[doc = include_str!("../docs/model.md")]
#[proc_macro_derive(Model)]
pub fn derive_model(item: TokenStream) -> TokenStream {
    // Reserved fields
    const RESERVED_FIELDS: [&str; 4] = ["created_at", "updated_at", "version", "edition"];

    // Input
    let input = parse_macro_input!(item as DeriveInput);

    // Model name
    let name = input.ident;

    // Parsing field attributes
    let mut field_constructors = Vec::new();
    let mut field_setters = Vec::new();
    if let Data::Struct(data) = input.data
        && let Fields::Named(fields) = data.fields
    {
        for field in fields.named.into_iter() {
            let type_name = parser::get_type_name(&field.ty);
            if let Some(ident) = field.ident
                && !type_name.is_empty()
            {
                let name = ident.to_string();
                let mut enable_setter = true;
                for attr in field.attrs.iter() {
                    let arguments = parser::parse_schema_attr(attr);
                    for (key, value) in arguments.into_iter() {
                        match key.as_str() {
                            "constructor" => {
                                if let Some(value) = value
                                    && let Some((cons_name, cons_fn)) = value.split_once("::")
                                {
                                    let cons_name_ident = format_ident!("{}", cons_name);
                                    let cons_fn_ident = format_ident!("{}", cons_fn);
                                    let constructor = if type_name == "String" {
                                        quote! {
                                            model.#ident = <#cons_name_ident>::#cons_fn_ident().to_string();
                                        }
                                    } else {
                                        quote! {
                                            model.#ident = <#cons_name_ident>::#cons_fn_ident().into();
                                        }
                                    };
                                    field_constructors.push(constructor);
                                }
                            }
                            "read_only" | "generated" | "reserved" => {
                                enable_setter = false;
                            }
                            _ => (),
                        }
                    }
                }
                if enable_setter && !RESERVED_FIELDS.contains(&name.as_str()) {
                    let setter = if type_name == "String" {
                        if name == "password" {
                            quote! {
                                if let Some(password) = data.parse_string("password") {
                                    use zino_core::orm::ModelHelper;
                                    match Self::encrypt_password(&password) {
                                        Ok(password) => self.password = password,
                                        Err(err) => validation.record_fail("password", err),
                                    }
                                }
                            }
                        } else {
                            quote! {
                                if let Some(value) = data.parse_string(#name) {
                                    self.#ident = value.into_owned();
                                }
                            }
                        }
                    } else if type_name == "Vec<String>" {
                        quote! {
                            if let Some(values) = data.parse_str_array(#name) {
                                self.#ident = values.into_iter().map(|s| s.to_owned()).collect();
                            }
                        }
                    } else if type_name == "Option<String>" {
                        quote! {
                            if let Some(value) = data.parse_string(#name) {
                                self.#ident = Some(value.into_owned());
                            }
                        }
                    } else if type_name == "Map" {
                        quote! {
                            if let Some(values) = data.parse_object(#name) {
                                self.#ident = values.clone();
                            }
                        }
                    } else if parser::check_vec_type(&type_name) {
                        quote! {
                            if let Some(values) = data.parse_array(#name) {
                                self.#ident = values;
                            }
                        }
                    } else if let Some(type_generics) = parser::parse_option_type(&type_name) {
                        let parser_ident = format_ident!("parse_{}", type_generics.to_lowercase());
                        quote! {
                            if let Some(result) = data.#parser_ident(#name) {
                                match result {
                                    Ok(value) => self.#ident = Some(value),
                                    Err(err) => {
                                        let raw_value = data.parse_string(#name);
                                        let raw_value_str = raw_value
                                            .as_deref()
                                            .unwrap_or_default();
                                        let message = format!("{err}: `{raw_value_str}`");
                                        validation.record(#name, message);
                                    },
                                }
                            }
                        }
                    } else {
                        let parser_ident = format_ident!("parse_{}", type_name.to_lowercase());
                        quote! {
                            if let Some(result) = data.#parser_ident(#name) {
                                match result {
                                    Ok(value) => self.#ident = value,
                                    Err(err) => {
                                        let raw_value = data.parse_string(#name);
                                        let raw_value_str = raw_value
                                            .as_deref()
                                            .unwrap_or_default();
                                        let message = format!("{err}: `{raw_value_str}`");
                                        validation.record(#name, message);
                                    },
                                }
                            }
                        }
                    };
                    field_setters.push(setter);
                }
            }
        }
    }

    // Output
    let model_constructor = if field_constructors.is_empty() {
        quote! { Self::default() }
    } else {
        quote! {
            let mut model = Self::default();
            #(#field_constructors)*
            model
        }
    };
    let output = quote! {
        use zino_core::validation::Validation;

        impl zino_core::model::Model for #name {
            #[inline]
            fn new() -> Self {
                #model_constructor
            }

            #[must_use]
            fn read_map(&mut self, data: &Map) -> Validation {
                let mut validation = Validation::new();
                if data.is_empty() {
                    validation.record("data", "should be nonempty");
                } else {
                    #(#field_setters)*
                }
                validation
            }
        }
    };

    TokenStream::from(output)
}
