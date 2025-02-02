use log::debug;

use crate::jsonpp::{abs_fetch, make_absolute, ref_chain, Function, JsonPP, PathChunk};

pub(crate) fn evaluate(parsed: JsonPP) -> serde_json::Value {
    // Find all the dynamics
    // Update their internals
    let mut dynamic_paths = vec![];
    let mut root = preprocess(&mut dynamic_paths, vec![], parsed);

    while !dynamic_paths.is_empty() {
        let mut resolved = vec![];
        // Resolve all the ones without dependencies
        for dp in dynamic_paths.iter() {
            let JsonPP::Dynamic(dv) = abs_fetch(dp, &root) else {
                panic!("Fetching dynamics yields non-dynamic");
            };

            if dv.dependencies.is_empty() {
                let val = dv.clone().resolve(dp, &root);
                insert(dp, &mut root, val);
                resolved.push(dp.clone());
            }
        }

        if resolved.is_empty() {
            // No dynamics were resolved, there is a reference cycle
            debug!("{:?}", &root);
            debug!("{:?}", &dynamic_paths);
            panic!("Reference cycle");
        }

        // Update existing dependencies
        dynamic_paths.retain(|path| !resolved.contains(path));
        for dp in dynamic_paths.iter() {
            let JsonPP::Dynamic(dv) = abs_fetch(dp, &root) else {
                panic!("Fetching dynamics yields non-dynamic");
            };

            let mut new_dyn = dv.clone();
            new_dyn
                .dependencies
                .retain(|path| !resolved.contains(&make_absolute(dp, path)));
            insert(dp, &mut root, JsonPP::Dynamic(new_dyn));
        }
    }

    let Ok(out) = root.clone().try_into() else {
        panic!("No dynamics left and still can't make it into serde_json::Value");
    };

    out
}

fn preprocess(dyn_paths: &mut Vec<Vec<PathChunk>>, path: Vec<PathChunk>, value: JsonPP) -> JsonPP {
    match value {
        JsonPP::Dynamic(mut dyn_val) => {
            dyn_val.path = path.clone();
            dyn_paths.push(path.clone());

            let mut refs = vec![];

            if dyn_val.fun == Function::Ref {
                assert_eq!(dyn_val.args.len(), 1);
                match dyn_val.args[0].clone() {
                    JsonPP::String(string) => {
                        refs.push(ref_chain(string));
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

                    if let JsonPP::Dynamic(ref inner_dyn) = inner {
                        refs.push(temp_path);
                        refs.extend(inner_dyn.dependencies.clone());
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

fn insert<'a>(path: &[PathChunk], root: &'a mut JsonPP, value: JsonPP) {
    // Put the given value in the designated spot
    if path.is_empty() {
        *root = value;
        return;
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
