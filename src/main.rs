use clap::Parser;
use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

#[derive(Parser, Debug, Clone)]
#[clap(name = "fkinstall", version)]
struct Args {
    #[clap(short, long, help = "Program to Install")]
    install: Option<String>,
    #[clap(short, long, help = "Search term")]
    search: Option<String>,
    #[clap(short, long, help = "List all available programs")]
    list: bool,
    #[clap(short, long, help = "Update installed programs")]
    update: bool,
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

    // Search
    if let Some(search_term) = &args.search {
        println!("Search results for '{}' :", search_term);
        for app in project_list
            .iter()
            .filter(|app| app.name.contains(search_term))
        {
            println!("{}", app.name);
        }
    }
    // List
    else if args.list {
        println!("Available programs:");
        for app in project_list {
            println!("- {}", app.name);
        }
    }
    // Install
    else if let Some(install) = &args.install {
        let app_opt = project_list.iter().find(|app| app.name == *install);
        if let Some(app) = app_opt {
            let url_parts: Vec<&str> = app.url.trim_end_matches(".git").split('/').collect();
            if url_parts.len() >= 2 {
                let owner = url_parts[url_parts.len() - 2];
                let repo = url_parts[url_parts.len() - 1];
                let client = reqwest::blocking::Client::new();
                // Fetch all releases and pick the most recent linux release
                let api_url = format!("https://api.github.com/repos/{}/{}/releases", owner, repo);
                let resp = client
                    .get(&api_url)
                    .header("User-Agent", "fkinstall")
                    .send()
                    .expect("Failed to fetch releases");
                let releases: Vec<serde_json::Value> =
                    resp.json().expect("Failed to parse releases JSON");
                // Select the first Linux release by tag_name
                if let Some(rel) = releases.iter().find(|rel| {
                    rel["tag_name"]
                        .as_str()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains("linux")
                }) {
                    let assets = rel["assets"].as_array().expect("No assets in release");
                    // Find the asset matching the program name
                    if let Some(asset) = assets
                        .iter()
                        .find(|asset| asset["name"].as_str().unwrap_or("") == *install)
                    {
                        let asset_name = asset["name"].as_str().unwrap();
                        let download_url = asset["browser_download_url"].as_str().unwrap();
                        let bin_dir = format!("{}/.dev/bin", home);
                        if !Path::new(&bin_dir).exists() {
                            fs::create_dir_all(&bin_dir).expect("Failed to create bin dir");
                        }
                        let dest_path = format!("{}/{}", bin_dir, asset_name);
                        if Path::new(&dest_path).exists() {
                            fs::remove_file(&dest_path).expect("Failed to remove existing file");
                        }
                        let resp2 = client
                            .get(download_url)
                            .header("User-Agent", "fkinstall")
                            .send()
                            .expect("Failed to download asset");
                        let bytes = resp2.bytes().expect("Failed to read asset bytes");
                        fs::write(&dest_path, &bytes).expect("Failed to write binary");
                        let mut perms = fs::metadata(&dest_path)
                            .expect("Failed to get metadata")
                            .permissions();
                        perms.set_mode(0o755);
                        fs::set_permissions(&dest_path, perms).expect("Failed to set permissions");
                        println!("Installed {} to {}", asset_name, dest_path);
                    } else {
                        eprintln!("No asset named '{}' in the Linux release", install);
                    }
                } else {
                    eprintln!("No Linux release found for this repository");
                }
            } else {
                eprintln!("Invalid GitHub URL: {}", app.url);
            }
        } else {
            eprintln!("App '{}' not found", install);
        }
    }
    // Update
    else if args.update {
        let bin_dir = format!("{}/.dev/bin", home);
        if Path::new(&bin_dir).exists() {
            println!("Updating installed programs in {}", bin_dir);
            for entry in fs::read_dir(&bin_dir)
                .expect("Failed to read bin directory")
                .flatten()
            {
                if let Some(name) = entry.file_name().to_str() {
                    if let Some(app) = project_list.iter().find(|a| a.name == name) {
                        let parts: Vec<&str> =
                            app.url.trim_end_matches(".git").split('/').collect();
                        if parts.len() >= 2 {
                            let owner = parts[parts.len() - 2];
                            let repo = parts[parts.len() - 1];
                            let client = reqwest::blocking::Client::new();
                            let api =
                                format!("https://api.github.com/repos/{}/{}/releases", owner, repo);
                            let resp = client
                                .get(&api)
                                .header("User-Agent", "fkinstall")
                                .send()
                                .expect("Failed to fetch releases");
                            let rels: Vec<serde_json::Value> =
                                resp.json().expect("Failed to parse releases");
                            if let Some(rel) = rels.iter().find(|r| {
                                r["tag_name"]
                                    .as_str()
                                    .unwrap_or("")
                                    .to_lowercase()
                                    .contains("linux")
                            }) {
                                let assets = rel["assets"].as_array().expect("No assets");
                                if let Some(asset) = assets
                                    .iter()
                                    .find(|a| a["name"].as_str().unwrap_or("") == name)
                                {
                                    let url = asset["browser_download_url"].as_str().unwrap();
                                    let dest = format!("{}/{}", bin_dir, name);
                                    if Path::new(&dest).exists() {
                                        fs::remove_file(&dest).expect("rm failed");
                                    }
                                    let b = client
                                        .get(url)
                                        .header("User-Agent", "fkinstall")
                                        .send()
                                        .expect("dl")
                                        .bytes()
                                        .expect("bytes");
                                    fs::write(&dest, &b).expect("write");
                                    let mut p = fs::metadata(&dest).expect("meta").permissions();
                                    p.set_mode(0o755);
                                    fs::set_permissions(&dest, p).expect("chmod");
                                    println!("Updated {}", name);
                                }
                            }
                        }
                    }
                }
            }
        } else {
            println!("No .dev/bin at {}", bin_dir);
        }
    }
}
