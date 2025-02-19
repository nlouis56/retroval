mod historical;
mod strategy;
mod testing;

fn print_metrics(metrics: &testing::Metrics, config: &historical::Config) {
    let profit_percentage = metrics.total_profit / config.base_funds * 100.0;
    let max_drawdown_percentage = metrics.max_drawdown / config.base_funds * 100.0;
    println!("Backtest results on {}:", config.pair);
    println!("Total trades: {}", metrics.total_trades);
    println!("Total profit: {:.2} {} ({:.2}%)", metrics.total_profit, config.quote_currency, profit_percentage);
    println!("Total commission: {:.2} {}", metrics.total_commission, config.quote_currency);
    println!("Win rate: {:.2}%", metrics.win_rate * 100.0);
    println!("Average profit: {:.2} {}", metrics.avg_profit, config.quote_currency);
    println!("Average loss: {:.2} {}", metrics.avg_loss, config.quote_currency);
    println!("Max drawdown: {:.2} {} ({:.2}%)", metrics.max_drawdown, config.quote_currency, max_drawdown_percentage);
    println!("Max drawdown duration: {} ({} timeframe)", metrics.max_drawdown_duration, config.timeframe);
}

fn main() {
    let config = historical::read_config("config.json");
    let klines = match historical::read_klines(&config.data_path, config.get_headers()) {
        Ok(klines) => klines,
        Err(e) => panic!("Error while reading klines: {:?}", e),

    };
    let metrics = testing::run_simulation(&config, &klines);
    print_metrics(&metrics, &config);
}
