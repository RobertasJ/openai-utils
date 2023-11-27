use crate::chat_completion_delta::forward_stream;
use crate::error::{InternalError, OpenAIError};
use crate::error::UtilsResult;
use crate::{calculate_message_tokens, DeltaReceiver};
use crate::{Chat, OPENAI_API_KEY};
use crate::{Function, Message};
use log::{error, trace};
use reqwest::Method;
use reqwest_eventsource::RequestBuilderExt;
use schemars::JsonSchema;
use serde::Deserialize;
use std::{collections::HashMap, vec};
use serde_json::to_string_pretty;
use tokio::sync::mpsc;

#[derive(Debug, Clone, serde_derive::Serialize, serde_derive::Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<Message>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub functions: Option<Vec<Function>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub logit_bias: Option<HashMap<u64, f64>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

impl ChatCompletionRequest {
    fn new() -> Self {
        Self {
            model: "gpt-3.5-turbo".to_string(),
            messages: vec![],
            functions: None,
            function_call: None,
            temperature: None,
            top_p: None,
            n: None,
            stream: None,
            stop: None,
            max_tokens: None,
            presence_penalty: None,
            frequency_penalty: None,
            logit_bias: None,
            user: None,
        }
    }
}

#[derive(Debug, Clone, serde_derive::Serialize, serde_derive::Deserialize)]
pub struct AiAgent {
    pub model: String,

    pub system_message: Option<Message>,

    pub messages: Vec<Message>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub functions: Option<Vec<Function>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub logit_bias: Option<HashMap<u64, f64>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

impl AiAgent {
    // request part
    pub fn build_request(&self, stream: bool) -> ChatCompletionRequest {
        let messages = if let Some(system_message) = &self.system_message {
            let mut messages = self.messages.clone();
            messages.push(system_message.clone());
            messages
        } else {
            self.messages.clone()
        };

        ChatCompletionRequest {
            model: self.model.clone(),
            messages,
            functions: self.functions.clone(),
            function_call: self.function_call.clone(),
            temperature: self.temperature,
            top_p: self.top_p,
            n: self.n,
            stream: Some(stream),
            stop: self.stop.clone(),
            max_tokens: self.max_tokens,
            presence_penalty: self.presence_penalty,
            frequency_penalty: self.frequency_penalty,
            logit_bias: self.logit_bias.clone(),
            user: self.user.clone(),
        }
    }

    pub async fn create(&self) -> UtilsResult<Chat> {
        let api_key = OPENAI_API_KEY.read().expect("failed to get lock").clone().ok_or_else(|| InternalError::ConfigurationError("API key not set".to_string()))?;
        
        trace!("request body: {}", to_string_pretty(&self.build_request(false)).unwrap());
        let req = reqwest::Client::new()
            .post("https://api.openai.com/v1/chat/completions")
            .json(&self.build_request(false))
            .bearer_auth(api_key)
            .header("Content-Type", "application/json")
            .send()
            .await.map_err(|e| InternalError::RequestBuildError(e))?;

        let res = req.text().await.map_err(|e| InternalError::RequestBuildError(e))?;
        serialize(&res)
    }

    pub async fn create_stream(&self) -> UtilsResult<DeltaReceiver> {
        let api_key = OPENAI_API_KEY.read()
            .expect("failed to get lock")
            .as_ref()
            .ok_or_else(|| InternalError::ConfigurationError("API key not set".to_string()))?
            .to_string();

        let (tx, rx) = mpsc::channel(64);
        trace!("request body: {}", to_string_pretty(&self.build_request(true)).unwrap());
        let es = reqwest::Client::new()
            .request(Method::POST, "https://api.openai.com/v1/chat/completions")
            .json(&self.build_request(true))
            .bearer_auth(api_key)
            .header("Content-Type", "application/json")
            .eventsource()
            .expect("cannot create eventsource? shouldn't happen i think.");

        tokio::spawn(async move {
            if let Err(e) = forward_stream(es, tx).await {
                error!("Error in forward_stream: {}", e);
            }
        });

        let usage = self.build_request(true).messages.iter().fold(3, |acc, m| {
            acc + calculate_message_tokens(m) + 4
        });

        Ok(DeltaReceiver::from(rx, self, usage))
    }


    // builder part

    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            system_message: None,
            messages: vec![],
            functions: None,
            function_call: None,
            temperature: None,
            top_p: None,
            n: None,
            stop: None,
            max_tokens: None,
            presence_penalty: None,
            frequency_penalty: None,
            logit_bias: None,
            user: None,
        }
    }

    pub fn with_system_message<'a>(mut self, system_message: impl Into<&'a str>) -> Self {
        self.system_message = Some(Message::new("system").with_content(system_message.into()));
        self
    }

    pub fn with_messages(mut self, messages: Vec<Message>) -> Self {
        self.messages = messages;
        self
    }

    pub fn with_function_call(mut self, function_call: impl Into<String>) -> Self {
        self.function_call = Some(function_call.into());
        self
    }

    pub fn with_temperature(mut self, temperature: f64) -> Self {
        self.temperature = Some(temperature);
        self
    }

    pub fn with_top_p(mut self, top_p: f64) -> Self {
        self.top_p = Some(top_p);
        self
    }

    pub fn with_n(mut self, n: u64) -> Self {
        self.n = Some(n);
        self
    }

    pub fn with_stop(mut self, stop: Vec<String>) -> Self {
        self.stop = Some(stop);
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: u64) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    pub fn with_presence_penalty(mut self, presence_penalty: f64) -> Self {
        self.presence_penalty = Some(presence_penalty);
        self
    }

    pub fn with_frequency_penalty(mut self, frequency_penalty: f64) -> Self {
        self.frequency_penalty = Some(frequency_penalty);
        self
    }

    pub fn with_logit_bias(mut self, logit_bias: HashMap<u64, f64>) -> Self {
        self.logit_bias = Some(logit_bias);
        self
    }

    pub fn with_user(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }

    // mutably update part

    pub fn push_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    pub fn push_function<FunctionArgs, Func, T>(&mut self, function: &Func, function_name: &str)
    where
        FunctionArgs: JsonSchema,
        Func: FnMut(FunctionArgs) -> T,
    {
        if let Some(functions) = &mut self.functions {
            functions.push(Function::from(function, function_name));
        } else {
            self.functions = Some(vec![Function::from(function, function_name)]);
        }
    }

    pub fn push_stop(&mut self, stop: impl Into<String>) {
        if let Some(stops) = &mut self.stop {
            stops.push(stop.into());
        } else {
            self.stop = Some(vec![stop.into()]);
        }
    }

    pub fn push_logit_bias(&mut self, logit_bias: (u64, f64)) {
        if let Some(logit_biases) = &mut self.logit_bias {
            logit_biases.insert(logit_bias.0, logit_bias.1);
        } else {
            let mut logit_biases = HashMap::new();
            logit_biases.insert(logit_bias.0, logit_bias.1);
            self.logit_bias = Some(logit_biases);
        }
    }
}

pub fn serialize<'a, T: Deserialize<'a>>(res: &'a str) -> UtilsResult<T> {
    match serde_json::from_str::<T>(res) {
        Ok(chat) => Ok(chat),
        Err(_) => {
            #[derive(Deserialize)]
            struct TempWrapper {
                error: OpenAIError
            }

            let err =
                serde_json::from_str::<TempWrapper>(res).unwrap_or_else(|_| panic!("{}", res));
            Err(err.error.into())
        }
    }
}