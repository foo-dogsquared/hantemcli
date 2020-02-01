use std::path::PathBuf;

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct Hantemcli {
    #[structopt(
        required = true,
        multiple = true,
        parse(from_os_str),
        help = "The path of the template file."
    )]
    pub templates: Vec<PathBuf>,

    #[structopt(
        last = true,
        multiple = true,
        parse(from_os_str),
        value_name = "data",
        help = "The path of the data files. Accepts JSON, HJSON, INI, TOML, and YAML format."
    )]
    pub data_paths: Vec<PathBuf>,

    #[structopt(short, long, parse(from_os_str), help = "Write the output to a file.")]
    pub output: Option<PathBuf>,

    #[structopt(short, long, help = "The name of the root template to be used.")]
    pub root: Option<String>,

    #[structopt(short, long, help = "Set the renderer to be strict.")]
    pub strict: bool,

    #[structopt(
        short,
        long,
        help = "Set the file extension to be searched.",
        default_value = "hbs"
    )]
    pub extension: String,
}
