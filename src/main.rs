mod buffer;
mod editor;
mod logger;

use buffer::Buffer;
use editor::Editor;
use logger::Logger;
use once_cell::sync::OnceCell;

pub(crate) static LOGGER: OnceCell<Logger> = OnceCell::new();

fn main() -> anyhow::Result<()> {
    let file = std::env::args().nth(1);
    let buffer = Buffer::from_file(file);
    // let buffer =
    let editor = Editor::new(buffer);
    editor.unwrap().run()
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        let log_message = format!($($arg)*);
        $crate::LOGGER.get_or_init(|| $crate::Logger::new("red.log")).log(&log_message);
    };
}
