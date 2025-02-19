use chrono::NaiveDateTime;
use csv::Reader;
use std::fs::File;
use std::collections::HashMap;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RawKline {
    pub timestamp: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

#[derive(Debug)]
pub struct Kline {
    pub timestamp: NaiveDateTime,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

fn to_klines(raw_klines: Vec<RawKline>) -> Vec<Kline> {
    raw_klines
        .iter()
        .map(|raw_kline| Kline {
            timestamp: NaiveDateTime::parse_from_str(&raw_kline.timestamp, "%Y-%m-%d %H:%M:%S").unwrap(),
            open: raw_kline.open,
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
            timestamp: record.get(&headers["timestamp"]).unwrap().parse::<String>().unwrap(),
            open: record.get(&headers["open"]).unwrap().parse::<f64>().unwrap(),
            high: record.get(&headers["high"]).unwrap().parse::<f64>().unwrap(),
            low: record.get(&headers["low"]).unwrap().parse::<f64>().unwrap(),
            close: record.get(&headers["close"]).unwrap().parse::<f64>().unwrap(),
            volume: record.get(&headers["volume"]).unwrap().parse::<f64>().unwrap(),
        };
        raw_klines.push(kline);
    }
    Ok(to_klines(raw_klines))
}
