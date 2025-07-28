/// Game version as string
pub const RELEASE_VERSION_STR: &str = "0.1";

/// Duration of one game tick
pub const TICK_DURATION_MS: u64 = 50;

/// Tick cycle length for cursor blink
pub const CURSOR_BLINK_CYCLE_LEN: u64 = 5;

/// Size of the console window in characters
pub const CONSOLE_WINDOW_SIZE: (usize, usize) = (MAIN_WIN_SHAPE.0 + 8, MAIN_WIN_SHAPE.1 + 56);

/// Size of the main window (also size of the galaxy)
pub const MAIN_WIN_SHAPE: (usize, usize) = (33, 65);
