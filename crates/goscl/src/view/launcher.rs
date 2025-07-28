use eyre::Result;
use ratatui::{
    backend::CrosstermBackend,
    crossterm::{
        execute,
        terminal::{
            disable_raw_mode, enable_raw_mode, size, EnterAlternateScreen, LeaveAlternateScreen,
            SetSize, SetTitle,
        },
    },
};
use std::io::{stdout, Stdout};

type Terminal = ratatui::Terminal<CrosstermBackend<Stdout>>;

pub struct Launcher {
    original_size: (u16, u16),
    terminal: Terminal,
}

impl Launcher {
    pub fn init(size: (u16, u16)) -> Result<Self> {
        set_panic_hook();

        let original_size = resize_terminal(size)?;
        init_terminal()?;

        let backend = CrosstermBackend::new(stdout());

        let mut terminal = Terminal::new(backend)?;
        terminal.clear()?;

        Ok(Self {
            original_size,
            terminal,
        })
    }

    pub fn terminal_mut(&mut self) -> &mut Terminal {
        &mut self.terminal
    }

    pub fn fini(&self) -> Result<()> {
        try_restore_terminal()?;
        resize_terminal(self.original_size)?;
        Ok(())
    }
}

fn set_panic_hook() {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // FIXME restore console size
        restore_terminal();
        hook(info);
    }));
}

fn resize_terminal((rows, cols): (u16, u16)) -> Result<(u16, u16)> {
    let old_size = size()?;
    execute!(stdout(), SetSize(rows, cols))?;
    Ok(old_size)
}

fn init_terminal() -> Result<()> {
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    execute!(stdout(), SetTitle(format!("GAME OF STONKS")))?;
    Ok(())
}

fn restore_terminal() {
    if let Err(err) = try_restore_terminal() {
        eprintln!("Failed to restore terminal: {err}",);
    }
}

fn try_restore_terminal() -> Result<()> {
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;
    Ok(())
}
