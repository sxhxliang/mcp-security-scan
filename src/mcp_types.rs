use rmcp::{model::{Annotated, Prompt, RawResource, Tool}, service::RunningService, RoleClient, ServiceExt};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::{collections::HashMap, process::Stdio};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Entity {
    Prompt(Prompt),
    Resource(Annotated<RawResource>),
    Tool(Tool),

}
impl Entity  {
    pub fn description(&self) -> Option<String> {
        match self {
            Entity::Prompt(prompt) => prompt.description.clone(),
            Entity::Resource(resource) => resource.description.clone(),
            Entity::Tool(tool) => {
                match tool.description {
                    Some(ref desc) => Some(desc.clone().into_owned()),
                    None => None,
                }
            },
        }
    }

    pub fn name(&self) -> String {
        match self {
            Entity::Prompt(prompt) => prompt.name.clone(),
            Entity::Resource(resource) => resource.name.clone(),
            Entity::Tool(tool) => tool.name.clone().into_owned(),
        }
    }

}

pub fn entity_type_to_str(entity: &Entity) -> &'static str {
    match entity {
        Entity::Prompt(_) => "prompt",
        Entity::Resource(_) => "resource",
        Entity::Tool(_) => "tool",
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedEntity {
    pub hash: String,
    pub r#type: String,
    pub verified: bool,
    pub timestamp: DateTime<Utc>,
    pub description: Option<String>,
}

pub type ScannedEntities = HashMap<String, ScannedEntity>;

#[derive(Debug, Clone)]
pub struct Result<T> {
    pub value: Option<T>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SSEServer {
    pub url: String,
    pub r#type: Option<String>,
    pub headers: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StdioServer {
    pub command: String,
    pub args: Option<Vec<String>>,
    pub r#type: Option<String>,
    pub env: Option<HashMap<String, String>>,
}

pub trait MCPConfig {
    fn get_servers(&self) -> HashMap<String, Server>;
    fn set_servers(&mut self, servers: HashMap<String, Server>);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeConfigFile {
    pub mcp_servers: HashMap<String, Server>,
}

impl MCPConfig for ClaudeConfigFile {
    fn get_servers(&self) -> HashMap<String, Server> {
        self.mcp_servers.clone()
    }

    fn set_servers(&mut self, servers: HashMap<String, Server>) {
        self.mcp_servers = servers;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CursorMCPConfig {
    pub inputs: Option<Vec<serde_json::Value>>,
    pub mcp_servers: HashMap<String, Server>,
}

impl MCPConfig for CursorMCPConfig {
    fn get_servers(&self) -> HashMap<String, Server> {
        self.mcp_servers.clone()
    }

    fn set_servers(&mut self, servers: HashMap<String, Server>) {
        self.mcp_servers = servers;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VSCodeMCPConfig {
    pub inputs: Option<Vec<serde_json::Value>>,
    pub servers: HashMap<String, Server>,
}

impl MCPConfig for VSCodeMCPConfig {
    fn get_servers(&self) -> HashMap<String, Server> {
        self.servers.clone()
    }

    fn set_servers(&mut self, servers: HashMap<String, Server>) {
        self.servers = servers;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VSCodeConfigFile {
    pub mcp: VSCodeMCPConfig,
}

impl MCPConfig for VSCodeConfigFile {
    fn get_servers(&self) -> HashMap<String, Server> {
        self.mcp.servers.clone()
    }

    fn set_servers(&mut self, servers: HashMap<String, Server>) {
        self.mcp.servers = servers;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged, rename_all = "lowercase",  )]
pub enum Server {
    SSE(SSEServer),
    Stdio(StdioServer),
}

impl Server {
    pub async fn start(&self) -> anyhow::Result<RunningService<RoleClient, ()>> {
        let client = match self {
            Server::SSE(server) => {
                let transport = rmcp::transport::sse::SseTransport::start(server.url.clone()).await?;
                ().serve(transport).await?
            },
            Server::Stdio(server) => {
                let transport = rmcp::transport::child_process::TokioChildProcess::new(
                    tokio::process::Command::new(server.command.clone())
                        .args(server.args.clone().unwrap_or_default())
                        // .envs(Some(server.env.clone()))
                        .stderr(Stdio::inherit())
                        .stdout(Stdio::inherit()),
                )?;
                ().serve(transport).await?
            }
        };
        Ok(client)
    }
}