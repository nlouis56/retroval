use std::{fs::OpenOptions, io::{BufWriter, Write}};
use chrono::NaiveDateTime;
use crate::{config, historical};
use crate::strategy::{Strategy, Signal, SimpleStrategy};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Direction {
    Long,
    Short,
    Flat,
}

impl std::fmt::Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Direction::Long => write!(f, "Long"),
            Direction::Short => write!(f, "Short"),
            Direction::Flat => write!(f, "Flat"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Trade {
    pub entry_date: NaiveDateTime,
    pub exit_date: Option<NaiveDateTime>,
    pub entry_price: f64,
    pub exit_price: Option<f64>,
    pub direction: Direction,
    pub allocated: f64,
    pub profit: Option<f64>,
    pub commission: f64,
}

pub struct SessionRecap {
    pub trades: Vec<Trade>,
    pub equity_curve: Vec<(NaiveDateTime, f64)>,
    pub metrics: Metrics,
}

impl SessionRecap {
    pub fn new(trades: Vec<Trade>, equity_curve: Vec<(NaiveDateTime, f64)>, metrics: Metrics) -> Self {
        Self {
            trades,
            equity_curve,
            metrics,
        }
    }
}

struct Portfolio<'a> {
    cash: f64,
    open_trade: Option<Trade>,
    closed_trades: Vec<Trade>,
    equity_curve: Vec<(NaiveDateTime, f64)>,
    commission_rate: f64,
    slippage: f64,
    trade_fraction: f64,
    log_buffer: Vec<String>,
    log_buffer_size: usize,
    config: &'a config::Config,
}

impl<'a> Portfolio<'a> {
    fn new(initial_equity: f64, commission_rate: f64, slippage: f64, trade_fraction: f64, config: &'a config::Config) -> Self {
        Self {
            cash: initial_equity,
            open_trade: None,
            closed_trades: Vec::new(),
            equity_curve: Vec::new(),
            commission_rate,
            slippage,
            trade_fraction,
            log_buffer: Vec::new(),
            log_buffer_size: 10,
            config,
        }
    }

    fn flush_log_buffer(&mut self) {
        if self.log_buffer.len() < self.log_buffer_size {
            return;
        }
        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.config.log_file)
            .unwrap();
        let mut log_writer = BufWriter::new(log_file);
        for log in self.log_buffer.iter() {
            log_writer.write_all(log.as_bytes()).unwrap();
            log_writer.write_all(b"\n").unwrap();
        }
        self.log_buffer.clear();
    }

    fn total_equity(&self, current_price: f64) -> f64 {
        if let Some(trade) = &self.open_trade {
            let trade_value = if trade.direction == Direction::Long {
                // Long: value scales with price / entry_price.
                trade.allocated * (current_price / trade.entry_price)
            } else {
                // Short: profit when price falls.
                trade.allocated * (trade.entry_price / current_price)
            };
            self.cash + trade_value
        } else {
            self.cash
        }
    }

    fn update(&mut self, date: NaiveDateTime, price: f64) {
        let equity = self.total_equity(price);
        self.equity_curve.push((date, equity));
    }

    pub fn enter_trade(&mut self, date: NaiveDateTime, price: f64, direction: Direction, log_level: &config::LogLevel) {
        if !self.open_trade.is_none() {
            match log_level {
                config::LogLevel::None => {}
                _ => {
                    self.log_buffer.push(format!("{}: Trade already open, cannot enter another trade.", date));
                    self.flush_log_buffer();
                }
            }
            return;
        }
        let allocated = self.cash * self.trade_fraction;
        if allocated <= 0.0 {
            match log_level {
                config::LogLevel::None => {}
                _ => {
                    self.log_buffer.push(format!("{}: Not enough cash to enter trade.", date));
                    self.flush_log_buffer();
                }
            }
            return;
        }
        let effective_entry_price = if direction == Direction::Long {
            price * (1.0 + self.slippage)
        } else { // Short
            price * (1.0 - self.slippage)
        };
        let purchased_amount = allocated / effective_entry_price;
        let entry_commission = (self.commission_rate * allocated) / 100.0;
        self.cash -= allocated;
        let trade = Trade {
            entry_date: date,
            exit_date: None,
            entry_price: effective_entry_price,
            exit_price: None,
            direction,
            allocated,
            profit: None,
            commission: entry_commission,
        };

        match log_level {
            config::LogLevel::All => {
                self.log_buffer.push(format!(
                    "{}: Entering {} trade at effective price {:.2}. Entry commission is {:.2}. Allocated: {:.2} {} ({:.4} {}), {:.2} {} remaining)",
                    date,
                    direction,
                    effective_entry_price,
                    entry_commission,
                    allocated,
                    self.config.quote_currency,
                    purchased_amount,
                    self.config.base_currency,
                    self.cash,
                    self.config.quote_currency
                ));
                self.flush_log_buffer();
            }
            _ => {}
        }

        self.open_trade = Some(trade);
    }

    fn exit_trade(&mut self, date: NaiveDateTime, price: f64, log_level: &config::LogLevel) {
        let mut trade = match self.open_trade.take() {
            Some(trade) => trade,
            None => {
                match log_level {
                    config::LogLevel::None => {}
                    _ => {
                        self.log_buffer.push(format!("{}: No trade to exit.", date));
                        self.flush_log_buffer();
                    }
                }
                return;
            }
        };
        let effective_exit_price = if trade.direction == Direction::Long {
            price * (1.0 - self.slippage)
        } else {
            price * (1.0 + self.slippage)
        };
        let exit_commission = (self.commission_rate * trade.allocated) / 100.0;
        let raw_profit = if trade.direction == Direction::Long {
            trade.allocated * ((effective_exit_price - trade.entry_price) / trade.entry_price)
        } else {
            trade.allocated * ((trade.entry_price - effective_exit_price) / trade.entry_price)
        };
        trade.commission += exit_commission;
        let net_profit = raw_profit - trade.commission;
        trade.exit_date = Some(date);
        trade.exit_price = Some(effective_exit_price);
        trade.profit = Some(net_profit);
        let final_trade_value = trade.allocated + net_profit;
        self.cash += final_trade_value;

        match log_level {
            config::LogLevel::All => {
                self.log_buffer.push(format!(
                    "{}: Exiting trade at effective price {:.2}, net profit: {:.2}. Total broker commission is {:.2} {} Now holding {:.2} {}.",
                    date,
                    effective_exit_price,
                    net_profit,
                    trade.commission,
                    self.config.quote_currency,
                    self.cash,
                    self.config.quote_currency
                ));
                self.flush_log_buffer();
            }
            _ => {}
        }

        self.closed_trades.push(trade);
    }
}

pub struct Metrics {
    pub total_trades: usize,
    pub total_profit: f64,
    pub total_commission: f64,
    pub win_rate: f64,
    pub avg_profit: f64,
    pub avg_loss: f64,
    pub max_drawdown: f64,
    pub max_drawdown_duration: usize,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            total_trades: 0,
            total_profit: 0.0,
            total_commission: 0.0,
            win_rate: 0.0,
            avg_profit: 0.0,
            avg_loss: 0.0,
            max_drawdown: 0.0,
            max_drawdown_duration: 0,
        }
    }

    pub fn compute(&mut self, trade_list: &Vec<Trade>) {
        let mut total_profit = 0.0;
        let mut total_commission = 0.0;
        let mut total_wins = 0;
        let mut total_losses = 0;
        let mut max_drawdown = 0.0;
        let mut max_drawdown_duration = 0;
        let mut current_drawdown = 0.0;
        let mut current_drawdown_duration = 0;

        for trade in trade_list.iter() {
            total_profit += trade.profit.unwrap();
            total_commission += trade.commission;
            if trade.profit.unwrap() > 0.0 {
                total_wins += 1;
            } else {
                total_losses += 1;
            }
            if trade.profit.unwrap() < 0.0 {
                current_drawdown += trade.profit.unwrap();
                current_drawdown_duration += 1;
            } else {
                if current_drawdown < max_drawdown {
                    max_drawdown = current_drawdown;
                    max_drawdown_duration = current_drawdown_duration;
                }
                current_drawdown = 0.0;
                current_drawdown_duration = 0;
            }
        }

        let total_trades = trade_list.len();
        let win_rate = if total_trades > 0 {
            total_wins as f64 / total_trades as f64
        } else {
            0.0
        };
        let avg_profit = if total_wins > 0 {
            total_profit / total_wins as f64
        } else {
            0.0
        };
        let avg_loss = if total_losses > 0 {
            total_profit / total_losses as f64
        } else {
            0.0
        };
        self.total_trades = total_trades;
        self.total_profit = total_profit;
        self.total_commission = total_commission;
        self.win_rate = win_rate;
        self.avg_profit = avg_profit;
        self.avg_loss = avg_loss;
        self.max_drawdown = max_drawdown;
        self.max_drawdown_duration = max_drawdown_duration;
    }
}

pub fn run_simulation(config: &config::Config, klines: &Vec<historical::Kline>) -> SessionRecap {
    let mut portfolio = Portfolio::new(
        config.base_funds,
        config.transaction_fee,
        config.slippage,
        0.1,
        config
    );
    let mut strategy = SimpleStrategy::new(14);
    for kline in klines.iter() {
        let signal = strategy.on_tick(kline);
        match signal {
            Some(Signal::Buy) => {
                portfolio.enter_trade(kline.timestamp, kline.close, Direction::Long, &config.log_level);
            }
            Some(Signal::Sell) => {
                portfolio.exit_trade(kline.timestamp, kline.close, &config.log_level);
            }
            Some(Signal::Hold) => {}
            None => { continue; }
        }
        portfolio.update(kline.timestamp, kline.close);
    }
    if portfolio.open_trade.is_some() {
        portfolio.exit_trade(klines.last().unwrap().timestamp, klines.last().unwrap().close, &config.log_level);
    }
    let trade_list = portfolio.closed_trades.clone();
    let equity_curve = portfolio.equity_curve.clone();
    let mut metrics = Metrics::new();
    metrics.compute(&trade_list);
    SessionRecap::new(trade_list, equity_curve, metrics)
}
