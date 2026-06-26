use log;
use fern;
use humantime::format_rfc3339_seconds;
use std::time::SystemTime;
use std::env;
use tokio;
use tokio::fs::{
    File,
    write,
    read,
    rename
};
use reqwest;
use rmcp::transport::streamable_http_server:: {
    StreamableHttpServerConfig,
    StreamableHttpService,
    session::local::LocalSessionManager
};

mod data_structs;
use data_structs::{
    Models,
    Chat,
    ChatResponse,
    Message
};

mod take_note_mcp;
use take_note_mcp::NoteTaker;


async fn ollama_url() -> String {

    match env::var("OLLAMA_HOST") {
        Ok(host) => format!("http://{}:11434", host),
        Err(_) => {
            log::debug!("OLLAMA_HOST environment variable not set, using localhost");
            "http://localhost:11434".to_string()
        }
    }
}

fn help_message() {
    log::info!("Ollama Tool Usage:");
    log::info!("  No arguments       : Check if Ollama is running");
    log::info!("  -r                 : List running models");
    log::info!("  -l                 : List available models");
    log::info!("  -c <prompt>        : Chat with a model using the provided prompt");
    log::info!("  -f                 : direct output to a log file");
    log::info!("  -m                 : start mpc servers");

    log::info!("ENVIRONMENT VARIABLES");
    log::info!("OLLAMA_HOST     => ollama server ip address : default = localhost");
    log::info!("LOG_FILE_NAME   => name of file to write logs to : default = ./llama_tool.log");
    log::info!("RUST_LOG        => log level value : default = info");
    log::info!("MCP_SERVER      => ip:port to bind mcp server to : default = 127.0.0.1:4000");
    log::info!("MCP_HOST        => allowlist hosts for inbound Host validation : default = 'localhost,127.0.0.1'");
    log::info!("MCP_ORIGIN     => allowlist for Origin validation : default = None");
}

#[tokio::main]
async fn main() {
     

    let mut args: Vec<String> = env::args().collect();

    let log_file_name:String = match env::var("LOG_FILE_NAME"){
        Ok(f) => f,
        Err(_) => String::from("llama_tool.log")
    };

    let mut log_to_file: bool = false;
    if let Some(arg_pos) = args.iter().position(|x| *x == "-f"){
        args.remove(arg_pos);
        log_to_file = true;
    }

    let log_level:log::LevelFilter = match env::var("RUST_LOG") {
        Ok(l) => {
            match l.as_str() {
                "trace" => log::LevelFilter::Trace,
                "debug" => log::LevelFilter::Debug,
                "info"  => log::LevelFilter::Info,
                "warn"  => log::LevelFilter::Warn,
                "error" => log::LevelFilter::Error,
                "off"   => log::LevelFilter::Off,
                _       => log::LevelFilter::Info
            }
        },
        Err(_) => log::LevelFilter::Info 
    };

    let mut dispatch = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {} {}] {}",
                format_rfc3339_seconds(SystemTime::now()),
                record.level(),
                record.target(),
                message
            ))
        })
        .level(log_level);

    if log_to_file {
        dispatch = dispatch.chain(
            match fern::log_file(log_file_name.as_str()){
                Ok(f) => f,
                Err(e) => {
                    eprint!("Failed to open/create log file {}\nErr: {}", log_file_name, e);
                    return
                }
            }
        );
    }else {
        dispatch = dispatch.chain(std::io::stdout());
    }

    match dispatch.apply() {
        Ok(_) => log::info!("logging initiated"),
        Err(e) => {
            eprint!("could not init logging\nErr: {}", e)
        }
    }; 
    log::debug!("Command-line arguments: {:?}", args);

    log::info!("llama tool started.");
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
                "-m" => start_mcp_server().await,
                _ => log::warn!("Unknown flag provided: {}. Exiting.", flag)
            }
        },
        3 => {
            let flag = &args[1];
            let prompt = &args[2];
            match flag.as_str() {
                "-c" => chat(prompt.to_string()).await,
                "-s" => save_chat(prompt).await,
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

    let url = ollama_url().await;

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
    let url = format!("{}/models/running", ollama_url().await);

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
    let url: String = format!("{}/api/tags", ollama_url().await);

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

async fn save_chat(chat_name: &String) {
    /*
    find current chat
    save it to new chat 
    create new chat file*/
    match rename("base.json", chat_name).await {
        Ok(v) => log::info!("successfully renamed base.json to {}", chat_name),
        Err(e) => {
            log::error!("could not rename base.json\n{e}");
            return
        }
    };

    let starting_message:Message = Message {
        role: String::from("system"),
        content: String::from("you are a very helpful assistant."),
        thinking: None,
        images: None,
        tool_calls: None
    };

    let new_chat: Chat = Chat {
        model: String::from("qwen3.6:35b"),
        messages: vec![starting_message],
        tools: None,
        think: None,
        stream: false
    };

    match  serde_json::to_string_pretty(&new_chat) {
        Ok(formatted_json) => {
            log::info!("converted new chat to string");
            match write("base.json", &formatted_json).await {
                Ok(_) => log::info!("saved file to base.json"),
                Err(e) => log::error!("could not write new base.json file\n{}", e)
            };
        },
        Err(e) => log::error!("could not convert chat struct to string {}", e)
    };
}
async fn chat(prompt: String) {

    let endpoint = format!("{}/api/chat", ollama_url().await);
    
    let current_chat_file = match File::open("base.json").await{
        Ok(file) => file,
        Err(e) => {
            log::error!("Failed to open base.json: {}", e);
            return;
        }
    };

    let chat_string = match read("base.json").await {
        Ok(file_name_bypes) => match String::from_utf8(file_name_bypes){
            Ok(s) => s,
            Err(e) => {
                log::error!("could not convert file from bytes to string");
                return
            }
        },
        Err(e) => {
            log::error!("could not read chat file");
            return
        }
    };

    let mut current_chat: Chat = match serde_json::from_str(&chat_string){ 
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

                match &chat_response.message.thinking {
                    Some(t) => log::debug!("thinking: {}",t),
                    None => log::debug!("no thinking")
                };

                log::debug!("done = {}", chat_response.done);
                current_chat.messages.push(chat_response.message);
            } else {
                log::error!("Chat request failed. Received status: {}", resp.status());
            }
        },
        Err(e) => {
            log::error!("Failed to connect to Ollama: {}", e);
        }
    }

    log::debug!("Current chat state: {}", current_chat);

    log::info!("writing chat state to base.json");

    let formatted_json:String = match serde_json::to_string_pretty(&current_chat){
        Ok(value) => value,
        Err(e) => {
            log::error!("unable to format json data");
            return
        }
    };

    match write("base.json", &formatted_json).await {
        Ok(_v) => log::info!("saved file to base.json"),
        Err(e)=> {
            log::error!("could not save current chat to file! {}", e );
            return
        }
    };
}


async fn start_mcp_server() {

    let mut mcp_server: String = String::from("127.0.0.1:4000");
    let mut mcp_host:   String = String::from("127.0.0.1");
    let mut mcp_origin: String = String::new();

    for (key, value) in env::vars() {
        match key.as_str() {
            "MCP_SERVER"    => mcp_server   = value,
            "MCP_HOST"      => mcp_host     = value,
            "MCP_ORIGIN"    => mcp_origin   = value,
            _               => continue 
        }
    }

    let mut server_config:StreamableHttpServerConfig = StreamableHttpServerConfig::default();

    
    if mcp_host.as_str() != "127.0.0.1" {
        server_config = server_config.with_allowed_hosts(
            mcp_host.split(',')
            .map(|x| x.to_string())
            .collect::<Vec<String>>()
        );
    }

    if !mcp_origin.is_empty() {
        server_config = server_config.with_allowed_origins(
            mcp_origin.split(',')
            .map( |x| x.to_string())
            .collect::<Vec<String>>()
        );
    }

    log::debug!("StreamableHttpServerConfig values --->\n{:#?}\n", server_config);

    log::info!("starting mcp server on {}", mcp_server);

    let ct = tokio_util::sync::CancellationToken::new();

    let service = StreamableHttpService::new(
        || Ok(NoteTaker::new()),
        LocalSessionManager::default().into(),
        server_config.with_cancellation_token(ct.child_token())
    );

    let router = axum::Router::new().nest_service("/mcp", service);
    let tcp_listener = match tokio::net::TcpListener::bind(mcp_server).await {
        Ok(listener) => listener,
        Err(e) => {
            log::error!("could not start mcp server\n{}", e);
            return
        }
    };

    let _ = axum::serve(tcp_listener, router)
                .with_graceful_shutdown(async move {
                    match tokio::signal::ctrl_c().await {
                        Ok(_) => log::info!("stopping server"),
                        Err(e)  => log::error!("could not await for shutdown\n{}", e)
                    };

                    ct.cancel();
                }).await;    
    
}