use std::{
    any::Any,
    sync::Arc
};
use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler, 
    handler::server::{
        router::tool::ToolRouter,
        wrapper::Parameters
    }, model::*, 
    service::RequestContext, 
    task_handler, 
    task_manager::{
        OperationProcessor,
        OperationResultTransport
    }, tool, tool_handler, tool_router
};

use tokio::sync::Mutex;
use tokio::fs::{
    read,
    write,
    try_exists
};

struct ToolCallOperationResult {
    id: String,
    result: Result<CallToolResult, McpError>
}

impl OperationResultTransport for ToolCallOperationResult {
    
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
            Ok(_) => log::info!("saved contents of current notes to qwen-notebook.txt"),
            Err(e) => {
                log::error!("could not save contents of current notes to file\n{}", e);
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
    
    fn _create_resource_text(&self, uri: &str, name: &str) -> Resource {
        RawResource::new(uri, name.to_string()).no_annotation()
    }

    /// Returns the `Mcp-Session-Id` of the current session (streamable HTTP only).
    #[tool(description = "Get the session ID for this connection")]
    fn get_session_id(&self, ctx: RequestContext<RoleServer>) -> Result<CallToolResult, McpError> {
        let session_id = ctx
            .extensions
            .get::<axum::http::request::Parts>()
            .and_then(|parts| parts.headers.get("mcp-session-id"))
            .map(|v| v.to_str().unwrap_or("(non-ascii)").to_owned());

        match session_id {
            Some(id) => Ok(CallToolResult::success(vec![Content::text(id)])),
            None => Ok(CallToolResult::success(vec![Content::text(
                "no session (not running over streamable HTTP?)",
            )])),
        }
    }

}

#[tool_handler(meta = Meta(rmcp::object!({"tool_meta_key": "tool_meta_value"})))]
#[task_handler]
impl ServerHandler for NoteTaker {


    fn get_info(&self) -> ServerInfo {
        let info = ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .build()
        )
        .with_server_info(Implementation::from_build_env())
        .with_protocol_version(ProtocolVersion::V_2025_11_25)
        .with_instructions("This server provides note taking tools. Tools: take_note.".to_string());

        log::debug!("server info ->\n{:?}", info);

        info
    }
    
    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        log::debug!("list resource template ->\n{:#?}", _request);
        Ok(ListResourceTemplatesResult {
            next_cursor: None,
            resource_templates: Vec::new(),
            meta: None,
        })
    }
    async fn initialize(
        &self,
        _request: InitializeRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, McpError> {
        if let Some(http_request_part) = context.extensions.get::<axum::http::request::Parts>() {
            let initialize_header = &http_request_part.headers;
            let initialize_uri = &http_request_part.uri;
            log::info!("Initalized request!");
            log::debug!("Request headers -> {:?}\nRequest uri -> {:?}", initialize_header, initialize_uri);
        }

        Ok(self.get_info())
    }
}