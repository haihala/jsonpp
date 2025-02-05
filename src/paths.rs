#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum PathChunk {
    Parent,
    Key(String),
    Index(usize),
    Argument(usize),
}

pub(crate) fn make_absolute(self_path: &[PathChunk], target_path: &[PathChunk]) -> Vec<PathChunk> {
    if target_path.first() == Some(&PathChunk::Parent) {
        // Relative path
        let mut out: Vec<PathChunk> = self_path.to_vec();
        // Skip the first part. This allows for easier self ref
        for chunk in target_path.iter().skip(1) {
            if *chunk == PathChunk::Parent {
                out.pop();
            } else {
                out.push(chunk.clone());
            }
        }

        return out;
    }

    target_path.to_vec()
}

pub(crate) fn ref_chain(path: String) -> Vec<PathChunk> {
    path.split(".")
        .map(|chunk| {
            if chunk.is_empty() {
                return PathChunk::Parent;
            }

            if chunk.starts_with("[") && chunk.ends_with("]") {
                let inner = &chunk[1..(chunk.len() - 1)];
                return PathChunk::Index(inner.parse().unwrap());
            }

            if chunk.starts_with("(") && chunk.ends_with(")") {
                let inner = &chunk[1..(chunk.len() - 1)];
                return PathChunk::Argument(inner.parse().unwrap());
            }

            PathChunk::Key(chunk.to_owned())
        })
        .collect()
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
        let target_path = vec![
            PathChunk::Parent,
            PathChunk::Parent,
            PathChunk::Key("Bar".to_owned()),
        ];
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
