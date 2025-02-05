use std::{
    fs::{File, OpenOptions},
    io::{stdin, Read, Write},
};

use log::{debug, info};

mod builtins;
mod evaluation;
mod jsonpp;
mod parsing;
mod paths;

#[derive(Debug, clap::Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(long)]
    input: Option<String>,
    #[arg(long)]
    output: Option<String>,
}
impl Args {
    pub fn execute(self) {
        let mut input_buf = vec![];
        let read_result = if let Some(path) = self.input {
            debug!("Reading file from path: {}", &path);
            let mut file = File::open(path).unwrap();
            file.read_to_end(&mut input_buf).unwrap()
        } else {
            stdin().read_to_end(&mut input_buf).unwrap()
        };

        debug!("Read in {read_result} bytes");

        info!("Parsing");
        let parsed = parsing::Parser::from(input_buf).parse();
        info!("Parsed input, evaluating");
        let evaluated = evaluation::evaluate(parsed);
        info!("Evaluated input");

        if let Some(path) = self.output {
            debug!("Outputting to file: {}", &path);
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .unwrap();

            let _ = file
                .write(&serde_json::to_vec_pretty(&evaluated).unwrap())
                .unwrap();
        } else {
            debug!("Outputting to stdout");
            println!("{}", serde_json::to_string_pretty(&evaluated).unwrap());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_file(path: &'static str) -> Vec<u8> {
        let mut file = File::open(path).unwrap();
        let mut contents = vec![];
        let _ = file.read_to_end(&mut contents).unwrap();
        contents
    }

    fn compare_serde(path: &'static str) {
        let contents = read_file(path);
        let parsed = parsing::Parser::from(contents.clone()).parse();
        dbg!(&parsed);
        let evaluated = evaluation::evaluate(parsed);
        dbg!(&evaluated);
        let serde_version: serde_json::Value = serde_json::from_slice(&contents).unwrap();
        dbg!(&serde_version);

        assert_eq!(evaluated, serde_version);
    }

    fn evaluate_to_equivalent(path1: &'static str, path2: &'static str) {
        let file1 = read_file(path1);
        let file2 = read_file(path2);

        let eval1 = evaluation::evaluate(parsing::Parser::from(file1).parse());
        let eval2 = evaluation::evaluate(parsing::Parser::from(file2).parse());

        assert_eq!(eval1, eval2);
    }

    #[test]
    fn regular_json() {
        compare_serde("parseables/wikipedia.json");
    }

    #[test]
    fn commented_json() {
        evaluate_to_equivalent("parseables/wikipedia.json", "parseables/wikipedia.jsonc");
    }

    #[test]
    fn strings_formats() {
        compare_serde("parseables/strings.json");
    }

    #[test]
    fn number_formats() {
        compare_serde("parseables/numbers.json");
    }

    #[test]
    fn exotic_number_formats() {
        // Serde fails to parse exponents with a decimal point,
        // but they are not in the json spec, but I originally misread
        // the spec and implemented them anyways
        let contents = read_file("parseables/exotic_numbers.json");
        let parsed = parsing::Parser::from(contents.clone()).parse();
        let evaluated = evaluation::evaluate(parsed);
        let serde_json::Value::Array(arr) = evaluated else {
            panic!("Non-array return when parsing exotic number array");
        };

        let pos_exp = 10.0f64.powf(1.2);
        let neg_exp = 10.0f64.powf(-1.2);
        let targets = [
            // In the exotic file these vary in e vs E and some have a + before the exponent
            1.2 * pos_exp,
            1.2 * pos_exp,
            1.2 * neg_exp,
            1.2 * pos_exp,
            1.2 * pos_exp,
            1.2 * neg_exp,
            -1.2 * pos_exp,
            -1.2 * pos_exp,
            -1.2 * neg_exp,
            -1.2 * pos_exp,
            -1.2 * pos_exp,
            -1.2 * neg_exp,
        ];

        for (elem, target) in arr.into_iter().zip(targets) {
            let serde_json::Value::Number(val) = elem else {
                panic!("Non-numeric value in the exotic number array");
            };

            assert!(val.is_f64());
            let float = val.as_f64().unwrap();
            assert_eq!(float, target);
        }
    }

    #[test]
    fn simple_dynamic() {
        evaluate_to_equivalent(
            "parseables/simple_dynamic.json++",
            "parseables/simple_dynamic_resolved.json",
        );
    }

    #[test]
    fn reference_dynamic() {
        evaluate_to_equivalent(
            "parseables/ref_dynamic.json++",
            "parseables/ref_dynamic_resolved.json",
        );
    }

    #[test]
    fn import_and_include() {
        evaluate_to_equivalent(
            "parseables/import.json++",
            "parseables/import_resolved.json",
        );
    }

    #[test]
    fn undefined_if() {
        evaluate_to_equivalent(
            "parseables/undefined.json++",
            "parseables/undefined_resolved.json",
        );
    }

    #[test]
    fn def_and_folds() {
        evaluate_to_equivalent("parseables/def.json++", "parseables/def_resolved.json");
    }
}
