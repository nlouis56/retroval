use plotters::{prelude::*, style::full_palette::GREY_A200};

use crate::historical::Kline;
use crate::config::Config;
use crate::testing::SessionRecap;

pub fn plot_graph(config: &Config, klines: &Vec<Kline>, recap: SessionRecap) -> Result<(), Box<dyn std::error::Error>> {
    let x_min = klines.iter().map(|k| k.timestamp).min().unwrap();
    let x_max = klines.iter().map(|k| k.timestamp).max().unwrap();
    let min_timestamp = x_min.and_utc().timestamp();
    let max_timestamp = x_max.and_utc().timestamp();

    let y_min = klines.iter().map(|k| k.low).min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
    let y_max = klines.iter().map(|k| k.high).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
    let min_price = y_min - 0.1 * y_min;
    let max_price = y_max + 0.1 * y_max;

    let pair = &config.pair;
    let title = format!("Backtesting results on {}", pair);

    println!("min_timestamp: {}, max_timestamp: {}", min_timestamp, max_timestamp);
    println!("min_price: {}, max_price: {}", min_price, max_price);

    let root_area = BitMapBackend::new(&config.log_graph_file, (1024, 768)).into_drawing_area();
    root_area.fill(&WHITE)?;
    let root_area = root_area.titled(&title, ("sans-serif", 60))?;

    let (main, equality_curve) = root_area.split_vertically(512);

    let mut chart = ChartBuilder::on(&main)
        .x_label_area_size(40)
        .y_label_area_size(40)
        .caption("Candlestick data", ("sans-serif", 15.0).into_font())
        .build_cartesian_2d(min_timestamp..max_timestamp, min_price..max_price)?;

    chart.configure_mesh().light_line_style(GREY_A200).draw()?;

    chart.draw_series(
        klines.iter().map(|candle| {
            CandleStick::new(candle.timestamp.and_utc().timestamp(), candle.open, candle.high, candle.low, candle.close, GREEN.filled(), RED, 5)
        }),
    )?;

    // Draw trades
    let trades = &recap.trades;
    chart.draw_series(
        trades.iter().map(|trade| {
            let profit = match trade.profit {
                Some(profit) => if profit > 0.0 { true } else { false },
                None => false,
            };
            let color = if profit { GREEN } else { RED };
            let x = trade.entry_date.and_utc().timestamp();
            let y = trade.entry_price;
            return Circle::new((x, y), 5, color.filled());
        }),
    )?;

    // draw equity curve
    let equity_curve = &recap.equity_curve;
    let (min_equity, max_equity) = equity_curve.iter().fold((f64::MAX, f64::MIN), |(min, max), (_, equity)| {
        (min.min(*equity), max.max(*equity))
    });
    let min_equity = min_equity - 0.1 * min_equity;
    let max_equity = max_equity + 0.1 * max_equity;
    let mut equity_chart = ChartBuilder::on(&equality_curve)
        .x_label_area_size(40)
        .y_label_area_size(40)
        .caption("Equity curve", ("sans-serif", 15.0).into_font())
        .build_cartesian_2d(min_timestamp..max_timestamp, min_equity..max_equity)?;
    equity_chart.draw_series(LineSeries::new(
        equity_curve.iter().map(|(timestamp, equity)| (timestamp.and_utc().timestamp(), *equity)),
        BLUE,
    ))?;

    println!("Result has been saved to {}", config.log_graph_file);

    Ok(())
}
