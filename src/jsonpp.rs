use std::{collections::HashMap, fmt::Display, fs::File, io::Read};

use strum_macros::EnumIter;

use crate::{evaluation, parsing};

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, Copy, EnumIter, PartialEq, Eq)]
pub(crate) enum Function {
    Sum,
    Sub,
    Mul,
    Div,
    Min,
    Max,
    Mod,
    Pow,
    Log,
    Len,
    Ref,
    Eq,
    Gt,
    Lt,
    Gte,
    Lte,
    If,
    Include,
    Import,
    Str,
    Int,
    Float,
    Range,
    Merge,
    Fold,
    Map,
    Filter,
    Reduce,
}

impl Display for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Function::Sum => "sum",
                Function::Sub => "sub",
                Function::Mul => "mul",
                Function::Div => "div",
                Function::Min => "min",
                Function::Max => "max",
                Function::Mod => "mod",
                Function::Pow => "pow",
                Function::Log => "log",
                Function::Len => "len",
                Function::Ref => "ref",
                Function::Eq => "eq",
                Function::Gt => "gt",
                Function::Lt => "lt",
                Function::Gte => "gte",
                Function::Lte => "lte",
                Function::If => "if",
                Function::Include => "include",
                Function::Import => "import",
                Function::Str => "str",
                Function::Int => "int",
                Function::Float => "float",
                Function::Range => "range",
                Function::Merge => "merge",
                Function::Fold => "fold",
                Function::Map => "map",
                Function::Filter => "filter",
                Function::Reduce => "reduce",
            }
        )
    }
}

impl From<&str> for Function {
    fn from(value: &str) -> Self {
        match value {
            "sum" => Function::Sum,
            "sub" => Function::Sub,
            "mul" => Function::Mul,
            "div" => Function::Div,
            "min" => Function::Min,
            "max" => Function::Max,
            "mod" => Function::Mod,
            "pow" => Function::Pow,
            "log" => Function::Log,
            "len" => Function::Len,
            "ref" => Function::Ref,
            "eq" => Function::Eq,
            "gt" => Function::Gt,
            "lt" => Function::Lt,
            "gte" => Function::Gte,
            "lte" => Function::Lte,
            "if" => Function::If,
            "include" => Function::Include,
            "import" => Function::Import,
            "str" => Function::Str,
            "int" => Function::Int,
            "float" => Function::Float,
            "range" => Function::Range,
            "merge" => Function::Merge,
            "fold" => Function::Fold,
            "map" => Function::Map,
            "filter" => Function::Filter,
            "reduce" => Function::Reduce,
            _ => panic!("Unrecognized function"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PathChunk {
    Parent,
    Key(String),
    Index(usize),
    Argument(usize),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Dynamic {
    pub fun: Function,
    pub args: Vec<JsonPP>,
    pub path: Vec<PathChunk>,
    pub dependencies: Vec<Vec<PathChunk>>,
}

impl Dynamic {
    pub fn resolve(self, path: &[PathChunk], root: &JsonPP) -> JsonPP {
        // Dynamic has no dependencies left, we can resolve it to a value
        match self.fun {
            Function::Sum => sum_impl(self.args),
            Function::Sub => sub_impl(self.args),
            Function::Mul => mul_impl(self.args),
            Function::Div => div_impl(self.args),
            Function::Mod => mod_impl(self.args),
            Function::Pow => pow_impl(self.args),
            Function::Log => log_impl(self.args),
            Function::Len => len_impl(self.args),
            Function::Ref => ref_impl(self.args, path, root),
            Function::Min => min_impl(self.args),
            Function::Max => max_impl(self.args),
            Function::Eq => eq_impl(self.args),
            Function::Gt => num_cmp(self.args, |a, b| a > b, |a, b| a > b),
            Function::Lt => num_cmp(self.args, |a, b| a < b, |a, b| a < b),
            Function::Gte => num_cmp(self.args, |a, b| a >= b, |a, b| a >= b),
            Function::Lte => num_cmp(self.args, |a, b| a <= b, |a, b| a <= b),
            Function::If => if_impl(self.args),
            Function::Include => include_impl(self.args),
            Function::Import => import_impl(self.args),
            Function::Str => str_impl(self.args),
            Function::Int => int_impl(self.args),
            Function::Float => float_impl(self.args),
            Function::Range => range_impl(self.args),
            Function::Merge => merge_impl(self.args),
            Function::Fold => todo!(),
            Function::Map => todo!(),
            Function::Filter => todo!(),
            Function::Reduce => todo!(),
        }
    }
}

fn num_cmp(
    args: Vec<JsonPP>,
    int_f: fn(i64, i64) -> bool,
    float_f: fn(f64, f64) -> bool,
) -> JsonPP {
    assert_eq!(args.len(), 2);

    let first_arg = args[0].clone();
    let second_arg = args[1].clone();

    JsonPP::Bool(match (first_arg.clone(), second_arg.clone()) {
        (JsonPP::Int(first), JsonPP::Int(second)) => int_f(first, second),
        (JsonPP::Float(first), JsonPP::Float(second)) => float_f(first, second),
        (JsonPP::Float(first), JsonPP::Int(second)) => float_f(first, second as f64),
        (JsonPP::Int(first), JsonPP::Float(second)) => float_f(first as f64, second),
        _ => panic!(
            "Invalid operands to a numeric function, {:?} and {:?}",
            first_arg, second_arg
        ),
    })
}

fn num_pair_op(
    int_f: fn(i64, i64) -> i64,
    float_f: fn(f64, f64) -> f64,
    first_arg: JsonPP,
    second_arg: JsonPP,
) -> JsonPP {
    match (first_arg.clone(), second_arg.clone()) {
        (JsonPP::Int(first), JsonPP::Int(second)) => JsonPP::Int(int_f(first, second)),
        (JsonPP::Float(first), JsonPP::Float(second)) => JsonPP::Float(float_f(first, second)),
        (JsonPP::Float(first), JsonPP::Int(second)) => JsonPP::Float(float_f(first, second as f64)),
        (JsonPP::Int(first), JsonPP::Float(second)) => JsonPP::Float(float_f(first as f64, second)),
        _ => panic!(
            "Invalid operands to a numeric function, {:?} and {:?}",
            first_arg, second_arg
        ),
    }
}

fn num_reduce(
    int_f: fn(i64, i64) -> i64,
    float_f: fn(f64, f64) -> f64,
    args: Vec<JsonPP>,
) -> JsonPP {
    args.into_iter()
        .reduce(|acc, next| num_pair_op(int_f, float_f, acc, next))
        .unwrap()
}

fn sum_impl(args: Vec<JsonPP>) -> JsonPP {
    num_reduce(|a, b| a + b, |a, b| a + b, args)
}

fn mul_impl(args: Vec<JsonPP>) -> JsonPP {
    num_reduce(|a, b| a * b, |a, b| a * b, args)
}

fn sub_impl(args: Vec<JsonPP>) -> JsonPP {
    assert_eq!(args.len(), 2);
    num_reduce(|a, b| a - b, |a, b| a - b, args)
}

fn div_impl(args: Vec<JsonPP>) -> JsonPP {
    assert_eq!(args.len(), 2);
    num_reduce(|a, b| a / b, |a, b| a / b, args)
}

fn mod_impl(args: Vec<JsonPP>) -> JsonPP {
    assert_eq!(args.len(), 2);
    num_reduce(|a, b| a % b, |a, b| a % b, args)
}

fn pow_impl(args: Vec<JsonPP>) -> JsonPP {
    assert_eq!(args.len(), 2);
    num_reduce(
        |a, b| {
            if b.is_positive() {
                a.pow(b as u32)
            } else {
                (a as f64).powf(b as f64).round() as i64
            }
        },
        f64::powf,
        args,
    )
}

fn log_impl(args: Vec<JsonPP>) -> JsonPP {
    assert_eq!(args.len(), 2);
    num_reduce(|a, b| i64::ilog(a, b) as i64, f64::log, args)
}

fn len_impl(args: Vec<JsonPP>) -> JsonPP {
    assert_eq!(args.len(), 1);

    JsonPP::Int(match &args[0] {
        JsonPP::String(inner) => inner.len() as i64,
        JsonPP::Array(inner) => inner.len() as i64,
        JsonPP::Object(inner) => inner.len() as i64,
        _ => panic!("Trying to get the length of something odd"),
    })
}

fn ref_impl(args: Vec<JsonPP>, self_path: &[PathChunk], root: &JsonPP) -> JsonPP {
    assert_eq!(args.len(), 1);
    let JsonPP::String(target) = args[0].clone() else {
        panic!("Non-string reference: {:?}", args);
    };

    let target_path = evaluation::ref_chain(target);

    evaluation::abs_fetch(&evaluation::make_absolute(self_path, &target_path), root).clone()
}

fn min_impl(args: Vec<JsonPP>) -> JsonPP {
    num_reduce(i64::min, f64::min, args)
}

fn max_impl(args: Vec<JsonPP>) -> JsonPP {
    num_reduce(i64::max, f64::max, args)
}

fn eq_impl(args: Vec<JsonPP>) -> JsonPP {
    assert_eq!(args.len(), 2);

    let first_arg = args[0].clone();
    let second_arg = args[1].clone();

    JsonPP::Bool(first_arg == second_arg)
}

fn if_impl(args: Vec<JsonPP>) -> JsonPP {
    assert_eq!(args.len(), 3); // Condition, if true, if not;
    let JsonPP::Bool(cond) = args[0].clone() else {
        panic!("If condition is not a boolean")
    };

    let index = if cond { 1 } else { 2 };
    args[index].clone()
}

fn include_impl(args: Vec<JsonPP>) -> JsonPP {
    assert_eq!(args.len(), 1);

    let JsonPP::String(path) = args[0].clone() else {
        panic!("Include path is not a string")
    };

    let mut file = File::open(path).unwrap();
    let mut buffer = vec![];
    file.read_to_end(&mut buffer).unwrap();

    let string: String = buffer.into_iter().map(char::from).collect();
    JsonPP::String(string.trim().to_owned())
}

fn import_impl(args: Vec<JsonPP>) -> JsonPP {
    assert_eq!(args.len(), 1);

    let JsonPP::String(path) = args[0].clone() else {
        panic!("Import path is not a string")
    };

    let mut file = File::open(path).unwrap();
    let mut buffer = vec![];
    file.read_to_end(&mut buffer).unwrap();

    evaluation::evaluate_raw(parsing::Parser::from(buffer).parse())
}

fn str_impl(args: Vec<JsonPP>) -> JsonPP {
    assert_eq!(args.len(), 1);

    JsonPP::String(match args[0].clone() {
        JsonPP::String(val) => val,

        JsonPP::Null => "null".to_owned(),
        JsonPP::Bool(val) => val.to_string(),
        JsonPP::Int(val) => val.to_string(),
        JsonPP::Float(val) => val.to_string(),

        JsonPP::Array(vec) => {
            format!(
                "[{}]",
                vec.into_iter()
                    .map(|elem| {
                        let JsonPP::String(val) = str_impl(vec![elem]) else {
                            panic!("Array element didn't convert to string")
                        };
                        val
                    })
                    .collect::<Vec<String>>()
                    .join(", ")
            )
        }
        JsonPP::Object(hash_map) => format!(
            "{{{}}}",
            hash_map
                .into_iter()
                .map(|(key, elem)| {
                    let JsonPP::String(val) = str_impl(vec![elem]) else {
                        panic!("Array element didn't convert to string")
                    };
                    format!("\"{}\": {}", key, val)
                })
                .collect::<Vec<String>>()
                .join(", ")
        ),
        // This is not supposed to be evaluated with a dynamic
        JsonPP::Dynamic(_) => panic!("Can't convert dynamic to string"),
    })
}

fn int_impl(args: Vec<JsonPP>) -> JsonPP {
    assert_eq!(args.len(), 1);

    JsonPP::Int(match args[0].clone() {
        JsonPP::Int(val) => val,

        JsonPP::Null => 0,
        JsonPP::Bool(val) => val as i64,
        JsonPP::Float(val) => val.round() as i64,
        JsonPP::String(val) => val.parse().expect("str to int parse failed"),
        other => panic!("Can't convert \"{:?}\" to int", other),
    })
}

fn float_impl(args: Vec<JsonPP>) -> JsonPP {
    assert_eq!(args.len(), 1);

    JsonPP::Float(match args[0].clone() {
        JsonPP::Float(val) => val,

        JsonPP::Null => 0.0,
        JsonPP::Bool(val) => val as i64 as f64,
        JsonPP::Int(val) => val as f64,
        JsonPP::String(val) => val.parse().expect("str to float parse failed"),
        other => panic!("Can't convert \"{:?}\" to float", other),
    })
}

fn range_impl(args: Vec<JsonPP>) -> JsonPP {
    assert_eq!(args.len(), 2);

    let JsonPP::Int(start) = args[0].clone() else {
        panic!("Range start is not an int")
    };
    let JsonPP::Int(end) = args[1].clone() else {
        panic!("Range end is not an int")
    };

    JsonPP::Array((start..end).map(JsonPP::Int).collect())
}

fn merge_impl(args: Vec<JsonPP>) -> JsonPP {
    // Works on strings, arrays and objects
    // All participants must be of the same type

    if args.iter().all(|el| matches!(el, JsonPP::String(_))) {
        return string_merge_impl(args);
    }

    if args.iter().all(|el| matches!(el, JsonPP::Array(_))) {
        return array_merge_impl(args);
    }

    if args.iter().all(|el| matches!(el, JsonPP::Object(_))) {
        return object_merge_impl(args);
    }

    panic!("Either mismatched array elements or illegal types of elements in merge");
}

fn string_merge_impl(args: Vec<JsonPP>) -> JsonPP {
    // All elements are strings
    JsonPP::String(
        args.into_iter()
            .map(|el| {
                let JsonPP::String(val) = el else {
                    unreachable!()
                };

                val
            })
            .collect::<Vec<String>>()
            .join(""),
    )
}

fn array_merge_impl(args: Vec<JsonPP>) -> JsonPP {
    // All elements are arrays
    JsonPP::Array(
        args.into_iter()
            .flat_map(|el| {
                let JsonPP::Array(val) = el else {
                    unreachable!()
                };

                val
            })
            .collect(),
    )
}

fn object_merge_impl(args: Vec<JsonPP>) -> JsonPP {
    // All elements are objects
    JsonPP::Object(
        args.into_iter()
            .flat_map(|el| {
                let JsonPP::Object(val) = el else {
                    unreachable!()
                };

                val
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absolute_path_equivalence() {
        let self_path = vec![
            PathChunk::Key("Foo".to_owned()),
            PathChunk::Key("Baz".to_owned()),
        ];
        // Target a sibling
        let target_path = vec![PathChunk::Parent, PathChunk::Key("Bar".to_owned())];
        let new_abs_path = evaluation::make_absolute(&self_path, &target_path);

        assert_eq!(
            vec![
                PathChunk::Key("Foo".to_owned()),
                PathChunk::Key("Bar".to_owned())
            ],
            new_abs_path
        )
    }
    #[test]
    fn base_path_ignored_for_absolute_paths() {
        let self_path = vec![
            PathChunk::Key("Foo".to_owned()),
            PathChunk::Key("Baz".to_owned()),
        ];
        // Target a sibling
        let target_path = vec![PathChunk::Key("Bar".to_owned())];
        let new_abs_path = evaluation::make_absolute(&self_path, &target_path);

        assert_eq!(vec![PathChunk::Key("Bar".to_owned())], new_abs_path)
    }
}
