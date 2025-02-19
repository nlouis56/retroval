use std::collections::HashMap;
use serde_json::{self, Map};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub enum LogLevel {
    All,
    Info,
    None,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub data_path: String,
    pub headers: Map<String, serde_json::Value>,
    pub base_funds: f64,
    pub transaction_fee: f64,
    pub slippage: f64,
    pub pair: String,
    pub timeframe: String,
    pub base_currency: String,
    pub quote_currency: String,
    pub log_level: LogLevel,
    pub log_file: String,
    pub log_graph: bool,
    pub log_graph_file: String,
}

impl Config {
    pub fn get_headers(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        for (header, value) in &self.headers {
            headers.insert(header.to_string(), value.as_str().unwrap().to_string());
        }
        headers
    }
}

pub fn read_config(file_path: &str) -> Config {
    let json = std::fs::read_to_string(file_path).expect("file not found");
    let config: Config = serde_json::from_str(&json).expect("error while parsing JSON");
    config
}
