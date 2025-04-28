use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use chrono::Utc;
use md5;

use crate::mcp_types::{entity_type_to_str, Entity, VerifyResult, ScannedEntities, ScannedEntity};
use crate::utils::upload_whitelist_entry;

#[derive(Debug)]
pub struct StorageFile {
    path: PathBuf,
    pub scanned_entities: ScannedEntities,
    pub whitelist: HashMap<String, String>,
}

impl StorageFile {
    pub fn new(path: &str) -> Self {
        let path = shellexpand::tilde(path).into_owned();
        let path = PathBuf::from(path);
        println!("store {:?}", path);
        let mut scanned_entities = HashMap::new();
        let mut whitelist = HashMap::new();

        if path.is_file() {
            println!("[bold]Legacy storage file detected at {:?}, converting to new format", path);
            let legacy_data = fs::read_to_string(&path).unwrap();
            let legacy_data: serde_json::Value = serde_json::from_str(&legacy_data).unwrap();

            if let Some(wl) = legacy_data.get("__whitelist") {
                whitelist = serde_json::from_value(wl.clone()).unwrap();
            }

            if let Ok(entities) = serde_json::from_value::<ScannedEntities>(legacy_data) {
                scanned_entities = entities;
            } else {
                println!("[bold red]Could not load legacy storage file {:?}", path);
            }

            fs::remove_file(&path).unwrap();
        }

        if path.is_dir() {
            println!("[bold]Loading storage from {:?}", path);
            let scanned_entities_path = path.join("scanned_entities.json");
            if scanned_entities_path.exists() {
                if let Ok(data) = fs::read_to_string(&scanned_entities_path) {
                    if let Ok(entities) = serde_json::from_str::<ScannedEntities>(&data) {
                        scanned_entities = entities;
                    } else {
                        println!("[bold red]Could not load scanned entities file {:?}", scanned_entities_path);
                    }
                }
            }

            let whitelist_path = path.join("whitelist.json");
            if whitelist_path.exists() {
                if let Ok(data) = fs::read_to_string(&whitelist_path) {
                    if let Ok(wl) = serde_json::from_str::<HashMap<String, String>>(&data) {
                        whitelist = wl;
                    }
                }
            }
        }

        Self {
            path,
            scanned_entities,
            whitelist,
        }
    }

    pub fn reset_whitelist(&mut self) {
        self.whitelist.clear();
        self.save();
    }

    pub fn compute_hash(&self, entity: Option<&Entity>) -> Option<String> {
        entity.and_then(|e| {
            e.description().map(|desc| {
                let mut hasher = md5::Context::new();
                hasher.consume(desc.as_bytes());
                format!("{:x}", hasher.compute())
            })
        })
    }

    pub fn check_and_update(&mut self, server_name: &str, entity: &Entity, verified: bool) -> (VerifyResult, Option<ScannedEntity>) {
        let entity_type = entity_type_to_str(entity);
        println!("Checking {} {}...", entity_type, entity.name());
        let key = format!("{}.{}.{}", server_name, entity_type, entity.name());
        let hash = self.compute_hash(Some(entity)).unwrap_or_default();
        
        let new_data = ScannedEntity {
            hash,
            r#type: entity_type.to_string(),
            verified,
            timestamp: Utc::now(),
            description: entity.description().map(|s| s.to_string()),
        };

        let mut changed = false;
        let mut message = None;
        let mut prev_data = None;

        if let Some(existing) = self.scanned_entities.get(&key) {
            prev_data = Some(existing.clone());
            changed = existing.hash != new_data.hash;
            if changed {
                message = Some(format!(
                    "{} description changed since previous scan at {}",
                    entity_type,
                    existing.timestamp.format("%d/%m/%Y, %H:%M:%S")
                ));
            }
        }

        self.scanned_entities.insert(key, new_data);
        (VerifyResult {
            value: Some(changed),
            message,
        }, prev_data)
    }

    pub fn print_whitelist(&self) {
        let mut keys: Vec<_> = self.whitelist.keys().collect();
        keys.sort();

        for key in &keys {
            let (entity_type, name) = if let Some(pos) = key.find('.') {
                let (t, n) = key.split_at(pos);
                (t, &n[1..])
            } else {
                ("tool", key.as_str())
            };
            println!("{} {} {}", entity_type, name, self.whitelist[*key]);
        }
        println!("[bold]{} entries in whitelist", keys.len());
    }

    pub fn add_to_whitelist(&mut self, entity_type: &str, name: &str, hash: &str, base_url: Option<&str>) {
        let key = format!("{}.{}", entity_type, name);
        self.whitelist.insert(key, hash.to_string());
        self.save();

        if let Some(url) = base_url {
            upload_whitelist_entry(name, hash, url);
        }
    }

    pub fn is_whitelisted(&self, entity: &Entity) -> bool {
        self.compute_hash(Some(entity))
            .map(|hash| self.whitelist.values().any(|v| v == &hash))
            .unwrap_or(false)
    }

    pub fn save(&self) {
        fs::create_dir_all(&self.path).unwrap();
        
        let scanned_entities_path = self.path.join("scanned_entities.json");
        fs::write(
            &scanned_entities_path,
            serde_json::to_string_pretty(&self.scanned_entities).unwrap(),
        ).unwrap();

        let whitelist_path = self.path.join("whitelist.json");
        fs::write(
            &whitelist_path,
            serde_json::to_string_pretty(&self.whitelist).unwrap(),
        ).unwrap();
    }
}