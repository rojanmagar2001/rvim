mod editor;

use editor::Editor;

fn main() -> anyhow::Result<()> {
    let editor = Editor::new();
    editor.unwrap().run()
}
