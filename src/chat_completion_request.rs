use crate::chat_completion_delta::forward_stream;
use crate::DeltaReceiver;
use crate::error::ApiErrorWrapper;
use crate::{Chat, OPENAI_API_KEY};
use crate::{Function, Message};
use log::debug;
use reqwest::Method;
use reqwest_eventsource::RequestBuilderExt;
use schemars::JsonSchema;
use serde::Deserialize;
use std::{collections::HashMap, vec};
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
    pub n: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub logit_bias: Option<HashMap<String, f64>>,

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
pub struct ChatCompletionRequestBuilder {
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
    pub n: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub logit_bias: Option<HashMap<String, f64>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

impl ChatCompletionRequestBuilder {
    // request part
    pub fn build_request(&self, stream: bool) -> ChatCompletionRequest {
        let messages = if let Some(system_message) = &self.system_message {
            let mut messages = self.messages.clone();
            messages.insert(0, system_message.clone());
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

    #[allow(clippy::await_holding_lock)]
    pub async fn create(&self) -> anyhow::Result<Chat> {
        let api_key = OPENAI_API_KEY.read().unwrap();
        let req = reqwest::Client::new()
            .post("https://api.openai.com/v1/chat/completions")
            .json(&self.build_request(false))
            .bearer_auth((*api_key).clone().unwrap())
            .header("Content-Type", "application/json")
            .send()
            .await?;

        let res = req.text().await?;

        serialize(&res)
    }

    #[allow(clippy::await_holding_lock)]
    pub async fn create_stream(&self) -> anyhow::Result<DeltaReceiver> {
        let lock = OPENAI_API_KEY.read().unwrap();
        let api_key = (*lock).clone().unwrap();

        let (tx, rx) = mpsc::channel(64);

        let es = reqwest::Client::new()
            .request(Method::POST, "https://api.openai.com/v1/chat/completions")
            .json(&self.build_request(true))
            .bearer_auth(api_key)
            .eventsource()?;
        tokio::spawn(forward_stream(es, tx));
        
        
        Ok(DeltaReceiver::from(rx, self))
    }

    // builder part

    pub fn new(model: &str) -> Self {
        Self {
            model: model.to_string(),
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

    pub fn with_system_message(mut self, system_message: &str) -> Self {
        self.system_message =
            Some(Message::new("system").with_content(system_message));
        self
    }

    pub fn with_messages(mut self, messages: Vec<Message>) -> Self {
        self.messages = messages;
        self
    }

    pub fn with_functions<FunctionArgs, Func>(mut self, functions: Vec<Func>) -> Self
    where
        FunctionArgs: JsonSchema,
        Func: Fn(FunctionArgs) + 'static,
    {
        let functions = functions.into_iter().map(|f| Function::from(f)).collect();
        self.functions = Some(functions);
        self
    }

    pub fn with_function_call(mut self, function_call: String) -> Self {
        self.function_call = Some(function_call);
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

    pub fn with_n(mut self, n: i64) -> Self {
        self.n = Some(n);
        self
    }

    pub fn with_stop(mut self, stop: Vec<String>) -> Self {
        self.stop = Some(stop);
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: i64) -> Self {
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

    pub fn with_logit_bias(mut self, logit_bias: HashMap<String, f64>) -> Self {
        self.logit_bias = Some(logit_bias);
        self
    }

    pub fn with_user(mut self, user: String) -> Self {
        self.user = Some(user);
        self
    }

    // mutably update part

    pub fn push_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    pub fn push_function<FunctionArgs, Func>(&mut self, function: Func)
    where
        FunctionArgs: JsonSchema,
        Func: Fn(FunctionArgs) + 'static,
    {
        if let Some(functions) = &mut self.functions {
            functions.push(Function::from(function));
        } else {
            self.functions = Some(vec![Function::from(function)]);
        }
    }

    pub fn push_stop(&mut self, stop: String) {
        if let Some(stops) = &mut self.stop {
            stops.push(stop);
        } else {
            self.stop = Some(vec![stop]);
        }
    }

    pub fn push_logit_bias(&mut self, logit_bias: (String, f64)) {
        if let Some(logit_biases) = &mut self.logit_bias {
            logit_biases.insert(logit_bias.0, logit_bias.1);
        } else {
            let mut logit_biases = HashMap::new();
            logit_biases.insert(logit_bias.0, logit_bias.1);
            self.logit_bias = Some(logit_biases);
        }
    }
}

pub fn serialize<'a, T: Deserialize<'a>>(res: &'a str) -> anyhow::Result<T> {
    debug!("response: {}", res);
    match serde_json::from_str::<T>(res) {
        Ok(chat) => Ok(chat),
        Err(_) => {
            let err =
                serde_json::from_str::<ApiErrorWrapper>(res).unwrap_or_else(|_| panic!("{}", res));
            Err(err.error.into())
        }
    }
}


