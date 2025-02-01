use std::collections::HashMap;

#[derive(Debug, Clone)]
pub(crate) enum JsonPP {
    Null,
    Bool(bool),
    String(String),
    Int(i64),
    Float(f64),
    Array(Vec<JsonPP>),
    Object(HashMap<String, JsonPP>),
    Dynamic(Dynamic),
}

impl TryInto<serde_json::Value> for JsonPP {
    type Error = ();

    fn try_into(self) -> Result<serde_json::Value, Self::Error> {
        Ok(match self {
            JsonPP::Null => serde_json::Value::Null,
            JsonPP::Bool(val) => serde_json::Value::Bool(val),
            JsonPP::String(val) => serde_json::Value::String(val),
            JsonPP::Int(val) => serde_json::Value::from(val),
            JsonPP::Float(val) => serde_json::Value::from(val),
            JsonPP::Array(vec) => serde_json::Value::Array(
                vec.into_iter()
                    .map(|elem| TryInto::<serde_json::Value>::try_into(elem))
                    .collect::<Result<Vec<serde_json::Value>, ()>>()?,
            ),
            JsonPP::Object(hash_map) => serde_json::Value::Object(
                hash_map
                    .into_iter()
                    .map(|(key, elem)| {
                        TryInto::<serde_json::Value>::try_into(elem)
                            .map(|converted| (key, converted))
                    })
                    .collect::<Result<serde_json::Map<String, serde_json::Value>, ()>>()?,
            ),
            JsonPP::Dynamic(_) => return Err(()),
        })
    }
}

#[derive(Debug)]
pub(crate) enum Function {
    Min,
    Max,
    Mod,
    Pow,
    Log,
    Ref,
    Import,
    Include,
}

#[derive(Debug, Clone)]
pub(crate) struct Dynamic;
