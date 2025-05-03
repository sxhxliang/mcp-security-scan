use std::{
    collections::HashMap, io::{self, Write}, sync::Arc
};

use anyhow::Result;

use crate::llm::{
    client::ChatClient,
    model::{CompletionRequest, Message},
};
use crate::mcp_types::Entity;

pub struct LLMSession {
    client: Arc<dyn ChatClient>,
    tool_set: HashMap<String, Vec<Entity>>,
    model: String,
    messages: Vec<Message>,
}

// struct Entity {

// }

impl LLMSession {
    pub fn new(client: Arc<dyn ChatClient>, tool_set: HashMap<String, Vec<Entity>>, model: String) -> Self {
        Self {
            client,
            tool_set,
            model,
            messages: Vec::new(),
        }
    }

    pub fn add_system_prompt(&mut self, prompt: impl ToString) {
        self.messages.push(Message::system(prompt));
    }


    pub async fn chat(&mut self, input: String) -> Result<()> {
 

        // for (server_name, entries ) in &self.tool_set {
        //     for entry in entries {
        //         self.messages.push(Message::assistant(format!("Tool: {:?}\nInputs: {:?}", entry.name(), entry.description())));
        //     }
        // }

        self.messages.push(Message::user(&input));

        // println!("User:\n{:?}", &self.messages);
        // create request
        let request = CompletionRequest {
            model: self.model.clone(),
            messages: self.messages.clone(),
            temperature: Some(0.7),
            tools: None,
        };

        self.messages.pop();
        // send request
        let response = self.client.complete(request).await?;
        
        if let Some(choice) = response.choices.first() {

            // println!("AI:\n{}", choice.message.content);
            let data = &choice.message.content.replace("\",\"name\":", "\",\"inputSchema\":{},\"name\":");
            // let data = data.replace("\",\"name\":", "\",\"inputSchema\":{},\"name\":");
            let data = data.replace("\"tool\":", "\"Tool\":");
            let data = data.replace("\"prompt\":", "\"Prompt\":");
            let data = data.replace("\"resource\":", "\"Resource\":");
            let data = data.replace("\"name\":", "\"uri\":\"\",\"name\":");
            let resp: std::result::Result<Vec<Entity>, serde_json::Error>  = serde_json::from_str(&data);

            match resp {
                Ok(entities) => {
                    for entity in entities {
                        println!("Entity: {:?}", entity);
                    }
                }
                Err(e) => {
                    println!("Error: {:?}", e);
                }
            }
        
        }
        Ok(())
    }
}

// 单元测试示例
#[cfg(test)]
mod tests {
    use anyhow::Ok;

    use super::*;



    #[test]
    fn test_response() -> anyhow::Result<()> {
        let data = "[{\"Resource\":{\"description\":\"\",\"name\":\"Resource 64\"}},{\"Tool\":{\"description\":\"Adds two numbers\",\"name\":\"add\"}},{\"Prompt\":{\"description\":\"A prompt with arguments\",\"name\":\"complex_prompt\"}}]".to_string();
        let data = data.replace("\",\"name\":", "\",\"inputSchema\":{},\"name\":");
        let data = data.replace("\"tool\":", "\"Tool\":");
        let data = data.replace("\"prompt\":", "\"Prompt\":");
        let data = data.replace("\"resource\":", "\"Resource\":");
        let data = data.replace("\"name\":", "\"uri\":\"\",\"name\":");
        let config:std::result::Result<Vec<Entity>, serde_json::Error>  = serde_json::from_str(&data);
        println!("{:?}", config);
        Ok(())
    }
}