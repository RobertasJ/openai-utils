// #![allow(unused)]

// extern crate proc_macro;
// use proc_macro::TokenStream;
// use std::collections::HashMap;
// use std::fmt::Display;
// use std::result::Result;
// use std::vec::Vec;

// use error::ClientError;
// use openai_rust::chat::stream::ChatResponseEvent;
// use openai_rust::chat::{ChatResponse, self};
// use openai_rust::Client;
// use openai_rust::futures_util::Stream;
// use openai_rust::{chat::Message, models::Model};
// use serde::{Deserialize, Serialize};

// mod error;

// #[derive(Clone, Serialize, Deserialize)]
// pub enum Engine {
//     Gpt35Turbo,
//     Gpt4,
// }

// impl Display for Engine {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             Engine::Gpt35Turbo => write!(f, "gpt-3.5-turbo"),
//             Engine::Gpt4 => write!(f, "gpt-4"),
//         }
//     }
// }

// pub enum Role {
//     User,
//     Assistant,
//     System,
// }

// impl Display for Role {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             Role::User => write!(f, "user"),
//             Role::Assistant => write!(f, "assistant"),
//             Role::System => write!(f, "system"),
//         }
//     }
// }

// type SystemMessage = fn(ctx: HashMap<String, String>) -> String;

// #[derive(Serialize, Deserialize, Default)]
// pub struct AiModel<T> {
//     ctx: HashMap<String, String>,
//     #[serde(skip)]
//     system_message: Option<SystemMessage>,
//     messages: Vec<Message>,
//     model_type: Option<Engine>,
//     #[serde(skip)]
//     client: Option<Client>,
//     temperature: f32,
//     phantomdata: std::marker::PhantomData<T>,
// }

// impl<T> AiModel<T> {
//     fn with_key(key: &str) -> Self<> {
//         Self {
//             ctx: HashMap::new(),
//             system_message: None,
//             messages: Vec::new(),
//             model_type: None,
//             client: Some(Client::new(key)),
//             temperature: 0.0,
//             phantomdata: std::marker::PhantomData,
//         }
//     }
// }

// struct Authed;
// struct Unauthed;

// impl<Authed> AiModel<Authed> {
//     pub fn add_message(&mut self, content: impl Into<String>, role: Role) {
//         self.messages.push(Message {
//             role: role.to_string(),
//             content: content.into(),
//         });
//     }
//     /// add any context you want to use when making the system message with the context function
//     pub fn system_message(&mut self, content: fn(ctx: HashMap<String, String>) -> String) {
//         self.system_message = Some(content);
//     }
    
//     pub fn context(&mut self, key: impl Into<String>, value: impl Into<String>) {
//         let key = key.into();
//         let value = value.into();
        
//         self.ctx.insert(key, value);
//     }
    
//     pub fn engine(&mut self, model_type: Engine) {
//         self.model_type = Some(model_type);
//     }
    
//     pub async fn create_chat(&mut self) -> anyhow::Result<ChatResponse> {
//         let client = self.client.as_ref().ok_or(ClientError::Client)?;
//         let mut messages = self.messages.clone();
//         if let Some(system_message) = &mut self.system_message {
//             messages.push(Message {
//                 role: Role::System.to_string(),
//                 content: system_message(self.ctx.clone()),
//             });
//         }
        
//         let model = self.model_type.clone().ok_or(ClientError::Model)?.to_string();
        
//         let mut args = chat::ChatArguments::new(model, messages);
        
//         args.temperature = Some(self.temperature);
        
//         let response = client.create_chat(args).await?;
//         Ok(response)
//     }
    
//     pub async fn create_chat_stream(&mut self) -> 
//     anyhow::Result<impl Stream<Item = Result<Vec<ChatResponseEvent>, anyhow::Error>>> {
//         let client = self.client.as_ref().ok_or(ClientError::Client)?;
//         let mut messages = self.messages.clone();
//         if let Some(system_message) = &mut self.system_message {
//             messages.push(Message {
//                 role: Role::System.to_string(),
//                 content: system_message(self.ctx.clone()),
//             });
//         }
        
//         let model = self.model_type.clone().ok_or(ClientError::Model)?.to_string();
        
//         let mut args = chat::ChatArguments::new(model, messages);
        
//         args.temperature = Some(self.temperature);
        
//         let response = client.create_chat_stream(args).await?;
//         Ok(response)
//     }
    
//     fn temperature(&mut self, temperature: f32) {
//         self.temperature = temperature;
//     }
    
// }
