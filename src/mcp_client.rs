use std::path::Path;
use shellexpand::tilde;
use crate::mcp_types::{ClaudeConfigFile, CursorMCPConfig, MCPConfig, VSCodeConfigFile, VSCodeMCPConfig};


pub fn scan_mcp_config_file(path: &str) -> anyhow::Result<Box<dyn MCPConfig>> {
     // 扩展路径中的 ~ 符号
     let expanded_path = tilde(path).into_owned();
     let path = Path::new(&expanded_path);
 
     // 读取文件内容
    let contents = std::fs::read_to_string(path)?;
    parse_and_validate(&contents)
}

fn parse_and_validate(config: &str) -> anyhow::Result<Box<dyn MCPConfig>> {
    let models: Vec<Box<dyn Fn(&str) -> Result<Box<dyn MCPConfig>, serde_json::Error>>> = vec![
        Box::new(|v| Ok(Box::new(serde_json::from_str::<ClaudeConfigFile>(v)?))),
        Box::new(|v| Ok(Box::new(serde_json::from_str::<VSCodeConfigFile>(v)?))),
        Box::new(|v| Ok(Box::new(serde_json::from_str::<VSCodeMCPConfig>(v)?))),
        Box::new(|v| Ok(Box::new(serde_json::from_str::<CursorMCPConfig>(v)?))),
    ];
    
    let mut errors = Vec::new();
    
    for model in models {
        match model(config) {
            Ok(config) => {
                return Ok(config)
            }
            Err(e) => errors.push(e),
        }
    }

    
    let model_names = vec!["ClaudeConfigFile", "VSCodeConfigFile", "VSCodeMCPConfig", "CursorMCPConfig"];
    // let error_messages = errors.iter()
    //     .map(|e| e.to_string())
    //     .collect::<Vec<_>>()
    //     .join("\n");
    
    Err(anyhow::format_err!("Could not parse config file with models: {:?}", model_names))
}

// 单元测试示例
#[cfg(test)]
mod tests {
    use anyhow::Ok;

    use super::*;

    
    #[test]
    fn test_cursor_config() -> anyhow::Result<()> {
    
        let json_data = r#"
        {
            "mcpServers": {
                "test": {
                    "url": "0.0.0.0:8080/sse"
                }
            }
        }"#;

        // 解析Claude配置
        let claude_config: ClaudeConfigFile = serde_json::from_str(json_data)?;
        Ok(())
    }
    
}