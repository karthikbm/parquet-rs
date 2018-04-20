// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

//! Contains Row enum that is used to represent record in Rust.

use std::fmt;

use basic::{LogicalType, Type as PhysicalType};
use chrono::{Local, TimeZone};
use data_type::{ByteArray, Int96};

/// Macro as a shortcut to generate 'not yet implemented' panic error.
macro_rules! nyi {
  ($physical_type:ident, $logical_type:ident, $value:ident) => ({
    unimplemented!(
      "Conversion for physical type {}, logical type {}, value {:?}",
      $physical_type,
      $logical_type,
      $value
    );
  });
}

/// `Row` represents a nested Parquet record.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
  fields: Vec<(String, Field)>
}

/// Constructs a `Row` from the list of `fields` and returns it.
#[inline]
pub fn make_row(fields: Vec<(String, Field)>) -> Row {
  Row { fields: fields }
}

// TODO: implement `getXXX` for different `Field`s

impl fmt::Display for Row {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{{")?;
    for (i, &(ref key, ref value)) in self.fields.iter().enumerate() {
      key.fmt(f)?;
      write!(f, ": ")?;
      value.fmt(f)?;
      if i < self.fields.len() - 1 {
        write!(f, ", ")?;
      }
    }
    write!(f, "}}")
  }
}


/// `List` represents a list which contains an array of elements.
#[derive(Clone, Debug, PartialEq)]
pub struct List {
  elements: Vec<Field>
}

/// Constructs a `List` from the list of `fields` and returns it.
#[inline]
pub fn make_list(elements: Vec<Field>) -> List {
  List { elements: elements }
}

// TODO: implement `getXXX` for different `Fields` in the `List`.


/// `Map` represents a map which contains an list of key->value pairs.
#[derive(Clone, Debug, PartialEq)]
pub struct Map {
  entries: Vec<(Field, Field)>
}

/// Constructs a `Map` from the list of `entries` and returns it.
#[inline]
pub fn make_map(entries: Vec<(Field, Field)>) -> Map {
  Map { entries: entries }
}

// TODO: implement `getKeys`, `getValues`, etc., for `Map`.


/// API to represent a single field in a `Row`.
#[derive(Clone, Debug, PartialEq)]
pub enum Field {
  // Primitive types

  /// Null value.
  Null,
  /// Boolean value (`true`, `false`).
  Bool(bool),
  /// Signed integer INT_8.
  Byte(i8),
  /// Signed integer INT_16.
  Short(i16),
  /// Signed integer INT_32.
  Int(i32),
  /// Signed integer INT_64.
  Long(i64),
  /// IEEE 32-bit floating point value.
  Float(f32),
  /// IEEE 64-bit floating point value.
  Double(f64),
  /// UTF-8 encoded character string.
  Str(String),
  /// General binary value.
  Bytes(ByteArray),
  /// Date without a time of day, stores the number of days from the
  /// Unix epoch, 1 January 1970.
  Date(u32),
  /// Milliseconds from the Unix epoch, 1 January 1970.
  Timestamp(u64),

  // ----------------------------------------------------------------------
  // Complex types

  /// Struct, child elements are tuples of field-value pairs.
  Group(Row),
  /// List of elements.
  ListInternal(List),
  /// List of key-value pairs.
  MapInternal(Map)
}

impl Field {
  /// Converts Parquet BOOLEAN type with logical type into `bool` value.
  pub fn convert_bool(
    _physical_type: PhysicalType,
    _logical_type: LogicalType,
    value: bool
  ) -> Self {
    Field::Bool(value)
  }

  /// Converts Parquet INT32 type with logical type into `i32` value.
  pub fn convert_int32(
    physical_type: PhysicalType,
    logical_type: LogicalType,
    value: i32
  ) -> Self {
    match logical_type {
      LogicalType::INT_8 => Field::Byte(value as i8),
      LogicalType::INT_16 => Field::Short(value as i16),
      LogicalType::INT_32 | LogicalType::NONE => Field::Int(value),
      LogicalType::DATE => Field::Date(value as u32),
      _ => nyi!(physical_type, logical_type, value)
    }
  }

  /// Converts Parquet INT64 type with logical type into `i64` value.
  pub fn convert_int64(
    physical_type: PhysicalType,
    logical_type: LogicalType,
    value: i64
  ) -> Self {
    match logical_type {
      LogicalType::INT_64 | LogicalType::NONE => Field::Long(value),
      _ => nyi!(physical_type, logical_type, value)
    }
  }

  /// Converts Parquet INT96 (nanosecond timestamps) type and logical type into
  /// `Timestamp` value.
  pub fn convert_int96(
    _physical_type: PhysicalType,
    _logical_type: LogicalType,
    value: Int96
  ) -> Self {
    let julian_to_unix_epoch_days: u64 = 2_440_588;
    let milli_seconds_in_a_day: u64 = 86_400_000;
    let nano_seconds_in_a_day: u64 = milli_seconds_in_a_day * 1_000_000;

    let days_since_epoch = value.data()[2] as u64 - julian_to_unix_epoch_days;
    let nanoseconds: u64 = ((value.data()[1] as u64) << 32) + value.data()[0] as u64;
    let nanos = days_since_epoch * nano_seconds_in_a_day + nanoseconds;
    let millis = nanos / 1_000_000;

    Field::Timestamp(millis)
  }

  /// Converts Parquet FLOAT type with logical type into `f32` value.
  pub fn convert_float(
    _physical_type: PhysicalType,
    _logical_type: LogicalType,
    value: f32
  ) -> Self {
    Field::Float(value)
  }

  /// Converts Parquet DOUBLE type with logical type into `f64` value.
  pub fn convert_double(
    _physical_type: PhysicalType,
    _logical_type: LogicalType,
    value: f64
  ) -> Self {
    Field::Double(value)
  }

  /// Converts Parquet BYTE_ARRAY type with logical type into either UTF8 string or
  /// array of bytes.
  pub fn convert_byte_array(
    physical_type: PhysicalType,
    logical_type: LogicalType,
    value: ByteArray
  ) -> Self {
    match physical_type {
      PhysicalType::BYTE_ARRAY => {
        match logical_type {
          LogicalType::UTF8 | LogicalType::ENUM | LogicalType::JSON => {
            let value = unsafe { String::from_utf8_unchecked(value.data().to_vec()) };
            Field::Str(value)
          },
          LogicalType::BSON | LogicalType::NONE => Field::Bytes(value),
          _ => nyi!(physical_type, logical_type, value)
        }
      },
      _ => nyi!(physical_type, logical_type, value)
    }
  }
}

impl fmt::Display for Field {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match *self {
      Field::Null => write!(f, "null"),
      Field::Bool(value) => write!(f, "{}", value),
      Field::Byte(value) => write!(f, "{}", value),
      Field::Short(value) => write!(f, "{}", value),
      Field::Int(value) => write!(f, "{}", value),
      Field::Long(value) => write!(f, "{}", value),
      Field::Float(value) => write!(f, "{:?}", value),
      Field::Double(value) => write!(f, "{:?}", value),
      Field::Str(ref value) => write!(f, "\"{}\"", value),
      Field::Bytes(ref value) => write!(f, "{:?}", value.data()),
      Field::Date(value) => write!(f, "{}", convert_date_to_string(value)),
      Field::Timestamp(value) => write!(f, "{}", convert_timestamp_to_string(value)),
      Field::Group(ref fields) => write!(f, "{}", fields),
      Field::ListInternal(ref list) => {
        let elems = &list.elements;
        write!(f, "[")?;
        for (i, field) in elems.iter().enumerate() {
          field.fmt(f)?;
          if i < elems.len() - 1 {
            write!(f, ", ")?;
          }
        }
        write!(f, "]")
      },
      Field::MapInternal(ref map) => {
        let entries = &map.entries;
        write!(f, "{{")?;
        for (i, &(ref key, ref value)) in entries.iter().enumerate() {
          key.fmt(f)?;
          write!(f, " -> ")?;
          value.fmt(f)?;
          if i < entries.len() - 1 {
            write!(f, ", ")?;
          }
        }
        write!(f, "}}")
      }
    }
  }
}

/// Helper method to convert Parquet date into a string.
/// Input `value` is a number of days since the epoch in UTC.
/// Date is displayed in local timezone.
#[inline]
fn convert_date_to_string(value: u32) -> String {
  static NUM_SECONDS_IN_DAY: i64 = 60 * 60 * 24;
  let dt = Local.timestamp(value as i64 * NUM_SECONDS_IN_DAY, 0).date();
  format!("{}", dt.format("%Y-%m-%d %:z"))
}

/// Helper method to convert Parquet timestamp into a string.
/// Input `value` is a number of milliseconds since the epoch in UTC.
/// Datetime is displayed in local timezone.
#[inline]
fn convert_timestamp_to_string(value: u64) -> String {
  let dt = Local.timestamp((value / 1000) as i64, 0);
  format!("{}", dt.format("%Y-%m-%d %H:%M:%S %:z"))
}


#[cfg(test)]
mod tests {
  use super::*;
  use chrono;

  #[test]
  fn test_row_convert_bool() {
    // BOOLEAN value does not depend on logical type
    let row = Field::convert_bool(PhysicalType::BOOLEAN, LogicalType::NONE, true);
    assert_eq!(row, Field::Bool(true));

    let row = Field::convert_bool(PhysicalType::BOOLEAN, LogicalType::NONE, false);
    assert_eq!(row, Field::Bool(false));
  }

  #[test]
  fn test_row_convert_int32() {
    let row = Field::convert_int32(PhysicalType::INT32, LogicalType::INT_8, 111);
    assert_eq!(row, Field::Byte(111));

    let row = Field::convert_int32(PhysicalType::INT32, LogicalType::INT_16, 222);
    assert_eq!(row, Field::Short(222));

    let row = Field::convert_int32(PhysicalType::INT32, LogicalType::INT_32, 333);
    assert_eq!(row, Field::Int(333));

    let row = Field::convert_int32(PhysicalType::INT32, LogicalType::NONE, 444);
    assert_eq!(row, Field::Int(444));

    let row = Field::convert_int32(PhysicalType::INT32, LogicalType::DATE, 14611);
    assert_eq!(row, Field::Date(14611));
  }

  #[test]
  fn test_row_convert_int64() {
    let row = Field::convert_int64(PhysicalType::INT64, LogicalType::INT_64, 1111);
    assert_eq!(row, Field::Long(1111));

    let row = Field::convert_int64(PhysicalType::INT64, LogicalType::NONE, 2222);
    assert_eq!(row, Field::Long(2222));
  }

  #[test]
  fn test_row_convert_int96() {
    // INT96 value does not depend on logical type
    let value = Int96::from(vec![0, 0, 2454923]);
    let row = Field::convert_int96(PhysicalType::INT96, LogicalType::NONE, value);
    assert_eq!(row, Field::Timestamp(1238544000000));

    let value = Int96::from(vec![4165425152, 13, 2454923]);
    let row = Field::convert_int96(PhysicalType::INT96, LogicalType::NONE, value);
    assert_eq!(row, Field::Timestamp(1238544060000));
  }

  #[test]
  fn test_row_convert_float() {
    // FLOAT value does not depend on logical type
    let row = Field::convert_float(PhysicalType::FLOAT, LogicalType::NONE, 2.31);
    assert_eq!(row, Field::Float(2.31));
  }

  #[test]
  fn test_row_convert_double() {
    // DOUBLE value does not depend on logical type
    let row = Field::convert_double(PhysicalType::FLOAT, LogicalType::NONE, 1.56);
    assert_eq!(row, Field::Double(1.56));
  }

  #[test]
  fn test_row_convert_byte_array() {
    // UTF8
    let value = ByteArray::from(vec![b'A', b'B', b'C', b'D']);
    let row = Field::convert_byte_array(
      PhysicalType::BYTE_ARRAY, LogicalType::UTF8, value);
    assert_eq!(row, Field::Str("ABCD".to_string()));

    // ENUM
    let value = ByteArray::from(vec![b'1', b'2', b'3']);
    let row = Field::convert_byte_array(
      PhysicalType::BYTE_ARRAY, LogicalType::ENUM, value);
    assert_eq!(row, Field::Str("123".to_string()));

    // JSON
    let value = ByteArray::from(vec![b'{', b'"', b'a', b'"', b':', b'1', b'}']);
    let row = Field::convert_byte_array(
      PhysicalType::BYTE_ARRAY, LogicalType::JSON, value);
    assert_eq!(row, Field::Str("{\"a\":1}".to_string()));

    // NONE
    let value = ByteArray::from(vec![1, 2, 3, 4, 5]);
    let row = Field::convert_byte_array(
      PhysicalType::BYTE_ARRAY, LogicalType::NONE, value.clone());
    assert_eq!(row, Field::Bytes(value));

    // BSON
    let value = ByteArray::from(vec![1, 2, 3, 4, 5]);
    let row = Field::convert_byte_array(
      PhysicalType::BYTE_ARRAY, LogicalType::BSON, value.clone());
    assert_eq!(row, Field::Bytes(value));
  }

  #[test]
  fn test_convert_date_to_string() {
    fn check_date_conversion(y: u32, m: u32, d: u32) {
      let datetime = chrono::NaiveDate::from_ymd(y as i32, m, d).and_hms(0, 0, 0);
      let dt = Local.from_utc_datetime(&datetime);
      let res = convert_date_to_string((dt.timestamp() / 60 / 60 / 24) as u32);
      let exp = format!("{}", dt.format("%Y-%m-%d %:z"));
      assert_eq!(res, exp);
    }

    check_date_conversion(2010, 01, 02);
    check_date_conversion(2014, 05, 01);
    check_date_conversion(2016, 02, 29);
    check_date_conversion(2017, 09, 12);
    check_date_conversion(2018, 03, 31);
  }

  #[test]
  fn test_convert_timestamp_to_string() {
    fn check_datetime_conversion(y: u32, m: u32, d: u32, h: u32, mi: u32, s: u32) {
      let datetime = chrono::NaiveDate::from_ymd(y as i32, m, d).and_hms(h, mi, s);
      let dt = Local.from_utc_datetime(&datetime);
      let res = convert_timestamp_to_string(dt.timestamp_millis() as u64);
      let exp = format!("{}", dt.format("%Y-%m-%d %H:%M:%S %:z"));
      assert_eq!(res, exp);
    }

    check_datetime_conversion(2010, 01, 02, 13, 12, 54);
    check_datetime_conversion(2011, 01, 03, 08, 23, 01);
    check_datetime_conversion(2012, 04, 05, 11, 06, 32);
    check_datetime_conversion(2013, 05, 12, 16, 38, 00);
    check_datetime_conversion(2014, 11, 28, 21, 15, 12);
  }

  #[test]
  fn test_row_display() {
    // Primitive types
    assert_eq!(format!("{}", Field::Null), "null");
    assert_eq!(format!("{}", Field::Bool(true)), "true");
    assert_eq!(format!("{}", Field::Bool(false)), "false");
    assert_eq!(format!("{}", Field::Byte(1)), "1");
    assert_eq!(format!("{}", Field::Short(2)), "2");
    assert_eq!(format!("{}", Field::Int(3)), "3");
    assert_eq!(format!("{}", Field::Long(4)), "4");
    assert_eq!(format!("{}", Field::Float(5.0)), "5.0");
    assert_eq!(format!("{}", Field::Float(5.1234)), "5.1234");
    assert_eq!(format!("{}", Field::Double(6.0)), "6.0");
    assert_eq!(format!("{}", Field::Double(6.1234)), "6.1234");
    assert_eq!(format!("{}", Field::Str("abc".to_string())), "\"abc\"");
    assert_eq!(format!("{}", Field::Bytes(ByteArray::from(vec![1, 2, 3]))), "[1, 2, 3]");
    assert_eq!(format!("{}", Field::Date(14611)), convert_date_to_string(14611));
    assert_eq!(
      format!("{}", Field::Timestamp(1262391174000)),
      convert_timestamp_to_string(1262391174000)
    );

    // Complex types
    let fields = vec![
      ("x".to_string(), Field::Null),
      ("Y".to_string(), Field::Int(2)),
      ("z".to_string(), Field::Float(3.1)),
      ("a".to_string(), Field::Str("abc".to_string()))
    ];
    let row = Field::Group(make_row(fields));
    assert_eq!(format!("{}", row), "{x: null, Y: 2, z: 3.1, a: \"abc\"}");

    let row = Field::ListInternal(
      make_list(vec![
        Field::Int(2),
        Field::Int(1),
        Field::Null,
        Field::Int(12)
      ]));
    assert_eq!(format!("{}", row), "[2, 1, null, 12]");

    let row = Field::MapInternal(
      make_map(vec![
        (Field::Int(1), Field::Float(1.2)),
        (Field::Int(2), Field::Float(4.5)),
        (Field::Int(3), Field::Float(2.3))
      ]));
    assert_eq!(format!("{}", row), "{1 -> 1.2, 2 -> 4.5, 3 -> 2.3}");
  }
}