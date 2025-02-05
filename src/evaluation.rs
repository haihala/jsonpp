use std::collections::{HashMap, HashSet};

use log::debug;

use crate::{
    jsonpp::{Definition, Dynamic, JsonPP},
    paths::{make_absolute, ref_chain, PathChunk},
};

pub(crate) fn evaluate_raw(parsed: JsonPP) -> JsonPP {
    // Find all the dynamics
    // Update their internals
    let mut dynamic_paths: HashSet<Vec<PathChunk>> = vec![].into_iter().collect();
    let mut root = preprocess(&mut dynamic_paths, vec![], parsed);

    while !dynamic_paths.is_empty() {
        let mut progressing = false;
        // Resolve all the ones without dependencies
        for dyn_path in dynamic_paths.clone().iter() {
            let JsonPP::Dynamic(dyn_val) = abs_fetch(dyn_path, &root).unwrap() else {
                panic!("Fetching dynamics yields non-dynamic");
            };

            let dyn_deps = dyn_val.dependencies.iter().filter(|dep| {
                let path = make_absolute(dyn_path, dep);

                let target = abs_fetch(&path, &root).unwrap();
                matches!(target, JsonPP::Dynamic(_))
            });

            if dyn_deps.count() == 0 {
                progressing = true;
                let val = dyn_val.clone().resolve(dyn_path, &root);
                let processed = preprocess(&mut dynamic_paths, dyn_path.clone(), val);
                if !matches!(processed, JsonPP::Dynamic(_)) {
                    // Resolved into something non-dynamic
                    dynamic_paths.remove(dyn_path);
                }
                insert(dyn_path, &mut root, processed);
            }
        }

        if !progressing {
            // No dynamics were resolved, there is a reference cycle
            debug!("{:?}", &root);
            debug!("{:?}", &dynamic_paths);
            panic!("Reference cycle");
        }
    }

    root
}

pub(crate) fn evaluate(parsed: JsonPP) -> serde_json::Value {
    let root = evaluate_raw(parsed);

    let Ok(out) = root.clone().try_into() else {
        panic!("No dynamics left and still can't make it into serde_json::Value");
    };

    out
}

fn preprocess(
    dyn_paths: &mut HashSet<Vec<PathChunk>>,
    path: Vec<PathChunk>,
    value: JsonPP,
) -> JsonPP {
    match value {
        JsonPP::Dynamic(mut dyn_val) => {
            if dyn_val.is_def() {
                // Evaluate it immediately
                // root is not used for definitions
                return dyn_val.resolve(&path, &JsonPP::Null);
            }

            dyn_val.path = path.clone();
            dyn_paths.insert(path.clone());

            let mut refs = vec![];

            if dyn_val.is_ref() {
                assert_eq!(dyn_val.args.len(), 2);
                match dyn_val.args[1].clone() {
                    JsonPP::String(string) => {
                        refs.push(make_absolute(&path, &ref_chain(string)));
                    }
                    JsonPP::Dynamic(_) => {}
                    other => panic!("Trying to call ref on {:?}", other),
                }
            }

            dyn_val.args = dyn_val
                .args
                .into_iter()
                .enumerate()
                .map(|(index, arg)| {
                    let mut temp_path = path.clone();
                    temp_path.push(PathChunk::Argument(index));
                    let inner = preprocess(dyn_paths, temp_path.clone(), arg.to_owned());

                    if matches!(inner, JsonPP::Dynamic(_)) {
                        refs.push(temp_path);
                    };

                    inner
                })
                .collect();

            dyn_val.dependencies = refs;
            JsonPP::Dynamic(dyn_val)
        }
        JsonPP::Array(arr) => JsonPP::Array(
            arr.into_iter()
                .enumerate()
                .map(|(index, val)| {
                    let mut temp_path = path.clone();
                    temp_path.push(PathChunk::Index(index.to_owned()));
                    preprocess(dyn_paths, temp_path, val.to_owned())
                })
                .collect(),
        ),
        JsonPP::Object(obj) => JsonPP::Object(
            obj.into_iter()
                .map(|(key, val)| {
                    let mut temp_path = path.clone();
                    temp_path.push(PathChunk::Key(key.to_owned()));
                    (key, preprocess(dyn_paths, temp_path, val.to_owned()))
                })
                .collect(),
        ),
        _ => value,
    }
}

fn insert(path: &[PathChunk], root: &mut JsonPP, value: JsonPP) {
    // Put the given value in the designated spot
    if path.is_empty() {
        *root = value;
        return;
    }

    let next = &path[0];
    let rest = &path[1..];

    match next {
        PathChunk::Parent => {
            panic!("You are not supposed to have a parent in the path when inserting")
        }
        PathChunk::Key(key) => {
            let JsonPP::Object(inner) = root else {
                debug!("{:?}, {:?}, {:?}", root, key, path);
                panic!("Accessing with a key");
            };

            insert(rest, inner.get_mut(key).unwrap(), value)
        }
        PathChunk::Index(index) => {
            let JsonPP::Array(inner) = root else {
                debug!("{:?}, {:?}, {:?}", root, index, path);
                panic!("Accessing with an index");
            };

            insert(rest, &mut inner[*index], value)
        }
        PathChunk::Argument(index) => {
            let JsonPP::Dynamic(inner) = root else {
                debug!("{:?}, {:?}, {:?}", root, index, path);
                panic!("Accessing with an argument");
            };

            insert(rest, &mut inner.args[*index], value)
        }
    }
}

pub(crate) fn abs_fetch<'a>(path: &[PathChunk], root: &'a JsonPP) -> Option<&'a JsonPP> {
    if path.is_empty() {
        return Some(root);
    }

    let next = &path[0];
    let rest = &path[1..];

    match next {
        PathChunk::Parent => panic!("Absolute path fetching needs an absolute path"),
        PathChunk::Key(key) => {
            let JsonPP::Object(inner) = root else {
                debug!("Accessing with a key: {:?}, {:?}, {:?}", root, key, path);
                return None;
            };

            abs_fetch(rest, &inner[key])
        }
        PathChunk::Index(index) => {
            let JsonPP::Array(inner) = root else {
                debug!(
                    "Accessing with an index: {:?}, {:?}, {:?}",
                    root, index, path
                );
                return None;
            };

            abs_fetch(rest, &inner[*index])
        }
        PathChunk::Argument(index) => {
            let JsonPP::Dynamic(inner) = root else {
                debug!(
                    "Accessing with an argument: {:?}, {:?}, {:?}",
                    root, index, path
                );
                return None;
            };

            abs_fetch(rest, &inner.args[*index])
        }
    }
}

pub(crate) fn definition_substitution(def: Definition, args: Vec<JsonPP>) -> JsonPP {
    assert_eq!(def.vars.len(), args.len());
    // Substitute all identifiers that corresponding values in the template
    let subs: HashMap<String, JsonPP> = def.vars.into_iter().zip(args).collect();

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
