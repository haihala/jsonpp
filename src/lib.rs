use std::{
    fs::{File, OpenOptions},
    io::{stdin, Read, Write},
};

use clap;
use log::debug;
use serde_json;

mod evaluation;
mod jsonpp;
mod parsing;

#[derive(Debug, clap::Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    input: Option<String>,
    output: Option<String>,
}
impl Args {
    pub fn execute(self) {
        let mut input_buf = vec![];
        let read_result = if let Some(path) = self.input {
            let mut file = File::open(path).unwrap();
            file.read_to_end(&mut input_buf).unwrap()
        } else {
            stdin().read_to_end(&mut input_buf).unwrap()
        };

        debug!("Read in {read_result} bytes");

        debug!("Parsing");
        let parsed = parsing::parse(input_buf);
        debug!("Parsed input");

        debug!("Evaluating");
        let evaluated = evaluation::evaluate(parsed);
        debug!("Evaluated input");

        if let Some(path) = self.output {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .unwrap();

            file.write(&serde_json::to_vec_pretty(&evaluated).unwrap())
                .unwrap();
        } else {
            println!("{}", serde_json::to_string_pretty(&evaluated).unwrap());
        }
    }
}
