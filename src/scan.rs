use anyhow::Result;
use rmcp::model;

use crate::cli::WhitelistArgs;
use crate::llm;
use crate::mcp_client::scan_mcp_config_file;
use crate::mcp_types::{Entity, Server, VerifyResult, entity_type_to_str};
use crate::storage_file::StorageFile;
use crate::verify_api::verify_server;
use colored::Colorize;
use std::collections::HashMap;
use std::sync::Arc;

pub struct MCPScanner {
    paths: Vec<String>,
    base_url: String,
    checks_per_server: usize,
    storage_file: StorageFile,
    server_timeout: i64,
    suppress_mcpserver_io: bool,
    llm_api_key: Option<String>,
    llm_api_url: Option<String>,
}

impl MCPScanner {
    pub fn new(
        storage_path: &str,
        base_url: &str,
        server_timeout: i64,
        suppress_mcpserver_io: bool,
        checks_per_server: usize,
        llm_api_key: Option<String>,
        llm_api_url: Option<String>,
    ) -> Self {
        Self {
            paths: Vec::new(),
            base_url: base_url.to_string(),
            checks_per_server,
            storage_file: StorageFile::new(storage_path),
            server_timeout,
            suppress_mcpserver_io,
            llm_api_key,
            llm_api_url,
        }
    }

    pub async fn scan_files(&mut self, files: &Vec<String>) {
        for file in files {
            if let Err(e) = self.scan(file, true, false).await {
                eprintln!("Error scanning {}: {}", file, e);
            }
        }
    }

    pub async fn scan(&mut self, path: &str, verbose: bool, inspect_only: bool) -> Result<()> {
        println!("Scanning {}", path);
        let servers = match scan_mcp_config_file(path) {
            Ok(config) => config.get_servers(),
            Err(e) => {
                if verbose {
                    println!("{}: {}", path, e);
                }
                return Err(e);
            }
        };

        if verbose {
            println!(
                "{}: found {} server{}",
                path,
                servers.len(),
                if servers.len() == 1 { "" } else { "s" }
            );
        }

        let mut servers_with_entities: HashMap<String, Vec<Entity>> = HashMap::new();

        println!("LLM key{:?}", self.llm_api_key);
        println!("LLM url {:?}", self.llm_api_url);
        let client = llm::client::OpenAIClient::new(
                            self.llm_api_key.clone().unwrap_or_default(), 
                            self.llm_api_url.clone(), 
                            None);
        println!("LLM session initialized {:?}", client);

        let mut llm_session = llm::session::LLMSession::new(
            Arc::new(client), HashMap::new(), "Qwen/Qwen3-8B".into()
        );
        llm_session.add_system_prompt("/no_think 你是一个Json 数据翻译助手，将json数据中的value翻译成中文,注意，1、不要翻译json的key,只翻译value。 /no_think");

        for (server_name, server_config) in servers {
            let entities: Vec<Entity> = match self.check_server(&server_config).await {
                Ok((prompts, resources, tools)) => {
                    tools.into_iter().chain(prompts).chain(resources).collect()
                }
                Err(e) => {
                    if verbose {
                        eprintln!("{}: {}", server_name, e);
                    }
                    continue;
                }
            };
            println!(
                "{}: found {} entity{}",
                server_name,
                entities.len(),
                if entities.len() == 1 { "" } else { "s" }
            );
            for entity in &entities {
                match entity {
                    Entity::Tool(tool) => println!(
                        "  -  ✅ verified {}: {}",
                        "tool".bright_yellow(),
                        tool.name.clone().into_owned().bright_green()
                    ),
                    Entity::Prompt(prompt) => println!(
                        "  -  ✅ verified {}: {}",
                        "prompt".bright_yellow(),
                        prompt.name.to_owned().bright_green()
                    ),
                    Entity::Resource(resource) => println!(
                        "  -  ✅ verified {}: {}",
                        "resource".bright_yellow(),
                        resource.name.to_owned().bright_green()
                    ),
                }
            }
            servers_with_entities.insert(server_name.clone(), entities.clone());

            if !inspect_only {
                self.verify_and_report_entities(&server_name, &entities, verbose)
                    .await?;
            }else {
                println!("{}", "Inspection mode enabled, skipping verification".bright_yellow());


                let new_entities = entities.iter().map(|entity| match entity {
                    Entity::Tool(tool) => {
                        serde_json::json!({
                            "Tool":{
                                "name": tool.name.to_string(),
                                "description": tool.description.as_deref().unwrap_or("")
                            }
                        })
                    }
                    Entity::Prompt(prompt) => {
                        serde_json::json!({
                            "Prompt":{
                                "name": prompt.name.to_string(),
                                "description": prompt.description.as_deref().unwrap_or("")
                            }
                        })
                    }
                    Entity::Resource(resource) => {
                        serde_json::json!({
                            "Resource":{
                                "name": resource.name.to_string(),
                                "description": resource.description.as_deref().unwrap_or("")
                            }
                        })
                    }
                }).collect::<Vec<_>>();

                // println!("{:#?}", serde_json::to_string(&new_entities));

                let _ = llm_session.chat(serde_json::to_string(&new_entities).unwrap()).await;


            }
        }

        Ok(())
    }

    async fn check_server(
        &self,
        server_config: &Server,
    ) -> anyhow::Result<(Vec<Entity>, Vec<Entity>, Vec<Entity>)> {
        // let duration = Duration::from_secs_f64(self.server_timeout as f64);
        let client = server_config.start().await?;
        let server = client.peer().clone();
        let capabilities = server.peer_info().capabilities.clone();

        let tools = match capabilities.tools {
            Some(_) => server
                .list_all_tools()
                .await?
                .into_iter()
                .map(|t| Entity::Tool(t))
                .collect::<Vec<_>>(),
            None => vec![],
        };
        let prompts = match capabilities.prompts {
            Some(_) => server
                .list_all_prompts()
                .await?
                .into_iter()
                .map(|p| Entity::Prompt(p))
                .collect::<Vec<_>>(),
            None => vec![],
        };
        let resources = match capabilities.resources {
            Some(_) => server
                .list_all_resources()
                .await?
                .into_iter()
                .map(|r| Entity::Resource(r))
                .collect::<Vec<_>>(),
            None => vec![],
        };
        client.cancel().await?;
        Ok((prompts, resources, tools))
    }

    async fn verify_and_report_entities(
        &mut self,
        server_name: &str,
        entities: &Vec<Entity>,
        verbose: bool,
    ) -> anyhow::Result<()> {
        let (verification_result_tools, verification_result_prompts, verification_result_resources) =
            verify_server(entities, &self.base_url).await;

        let verification_results: Vec<_> = verification_result_tools
            .into_iter()
            .chain(verification_result_prompts)
            .chain(verification_result_resources)
            .collect();

        if verification_results.is_empty() {
            return Ok(());
        }

        for (entity, verified) in entities.iter().zip(verification_results) {
            let mut additional_text = None;

            // 检查实体是否变更
            let (changed, prev_data) = self.storage_file.check_and_update(
                server_name,
                entity,
                verified.value.unwrap_or(false),
            );
            // println!("changed: {:?}", changed);

            if changed.value.unwrap() && prev_data.is_some() {
                let prev = prev_data.unwrap();
                additional_text = Some(format!(
                    "Previous description({}):\n{}",
                    prev.timestamp.format("%d/%m/%Y, %H:%M:%S"),
                    prev.description.unwrap_or_default()
                ));
            }

            // 检查是否在白名单中
            let verified = if self.storage_file.is_whitelisted(entity) {
                println!("whitelisted");
                VerifyResult {
                    value: Some(true),
                    message: Some(format!(
                        "whitelisted {}",
                        verified.message.unwrap_or_default()
                    )),
                }
            } else if !verified.value.unwrap() || changed.value.unwrap() {
                println!("not whitelisted");
                let hash = self
                    .storage_file
                    .compute_hash(Some(entity))
                    .unwrap_or_default();
                let message = format!(
                    "You can whitelist this {} by running `mcp-scan whitelist {} '{}' {}`",
                    entity_type_to_str(entity),
                    entity_type_to_str(entity),
                    entity.name(),
                    hash
                );

                additional_text = match additional_text {
                    Some(text) => Some(format!("{}\n\n{}", text, message)),
                    None => Some(message),
                };

                VerifyResult {
                    value: verified.value,
                    message: verified.message,
                }
            } else {
                verified
            };

            if verbose {
                println!(
                    "{} - {}: {}",
                    entity_type_to_str(entity),
                    entity.name(),
                    if verified.value.unwrap_or(false) {
                        "✅"
                    } else {
                        "❌"
                    }
                );

                if let Some(text) = additional_text {
                    println!("{}", text);
                }
            }
        }

        Ok(())
    }

    pub async fn inspect(&mut self, files: &Vec<String>) -> Result<(), anyhow::Error> {
        println!("{}", "Inspecting configurations...".bright_blue());
        // 实现检查逻辑
        for file in files {
            if let Err(e) = self.scan(file, true, true).await {
                eprintln!("Error scanning {}: {}", file, e);
            }
        }
        Ok(())
    }

    fn manage_whitelist(&self, args: &WhitelistArgs) -> anyhow::Result<()> {
        // let mut storage = if self.storage_file.exists() {
        //     Storage::load(&self.storage_file)?
        // } else {
        //     Storage::default()
        // };

        // if args.reset {
        //     storage.reset();
        //     storage.save(&self.storage_path)?;
        //     println!("{}", "Whitelist reset successfully".green());
        //     return Ok(());
        // }

        // if let (Some(t), Some(n), Some(h)) = (&args.entity_type, &args.entity_name, &args.entity_hash) {
        //     storage.add_entry(WhitelistEntry {
        //         entity_type: t.clone(),
        //         name: n.clone(),
        //         hash: h.clone(),
        //     });
        //     storage.save(&self.storage_path)?;
        //     println!("{}", "Whitelist updated successfully".green());
        // }

        // self.print_whitelist(&storage)
        Ok(())
    }

    fn print_whitelist(&self) -> Result<(), anyhow::Error> {
        println!("{}", "Current Whitelist:".underline().bright_blue());
        // for (name, entry) in &self.storage_file.scanned_entities {
        //     println!("Type: {}\nName: {}\nHash: {}\n",
        //         entry.entity_type.bright_yellow(),
        //         entry.name.bright_green(),
        //         entry.hash.bright_cyan());
        // }
        Ok(())
    }
}
