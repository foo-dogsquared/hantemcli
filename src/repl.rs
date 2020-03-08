use std::convert::{TryFrom, TryInto};
use std::env;
use std::error::Error;
use std::fmt::Debug;
use std::io::{self, Write};
use std::path::PathBuf;

use config::{self, Source};
use handlebars;
use toml;

use crate::templates;

static HELP_STRING: &str = "The Hantemcli has a few subcommands to evaluate. 

* add [data | template] FILES... - add the data/template in the respective cache
* cd PATH - change the current working directory of the process
* exit - exit the REPL
* help - view the help section
* render KEY - render the template with the data
* reset [data | template] - clear the data/template cache
* view [data | template] KEY - view the containing template string/data of the key
* pwd - print the current working directory of the process
";

#[derive(Debug)]
pub enum Type {
    Data,
    TemplateRegistry,
}

impl TryFrom<&str> for Type {
    type Error = String;

    fn try_from(string: &str) -> Result<Self, Self::Error> {
        match string {
            "data" => Ok(Self::Data),
            "templates" | "template" => Ok(Self::TemplateRegistry),
            _ => Err(format!("No such keyword as {}", string)),
        }
    }
}

#[derive(Debug)]
pub enum ReplCommand {
    Add(Type, Vec<PathBuf>),
    Reset(Type),
    View(Type, String),
    Render(String),
    ChangeDirectory(String),
    Pwd, // Present working directory
    Help,
    Exit,
}

impl TryFrom<&str> for ReplCommand {
    type Error = String;

    fn try_from(string: &str) -> Result<Self, Self::Error> {
        let mut args = string.split_whitespace();
        let command = match args.next() {
            Some(c) => c,
            None => return Err("No command given.".to_string()),
        };

        match command {
            "add" => {
                let _type = match args.next() {
                    Some(c) => c,
                    None => return Err("No subcommand given.".to_string()),
                };

                let subcommand = Type::try_from(_type)?;
                Ok(Self::Add(
                    subcommand,
                    args.map(|arg| PathBuf::from(arg)).collect(),
                ))
            }
            "reset" => {
                let _type = match args.next() {
                    Some(c) => c,
                    None => return Err("No subcommand given.".to_string()),
                };

                let subcommand = Type::try_from(_type)?;
                Ok(Self::Reset(subcommand))
            }
            "view" => {
                let _type = args.next().ok_or("No subcommand given".to_string())?;

                let subcommand = Type::try_from(_type)?;
                let key = args
                    .next()
                    .and_then(|v| Some(v.to_string()))
                    .ok_or("No key given.".to_string())?;
                Ok(Self::View(subcommand, key))
            }
            "render" => {
                let key = args
                    .next()
                    .and_then(|v| Some(v.to_string()))
                    .ok_or("No key given.".to_string())?;

                Ok(Self::Render(key))
            }
            "cd" => {
                let path = args.next().and_then(|v| Some(v.to_string())).ok_or("No path given.".to_string())?;

                Ok(Self::ChangeDirectory(path))
            }
            "pwd" => Ok(Self::Pwd),
            "help" | "?" => Ok(Self::Help),
            "exit" => Ok(Self::Exit),
            _ => Err(format!("No such keyword as {:?}", string)),
        }
    }
}

pub enum ReplError<'s> {
    ReadError(&'s str),
    EvalError(&'s str),
}

pub struct Repl {
    pub template_registry: handlebars::Handlebars<'static>,
    pub data: config::Config,
    pub prompt: String,
    pub file_extension: String,
}

impl Default for Repl {
    fn default() -> Self {
        Self {
            template_registry: handlebars::Handlebars::new(),
            data: config::Config::new(),
            prompt: "> ".to_string(),
            file_extension: "hbs".to_string(),
        }
    }
}

impl Repl {
    pub fn _loop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!(
            "This is the Hantemcli REPL. Allows for continuous use of the Handlebars registry."
        );
        println!("Enter the command 'help' for more information.\n");

        loop {
            print!("{}", self.prompt);

            io::stdout().flush().unwrap();
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            let command = match self.read(input) {
                Ok(v) => match v {
                    ReplCommand::Exit => return Ok(()),
                    _ => v,
                },
                Err(_e) => continue,
            };

            match self.eval(command) {
                Ok(_v) => continue,
                Err(e) => eprintln!("{:?}", e),
            };
        }
    }

    pub fn read(
        &self,
        read_line: String,
    ) -> Result<ReplCommand, String> {
        ReplCommand::try_from(read_line.as_ref())
    }

    /// Evaluate the command and immediately print the results.
    /// (These may be divided into its separate functions of evaluation and printing but whatever.)
    pub fn eval(
        &mut self,
        command: ReplCommand,
    ) -> Result<(), Box<dyn Error>> {
        match command {
            ReplCommand::Help => println!("{}", HELP_STRING),
            ReplCommand::Add(item, paths) => match item {
                Type::Data => {
                    // Getting the data from the files.
                    for data_path in paths.iter() {
                        match self.data.merge(
                            config::File::with_name(&data_path.to_string_lossy()).required(false),
                        ) {
                            Ok(_v) => println!(
                                "The data within the path {:?} has been merged.",
                                data_path
                            ),
                            Err(e) => eprintln!(
                                "An error occurred for the data file {:?}\n{}\n",
                                data_path, e
                            ),
                        }
                    }
                }
                Type::TemplateRegistry => {
                    let registered_paths = templates::register_from_path(
                        &mut self.template_registry,
                        paths,
                        &self.file_extension,
                    )?;

                    for registered_file in registered_paths {
                        println!(
                            "The template file {:?} has successfully registered.",
                            registered_file
                        );
                    }
                }
            },
            ReplCommand::Reset(item) => match item {
                Type::Data => {
                    self.data = config::Config::new();
                    println!("The data table has been cleared.");
                }
                Type::TemplateRegistry => {
                    self.template_registry = handlebars::Handlebars::new();
                    println!("The template registry has been cleared.");
                }
            },
            ReplCommand::View(item, key) => match item {
                Type::Data => {
                    let data: toml::Value = self.data.clone()
                        .try_into()
                        .expect("The resulting TOML has an value error occurred... That shouldn't happened.");

                    match data.get(key) {
                        Some(v) => println!("{}", toml::to_string_pretty(v)?),
                        None => eprintln!("There's no value for the given key."),
                    }
                }
                Type::TemplateRegistry => {
                    let template_store = self.template_registry.get_templates();

                    match template_store.get(&key) {
                        Some(v) => println!("{:?}", v),
                        None => eprintln!("There's no template for the given key."),
                    }
                }
            },
            ReplCommand::ChangeDirectory(path) => {
                match env::set_current_dir(&path) {
                    Ok(_v) => println!("Changed to {:?} successfully", path),
                    Err(e) => eprintln!("{}", e),
                }
            },
            ReplCommand::Pwd => {
                match env::current_dir() {
                    Ok(v) => println!("{:?}", v),
                    Err(e) => eprintln!("{}", e),
                }
            }
            ReplCommand::Render(key) => {
                let rendered_string = self
                    .template_registry
                    .render(&key, &self.data.clone().try_into::<toml::Value>()?)?;

                println!("{}", rendered_string);
            }
            _ => println!("WHAT?"),
        }

        Ok(())
    }
}
