use clap::Parser;
use reqwest::multipart::{Form, Part};
use serde::Deserialize;
use std::io::{self};
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use json5;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to the image file to upload
    file_path: PathBuf,

    /// URL of the microservice where the file will be uploaded
    #[arg(short, long)]
    url: Option<String>, // Change to Option<String>
}

#[derive(Deserialize)]
struct Config {
    log_level: Option<String>, // Change to Option<String>
    endpoint: Option<String>,    // Make endpoint optional
}

fn load_config(file_path: &str) -> Result<Config, io::Error> {
    let file_content = std::fs::read_to_string(file_path).expect("Failed to read config file");        
    // Parse the JSON5 configuration
    json5::from_str(&file_content).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Unable to parse config file"))
}

fn init_logger(log_level: &str) {
    std::env::set_var("RUST_LOG", log_level);
    env_logger::init();
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Attempt to load the configuration
    let (log_level, endpoint) = match load_config("anarchic-image-hosting-cli.json5") {
        Ok(config) => {
            // Use the log level from the config or default to "info"
            (config.log_level.unwrap_or_else(|| "info".to_string()), config.endpoint)
        }
        Err(err) => {
            eprintln!("Warning: Could not load config file: {}", err);
            // Set logging level to "error" if the config file can't be loaded
            ("error".to_string(), None)
        }
    };

    // Initialize the logger based on the determined log level
    init_logger(&log_level);

    // Parse the command-line arguments
    let args = Cli::parse();
    log::debug!("Parsed arguments: {:?}", args);

    // Determine the URL to use
    let url = args.url.clone().unwrap_or_else(|| {
        endpoint.clone().unwrap_or_else(|| {
            "http://localhost:8080".to_string() // Default value if both are absent
        })
    }) + "/upload";

    log::debug!("Using endpoint URL: {}", url);

    // Read the file contents
    let mut file = File::open(&args.file_path).await?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).await?;
    log::debug!("Read {} bytes from file: {:?}", buffer.len(), args.file_path);

    // Get the file name
    let file_name = args.file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown_file");
    log::debug!("Using file name: {}", file_name);

    // Create a multipart form
    let form = Form::new()
        .part("file", Part::bytes(buffer).file_name(file_name.to_owned()));
    log::debug!("Created multipart form with file part: {}", file_name);

    // Create an HTTP client
    let client = reqwest::Client::new();

    // Send the request to the microservice
    log::debug!("Sending request to URL: {}", url);
    let response = client
        .post(&url)
        .multipart(form)
        .send()
        .await?;

    // Store response status
    let status = response.status();
    let response_text = response.text().await?; // Read response text once

    // Check if the request was successful
    if status.is_success() {
        println!("{}", response_text);
        log::info!("File uploaded successfully: {}", response_text);
    } else {
        eprintln!("Failed to upload the file. Status: {}", status);
        eprintln!("Response: {}", response_text); // Use the stored response text
        log::error!("Failed to upload file. Status: {}. Response: {}", status, response_text);
    }

    Ok(())
}
