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
    #[arg(long)]
    force: bool,
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
                .create_new(!self.force)
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

    fn read_file(path: &str) -> Vec<u8> {
        let mut file = File::open(path).unwrap();
        let mut contents = vec![];
        let _ = file.read_to_end(&mut contents).unwrap();
        contents
    }

    fn compare_serde(path: &'static str) {
        let contents = read_file(&format!("parseables/serde_comparison/{}", path));
        let parsed = parsing::Parser::from(contents.clone()).parse();
        dbg!(&parsed);
        let evaluated = evaluation::evaluate(parsed);
        dbg!(&evaluated);
        let serde_version: serde_json::Value = serde_json::from_slice(&contents).unwrap();
        dbg!(&serde_version);

        assert_eq!(evaluated, serde_version);
    }

    fn evaluate_to_equivalent(path: &'static str) {
        let file1 = read_file(&format!("parseables/evaluation_inputs/{}.jsonpp", path));
        let file2 = read_file(&format!("parseables/evaluation_outputs/{}.json", path));

        let eval1 = evaluation::evaluate(parsing::Parser::from(file1).parse());
        let eval2 = evaluation::evaluate(parsing::Parser::from(file2).parse());

        assert_eq!(eval1, eval2);
    }

    #[test]
    fn regular_json() {
        compare_serde("wikipedia.json");
    }

    #[test]
    fn strings_formats() {
        compare_serde("strings.json");
    }

    #[test]
    fn number_formats() {
        compare_serde("numbers.json");
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
    fn commented_json() {
        evaluate_to_equivalent("wikipedia");
    }

    #[test]
    fn sums() {
        evaluate_to_equivalent("sum");
    }

    #[test]
    fn subs() {
        evaluate_to_equivalent("sub");
    }

    #[test]
    fn muls() {
        evaluate_to_equivalent("mul");
    }

    #[test]
    fn divs() {
        evaluate_to_equivalent("div");
    }

    #[test]
    fn pows() {
        evaluate_to_equivalent("pow");
    }

    #[test]
    fn logs() {
        evaluate_to_equivalent("log");
    }

    #[test]
    fn lens() {
        evaluate_to_equivalent("len");
    }

    #[test]
    fn mins() {
        evaluate_to_equivalent("min");
    }

    #[test]
    fn maxs() {
        evaluate_to_equivalent("max");
    }

    #[test]
    fn mods() {
        evaluate_to_equivalent("mod");
    }

    #[test]
    fn eqs() {
        evaluate_to_equivalent("eq");
    }

    #[test]
    fn lts() {
        evaluate_to_equivalent("lt");
    }

    #[test]
    fn ltes() {
        evaluate_to_equivalent("lte");
    }

    #[test]
    fn gts() {
        evaluate_to_equivalent("gt");
    }

    #[test]
    fn gtes() {
        evaluate_to_equivalent("gte");
    }

    #[test]
    fn strs() {
        evaluate_to_equivalent("str");
    }

    #[test]
    fn ints() {
        evaluate_to_equivalent("int");
    }

    #[test]
    fn floats() {
        evaluate_to_equivalent("float");
    }

    #[test]
    fn merges() {
        evaluate_to_equivalent("merge");
    }

    #[test]
    fn ranges() {
        evaluate_to_equivalent("range");
    }

    #[test]
    fn simple_dynamic() {
        evaluate_to_equivalent("simple_dynamic");
    }

    #[test]
    fn reference_dynamic() {
        evaluate_to_equivalent("ref_dynamic");
    }

    #[test]
    fn import_and_include() {
        evaluate_to_equivalent("import");
    }

    #[test]
    fn undefined_if() {
        evaluate_to_equivalent("undefined");
    }

    #[test]
    fn self_ref() {
        evaluate_to_equivalent("self_ref");
    }

    #[test]
    fn keys_vals() {
        let file = read_file("parseables/keys_vals.jsonpp");
        let eval = evaluation::evaluate(parsing::Parser::from(file).parse());
        dbg!(&eval);
        // Keys and values don't guarantee order
        let serde_json::Value::Object(obj) = eval else {
            panic!("Not an object");
        };

        let serde_json::Value::Array(keys) = obj.get_key_value("keys").unwrap().1 else {
            panic!("Keys is not an array");
        };
        let serde_json::Value::Array(values) = obj.get_key_value("values").unwrap().1 else {
            panic!("Values is not an array");
        };

        for i in 1..=5 {
            assert!(keys.contains(&serde_json::Value::String(format!("key{}", i))));
            assert!(values.contains(&serde_json::Value::Number(i.into())));
        }
    }

    #[test]
    fn def_and_folds() {
        evaluate_to_equivalent("def");
    }

    #[test]
    fn def_dyn_def() {
        // Definitions in dynamics in definitions
        // This will break if evaluation order breaks
        evaluate_to_equivalent("def_dyn_def");
    }
}
