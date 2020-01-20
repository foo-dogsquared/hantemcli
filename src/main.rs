use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::process;

use config;
use handlebars;
use structopt::StructOpt;
use toml;

mod args;
mod templates;

use args::Hantemcli;

static ERROR_EXIT_STATUS: i64 = 1;
static FILE_EXTENSION: &str = "hbs";
static FILE_EXTENSION_WITH_DOT: &str = ".hbs";

fn main() {
    let args = Hantemcli::from_args();

    match parse_args(args) {
        Ok(_) => (),
        Err(e) => {
            eprintln!("Final error:\n---\n{}\n---\n", e);
            process::exit(ERROR_EXIT_STATUS as i32);
        }
    }
}

pub fn parse_args(args: Hantemcli) -> Result<(), Box<dyn Error>> {
    let mut template_registry = handlebars::Handlebars::new();
    template_registry.set_strict_mode(args.strict);

    let mut raw_config: config::Config = config::Config::new();
    for data_path in args.data_paths.iter() {
        match raw_config
            .merge(config::File::with_name(&data_path.to_string_lossy()).required(false))
        {
            Ok(_v) => (),
            Err(e) => eprintln!(
                "An error occurred for the data file {:?}\n{}\n",
                data_path, e
            ),
        }
    }
    let data: toml::Value = raw_config
        .try_into()
        .expect("The resulting TOML has an value error occurred... That shouldn't happened.");

    for template in args.templates {
        if template.is_dir() {
            match template_registry.register_templates_directory(FILE_EXTENSION_WITH_DOT, template)
            {
                Ok(_v) => continue,
                Err(e) => eprintln!("{}", e),
            }
        } else {
            match template.extension() {
                Some(v) => {
                    let extension_str: &str = v.to_str().unwrap();
                    if extension_str != FILE_EXTENSION {
                        eprintln!("{:?} does not have the required extension.", template);
                        continue;
                    }
                }
                None => continue,
            }

            let name = match templates::path_without_extension(&template) {
                Some(v) => v,
                None => {
                    eprintln!("{:?} have an error getting the file path.", template);
                    continue;
                }
            };

            match template_registry.register_template_file(&name.to_string_lossy(), template) {
                Ok(_v) => (),
                Err(e) => eprintln!("{}", e),
            };
        }
    }

    let root = match args.root {
        Some(v) => v,
        None => template_registry
            .get_templates()
            .keys()
            .min()
            .expect("There's no templates registered in the registry.")
            .clone(),
    };

    match template_registry.render(&root, &data) {
        Ok(v) => match args.output {
            Some(output_path) => {
                let mut output_file = File::create(output_path)?;

                output_file.write(v.as_bytes())?;
            }
            None => println!("{}", v),
        },
        Err(e) => {
            eprintln!("\nA Handlebars template error has occurred.");
            eprintln!("{}\n", e);
            return Err(Box::new(e));
        }
    };

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_usage_test() {
        let args = vec![
            "hantemcli",
            "tests/template.hbs",
            "--",
            "tests/default.toml",
            "tests/dev.toml",
        ];
        let parsed_args = Hantemcli::from_iter(args.iter());

        let result = parse_args(parsed_args);
        assert!(result.is_ok());
    }
}
