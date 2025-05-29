use std::{collections::HashMap, iter::Peekable};

use crate::{
    jsonpp::{Dynamic, JsonPP},
    paths::PathChunk,
    tokenizing::Token,
};

pub fn build_ast(token_stream: impl Iterator<Item = Token>) -> JsonPP {
    let mut peekable = token_stream.peekable();
    build(&mut peekable, vec![])
}

fn build(token_stream: &mut Peekable<impl Iterator<Item = Token>>, path: Vec<PathChunk>) -> JsonPP {
    let Some(next_token) = token_stream.next() else {
        panic!("Stream ran out")
    };

    match next_token {
        Token::Int(num) => JsonPP::Int(num),
        Token::Float(num) => JsonPP::Float(num),
        Token::Text(txt) => JsonPP::String(txt),
        Token::Ident(ident) if ident == "undefined" => JsonPP::Undefined,
        Token::Ident(ident) if ident == "null" => JsonPP::Null,
        Token::Ident(ident) if ident == "true" => JsonPP::Bool(true),
        Token::Ident(ident) if ident == "false" => JsonPP::Bool(false),
        Token::Ident(ident) => JsonPP::Identifier(ident),

        Token::OpenParanthesis => {
            let mut args = vec![];
            while let Some(token) = token_stream.peek() {
                if token == &Token::CloseParanthesis {
                    assert_eq!(token_stream.next().unwrap(), Token::CloseParanthesis);
                    return JsonPP::Dynamic(Dynamic {
                        path,
                        args,
                        ..Default::default()
                    });
                }

                let mut new_path = path.clone();
                new_path.push(PathChunk::Argument(args.len()));
                args.push(build(token_stream, new_path));
            }

            panic!("Token stream ran dry mid parse (Dynamic)")
        }
        Token::OpenBracket => {
            let mut args = vec![];
            while let Some(token) = token_stream.peek() {
                if token == &Token::CloseBracket {
                    assert_eq!(token_stream.next().unwrap(), Token::CloseBracket);
                    return JsonPP::Array(args);
                }

                let mut new_path = path.clone();
                new_path.push(PathChunk::Index(args.len()));
                args.push(build(token_stream, new_path));
            }

            panic!("Token stream ran dry mid parse (Array)")
        }
        Token::OpenBrace => {
            let mut args: HashMap<String, JsonPP> = HashMap::new();
            while let Some(token) = token_stream.next() {
                if token == Token::CloseBrace {
                    return JsonPP::Object(args);
                }

                if let Token::Text(key) = token {
                    let colon = token_stream.next().expect("Colon of object");
                    assert_eq!(colon, Token::Colon);

                    let mut new_path = path.clone();
                    new_path.push(PathChunk::Key(key.to_string()));
                    args.insert(key.to_string(), build(token_stream, new_path));
                }
            }

            panic!("Token stream ran dry mid parse (Array)")
        }

        closer => panic!("Ran into a closing {:?} unexpectedly", closer),
    }
}
