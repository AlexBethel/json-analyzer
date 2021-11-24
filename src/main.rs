//! A simple program for generating data structure declarations from a
//! JSON file.

use std::{collections::HashMap, iter::once, path::Path};

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
#[derive(Debug, PartialEq, Eq, Clone)]
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

    /// One of several possible types. An empty Variant is also used
    /// to represent an unknown type.
    Variant(Vec<DataType>),
}

impl DataType {
    /// Generate a data type that could represent something of this
    /// type, or of the `other` type.
    pub fn unify(self, other: DataType) -> Self {
        match (self, other) {
            (t1, t2) if t1 == t2 => t1,
            (DataType::Variant(types), t2) => {
                if types.is_empty() {
                    t2
                } else if types.contains(&t2) {
                    DataType::Variant(types)
                } else {
                    DataType::Variant(types.into_iter().chain(once(t2)).collect())
                }
            }
            (DataType::Float, DataType::Int) | (DataType::Int, DataType::Float) => DataType::Float,
            (t1, t2) => DataType::Variant(vec![t1, t2]),
        }
    }

    /// Create a data type that can reprent the given value.
    pub fn from_json_value(v: &JsonValue) -> Self {
        match v {
            JsonValue::Null => Self::Null,
            JsonValue::Short(_) => Self::String,
            JsonValue::String(_) => Self::String,
            JsonValue::Number(n) => {
                let float = f64::from(*n);
                if float == float.floor() {
                    Self::Int
                } else {
                    Self::Float
                }
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    use json::short::Short;

    #[test]
    fn basic_types() {
        assert_eq!(DataType::from_json_value(&JsonValue::Null), DataType::Null);
        assert_eq!(
            DataType::from_json_value(&JsonValue::String("hello".to_string())),
            DataType::String
        );

        let s = "foo";
        if json::short::MAX_LEN >= s.len() {
            // SAFETY: A Short is defined to be able to store at least
            // MAX_LEN bytes, and MAX_LEN >= the length of s,
            // therefore a Short is able to store a string of s's
            // length.
            let s = JsonValue::Short(unsafe { Short::from_slice(s) });

            assert_eq!(DataType::from_json_value(&s), DataType::String);
        } else {
            panic!("Failed to test Short");
        }

        assert_eq!(
            DataType::from_json_value(&JsonValue::Boolean(true)),
            DataType::Bool
        );
    }

    #[test]
    fn numbers() {
        assert_eq!(
            DataType::from_json_value(&JsonValue::Number(10.into())),
            DataType::Int
        );
        assert_eq!(
            DataType::from_json_value(&JsonValue::Number((10.5).into())),
            DataType::Float
        );
    }

    #[test]
    fn unification() {
        assert_eq!(
            DataType::unify(DataType::String, DataType::Bool),
            DataType::Variant(vec![DataType::String, DataType::Bool])
        );
        assert_eq!(
            DataType::unify(
                DataType::Variant(vec![DataType::String, DataType::Bool]),
                DataType::Null
            ),
            DataType::Variant(vec![DataType::String, DataType::Bool, DataType::Null])
        );
    }

    #[test]
    fn floats_override_ints() {
        assert_eq!(
            DataType::unify(DataType::Int, DataType::Float),
            DataType::Float
        );
    }

    #[test]
    fn structs() {
        let a = DataType::from_json_value(&json::object! {
            "null": null,
            "string": "hello",
            "number": 123,
            "bool": true,
            "object": {
                "hello": "world"
            },
            "arr": [1, 2, 3]
        });
        let b = DataType::Object(
            [
                ("null", DataType::Null),
                ("string", DataType::String),
                ("number", DataType::Int),
                ("bool", DataType::Bool),
                (
                    "object",
                    DataType::Object(
                        [("hello", DataType::String)]
                            .iter()
                            .map(|(name, typ)| (name.to_string(), (*typ).clone()))
                            .collect::<HashMap<String, DataType>>(),
                    ),
                ),
                ("arr", DataType::Array(Box::new(DataType::Int))),
            ]
            .iter()
            .map(|(name, typ)| (name.to_string(), (*typ).clone()))
            .collect::<HashMap<String, DataType>>(),
        );

        assert_eq!(a, b);
    }

    #[test]
    fn object_unification() {
        let arr = JsonValue::Array(vec![
            JsonValue::Number(1.into()),
            JsonValue::String("hello".to_string()),
        ]);
        let arr_typ = DataType::Array(Box::new(DataType::Variant(vec![
            DataType::Int,
            DataType::String,
        ])));
        assert_eq!(DataType::from_json_value(&arr), arr_typ);

        let objs = JsonValue::Array(vec![
            json::object! {
                "foo": "bar"
            },
            json::object! {
                "foo": 123,
                "baz": true
            },
        ]);
        let objs_type = DataType::Array(Box::new(DataType::Object(
            [
                (
                    "foo",
                    DataType::Variant(vec![DataType::String, DataType::Int]),
                ),
                (
                    "baz",
                    DataType::Variant(vec![DataType::Null, DataType::Bool]),
                ),
            ]
            .iter()
            .map(|(name, typ)| (name.to_string(), (*typ).clone()))
            .collect::<HashMap<String, DataType>>(),
        )));

        assert_eq!(DataType::from_json_value(&objs), objs_type);
    }
}
