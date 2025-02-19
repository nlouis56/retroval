use crate::historical::Kline;
use crate::testing::Direction;

pub enum Signal {
    Buy,
    Sell,
    Hold,
}

pub trait Strategy {
    fn on_tick(&mut self, kline: &Kline) -> Option<Signal>;
}

pub struct SimpleStrategy {
    position: Direction,
    sma_window: usize,
    prices: Vec<f64>,
}

impl SimpleStrategy {
    pub fn new(sma_window: usize) -> Self {
        Self {
            position: Direction::Flat,
            sma_window,
            prices: Vec::new(),
        }
    }

    /// Calculate the simple moving average (SMA) for the most recent `sma_window` prices.
    fn calculate_sma(&self) -> Option<f64> {
        if self.prices.len() < self.sma_window {
            None
        } else {
            let sum: f64 = self.prices[self.prices.len() - self.sma_window..]
                .iter()
                .sum();
            Some(sum / self.sma_window as f64)
        }
    }
}

impl Strategy for SimpleStrategy {
    fn on_tick(&mut self, kline: &Kline) -> Option<Signal> {
        self.prices.push(kline.close);
        if let Some(sma) = self.calculate_sma() {
            // If price is above the SMA and we're not already long, signal a Buy.
            if kline.close > sma && self.position != Direction::Long {
                self.position = Direction::Long;
                return Some(Signal::Buy);
            }
            // If price is below the SMA and we're not already short, signal a Sell.
            else if kline.close < sma && self.position != Direction::Short {
                self.position = Direction::Short;
                return Some(Signal::Sell);
            }
        }
        return Some(Signal::Hold);
    }
}
