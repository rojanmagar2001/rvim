mod buffer;
mod editor;

use buffer::Buffer;
use editor::Editor;

fn main() -> anyhow::Result<()> {
    let file = std::env::args().nth(1);
    let buffer = Buffer::from_file(file);
    // let buffer =
    let editor = Editor::new(buffer);
    editor.unwrap().run()
}
