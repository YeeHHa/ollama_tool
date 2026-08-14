use std::{
    any::Any, collections::HashMap, env, fs, io::Read, sync::Arc
};
use axum::http::{request::Builder, response};
use uuid::Uuid;
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
    header
};

use super::errors_trait::LogError;
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

#[derive(Clone, Deserialize, Serialize, JsonSchema)]
pub struct FetchUrl {
    session_uuid: String,
    id: u32
}

#[derive(Clone)]
pub struct WebSearch {
    tool_router: ToolRouter<WebSearch>,
    processor: Arc<Mutex<OperationProcessor>>,
    search_results: Arc<Mutex<HashMap<Uuid,Vec<SearchResult>>>>,
    r_client: reqwest::Client
}



#[tool_router]
impl WebSearch  {
    
    pub fn new() -> WebSearch {

        let mut default_headers = header::HeaderMap::new();

        default_headers.insert(
            header::ACCEPT, 
            header::HeaderValue::from_static("application/json, text/html")
        );
        let user_agent: String = match env::var("CUSTOM_USER_AGENT") {
            Ok(ua) => ua,
            Err(e) => {
                log::error!("{}",e);
                log::warn!("CUSTOM_USER_AGENT env var not found. defaulting to cargo env values");
                concat!(
                    env!("CARGO_PKG_NAME"),
                    "/",
                    env!("CARGO_PKG_VERSION"),
                    " ",
                    "(",
                    env!("CARGO_PKG_REPOSITORY"),
                    ")"
                ).into()
            }
        };

        let mut ca_cert: Option<String> = match env::var("CA_PATH") {
            Ok(ca) => Some(ca),
            Err(e) => {
                log::warn!("CA_PATH env variable not set. using local CAs {}", e);
                None
            }
        };



        let builder = match ca_cert {
            Some(ca) => {
                log::debug!("adding local CA cert to client CA location:{}", ca);
            
            //read CA cert
            //build cert struct from reqwest
            //pass to builder 

                match fs::File::open(ca) {
                    Ok(mut ca_file) => {
                            log::debug!("got bytes from CA file");
                            let mut bytes = vec![];

                            ca_file.read_to_end(&mut bytes).unwrap_or_default();

                            match reqwest::Certificate::from_pem(&mut bytes)  {
                                Ok(pem) => {
                                    log::debug!("reqwest Certificate created");
                                    log::info!("added CA cert  to reqwest client");
                                    let cert = vec![pem];
                                    reqwest::ClientBuilder::new()
                                        .user_agent(user_agent)
                                        .tls_certs_merge(cert)
                                    //let client = builder.build().unwrap();

                                },
                                Err(e) => {
                                    log::error!("could not create Certificate from\n{}", e);
                                    log::warn!("local CA will not be added to client");
                                    reqwest::ClientBuilder::new()
                                        .user_agent(user_agent)
                                }
                            }
                    },
                    Err(e) => {
                        log::warn!("Could not read CA file \nerror: {}",  e);
                        log::warn!("local CA will not be added to client");
                        reqwest::ClientBuilder::new()
                            .user_agent(user_agent)
                    }
                }
            },
            None => reqwest::ClientBuilder::new().user_agent(user_agent)
                
        };

                     

        let client = match builder.build() {
                Ok(c) => c,
                Err(e) => {
                    log::error!("could not build reqwest client for websearch mcp server\n{}",e);
                    panic!("unable to continue with out reqwest client");
                }
        };

        Self {
            tool_router: Self::tool_router(),
            processor: Arc::new(Mutex::new(OperationProcessor::new())),
            search_results: Arc::new(Mutex::new(HashMap::new())),
            r_client: client 
        }
    }

    
    #[tool(description = "Get a json response from a web search engine query")]
    async fn web_search(
        &self,
        Parameters(WebSearchQuery { query }): Parameters<WebSearchQuery>
    ) -> Result<CallToolResult, McpError> {

        log::info!("got the following search query\n{:#?}" ,query);

        log::debug!("creating new searxng paramers");
        let params = SearxngParams::new(&query);
        log::debug!("params for query <{}>\n{:?}", query, params);

        let web_search_endpoint = match std::env::var("SEARXNG_URL") {
            Ok(endpoint) => {
                log::debug!("got searxng url from env var -> {}", endpoint);
                match reqwest::Url::parse(&endpoint) {
                    Ok(url) => url,
                    Err(e) => return self.log_error(
                        McpError::internal_error("unable to connect to search  engine", None), 
                        Some(
                            format!("unable to parse searxng url from SEARXNG_URL {}\n{}",
                                endpoint, 
                                e
                            )
                        )
                    )
                }
            },
            Err(e) => return self.log_error(
                McpError::internal_error("Could not complete web_search", None),
                Some(
                    format!(
                        "SEARXNG_URL env var not set! cannot find searxng instance\n{}",
                        e
                    )
                )
            )
        };

        log::debug!("creating requests client");
        let searxng_response: SearxngResponse = match self.r_client 
            .get(web_search_endpoint)
            .query(&params)
            .send()
            .await {
            Ok(res) => {
                let sc: reqwest::StatusCode = res.status();
                if sc.is_success() {

                    if let Some(content_type) = res.headers().get(header::CONTENT_TYPE) {
                        let ct = match content_type.to_str() {
                            Ok(ct) => ct,
                            Err(e) => return self.log_error(
                                McpError::internal_error("could not complete web_search request", None), 
                                Some(
                                    format!(
                                        "could not parse content-type from searxng response\n{}", 
                                        e
                                    )
                                )
                            )
                            
                        };
                        log::debug!("content type is {}" ,ct);
                        match ct {
                            "application/json" => {
                                log::info!("got json");

                            },
                            _ => log::error!("searxng responed with invalid content-type header")
                        };
                    }
                    match res.json().await {
                        Ok(s_result) => s_result,
                        Err(e) => return self.log_error(
                            McpError::internal_error("could not complete web_search", None),
                            Some(
                                format!(
                                    "could not parse SearxngResponse from reqwest\n{:?}"
                                    ,e
                                )
                            )
                        ) 
                    }
                }else {
                    return self.log_error(
                        McpError::internal_error("could not complete web_search", None),
                        Some(
                            format!(
                                "searxng response was not 200 - status code = {}",res.status()
                            )
                        )
                    )
                }
            },
            Err(e) => return self.log_error(
                McpError::internal_error("could not complete web_search", None),
                Some(
                    format!("searxng request was unsuccessful {e}")
                )
            ) 
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


        let session_uuid: Uuid = Uuid::new_v4();
        log::debug!("adding SearchResult for session {}",session_uuid);
        {
            let mut state = self.search_results.lock().await;
            let _ = state.insert(session_uuid, search_results.clone()); 
            log::info!("session SearchResult saved");
            log::debug!("current state\n\n{:#?}\n\n", state);
        }
        let response = match Content::json(search_results) {
            Ok(json) => json,
            Err(e) => return self.log_error(
                McpError::internal_error("could not complete web_search", None),
                Some(
                    format!(
                        "could not convert search results to callToolResult\n{}",
                        e
                    )
                ) 
            )
        };
        
        Ok(
            CallToolResult::success(
                vec![
                    response,
                    Content::text(
                        "success!".to_string()
                    ),
                    Content::text(
                        format!("session_uuid: {}",session_uuid)
                    )
                ]
            )
        )
        
    }

    #[tool(description = "fetch the contents of a url. requires the session_uuid and id from a pervious web_search tool call")]
    async fn fetch_url(
        &self,
        Parameters(FetchUrl { session_uuid, id }): Parameters<FetchUrl>
    ) -> Result<CallToolResult, McpError> {
        log::info!("starting fetch_url tool call");

        let session_uuid = match Uuid::parse_str(session_uuid.as_str()) {
            Ok(u) => u,
            Err(e) => return self.log_error(
                McpError::invalid_request("valid session_uuid is required", None),
                Some(
                    format!(
                        "could not parse uuid from session_uuid: {}\n{}", 
                        session_uuid, 
                        e
                    )
                )
            )
        };
        log::debug!("session_uuid: {}\tid: {}\n", session_uuid, id);

        log::debug!("checking app state for session_uuid {}", session_uuid);
        let mut state  = self.search_results.lock().await;
        if let Some(session_results) = state.get(&session_uuid){

            let target = match session_results
                .iter()
                .find(|x| x.id == id){
                    Some(result) => result,
                    None => {
                        log::info!("no search result found for id: {} in session_uuid {}",id, session_uuid);
                        return Ok(
                            CallToolResult::success(
                                vec![
                                    Content::text(
                                        format!("no search result found for id:{} in session_uuid: {}", id, session_uuid)
                                    )
                                ]
                            )
                        )
                    }
                };

            log::info!("found search result for id: {} in session_uuid: {}", id, session_uuid);
            log::debug!("search result struct\n{:#?}", target);
            log::info!("attempting to fetch content from url: {}", target.url);
            
            let url = match reqwest::Url::parse(&target.url) {
                Ok(u) => u,
                Err(e) => return self.log_error(
                    McpError::internal_error("unable to parse url", None), 
                    Some(
                        format!(
                            "Could not parse url from search result:{:#?}\n{}",
                            target, 
                            e
                        )
                    ) 
                )

            };

            let content: String = match self.r_client 
                .get(url)
                .send()
                .await {
                    Ok(response) => {
                        log::debug!("response struct {:#?}", response);
                        if let Some(content_length) = response.content_length(){
                            log::info!("response Content-length: {}", content_length);
                        }
                        if response.status().is_success() {
                            match response.text().await {
                                Ok(text) => text,
                                Err(e) => return self.log_error(
                                    McpError::internal_error(
                                        format!("could not fetch content of url: {}",target.url), 
                                        None
                                    ), 
                                    Some(
                                        format!("could not parse response text for url: {}\n{}", target.url,e)
                                    )
                                )
                            }
                        }
                        else {
                            return Err(
                                McpError::invalid_request(
                                    format!("fetching url content was unsuccessful. status_code: {}", response.status()), 
                                    None
                                )
                            )
                        }
                    },
                    Err(e) => return self.log_error(
                        McpError::internal_error(format!("could not fetch content of url: {}",target.url), None),
                        Some(format!("could not get response from url {}\n{}",target.url, e)))
                };
                
                Ok(
                    CallToolResult::success(
                        vec![
                            Content::text(
                                format!("fetching of content from url: {} was successful", target.url)
                            ),
                            Content::text(content)
                        ]
                    )
                )
        }else{
            return self.log_error(
                McpError::invalid_request("session_uuid not found in app state. cannot fetch url", None), 
                Some(format!("no session_uuid was found in app state for {}", session_uuid))
            )
        }
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
        .with_instructions(
            "This server provides web search functions. Tools: web_search fetch_url. 
            
            tool web_search may be used by independently.

            tool fetch_url requires a session_uuid and id that can be parsed from the output of a call to tool web_search.

            fetch_url steps:
                step 1: call tool web_search providing a web search query.

                step 2: parse web_search tool call result for 'session_uuid' value.

                step 3: select 'id' of url you want to fetch the contents of. 

                step 4: perform fetch_url tool call providing the session_uuid and id obtained in steps 2 and 3. 
            ".to_string());

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

impl LogError for WebSearch {}
