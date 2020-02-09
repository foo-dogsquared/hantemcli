use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process;

use config;
use handlebars;
use structopt::StructOpt;
use toml;

mod args;
mod repl;
mod templates;

use args::Hantemcli;

static ERROR_EXIT_STATUS: i64 = 1;

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

    // Getting the data from the files.
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
    raw_config.merge(config::Environment::new())?;

    // Merging the data from environment variables.
    raw_config.merge(config::Environment::new())?;

    if args.repl {
        let mut repl_env = repl::Repl::default();
        repl_env.data = raw_config;
        repl_env.template_registry = template_registry;

        repl_env._loop()?;
    } else {
        let data: toml::Value = raw_config
            .try_into()
            .expect("The resulting TOML has an value error occurred... That shouldn't happened.");

        let root = match args.root {
            Some(v) => v,
            None => template_registry
                .get_templates()
                .keys()
                .min()
                .expect("There's no templates registered in the registry.")
                .clone(),
        };
        let rendered_template = template_registry.render(&root, &data)?;

        match args.output {
            Some(output_path) => {
                let mut output_file = File::create(output_path)?;

                output_file.write(rendered_template.as_bytes())?;
            }
            None => println!("{}", rendered_template),
        }
    }

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
