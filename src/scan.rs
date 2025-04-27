use anyhow::Result;

use colored::Colorize;
use std::collections::HashMap;
use crate::cli::WhitelistArgs;
use crate::mcp_client::scan_mcp_config_file;
use crate::mcp_types::{entity_type_to_str, Entity, Server};
use crate::storage_file::StorageFile;

pub struct MCPScanner {
    paths: Vec<String>,
    base_url: String,
    checks_per_server: usize,
    storage_file: StorageFile,
    server_timeout: i64,
    suppress_mcpserver_io: bool,
}

impl MCPScanner {
    pub fn new(
        storage_path: &str,
        base_url: &str,
        server_timeout: i64,
        suppress_mcpserver_io: bool,
        checks_per_server: usize,
    ) -> Self {
        Self {
            paths: Vec::new(),
            base_url: base_url.to_string(),
            checks_per_server,
            storage_file: StorageFile::new(storage_path),
            server_timeout,
            suppress_mcpserver_io,
        }
    }

    pub async fn scan_files(&self, files: &Vec<String>) {
        for file in files {
            if let Err(e) = self.scan(file, true, true).await {
                eprintln!("Error scanning {}: {}", file, e);
            }
        }
    }

    pub async fn scan(&self, path: &str, verbose: bool, inspect_only: bool) -> Result<()> {
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
                    Entity::Tool(tool) => println!("  -  ✅ verified {}: {}", "tool".bright_yellow(), tool.name.clone().into_owned().bright_green()),
                    Entity::Prompt(prompt) => println!("  -  ✅ verified {}: {}", "prompt".bright_yellow(), prompt.name.to_owned().bright_green()),
                    Entity::Resource(resource) => println!("  -   ✅ verified {}: {}", "resource".bright_yellow(), resource.name.to_owned().bright_green()),
                }
            
            }
            servers_with_entities.insert(server_name.clone(), entities.clone());

            if !inspect_only {
                self.verify_and_report_entities(&server_name, &entities, verbose)?;
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
            Some(_) => server.list_all_tools().await?.into_iter().map(|t| {
                    Entity::Tool(t)
                }).collect::<Vec<_>>(),
            None => vec![],
            
        };
        let prompts = match capabilities.prompts {
            Some(_) => server.list_all_prompts().await?.into_iter().map(|p| {
                Entity::Prompt(p)
            }).collect::<Vec<_>>(),
            None => vec![],
        };
        let resources = match capabilities.resources {
            Some(_) => server.list_all_resources().await?.into_iter().map(|r| {
                Entity::Resource(r)
            }).collect::<Vec<_>>(),
            None => vec![],
        };
        client.cancel().await?;
        Ok((prompts, resources, tools))
    }

    fn verify_and_report_entities(
        &self,
        server_name: &str,
        entities: &Vec<Entity>,
        verbose: bool,
    ) -> Result<()> {
        // TODO: Implement verification and reporting logic
        Ok(())
    }

    pub async fn inspect(&self) -> Result<(), anyhow::Error> {
        println!("{}", "Inspecting configurations...".bright_blue());
        // 实现检查逻辑
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
        // for entry in &storage.whitelist {
        //     println!("Type: {}\nName: {}\nHash: {}\n", 
        //         entry.entity_type.bright_yellow(),
        //         entry.name.bright_green(),
        //         entry.hash.bright_cyan());
        // }
        Ok(())
    }
}
