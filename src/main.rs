use log;
use env_logger::{self, Env};
use std::{env, io::Write};
use tokio;
use reqwest;
use std::fs::File;
use crate::data_structs::{
    Model, 
    Models,
    Chat,
    ChatResponse,
    Message
};

mod data_structs;


fn ollama_url() -> String {

    "http://localhost:11434".to_string()
}

fn help_message() {
    log::info!("Ollama Tool Usage:");
    log::info!("  No arguments       : Check if Ollama is running");
    log::info!("  -r                 : List running models");
    log::info!("  -l                 : List available models");
    log::info!("  -c <prompt>        : Chat with a model using the provided prompt");
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
                "-h" => help_message(),
                "-r" => get_running_models().await,
                "-l" => list_available_models().await,
                "-c" => {
                    log::warn!("No prompt provided for chat. Exiting.");
                    help_message();
                },
                _ => log::warn!("Unknown flag provided: {}. Exiting.", flag)
            }
        },
        3 => {
            let flag = &args[1];
            let prompt = &args[2];
            match flag.as_str() {
                "-c" => chat(prompt.to_string()).await,
                _ => {
                    log::warn!("Unknown flag provided: {}. Exiting.", flag);
                    help_message();
                }
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

async fn chat(prompt: String) {

    let endpoint = format!("{}/api/chat", ollama_url());
    
    let current_chat_file = match File::open("base.json"){
        Ok(file) => file,
        Err(e) => {
            log::error!("Failed to open base.json: {}", e);
            return;
        }
    };

    let mut current_chat: Chat = match serde_json::from_reader(current_chat_file){
        Ok(chat) => chat,
        Err(e) => {
            log::error!("Failed to parse base.json: {}", e);
            return;
        }
    };

    let new_message = Message {
        role: "user".to_string(),
        content: prompt,
        thinking: None,
        images: None,
        tool_calls: None,
    };

    current_chat.messages.push(new_message);
    log::info!("sending request to ollama server");
    let response = reqwest::Client::new()
        .post(&endpoint)
        .json(&current_chat)
        .send()
        .await;


    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                
                let chat_response: ChatResponse = match resp.json().await {
                    Ok(json) => json,
                    Err(e) => {
                        log::error!("Failed to parse chat response: {}", e);
                        return;
                    }
                };
                log::debug!("Chat response received: {:?}", chat_response.message);
                log::info!("{}", chat_response.message.content);
                current_chat.messages.push(chat_response.message);
            } else {
                log::error!("Chat request failed. Received status: {}", resp.status());
            }
        },
        Err(e) => {
            log::error!("Failed to connect to Ollama: {}", e);
        }
    }

    log::debug!("Current chat state: {:?}", current_chat);

    log::info!("writing chat state to base.json");

    match File::create("base.json") {
        Ok(mut file) => {
            if let Err(e) = serde_json::to_writer_pretty(&mut file, &current_chat) {
                log::error!("Failed to write chat state to base.json: {}", e);
            } else {
                log::info!("Chat state successfully written to base.json");
            }
        },
        Err(e) => {
            log::error!("Failed to create base.json: {}", e);
        }
    }
}
