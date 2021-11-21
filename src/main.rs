//! A simple program for generating data structure declarations from a
//! JSON file.

use std::{collections::HashMap, path::Path};

use clap::Arg;
use json::JsonValue;

fn main() {
    let app = clap::App::new("json-analyzer")
        .arg(
            Arg::with_name("file")
                .index(1)
                .help("The JSON file to analyze")
                .required(true),
        )
        .get_matches();

    let filename = Path::new(app.value_of_os("file").expect("Required option"));
    let text = match std::fs::read_to_string(filename) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Unable to open {:?} for reading: {}", filename, e);
            return;
        }
    };

    let data = match json::parse(&text) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Invalid json data: {}", e);
            return;
        }
    };

    let typ = DataType::from_json_value(&data);
    println!("{:?}", typ);
}

/// Types of data in a JSON structure.
#[derive(Debug, PartialEq, Eq)]
enum DataType {
    /// Data that is always Null. In practice, this is usually
    /// combined with `Variant` to create an optional value.
    Null,

    /// A string of characters.
    String,

    /// A number that must always be an integer.
    Int,

    /// A number that can be either a float or an integer.
    Float,

    /// A boolean.
    Bool,

    /// A heterogeneous data structure with named elements, like a
    /// struct.
    Object(HashMap<String, DataType>),

    /// An array of elements with the same type.
    Array(Box<DataType>),

    /// One of several possible types.
    Variant(Vec<DataType>),
}

impl DataType {
    /// Generate a data type that could represent something of this
    /// type, or of the `other` type.
    pub fn unify(self, _other: DataType) -> Self {
        todo!()
    }

    /// Create a data type that can reprent the given value.
    pub fn from_json_value(v: &JsonValue) -> Self {
        match v {
            JsonValue::Null => Self::Null,
            JsonValue::Short(_) => Self::String,
            JsonValue::String(_) => Self::Int,
            JsonValue::Number(_) => todo!(),
            JsonValue::Boolean(_) => Self::Bool,
            JsonValue::Object(obj) => Self::Object(
                obj.iter()
                    .map(|(key, value)| (key.to_string(), Self::from_json_value(value)))
                    .collect(),
            ),
            JsonValue::Array(elems) => Self::Array(Box::new(
                elems
                    .iter()
                    .map(Self::from_json_value)
                    .reduce(Self::unify)
                    .unwrap_or(Self::Variant(Vec::new())),
            )),
        }
    }
}
