use clap::Parser;
use std::env;
use std::fs;
use std::path::Path;

#[derive(Parser, Debug, Clone)]
#[clap(name = "fkinstall", version)]
struct Args {
    #[clap(short, long, help = "Program to Install")]
    install: Option<String>,
    #[clap(short, long, help = "Search term")]
    search: Option<String>,
    #[clap(short, long, help = "Update installed programs")]
    update: Option<String>,
    #[clap(short, long, help = "Print version")]
    version: Option<String>,
}

fn check_config() {
    // Get $HOME
    let home = env::var("HOME").expect("Could not determine home directory");
    let conf_dir = format!("{}/.config", home);
    let conf_file = format!("{}/fkinstall.conf", conf_dir);

    // Create ~/.config directory if it doesn't exist
    if !Path::new(&conf_dir).exists() {
        if let Err(e) = fs::create_dir(&conf_dir) {
            eprintln!("Failed to create directory {}: {}", conf_dir, e);
            return;
        }
    }

    // Create ~/.config/fkinstall.conf if it doesn't exist
    if !Path::new(&conf_file).exists() {
        println!("Creating config file...");
        if let Err(e) = fs::File::create(&conf_file) {
            eprintln!("Failed to create file {}: {}", conf_file, e);
        } else if let Err(e) = fs::write(
            &conf_file,
            "version = 1\nurl = https://coobyk.github.io/misc/fkinstall.json\nos = linux",
        ) {
            eprintln!("Failed to write to file {}: {}", conf_file, e);
            return;
        }
    }
}

fn main() {
    check_config();
}
