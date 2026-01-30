use log;
use env_logger::{self, Env};
use std::env;
use tokio;
use reqwest;

use crate::data_structs::{Model, Models};

mod data_structs;


fn ollama_url() -> String {
    "http://localhost:11434".to_string()
}

fn help_message() {
    log::info!("Ollama Tool Usage:");
    log::info!("  No arguments       : Check if Ollama is running");
    log::info!("  -r                 : List running models");
    log::info!("  -l                 : List available models");
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init(); 
    
    log::info!("Ollama tool started.");

    let args: Vec<String> = env::args().collect();
    log::debug!("Command-line arguments: {:?}", args);

    match args.len() {
        1 => check_ollama_status().await,
        2 => {
            let flag = &args[1];
            match flag.as_str() {
                "-r" => get_running_models().await,
                "-l" => list_available_models().await,
                _ => log::warn!("Unknown flag provided: {}. Exiting.", flag)
            }
        },
        _ => {
            log::warn!("No valid command provided. Exiting.");
            help_message();
        }

    }
}



async fn check_ollama_status() {

    let url = ollama_url();

    let response = reqwest::get(&url).await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {

                let status: String = match resp.text().await {
                    Ok(text) => text,
                    Err(e) => {
                        log::error!("Failed to read response text: {}", e);
                        return;
                    }
                };
                log::info!("Ollama status =>  {}", status);
            } else {
                log::error!("Ollama is not running. Received status: {}", resp.status());
            }
        },
        Err(e) => {
            log::error!("Failed to connect to Ollama: {}", e);
        }
    }    

}

async fn get_running_models() {
    let url = format!("{}/models/running", ollama_url());

    let response = reqwest::get(&url).await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                let models: String = match resp.text().await {
                    Ok(text) => text,
                    Err(e) => {
                        log::error!("Failed to read response text: {}", e);
                        return;
                    }
                };
                log::info!("Running models =>  {}", models);
            } else {
                log::error!("Failed to retrieve running models. Received status: {}", resp.status());
            }
        },
        Err(e) => {
            log::error!("Failed to connect to Ollama: {}", e);
        }
    }    
}

async fn list_available_models() {
    let url: String = format!("{}/api/tags", ollama_url());

    let response:Result<reqwest::Response, reqwest::Error> = reqwest::get(&url).await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                let models: Models  = match resp.json().await {
                    Ok(json) => json,
                    Err(e) => {
                        log::error!("Failed to read response text: {}", e);
                        return;
                    }
                };
                if models.models.is_empty() {
                    log::info!("No available models found.");
                } else {
                    log::info!("Available models:");
                    for model in models.models {
                        model.display();
                        println!("---------------------------");
                    }
                }
            
            } else {
                log::error!("Failed to retrieve available models. Received status: {}", resp.status());
            }
        },
        Err(e) => {
            log::error!("Failed to connect to Ollama: {}", e);
        }
    }    
}