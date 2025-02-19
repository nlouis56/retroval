use chrono::NaiveDateTime;
use csv::Reader;
use std::fs::File;
use std::collections::HashMap;
use serde::Deserialize;
use serde_json::{self, Map};

#[derive(Debug, Deserialize)]
pub struct RawKline {
    pub open: String,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

#[derive(Debug)]
pub struct Kline {
    pub open: NaiveDateTime,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
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

fn to_klines(raw_klines: Vec<RawKline>) -> Vec<Kline> {
    raw_klines
        .iter()
        .map(|raw_kline| Kline {
            open: NaiveDateTime::parse_from_str(&raw_kline.open, "%Y-%m-%d %H:%M:%S").unwrap(),
            high: raw_kline.high,
            low: raw_kline.low,
            close: raw_kline.close,
            volume: raw_kline.volume,
        })
        .collect()
}

pub fn read_klines(file_path: &str, headers: HashMap<String, String>) -> Result<Vec<Kline>, csv::Error> {
    let mut raw_klines = Vec::new();
    let csvfile = File::open(file_path).expect("CSV file not found");
    let mut rdr = Reader::from_reader(csvfile);

    // read the csv file and fill the klines vector based on the provided headers
    for result in rdr.deserialize() {
        let record: HashMap<String, String> = result.expect("error while parsing CSV");
        let kline = RawKline {
            open: record.get(&headers["open"]).unwrap().parse::<String>().unwrap(),
            high: record.get(&headers["high"]).unwrap().parse::<f64>().unwrap(),
            low: record.get(&headers["low"]).unwrap().parse::<f64>().unwrap(),
            close: record.get(&headers["close"]).unwrap().parse::<f64>().unwrap(),
            volume: record.get(&headers["volume"]).unwrap().parse::<f64>().unwrap(),
        };
        raw_klines.push(kline);
    }
    Ok(to_klines(raw_klines))
}

pub fn read_config(file_path: &str) -> Config {
    let json = std::fs::read_to_string(file_path).expect("file not found");
    let config: Config = serde_json::from_str(&json).expect("error while parsing JSON");
    config
}
