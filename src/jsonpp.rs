use std::collections::HashMap;

use crate::{builtins, evaluation, paths::PathChunk};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum JsonPP {
    Undefined, // The point of this to filter things out
    Null,
    Bool(bool),
    String(String),
    Int(i64),
    Float(f64),
    Array(Vec<JsonPP>),
    Object(HashMap<String, JsonPP>),
    Identifier(String),
    Definition(Definition),
    Dynamic(Dynamic),
}
impl JsonPP {
    pub fn is_truthy(&self) -> bool {
        match self {
            JsonPP::Null | JsonPP::Undefined => false,
            JsonPP::Bool(val) => *val,
            JsonPP::String(val) => !val.is_empty(),
            JsonPP::Int(val) => *val != 0,
            JsonPP::Float(val) => *val != 0.0,
            JsonPP::Array(vec) => !vec.is_empty(),
            JsonPP::Object(hash_map) => !hash_map.is_empty(),
            other => panic!("Cannot evaluate truthiness of '{:?}'", other),
        }
    }
}

impl TryInto<Option<serde_json::Value>> for JsonPP {
    type Error = JsonPP;

    fn try_into(self) -> Result<Option<serde_json::Value>, Self::Error> {
        Ok(Some(match self {
            JsonPP::Null => serde_json::Value::Null,
            JsonPP::Bool(val) => serde_json::Value::Bool(val),
            JsonPP::String(val) => serde_json::Value::String(val),
            JsonPP::Int(val) => serde_json::Value::from(val),
            JsonPP::Float(val) => serde_json::Value::from(val),
            JsonPP::Array(vec) => serde_json::Value::Array(
                vec.into_iter()
                    .filter_map(|elem| {
                        TryInto::<Option<serde_json::Value>>::try_into(elem).unwrap_or_else(|el| {
                            dbg!(&el);
                            panic!("Invalid element {:?}", el);
                        })
                    })
                    .collect::<Vec<serde_json::Value>>(),
            ),
            JsonPP::Object(hash_map) => serde_json::Value::Object(
                hash_map
                    .into_iter()
                    .filter_map(|(key, elem)| {
                        TryInto::<Option<serde_json::Value>>::try_into(elem)
                            .unwrap_or_else(|el| {
                                dbg!(&el);
                                panic!("Invalid element {:?}", el);
                            })
                            .map(|converted| (key, converted))
                    })
                    .collect::<serde_json::Map<String, serde_json::Value>>(),
            ),
            // This gets stripped out quietly
            JsonPP::Undefined | JsonPP::Definition(_) => return Ok(None),
            // These should panic
            JsonPP::Identifier(_) | JsonPP::Dynamic(_) => return Err(self),
        }))
    }
}

impl TryInto<serde_json::Value> for JsonPP {
    type Error = ();

    fn try_into(self) -> Result<serde_json::Value, Self::Error> {
        TryInto::<Option<serde_json::Value>>::try_into(self)
            .map(|inner| inner.unwrap())
            .map_err(|_| ())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Definition {
    pub vars: Vec<String>,
    pub template: Box<JsonPP>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Dynamic {
    pub args: Vec<JsonPP>,
    pub path: Vec<PathChunk>,
    pub dependencies: Vec<Vec<PathChunk>>,
}

impl Dynamic {
    pub fn is_def(&self) -> bool {
        self.args[0] == JsonPP::Identifier("def".to_owned())
    }

    pub fn is_ref(&self) -> bool {
        self.args[0] == JsonPP::Identifier("ref".to_owned())
    }
}

impl Dynamic {
    pub fn resolve(self, path: &[PathChunk], root: &JsonPP) -> JsonPP {
        // Dynamic has no dependencies left, we can resolve it to a value
        assert!(!self.args.is_empty());
        let (cmd, args) = self.args.split_at(1);

        match cmd[0].to_owned() {
            JsonPP::Identifier(fun) => match fun.as_str() {
                "sum" => builtins::sum_impl(args.to_vec()),
                "sub" => builtins::sub_impl(args.to_vec()),
                "mul" => builtins::mul_impl(args.to_vec()),
                "div" => builtins::div_impl(args.to_vec()),
                "mod" => builtins::mod_impl(args.to_vec()),
                "pow" => builtins::pow_impl(args.to_vec()),
                "log" => builtins::log_impl(args.to_vec()),
                "len" => builtins::len_impl(args.to_vec()),
                "ref" => builtins::ref_impl(args.to_vec(), path, root),
                "min" => builtins::min_impl(args.to_vec()),
                "max" => builtins::max_impl(args.to_vec()),
                "eq" => builtins::eq_impl(args.to_vec()),
                "gt" => builtins::num_cmp(args.to_vec(), |a, b| a > b, |a, b| a > b),
                "lt" => builtins::num_cmp(args.to_vec(), |a, b| a < b, |a, b| a < b),
                "gte" => builtins::num_cmp(args.to_vec(), |a, b| a >= b, |a, b| a >= b),
                "lte" => builtins::num_cmp(args.to_vec(), |a, b| a <= b, |a, b| a <= b),
                "if" => builtins::if_impl(args.to_vec()),
                "include" => builtins::include_impl(args.to_vec()),
                "import" => builtins::import_impl(args.to_vec()),
                "str" => builtins::str_impl(args.to_vec()),
                "int" => builtins::int_impl(args.to_vec()),
                "float" => builtins::float_impl(args.to_vec()),
                "range" => builtins::range_impl(args.to_vec()),
                "merge" => builtins::merge_impl(args.to_vec()),
                "def" => builtins::def_impl(args.to_vec()),
                "map" => builtins::map_impl(args.to_vec()),
                "filter" => builtins::filter_impl(args.to_vec()),
                "reduce" => builtins::reduce_impl(args.to_vec()),
                other => panic!("Unrecognized function '{}'", other),
            },
            JsonPP::Definition(def) => evaluation::definition_substitution(def, args.to_vec()),
            other => panic!("Cannot call '{:?}'", other),
        }
    }
}
