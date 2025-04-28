
use reqwest::Client;
use crate::mcp_types::{Entity, VerifyResult};

pub async fn verify_server(
    entities: &Vec<Entity>,
    base_url: &str,
) -> (Vec<VerifyResult>, Vec<VerifyResult>, Vec<VerifyResult>) {
    if entities.is_empty() {
        return (vec![], vec![], vec![]);
    }

    let mut messages = Vec::new();
    let num_tools = entities.iter().filter(|e| matches!(e, Entity::Tool(_))).count();
    let num_prompts = entities.iter().filter(|e| matches!(e, Entity::Prompt(_))).count();
    let num_resources = entities.iter().filter(|e| matches!(e, Entity::Resource(_))).count();
    
    for entity in entities {
        match entity {
            Entity::Prompt(prompt) =>{
                messages.push(serde_json::json!({ 
                    "role": "system",
                    "content": format!("Prompt Name:{}\nPrompt Description:{:?}", prompt.name, prompt.description)
                }));
            },
            Entity::Resource(resource) => {
                messages.push(serde_json::json!({ 
                    "role": "system",
                    "content": format!("Resource Name:{}\nResource Description:{:?}", resource.name, resource.description)
                }));
            }
            Entity::Tool(tool) => {
                messages.push(serde_json::json!({ 
                    "role": "system",
                    "content": format!("Tool Name:{}\nTool Description:{:?}", tool.name, tool.description)
                }));
            },
        }
    }

    let url = format!("{}/api/v1/public/mcp", base_url);
    let client = Client::new();
    println!("count messages: {}", messages.len());
    // println!("{}", url); // Debug print to see the URL being sent to the server
    // println!("{}", serde_json::to_string(&messages).unwrap()); // Debug print to see the messages being sent to the server in jso
    match client.post(&url)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({ "messages": messages }))
        .send()
        .await {
            Ok(response) => {
                if response.status().is_success() {
                    let response_content: serde_json::Value = response.json().await.unwrap();
                    let mut results = messages.iter()
                        .map(|_| VerifyResult { value: Some(true), message: Some("verified".to_string()) })
                        .collect::<Vec<_>>();
                    
                    if let Some(errors) = response_content.get("errors") {
                        for error in errors.as_array().unwrap() {
                            let key = error.get("key").unwrap().as_str().unwrap();
                            let idx = key.split(',').nth(1).unwrap().parse::<usize>().unwrap();
                            results[idx] = VerifyResult { 
                                value: Some(false), 
                                message: Some(format!("failed - {}", 
                                    error.get("args").unwrap().as_array().unwrap()
                                        .iter().map(|v| v.as_str().unwrap()).collect::<Vec<_>>().join(" ")))
                            };
                        }
                    }
                    
                    let (results_tools, remaining) = results.split_at(num_tools);
                    let (results_prompts, results_resources) = remaining.split_at(num_prompts);
                    
                    (results_tools.to_vec(), results_prompts.to_vec(), results_resources.to_vec())
                } else {
                    let error_msg = format!("Error: {} - {}", response.status(), response.text().await.unwrap());
                    (
                        vec![],
                        vec![],
                        vec![]
                        // vec![VerifyResult { value: None, message: Some(error_msg.clone()) }],
                        // vec![VerifyResult { value: None, message: Some(error_msg.clone()) }],
                        // vec![VerifyResult { value: None, message: Some(error_msg) }],
                    )
                }
            }
            Err(e) => {
                let errstr = e.to_string();
                (
                    vec![],
                    vec![],
                    vec![]
                    // vec![VerifyResult { value: None, message: Some(format!("could not reach verification server {}", errstr)) }],
                    // vec![VerifyResult { value: None, message: Some(format!("could not reach verification server {}", errstr)) }],
                    // vec![VerifyResult { value: None, message: Some(format!("could not reach verification server {}", errstr)) }],
                )
            }
        }
}