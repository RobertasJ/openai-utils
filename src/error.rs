use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("no client found")]
    Client,
    #[error("no model found")]
    Model,
}