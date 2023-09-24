#![allow(dead_code)]
#![feature(type_name_of_val)]

mod chat_completion;
mod chat_completion_delta;
mod chat_completion_request;

use lazy_static::lazy_static;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::{
    any::type_name_of_val,
    sync::{Arc, RwLock},
};

use schemars::{schema_for, JsonSchema};
use serde_derive::{Deserialize, Serialize};
use serde_json::Value;
pub use {
    chat_completion::ChatCompletion as Chat,
    chat_completion_delta::ChatCompletionDelta as ChatDelta,
    chat_completion_request::ChatCompletionRequest as ChatRequest,
    chat_completion_request::ChatCompletionRequestBuilder as ChatRequestBuilder,
};

lazy_static! {
    static ref OPENAI_API_KEY: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<FunctionCall>,
}

impl Message {
    pub fn new(role: String) -> Self {
        Self {
            role,
            content: None,
            name: None,
            function_call: None,
        }
    }

    pub fn with_content(mut self, content: String) -> Self {
        self.content = Some(content);
        self
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub parameters: Value,
}

impl Function {
    pub fn from<FunctionArgs, Func, T>(function: Func) -> Self
    where
        FunctionArgs: JsonSchema,
        Func: Fn(FunctionArgs) -> T + 'static,
    {
        let schema = schema_for!(FunctionArgs);
        let fn_type_name = type_name_of_val(&function);
        let parameters = serde_json::to_value(schema)
            .unwrap_or_else(|_| panic!("Failed to serialize schema for function {}", fn_type_name));

        let fn_name = fn_type_name.split("::").last().unwrap_or("");
        Self {
            name: fn_name.to_string(),
            description: match parameters.get("description") {
                Some(Value::String(s)) => Some(s.clone()),
                _ => None,
            },
            parameters,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub index: i64,
    pub message: Message,
    pub finish_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkChoice {
    pub index: i64,
    pub delta: Delta,

    #[serde(default)]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delta {
    pub role: Option<String>,

    pub content: Option<String>,

    pub function_call: Option<FunctionCallDelta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCallDelta {
    pub name: Option<String>,
    pub arguments: Option<Value>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
}

pub fn api_key(api_key: String) {
    let mut key = OPENAI_API_KEY.write().unwrap();
    *key = Some(api_key);
}
