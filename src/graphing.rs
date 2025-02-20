use chrono::NaiveDateTime;
use plotters::prelude::*;
use plotters::style::full_palette as palette;
use plotters::coord::{Shift, types::{RangedCoordi64, RangedCoordf64}};

use crate::historical::Kline;
use crate::config::{self, Config};
use crate::testing::{SessionRecap, Trade};

fn get_timestamp_range(klines: &Vec<Kline>) -> (i64, i64) {
    let x_min = klines.iter().map(|k| k.timestamp).min().unwrap();
    let x_max = klines.iter().map(|k| k.timestamp).max().unwrap();
    let min_timestamp = x_min.and_utc().timestamp();
    let max_timestamp = x_max.and_utc().timestamp();
    (min_timestamp, max_timestamp)
}

fn get_price_range(klines: &Vec<Kline>) -> (f64, f64) {
    let y_min = klines.iter().map(|k| k.low).min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
    let y_max = klines.iter().map(|k| k.high).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
    let min_price = y_min - 0.1 * y_min;
    let max_price = y_max + 0.1 * y_max;
    (min_price, max_price)
}

fn draw_trade_lines(trades: &Vec<Trade>, chart: &mut ChartContext<BitMapBackend, Cartesian2d<RangedCoordi64, RangedCoordf64>>, min_price: f64, max_price: f64) -> Result<(), Box<dyn std::error::Error>> {
    let entry_line_style = ShapeStyle {
        color: palette::BLUE.to_rgba(),
        filled: false,
        stroke_width: 1,
    };
    let exit_line_style = ShapeStyle {
        color: palette::PINK.to_rgba(),
        filled: false,
        stroke_width: 1,
    };
    // draw lines for trades here
    for trade in trades {
        let entry_ts = trade.entry_date.and_utc().timestamp();
        // Draw a vertical line for the trade entry (using a semi-transparent green)
        chart.draw_series(std::iter::once(PathElement::new(
            vec![(entry_ts, min_price), (entry_ts, max_price)],
            entry_line_style,
        )))?;

        if let Some(exit_date) = trade.exit_date {
            let exit_ts = exit_date.and_utc().timestamp();
            // Draw a vertical line for the trade exit (using a semi-transparent red)
            chart.draw_series(std::iter::once(PathElement::new(
                vec![(exit_ts, min_price), (exit_ts, max_price)],
                exit_line_style,
            )))?;
        }
    }
    Ok(())
}

fn make_equity_chart(chart_element: &DrawingArea<BitMapBackend<'_>, Shift>, equity_curve: &Vec<(NaiveDateTime, f64)>, min_equity: f64, max_equity: f64) -> Result<(), Box<dyn std::error::Error>> {
    let mut equity_chart = ChartBuilder::on(chart_element)
        .x_label_area_size(40)
        .y_label_area_size(40)
        .caption("Equity curve", ("sans-serif", 15.0).into_font())
        .build_cartesian_2d(0..equity_curve.len() as i64, min_equity..max_equity)?;

    equity_chart.configure_mesh().disable_x_mesh().draw()?;
    equity_chart.draw_series(LineSeries::new(
        equity_curve.iter().enumerate().map(|(i, (_, equity))| (i as i64, *equity)),
        BLUE,
    ))?;
    Ok(())
}

pub fn plot_graph(config: &Config, klines: &Vec<Kline>, recap: SessionRecap) -> Result<(), Box<dyn std::error::Error>> {
    let (min_timestamp, max_timestamp) = get_timestamp_range(klines);
    let (min_price, max_price) = get_price_range(klines);

    let pair = &config.pair;
    let title = format!("Backtesting results on {}", pair);

    let candle_px_width = 5;
    let chart_width = candle_px_width * klines.len() as u32;

    let root_area = BitMapBackend::new(&config.log_graph_file, (chart_width, 768)).into_drawing_area();
    root_area.fill(&WHITE)?;
    let root_area = root_area.titled(&title, ("sans-serif", 60))?;

    let (main, bottom_elem) = root_area.split_vertically(512);

    let mut cstick_chart = ChartBuilder::on(&main)
        .x_label_area_size(40)
        .y_label_area_size(40)
        .caption("Candlestick data", ("sans-serif", 15.0).into_font())
        .build_cartesian_2d(min_timestamp..max_timestamp, min_price..max_price)?;

    cstick_chart.configure_mesh().light_line_style(palette::GREY_A200).disable_x_mesh().draw()?;

    cstick_chart.draw_series(
        klines.iter().map(|candle| {
            CandleStick::new(candle.timestamp.and_utc().timestamp(), candle.open, candle.high, candle.low, candle.close, GREEN.filled(), RED, 5)
        }),
    )?;
    draw_trade_lines(&recap.trades, &mut cstick_chart, min_price, max_price)?;

    let curve = &recap.equity_curve;
    let (min_equity, max_equity) = curve.iter().fold((f64::MAX, f64::MIN), |(min, max), (_, equity)| {
        (min.min(*equity), max.max(*equity))
    });
    let min_equity = min_equity - 0.1 * min_equity;
    let max_equity = max_equity + 0.1 * max_equity;
    make_equity_chart(&bottom_elem, curve, min_equity, max_equity)?;

    match config.log_level {
        config::LogLevel::None => {}
        _ => {
            println!("Graph saved to {}", config.log_graph_file);
        }
    }

    Ok(())
}
