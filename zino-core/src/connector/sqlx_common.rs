use crate::{error::Error, helper, AvroValue, Map, Record};
use futures::TryStreamExt;
use serde::{
    de::DeserializeOwned,
    ser::{self, Serialize, SerializeMap, Serializer},
};
use sqlx::{database::HasValueRef, Column, ColumnIndex, Database, Decode, Row, TypeInfo, ValueRef};
use std::borrow::Cow;

/// A generic struct for the row.
pub(super) struct SerializeRow<R: Row>(pub(super) R);

impl<'r, R: Row> Serialize for &'r SerializeRow<R>
where
    usize: ColumnIndex<R>,
    Cow<'r, str>: Decode<'r, <R as Row>::Database>,
    f64: Decode<'r, <R as Row>::Database>,
    i64: Decode<'r, <R as Row>::Database>,
    bool: Decode<'r, <R as Row>::Database>,
{
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let columns = self.0.columns();
        let mut map = serializer.serialize_map(Some(columns.len()))?;
        for col in columns {
            let key = col.name();
            match self.0.try_get_raw(col.ordinal()) {
                Ok(raw_value) if !raw_value.is_null() => match raw_value.type_info().name() {
                    "BOOL" | "BOOLEAN" => map_serialize::<_, _, bool>(&mut map, key, raw_value)?,
                    "DOUBLE" | "DOUBLE PRECISION" | "FLOAT" | "FLOAT4" | "FLOAT8" | "NUMERIC"
                    | "REAL" => map_serialize::<_, _, f64>(&mut map, key, raw_value)?,
                    "BIGINT" | "BIGINT UNSIGNED" | "BIGSERIAL" | "INT" | "INT2" | "INT4"
                    | "INT8" | "INTEGER" | "INT UNSIGNED" | "SERIAL" | "SMALLINT"
                    | "SMALLINT UNSIGNED" | "SMALLSERIAL" | "TINYINT" | "TINYINT UNSIGNED" => {
                        map_serialize::<_, _, i64>(&mut map, key, raw_value)?
                    }
                    // Deserialize as a string by default
                    _ => map_serialize::<_, _, Cow<'_, str>>(&mut map, key, raw_value)?,
                },
                _ => map.serialize_entry(key, &())?, // Serialize null
            }
        }
        map.end()
    }
}

fn map_serialize<'r, M: SerializeMap, DB: Database, T: Decode<'r, DB> + Serialize>(
    map: &mut M,
    key: &str,
    raw_value: <DB as HasValueRef<'r>>::ValueRef,
) -> Result<(), M::Error> {
    let value = T::decode(raw_value).map_err(ser::Error::custom)?;
    map.serialize_entry(key, &value)
}

pub(super) macro impl_sqlx_connector($pool:ty) {
    async fn execute(&self, query: &str, params: Option<&Map>) -> Result<Option<u64>, Error> {
        let (sql, values) = helper::prepare_sql_query(query, params, '?');
        let mut query = sqlx::query(&sql);
        for value in values {
            query = query.bind(value.to_string());
        }

        let query_result = query.execute(self).await?;
        Ok(Some(query_result.rows_affected()))
    }

    async fn query(&self, query: &str, params: Option<&Map>) -> Result<Vec<Record>, Error> {
        let (sql, values) = helper::prepare_sql_query(query, params, '?');
        let mut query = sqlx::query(&sql);
        for value in values {
            query = query.bind(value.to_string());
        }

        let mut rows = query.fetch(self);
        let mut records = Vec::new();
        while let Some(row) = rows.try_next().await? {
            let value = apache_avro::to_value(&SerializeRow(row))?;
            if let AvroValue::Record(record) = value {
                records.push(record);
            }
        }
        Ok(records)
    }

    async fn query_as<T: DeserializeOwned>(
        &self,
        query: &str,
        params: Option<&Map>,
    ) -> Result<Vec<T>, Error> {
        let (sql, values) = helper::prepare_sql_query(query, params, '?');
        let mut query = sqlx::query(&sql);
        for value in values {
            query = query.bind(value.to_string());
        }

        let mut rows = query.fetch(self);
        let mut data = Vec::new();
        while let Some(row) = rows.try_next().await? {
            let json_value = serde_json::to_value(&SerializeRow(row))?;
            let value = serde_json::from_value(json_value)?;
            data.push(value);
        }
        Ok(data)
    }

    async fn query_one(&self, query: &str, params: Option<&Map>) -> Result<Option<Record>, Error> {
        let (sql, values) = helper::prepare_sql_query(query, params, '?');
        let mut query = sqlx::query(&sql);
        for value in values {
            query = query.bind(value.to_string());
        }

        let data = if let Some(row) = query.fetch_optional(self).await? {
            let value = apache_avro::to_value(&SerializeRow(row))?;
            if let AvroValue::Record(record) = value {
                Some(record)
            } else {
                None
            }
        } else {
            None
        };
        Ok(data)
    }

    async fn query_one_as<T: DeserializeOwned>(
        &self,
        query: &str,
        params: Option<&Map>,
    ) -> Result<Option<T>, Error> {
        let (sql, values) = helper::prepare_sql_query(query, params, '?');
        let mut query = sqlx::query(&sql);
        for value in values {
            query = query.bind(value.to_string());
        }

        if let Some(row) = query.fetch_optional(self).await? {
            let json_value = serde_json::to_value(&SerializeRow(row))?;
            serde_json::from_value(json_value).map_err(Error::from)
        } else {
            Ok(None)
        }
    }
}
