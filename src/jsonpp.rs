use std::{collections::HashMap, fs::File, io::Read};

use crate::{evaluation, parsing};

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
    fn is_truthy(&self) -> bool {
        match self {
            JsonPP::Null => false,
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
                    .filter_map(|elem| TryInto::<serde_json::Value>::try_into(elem).ok())
                    .collect::<Vec<serde_json::Value>>(),
            ),
            JsonPP::Object(hash_map) => serde_json::Value::Object(
                hash_map
                    .into_iter()
                    .filter_map(|(key, elem)| {
                        TryInto::<serde_json::Value>::try_into(elem)
                            .map(|converted| (key, converted))
                            .ok()
                    })
                    .collect::<serde_json::Map<String, serde_json::Value>>(),
            ),
            JsonPP::Undefined
            | JsonPP::Identifier(_)
            | JsonPP::Definition(_)
            | JsonPP::Dynamic(_) => return Err(()),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum PathChunk {
    Parent,
    Key(String),
    Index(usize),
    Argument(usize),
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
                "sum" => sum_impl(args.to_vec()),
                "sub" => sub_impl(args.to_vec()),
                "mul" => mul_impl(args.to_vec()),
                "div" => div_impl(args.to_vec()),
                "mod" => mod_impl(args.to_vec()),
                "pow" => pow_impl(args.to_vec()),
                "log" => log_impl(args.to_vec()),
                "len" => len_impl(args.to_vec()),
                "ref" => ref_impl(args.to_vec(), path, root),
                "min" => min_impl(args.to_vec()),
                "max" => max_impl(args.to_vec()),
                "eq" => eq_impl(args.to_vec()),
                "gt" => num_cmp(args.to_vec(), |a, b| a > b, |a, b| a > b),
                "lt" => num_cmp(args.to_vec(), |a, b| a < b, |a, b| a < b),
                "gte" => num_cmp(args.to_vec(), |a, b| a >= b, |a, b| a >= b),
                "lte" => num_cmp(args.to_vec(), |a, b| a <= b, |a, b| a <= b),
                "if" => if_impl(args.to_vec()),
                "include" => include_impl(args.to_vec()),
                "import" => import_impl(args.to_vec()),
                "str" => str_impl(args.to_vec()),
                "int" => int_impl(args.to_vec()),
                "float" => float_impl(args.to_vec()),
                "range" => range_impl(args.to_vec()),
                "merge" => merge_impl(args.to_vec()),
                "def" => def_impl(args.to_vec()),
                "map" => map_impl(args.to_vec()),
                "filter" => filter_impl(args.to_vec()),
                "reduce" => reduce_impl(args.to_vec()),
                other => panic!("Unrecognized function '{}'", other),
            },
            JsonPP::Definition(def) => definition_substitution(def, args.to_vec()),
            other => panic!("Cannot call '{:?}'", other),
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

    evaluation::abs_fetch(&evaluation::make_absolute(self_path, &target_path), root)
        .cloned()
        .unwrap()
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

    let index = if args[0].is_truthy() { 1 } else { 2 };
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

    parsing::Parser::from(buffer).parse()
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
        other => panic!("Can't convert {:?} to string", other),
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

fn def_impl(args: Vec<JsonPP>) -> JsonPP {
    assert!(args.len() >= 2);
    let vars = args
        .clone()
        .into_iter()
        .take(args.len() - 1)
        .map(|el| {
            let JsonPP::Identifier(val) = el else {
                panic!("Only identifiers allowed for definition parameters");
            };

            val
        })
        .collect();
    JsonPP::Definition(Definition {
        vars,
        template: Box::new(args.last().unwrap().clone()),
    })
}

fn map_impl(args: Vec<JsonPP>) -> JsonPP {
    assert_eq!(args.len(), 2);

    let callable = args[0].clone();

    match args[1].clone() {
        JsonPP::Array(arr) => JsonPP::Array(
            arr.into_iter()
                .map(|el| {
                    JsonPP::Dynamic(Dynamic {
                        args: vec![callable.clone(), el],
                        ..Default::default()
                    })
                })
                .collect(),
        ),
        JsonPP::Object(obj) => JsonPP::Object(
            obj.into_iter()
                .map(|(key, el)| {
                    (
                        key,
                        JsonPP::Dynamic(Dynamic {
                            args: vec![callable.clone(), el],
                            ..Default::default()
                        }),
                    )
                })
                .collect(),
        ),
        other => panic!("Can't map over '{:?}'", other),
    }
}

fn filter_impl(args: Vec<JsonPP>) -> JsonPP {
    assert_eq!(args.len(), 2);

    let callable = args[0].clone();

    match args[1].clone() {
        JsonPP::Array(arr) => JsonPP::Array(
            arr.into_iter()
                .map(|el| {
                    let cond = JsonPP::Dynamic(Dynamic {
                        args: vec![callable.clone(), el.clone()],
                        ..Default::default()
                    });

                    JsonPP::Dynamic(Dynamic {
                        args: vec![
                            JsonPP::Identifier("if".to_owned()),
                            cond,
                            el,
                            JsonPP::Undefined,
                        ],
                        ..Default::default()
                    })
                })
                .collect(),
        ),
        JsonPP::Object(obj) => JsonPP::Object(
            obj.into_iter()
                .map(|(key, el)| {
                    (key, {
                        let cond = JsonPP::Dynamic(Dynamic {
                            args: vec![callable.clone(), el.clone()],
                            ..Default::default()
                        });

                        JsonPP::Dynamic(Dynamic {
                            args: vec![
                                JsonPP::Identifier("if".to_owned()),
                                cond,
                                el,
                                JsonPP::Undefined,
                            ],
                            ..Default::default()
                        })
                    })
                })
                .collect(),
        ),
        other => panic!("Can't filter over '{:?}'", other),
    }
}

fn reduce_impl(args: Vec<JsonPP>) -> JsonPP {
    assert_eq!(args.len(), 2);

    let callable = args[0].clone();

    dbg!(match args[1].clone() {
        JsonPP::Array(arr) => arr
            .into_iter()
            .reduce(|acc, el| {
                JsonPP::Dynamic(Dynamic {
                    args: vec![callable.clone(), acc, el.clone()],
                    ..Default::default()
                })
            })
            .unwrap_or(JsonPP::Undefined),
        other => panic!("Can't reduce over '{:?}'", other),
    })
}

fn definition_substitution(def: Definition, args: Vec<JsonPP>) -> JsonPP {
    assert_eq!(def.vars.len(), args.len());
    // Substitute all identifiers that corresponding values in the template
    let subs: HashMap<String, JsonPP> = def.vars.into_iter().zip(args.into_iter()).collect();

    recursive_substitute(*def.template, &subs)
}

fn recursive_substitute(object: JsonPP, sub_table: &HashMap<String, JsonPP>) -> JsonPP {
    match object {
        JsonPP::Identifier(ident) if sub_table.contains_key(&ident) => {
            sub_table.get(&ident).unwrap().clone()
        }

        JsonPP::Array(vec) => JsonPP::Array(
            vec.into_iter()
                .map(|elem| recursive_substitute(elem, sub_table))
                .collect(),
        ),
        JsonPP::Object(hash_map) => JsonPP::Object(
            hash_map
                .into_iter()
                .map(|(key, val)| (key, recursive_substitute(val, sub_table)))
                .collect(),
        ),
        JsonPP::Definition(definition) => JsonPP::Definition(Definition {
            template: Box::new(recursive_substitute(*definition.template, sub_table)),
            ..definition
        }),
        JsonPP::Dynamic(dynamic) => JsonPP::Dynamic(Dynamic {
            args: dynamic
                .args
                .into_iter()
                .map(|arg| recursive_substitute(arg, sub_table))
                .collect(),
            ..dynamic
        }),

        // Contains primitives and non-matching identifiers, just leave those alone
        other => other,
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
