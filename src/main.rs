use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process;

use config;
use handlebars;
use structopt::StructOpt;
use toml;
use walkdir;

mod args;
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

    // Sanitizing the path naively.
    let extension = match args.extension.starts_with(".") {
        true => args.extension,
        false => format!(".{}", args.extension),
    };

    // A closure to easily register a path into the template registry.
    // It will return a boolean indicating the success of the registration.
    let mut register_file_to_template_registry = |template: &Path, base_dir: &Path| -> bool {
        let normalized_base_dir = templates::naively_normalize_path(&base_dir);

        let name = match templates::path_without_extension(&template) {
            Some(v) => templates::naively_normalize_path(v),
            None => {
                eprintln!("{:?} have an error getting the file path.", template);
                return false;
            }
        };

        let relpath_from_base_dir = match templates::relative_path_from(&name, &normalized_base_dir)
        {
            Some(v) => v,
            None => {
                eprintln!("{:?} has an error getting the relative path of the template. How's that possible?", template);
                return false;
            }
        };

        match template_registry
            .register_template_file(relpath_from_base_dir.to_str().unwrap(), &template)
        {
            Ok(_v) => (),
            Err(e) => {
                eprintln!("Template file {:?} has an error.", &template);
                eprintln!("{}", e);
                return false;
            }
        };

        return true;
    };

    for template in args.templates {
        if template.is_dir() {
            let walker = walkdir::WalkDir::new(&template).min_depth(1).into_iter();
            for entry in walker.filter_map(|e| e.ok()).filter(|e| {
                e.path().is_file() && templates::has_file_extension(e.path(), &extension)
            }) {
                register_file_to_template_registry(&entry.path(), &template);
            }
        } else {
            if !templates::has_file_extension(&template, &extension) {
                continue;
            }

            register_file_to_template_registry(
                &template,
                &template.parent().unwrap_or(Path::new("./")),
            );
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
