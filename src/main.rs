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
    #[clap(short, long, help = "List all available programs")]
    list: Option<String>,
    #[clap(short, long, help = "Update installed programs")]
    update: Option<String>,
}

#[derive(serde::Deserialize)]
struct App {
    name: String,
    url: String,
}

fn check_config(conf_dir: &str, conf_file: &str) {
    // Create ~/.config directory if it doesn't exist
    if !Path::new(&conf_dir).exists() {
        if let Err(e) = fs::create_dir(conf_dir) {
            eprintln!("Failed to create directory {}: {}", conf_dir, e);
            return;
        }
    }

    // Create ~/.config/fkinstall.conf if it doesn't exist
    if !Path::new(&conf_file).exists() {
        println!("Creating config file...");
        if let Err(e) = fs::File::create(conf_file) {
            eprintln!("Failed to create file {}: {}", conf_file, e);
        } else if let Err(e) = fs::write(
            conf_file,
            "version = 1\nurl = https://coobyk.github.io/misc/fkinstall.json\nos = linux",
        ) {
            eprintln!("Failed to write to file {}: {}", conf_file, e);
            return;
        }
    }
}

fn fetch_list(conf_file: &str) -> Vec<App> {
    let conf_file = fs::read_to_string(conf_file).expect("Could not read config");
    let lines = conf_file.lines();
    let mut url = String::new();
    for line in lines {
        if line.contains("url") {
            url = line.split(" = ").collect::<Vec<&str>>()[1].to_string();
        }
    }
    let response = reqwest::blocking::get(url).expect("Failed to get apps list");
    let body = response.text().expect("Failed to read body");
    let apps_json: serde_json::Value =
        serde_json::from_str(&body).expect("Failed to parse apps list as json");
    let apps_vec: Vec<App> = serde_json::from_value(apps_json["apps"].clone())
        .expect("Failed to parse apps list as Vec<App>");
    for app in &apps_vec {
        println!("{}", app.name);
    }
    apps_vec
}

fn main() {
    let args = Args::parse();
    // Get $HOME
    let home = env::var("HOME").expect("Could not determine home directory");
    let conf_dir = format!("{}/.config", home);
    let conf_file = format!("{}/fkinstall.conf", conf_dir);
    check_config(&conf_dir, &conf_file);
    let project_list = fetch_list(&conf_file);
    if let Some(search_term) = &args.search {
        println!("Search results for '{}' :", search_term);
        for app in project_list
            .iter()
            .filter(|app| app.name.contains(search_term))
        {
            println!("{}", app.name);
        }
    }
}
