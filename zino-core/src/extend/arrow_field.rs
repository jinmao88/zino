use crate::{extend::AvroRecordExt, BoxError, Record};
use apache_avro::types::Value;
use datafusion::arrow::{
    array::{
        self, Array, BinaryArray, BooleanArray, Float32Array, Float64Array, Int32Array, Int64Array,
        LargeBinaryArray, LargeStringArray, StringArray, UInt16Array, UInt32Array, UInt64Array,
    },
    datatypes::{DataType, Field},
};
use std::sync::Arc;

/// Extension trait for [`Field`](datafusion::arrow::datatypes::Field).
pub trait ArrowFieldExt {
    /// Attempts to create a `Field` from an Avro record entry.
    fn try_from_avro_record_entry(field: &str, value: &Value) -> Result<Field, BoxError>;

    /// Collects values in the Avro records with the specific field.
    fn collect_values_from_avro_records(&self, records: &[Record]) -> Arc<dyn Array + 'static>;
}

impl ArrowFieldExt for Field {
    fn try_from_avro_record_entry(field: &str, value: &Value) -> Result<Field, BoxError> {
        let data_type = match value {
            Value::Boolean(_) => DataType::Boolean,
            Value::Int(_) => DataType::Int32,
            Value::Long(_) => DataType::Int64,
            Value::Float(_) => DataType::Float32,
            Value::Double(_) => DataType::Float64,
            Value::Bytes(_) => DataType::Binary,
            Value::String(_) | Value::Uuid(_) => DataType::Utf8,
            _ => {
                let message = format!("fail to construct an Arrow field for the `{field}` field");
                return Err(message.into());
            }
        };
        Ok(Field::new(field, data_type, true))
    }

    fn collect_values_from_avro_records(&self, records: &[Record]) -> Arc<dyn Array + 'static> {
        let field = self.name().as_str();
        match self.data_type() {
            DataType::Boolean => {
                let values = records
                    .iter()
                    .map(|record| record.get_bool(field))
                    .collect::<Vec<_>>();
                Arc::new(BooleanArray::from(values))
            }
            DataType::Int32 => {
                let values = records
                    .iter()
                    .map(|record| record.get_i32(field))
                    .collect::<Vec<_>>();
                Arc::new(Int32Array::from(values))
            }
            DataType::Int64 => {
                let values = records
                    .iter()
                    .map(|record| record.get_i64(field))
                    .collect::<Vec<_>>();
                Arc::new(Int64Array::from(values))
            }
            DataType::UInt16 => {
                let values = records
                    .iter()
                    .map(|record| record.get_u16(field))
                    .collect::<Vec<_>>();
                Arc::new(UInt16Array::from(values))
            }
            DataType::UInt32 => {
                let values = records
                    .iter()
                    .map(|record| record.get_u32(field))
                    .collect::<Vec<_>>();
                Arc::new(UInt32Array::from(values))
            }
            DataType::UInt64 => {
                let values = records
                    .iter()
                    .map(|record| record.get_u64(field))
                    .collect::<Vec<_>>();
                Arc::new(UInt64Array::from(values))
            }
            DataType::Float32 => {
                let values = records
                    .iter()
                    .map(|record| record.get_f32(field))
                    .collect::<Vec<_>>();
                Arc::new(Float32Array::from(values))
            }
            DataType::Float64 => {
                let values = records
                    .iter()
                    .map(|record| record.get_f64(field))
                    .collect::<Vec<_>>();
                Arc::new(Float64Array::from(values))
            }
            DataType::Binary => {
                let values = records
                    .iter()
                    .map(|record| record.get_bytes(field))
                    .collect::<Vec<_>>();
                Arc::new(BinaryArray::from(values))
            }
            DataType::LargeBinary => {
                let values = records
                    .iter()
                    .map(|record| record.get_bytes(field))
                    .collect::<Vec<_>>();
                Arc::new(LargeBinaryArray::from(values))
            }
            DataType::Utf8 => {
                let values = records
                    .iter()
                    .map(|record| record.get_str(field))
                    .collect::<Vec<_>>();
                Arc::new(StringArray::from(values))
            }
            DataType::LargeUtf8 => {
                let values = records
                    .iter()
                    .map(|record| record.get_str(field))
                    .collect::<Vec<_>>();
                Arc::new(LargeStringArray::from(values))
            }
            data_type => array::new_null_array(data_type, records.len()),
        }
    }
}
