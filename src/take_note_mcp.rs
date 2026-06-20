use std::{
    any::Any,
    sync::Arc
};
use rmcp::{
    ErrorData as McpError,
    ServerHandler, 
    handler::server::{
        router::tool::ToolRouter,
        wrapper::Parameters
    }, model::*, 
    task_handler, 
    task_manager::{
        OperationProcessor,
        OperationResultTransport
    }, 
    tool, 
    tool_handler, 
    tool_router
};

use tokio::sync::Mutex;
use tokio::fs::{
    read,
    write,
    try_exists
};

struct ToolOperationResult {
    id: String,
    result: Result<CallToolResult, McpError>
}

impl OperationResultTransport for ToolOperationResult {
    
    fn operation_id(&self) -> &String {
        &self.id
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}


#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct StructRequest {
    pub message: String
}


#[derive(Clone)]
pub struct NoteTaker {
    tool_router: ToolRouter<NoteTaker>,
    processor: Arc<Mutex<OperationProcessor>>
}

#[tool_router]
impl NoteTaker {
    pub fn new() -> NoteTaker {
        Self {
            tool_router: Self::tool_router(),
            processor: Arc::new(Mutex::new(OperationProcessor::new()))
        }
    }

    #[tool(description = "Write down a note to the log",
            execution(task_support = "optional")
    )]
    async fn take_note(
        &self,
        Parameters(StructRequest { message }): Parameters<StructRequest>,
    ) -> Result<CallToolResult, McpError> {

        let mut current_notes: String = String::new();

        match try_exists("qwens-notebook.txt").await {
            Ok(f) => {
                match f {
                    true => {
                        current_notes = match read("qwens-notebook.txt").await {
                            Ok(notes) => {
                                match String::from_utf8(notes) {
                                    Ok(s) => s,
                                    Err(e) => {
                                        log::error!("could not read notebook to write note\n{}",e);
                                        return Err(McpError::internal_error("could not open file", None))
                                    }
                                }
                            },
                            Err(e) => {
                                log::error!("could not open notebook to write note\n{}", e);
                                return Err(McpError::internal_error("could not open note book", None))
                            }

                        };
                    },
                    false => log::info!("qwens-notebook not found creating new file")
                }
            },
            Err(e) => {
                log::error!("Could not determine if qwen-notebook.txt exists\n{}",e);
                return Err(McpError::internal_error("could not open notebook", None))
            }
        };
        
        log::info!("appending note to notebook");
        log::debug!("the note is {}", message);

        current_notes.push('\n');
        current_notes.push_str(&message);

        log::info!("wrote message to notebook writing contents to qwen-notebook.txt");

        match write("qwens-notebook.txt", current_notes.as_bytes()).await {
            Ok(r) => log::info!("saved contents of current notes to qwen-notebook.txt"),
            Err(e) => {
                log::error!("could not save contents of current notes to file");
                return Err(McpError::internal_error("could note save file", None)) 
            }
        };


        Ok(
            CallToolResult::success(
                vec![
                    Content::text(
                        "successfully wrote note to notebook".to_string()
                    )
                ]
            )
        )
    }

}

impl ServerHandler for NoteTaker {
    
}