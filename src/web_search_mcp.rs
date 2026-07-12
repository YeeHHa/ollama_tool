use std::{
    any::Any,
    sync::Arc,
    collections::HashMap
};
use rmcp::{
    ErrorData as McpError,
    RoleServer, 
    ServerHandler, 
    handler::server::{
        router::tool::ToolRouter,
        wrapper::Parameters
    }, 
    model::*, 
    service::RequestContext, 
    task_handler, 
    task_manager::{
        OperationProcessor,
        OperationResultTransport
    }, 
    tool, 
    tool_handler, 
    tool_router
};
use schemars::JsonSchema;
use serde::{
    Deserialize,
    Serialize
};
use tokio::sync::Mutex;
use tokio::fs::{
    read,
    write,
    try_exists
};
use reqwest::{
    Client,
    header::{
        ACCEPT,
    }
};

use super::searxng_data::{
    SearxngParams,
    SearxngResponse
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

#[derive(Debug, Clone, Deserialize, Serialize)]
struct SearchResult{
    id: u32,
    title: String,
    content: String,
    url: String,
}

#[derive(Clone, Deserialize, Serialize, JsonSchema)]
pub struct WebSearchQuery {
    query: String
}

#[derive(Clone)]
pub struct WebSearch {
    tool_router: ToolRouter<WebSearch>,
    processor: Arc<Mutex<OperationProcessor>>,
    search_results: Arc<Mutex<HashMap<String,Vec<SearchResult>>>>
}


#[tool_router]
impl WebSearch  {
    
    pub fn new() -> WebSearch {
        Self {
            tool_router: Self::tool_router(),
            processor: Arc::new(Mutex::new(OperationProcessor::new())),
            search_results: Arc::new(Mutex::new(HashMap::new()))
        }
    }

    
    #[tool(description = "Get a json response from a web search engine query")]
    async fn web_search(
        &self,
        Parameters(WebSearchQuery { query }): Parameters<WebSearchQuery>,
        ctx: RequestContext<RoleServer>
    ) -> Result<CallToolResult, McpError> {

        log::info!("got the following search query\n{:#?}" ,query);

        log::debug!("creating new searxng paramers");
        let params = SearxngParams::new(&query);
        log::debug!("params for query <{}>\n{:?}", query, params);

        let web_search_endpoint = match std::env::var("SEARXNG_URL") {
            Ok(endpoint) => {
                log::debug!("got searxng url from env var -> {}", endpoint);
                endpoint
            },
            Err(e) => {
                log::error!("SEARXNG_URL env var not set! cannot find searxng instance\n{}",e);
                return Err(McpError::internal_error("Could not complete web_search", None))
            }
        };

        log::debug!("creating requests client");
        let response = Client::new()
            .get(web_search_endpoint)
            .query(&params)
            .header(ACCEPT, "application/json")
            .send()
            .await;

        let searxng_response: SearxngResponse = match response {
            Ok(res) => {
                if res.status().is_success() {
                    match res.json().await {
                        Ok(s_result) => s_result,
                        Err(e) => {
                            log::error!("could not parse SearxngResponse from reqwest\n{:?}",e);
                            return Err(
                                McpError::internal_error("could not complete web_search", None)
                            )
                        }
                    }
                }else {
                    log::error!("searxng response was not 200 - status code = {}",res.status());
                    return Err(
                        McpError::internal_error("could not complete web_search", None)
                    )
                }
            },
            Err(e) => {
                log::error!("searxng request was unsuccessful {e}");
                return Err(
                    McpError::internal_error("could not complete web_search", None)
                )
            }
        };

        log::info!("creating searxng results");

        let search_results:Vec<SearchResult> = searxng_response.results
            .iter()
            .enumerate()
            .map(
                |(index,r)|
                SearchResult {
                    id: index as u32,
                    title: r.title.clone(),
                    content: r.content.clone(),
                    url: r.url.clone()
                }
            )
            .collect();
        log::debug!("SearchResult created. adding to app state");

        if let Some(session_id) = self.get_session_id(&ctx) {
            log::debug!("adding SearchResult for session {}",session_id);
            let mut state = self.search_results.lock().await;
            state.insert(session_id, search_results.clone()); 
            log::info!("session SearchResult saved");
            log::debug!("current state\n\n{:#?}\n\n", state);
        }else{

            log::error!("could not save search results for qury {}",query);

            return Err(
                McpError::internal_error("Could not complete web_search", None)
            )
        }

        let response = match Content::json(search_results) {
            Ok(json) => json,
            Err(e) => {
                log::error!("could not convert search results to callToolResult\n{}",e);
                return Err(
                    McpError::internal_error("could not complete web_search", None)
                )
            }
        };

        Ok(
            CallToolResult::success(
                vec![
                    response,
                    Content::text(
                        "success!".to_string()
                    )
                ]
            )
        )
        
    }

    fn get_session_id(&self, ctx: &RequestContext<RoleServer>) -> Option<String> {
        ctx
            .extensions
            .get::<axum::http::request::Parts>()
            .and_then(|parts| parts.headers.get("mcp-session-id"))
            .map(|v| v.to_str().unwrap_or("(non-ascii)").to_owned())
    }

}

#[tool_handler(meta = Meta(rmcp::object!({"tool_meta_key": "tool_meta_value"})))]
#[task_handler]
impl ServerHandler for WebSearch {


    fn get_info(&self) -> ServerInfo {
        let info = ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .build()
        )
        .with_server_info(Implementation::from_build_env())
        .with_protocol_version(ProtocolVersion::V_2025_11_25)
        .with_instructions("This server provides web search functions. Tools: web_search.".to_string());

        log::debug!("server info ->\n{:?}", info);

        info
    }
    
    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
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
