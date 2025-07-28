use ratatui::crossterm::event;
use std::{
    io,
    io::Write,
    time::{Duration, Instant},
};

mod client;
mod common;
mod controller;
mod model;
mod view;

pub use client::*;
pub use common::*;
pub use controller::*;
pub use model::*;
pub use view::*;

fn main() {
    env_logger::Builder::from_default_env()
        .parse_filters("info")
        .init();

    log::info!("Game of Stonks");

    match main_impl() {
        Ok(()) => {
            log::info!("Terminated.");
        }
        Err(err) => {
            log::error!("Failure: {err:?}");
        }
    }

    print!("Press Enter to close...");
    io::stdout().flush().unwrap();
    let _ = io::stdin().read_line(&mut String::new());
}

fn main_impl() -> eyre::Result<()> {
    let mut launcher =
        Launcher::init((CONSOLE_WINDOW_SIZE.1 as u16, CONSOLE_WINDOW_SIZE.0 as u16))?;

    // let mut client = Client::new()?;

    // client.authenticate()?;

    // for _ in 0..3 {
    //     let order = requests::Order {
    //         symbol: Symbol(0),
    //         price: Creds(1000),
    //         quantity: 100,
    //         side: OrderSide::Buy,
    //     };

    //     log::info!("Placing order: {order:?}");

    //     client.send_player_request(PlayerRequest::CreateOrder(order))?;

    //     thread::sleep(Duration::from_secs(1));
    // }

    let mut model = GosClientModel::new();

    let mut view = GosClientView::new();

    let mut controller = GosClientController::new();

    // Set the desired tick rate
    let tick_rate = Duration::from_millis(TICK_DURATION_MS);
    let mut last_tick = Instant::now();

    while !controller.wants_to_quit() {
        // Wait for user input or until it is time for the next tick
        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            let event = event::read()?;
            controller.on_win_event(&mut model, &mut view, event);
        }

        // Update the game state based on the tick rate
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
            model.on_tick();
            controller.on_tick(&mut model, &mut view);
        }

        // Draw the current game state
        launcher
            .terminal_mut()
            .draw(|frame| view.view(&model, frame))?;
    }

    launcher.fini()?;

    Ok(())
}
