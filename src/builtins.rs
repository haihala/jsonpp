use std::{fs::File, io::Read};

use crate::{
    evaluation,
    jsonpp::{Definition, Dynamic, JsonPP},
    parsing,
    paths::{make_absolute, ref_chain, PathChunk},
};

pub(crate) fn num_cmp(
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

pub(crate) fn sum_impl(args: Vec<JsonPP>) -> JsonPP {
    num_reduce(|a, b| a + b, |a, b| a + b, args)
}

pub(crate) fn mul_impl(args: Vec<JsonPP>) -> JsonPP {
    num_reduce(|a, b| a * b, |a, b| a * b, args)
}

pub(crate) fn sub_impl(args: Vec<JsonPP>) -> JsonPP {
    assert_eq!(args.len(), 2);
    num_reduce(|a, b| a - b, |a, b| a - b, args)
}

pub(crate) fn div_impl(args: Vec<JsonPP>) -> JsonPP {
    assert_eq!(args.len(), 2);
    if matches!(args[1], JsonPP::Float(0.0) | JsonPP::Int(0)) {
        dbg!("(div {:?})", args);
        panic!("Division by zero");
    }
    num_reduce(|a, b| a / b, |a, b| a / b, args)
}

pub(crate) fn mod_impl(args: Vec<JsonPP>) -> JsonPP {
    assert_eq!(args.len(), 2);
    num_reduce(|a, b| a % b, |a, b| a % b, args)
}

pub(crate) fn pow_impl(args: Vec<JsonPP>) -> JsonPP {
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

pub(crate) fn log_impl(args: Vec<JsonPP>) -> JsonPP {
    assert_eq!(args.len(), 2);
    num_reduce(
        |a, b| b.ilog(a) as i64,
        |a, b| {
            if a == 1.0 {
                panic!("There is no base 1 logarithm")
            } else {
                b.log(a)
            }
        },
        args,
    )
}

pub(crate) fn len_impl(args: Vec<JsonPP>) -> JsonPP {
    assert_eq!(args.len(), 1);

    JsonPP::Int(match &args[0] {
        JsonPP::String(inner) => inner.len() as i64,
        JsonPP::Array(inner) => inner.len() as i64,
        JsonPP::Object(inner) => inner.len() as i64,
        _ => panic!("Trying to get the length of something odd"),
    })
}

pub(crate) fn ref_impl(args: Vec<JsonPP>, self_path: &[PathChunk], root: &JsonPP) -> JsonPP {
    let JsonPP::String(target) = args[0].clone() else {
        panic!("Non-string reference: {:?}", args);
    };

    let target_path = ref_chain(target);

    evaluation::abs_fetch(&make_absolute(self_path, &target_path), root)
        .cloned()
        .unwrap()
}

pub(crate) fn min_impl(args: Vec<JsonPP>) -> JsonPP {
    num_reduce(i64::min, f64::min, args)
}

pub(crate) fn max_impl(args: Vec<JsonPP>) -> JsonPP {
    num_reduce(i64::max, f64::max, args)
}

pub(crate) fn eq_impl(args: Vec<JsonPP>) -> JsonPP {
    assert_eq!(args.len(), 2);

    let first_arg = args[0].clone();
    let second_arg = args[1].clone();

    JsonPP::Bool(first_arg == second_arg)
}

pub(crate) fn if_impl(args: Vec<JsonPP>) -> JsonPP {
    assert_eq!(args.len(), 3); // Condition, if true, if not;

    let index = if args[0].is_truthy() { 1 } else { 2 };
    args[index].clone()
}

pub(crate) fn include_impl(args: Vec<JsonPP>) -> JsonPP {
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

pub(crate) fn import_impl(args: Vec<JsonPP>) -> JsonPP {
    assert_eq!(args.len(), 1);

    let JsonPP::String(path) = args[0].clone() else {
        panic!("Import path is not a string")
    };

    let mut file = File::open(path).unwrap();
    let mut buffer = vec![];
    file.read_to_end(&mut buffer).unwrap();

    parsing::Parser::from(buffer).parse()
}

pub(crate) fn str_impl(args: Vec<JsonPP>) -> JsonPP {
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

pub(crate) fn int_impl(args: Vec<JsonPP>) -> JsonPP {
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

pub(crate) fn float_impl(args: Vec<JsonPP>) -> JsonPP {
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

pub(crate) fn range_impl(args: Vec<JsonPP>) -> JsonPP {
    assert_eq!(args.len(), 2);

    let JsonPP::Int(start) = args[0].clone() else {
        panic!("Range start is not an int")
    };
    let JsonPP::Int(end) = args[1].clone() else {
        panic!("Range end is not an int")
    };

    JsonPP::Array((start..end).map(JsonPP::Int).collect())
}

pub(crate) fn merge_impl(args: Vec<JsonPP>) -> JsonPP {
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

pub(crate) fn def_impl(args: Vec<JsonPP>) -> JsonPP {
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

pub(crate) fn map_impl(args: Vec<JsonPP>) -> JsonPP {
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

pub(crate) fn filter_impl(args: Vec<JsonPP>) -> JsonPP {
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

pub(crate) fn reduce_impl(args: Vec<JsonPP>) -> JsonPP {
    assert_eq!(args.len(), 2);

    let callable = args[0].clone();

    match args[1].clone() {
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
    }
}

pub(crate) fn values_impl(args: Vec<JsonPP>) -> JsonPP {
    assert_eq!(args.len(), 1);
    let JsonPP::Object(obj) = args[0].clone() else {
        panic!("Non-object argument to 'values'");
    };

    JsonPP::Array(obj.values().cloned().collect())
}

pub(crate) fn keys_impl(args: Vec<JsonPP>) -> JsonPP {
    assert_eq!(args.len(), 1);
    let JsonPP::Object(obj) = args[0].clone() else {
        panic!("Non-object argument to 'keys'");
    };

    JsonPP::Array(
        obj.keys()
            .map(|key| JsonPP::String(key.to_string()))
            .collect(),
    )
}
