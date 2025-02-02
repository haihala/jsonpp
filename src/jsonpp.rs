use std::{collections::HashMap, fmt::Display};

use log::debug;
use strum_macros::EnumIter;

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
    Import,
    Include,
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
                Function::Import => "import",
                Function::Include => "include",
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
            "import" => Function::Import,
            "include" => Function::Include,
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

#[derive(Debug, Clone)]
pub(crate) struct Dynamic {
    pub fun: Function,
    pub args: Vec<JsonPP>,
    pub path: Vec<PathChunk>,
    pub dependencies: Vec<Vec<PathChunk>>,
}

impl Dynamic {
    pub fn resolve(self, path: &[PathChunk], root: &JsonPP) -> JsonPP {
        // Dynamic has no dependencies left, we can resolve it to a value
        assert!(self.dependencies.is_empty());
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
            Function::Min => todo!(),
            Function::Max => todo!(),
            Function::Import => todo!(),
            Function::Include => todo!(),
            Function::Fold => todo!(),
            Function::Map => todo!(),
            Function::Filter => todo!(),
            Function::Reduce => todo!(),
        }
    }
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

    let target_path = ref_chain(target);

    abs_fetch(&make_absolute(self_path, &target_path), root).clone()
}

pub(crate) fn make_absolute(self_path: &[PathChunk], target_path: &[PathChunk]) -> Vec<PathChunk> {
    if target_path.first() == Some(&PathChunk::Parent) {
        // Relative path
        let mut out: Vec<PathChunk> = self_path.iter().cloned().collect();
        for chunk in target_path {
            if *chunk == PathChunk::Parent {
                out.pop();
            } else {
                out.push(chunk.clone());
            }
        }

        return out;
    }

    return target_path.to_vec();
}

pub(crate) fn ref_chain(path: String) -> Vec<PathChunk> {
    path.split(".")
        .map(|chunk| {
            if chunk.is_empty() {
                return PathChunk::Parent;
            }

            if chunk.starts_with("[") && chunk.ends_with("]") {
                let inner = &chunk[1..(chunk.len() - 2)];
                return PathChunk::Index(inner.parse().unwrap());
            }

            if chunk.starts_with("(") && chunk.ends_with(")") {
                let inner = &chunk[1..(chunk.len() - 2)];
                return PathChunk::Argument(inner.parse().unwrap());
            }

            PathChunk::Key(chunk.to_owned())
        })
        .collect()
}

pub(crate) fn abs_fetch<'a>(path: &[PathChunk], root: &'a JsonPP) -> &'a JsonPP {
    if path.is_empty() {
        return root;
    }

    let next = &path[0];
    let rest = &path[1..];

    match next {
        PathChunk::Parent => todo!(),
        PathChunk::Key(key) => {
            let JsonPP::Object(inner) = root else {
                debug!("{:?}, {:?}, {:?}", root, key, path);
                panic!("Accessing with a key");
            };

            abs_fetch(rest, &inner[key])
        }
        PathChunk::Index(index) => {
            let JsonPP::Array(inner) = root else {
                debug!("{:?}, {:?}, {:?}", root, index, path);
                panic!("Accessing with an index");
            };

            abs_fetch(rest, &inner[*index])
        }
        PathChunk::Argument(index) => {
            let JsonPP::Dynamic(inner) = root else {
                debug!("{:?}, {:?}, {:?}", root, index, path);
                panic!("Accessing with an argument");
            };

            abs_fetch(rest, &inner.args[*index])
        }
    }
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
        let new_abs_path = make_absolute(&self_path, &target_path);

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
        let new_abs_path = make_absolute(&self_path, &target_path);

        assert_eq!(vec![PathChunk::Key("Bar".to_owned())], new_abs_path)
    }
}
