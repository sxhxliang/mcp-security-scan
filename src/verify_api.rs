
use reqwest::Client;
use rmcp::model::{Prompt, Resource, Tool};
use crate::mcp_types::Result;

pub async fn verify_server(
    tools: Vec<Tool>,
    prompts: Vec<Prompt>,
    resources: Vec<Resource>,
    base_url: &str,
) -> (Vec<Result<bool>>, Vec<Result<bool>>, Vec<Result<bool>>) {
    if tools.is_empty() && prompts.is_empty() && resources.is_empty() {
        return (vec![], vec![], vec![]);
    }

    let mut messages = Vec::new();
    
    for tool in &tools {
        messages.push(serde_json::json!({ 
            "role": "system",
            "content": format!("Tool Name:{}\nTool Description:{:?}", tool.name, tool.description)
        }));
    }
    
    for prompt in &prompts {
        messages.push(serde_json::json!({ 
            "role": "system",
            "content": format!("Prompt Name:{}\nPrompt Description:{:?}", prompt.name, prompt.description)
        }));
    }
    
    for resource in &resources {
        messages.push(serde_json::json!({ 
            "role": "system",
            "content": format!("Resource Name:{}\nResource Description:{:?}", resource.name, resource.description)
        }));
    }

    let url = format!("{}/api/v1/public/mcp", base_url);
    let client = Client::new();
    
    match client.post(&url)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({ "messages": messages }))
        .send()
        .await {
            Ok(response) => {
                if response.status().is_success() {
                    let response_content: serde_json::Value = response.json().await.unwrap();
                    let mut results = messages.iter()
                        .map(|_| Result { value: Some(true), message: Some("verified".to_string()) })
                        .collect::<Vec<_>>();
                    
                    if let Some(errors) = response_content.get("errors") {
                        for error in errors.as_array().unwrap() {
                            let key = error.get("key").unwrap().as_str().unwrap();
                            let idx = key.split(',').nth(1).unwrap().parse::<usize>().unwrap();
                            results[idx] = Result { 
                                value: Some(false), 
                                message: Some(format!("failed - {}", 
                                    error.get("args").unwrap().as_array().unwrap()
                                        .iter().map(|v| v.as_str().unwrap()).collect::<Vec<_>>().join(" ")))
                            };
                        }
                    }
                    
                    let (results_tools, remaining) = results.split_at(tools.len());
                    let (results_prompts, results_resources) = remaining.split_at(prompts.len());
                    
                    (results_tools.to_vec(), results_prompts.to_vec(), results_resources.to_vec())
                } else {
                    let error_msg = format!("Error: {} - {}", response.status(), response.text().await.unwrap());
                    (
                        vec![Result { value: None, message: Some(error_msg.clone()) }],
                        vec![Result { value: None, message: Some(error_msg.clone()) }],
                        vec![Result { value: None, message: Some(error_msg) }],
                    )
                }
            }
            Err(e) => {
                let errstr = e.to_string();
                (
                    vec![Result { value: None, message: Some(format!("could not reach verification server {}", errstr)) }],
                    vec![Result { value: None, message: Some(format!("could not reach verification server {}", errstr)) }],
                    vec![Result { value: None, message: Some(format!("could not reach verification server {}", errstr)) }],
                )
            }
        }
}