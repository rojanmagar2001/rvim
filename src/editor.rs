use std::io::{stdout, Write};

use anyhow::Result;
use crossterm::{
    cursor,
    event::{self, read},
    style::{self, Color, Stylize},
    terminal, ExecutableCommand, QueueableCommand,
};

pub enum Action {
    Quit,
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    EnterMode(Mode),
    AddChar(char),
    NewLine,
}

#[derive(Debug)]
pub enum Mode {
    Normal,
    Insert,
}

pub struct Editor {
    cx: u16,
    cy: u16,
    mode: Mode,
    size: (u16, u16),
    stdout: std::io::Stdout,
}

impl Drop for Editor {
    fn drop(&mut self) {
        _ = self.stdout.flush();
        _ = self.stdout.execute(terminal::LeaveAlternateScreen); // Switch back to main screen
        _ = terminal::disable_raw_mode();
    }
}

impl Editor {
    pub fn new() -> anyhow::Result<Self> {
        let mut stdout = stdout();
        terminal::enable_raw_mode()?;
        stdout
            .execute(terminal::EnterAlternateScreen)? // Switch to alternate screen
            .execute(terminal::Clear(terminal::ClearType::All))?; // Clear the screen
        Ok(Self {
            cx: 0,
            cy: 0,
            mode: Mode::Normal,
            size: terminal::size().unwrap(),
            stdout,
        })
    }

    pub fn draw(&mut self) -> anyhow::Result<()> {
        self.draw_statusline()?;
        self.stdout.queue(cursor::MoveTo(self.cx, self.cy))?; // Move cursor to (cx, cy)
        self.stdout.flush()?;

        Ok(())
    }

    pub fn draw_statusline(&mut self) -> anyhow::Result<()> {
        let mode = format!(" {:?}  ", self.mode).to_uppercase();
        let file = " src/main.rs";
        let pos = format!(" {}:{} ", self.cx, self.cy);

        let file_width = self.size.0 - mode.len() as u16 - pos.len() as u16 - 2;

        self.stdout.queue(cursor::MoveTo(0, self.size.1 - 2))?;
        self.stdout.queue(style::PrintStyledContent(
            mode.with(Color::Rgb { r: 0, g: 0, b: 0 }).on(Color::Rgb {
                r: 184,
                g: 144,
                b: 243,
            }),
        ))?;
        self.stdout
            .queue(style::PrintStyledContent("".bold().with(Color::Rgb {
                r: 184,
                g: 144,
                b: 243,
            })))?;
        self.stdout.queue(style::PrintStyledContent(
            format!("{:<width$}", file, width = file_width as usize)
                .with(Color::Rgb {
                    r: 255,
                    g: 255,
                    b: 255,
                })
                .bold()
                .on(Color::Rgb {
                    r: 60,
                    g: 70,
                    b: 89,
                }),
        ))?;

        self.stdout
            .queue(style::PrintStyledContent("".on(Color::Rgb {
                r: 184,
                g: 144,
                b: 243,
            })))?;

        self.stdout.queue(style::PrintStyledContent(
            pos.with(Color::Rgb { r: 0, g: 0, b: 0 })
                .bold()
                .on(Color::Rgb {
                    r: 184,
                    g: 144,
                    b: 243,
                }),
        ))?;

        self.stdout.flush()?;

        Ok(())
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        loop {
            self.draw()?;
            self.stdout.queue(cursor::MoveTo(self.cx, self.cy))?;
            self.stdout.flush()?;

            if let Some(action) = self.handle_event(read()?)? {
                match action {
                    Action::Quit => break,
                    Action::MoveUp => {
                        self.cy = self.cy.saturating_sub(1);
                    }
                    Action::MoveDown => {
                        self.cy = self.cy.saturating_add(1);
                    }
                    Action::MoveLeft => {
                        self.cx = self.cx.saturating_sub(1);
                    }
                    Action::MoveRight => {
                        self.cx = self.cx.saturating_add(1);
                    }
                    Action::EnterMode(new_mode) => {
                        self.mode = new_mode;
                    }
                    Action::AddChar(c) => {
                        self.stdout.queue(cursor::MoveTo(self.cx, self.cy))?;
                        self.stdout.queue(style::Print(c))?;
                        self.cx += 1;
                    }
                    Action::NewLine => {
                        self.cx = 0;
                        self.cy += 1;
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }

    fn handle_event(&mut self, ev: event::Event) -> Result<Option<Action>> {
        if matches!(ev, event::Event::Resize(_, _)) {
            self.size = terminal::size()?;
        }
        match self.mode {
            Mode::Normal => self.handle_normal_event(ev),
            Mode::Insert => self.handle_insert_event(ev),
        }
    }

    fn handle_normal_event(&self, ev: event::Event) -> Result<Option<Action>> {
        match ev {
            event::Event::Key(event) => match event.code {
                event::KeyCode::Char('q') => Ok(Some(Action::Quit)),
                event::KeyCode::Left | event::KeyCode::Char('h') => Ok(Some(Action::MoveLeft)),
                event::KeyCode::Down | event::KeyCode::Char('j') => Ok(Some(Action::MoveDown)),
                event::KeyCode::Up | event::KeyCode::Char('k') => Ok(Some(Action::MoveUp)),
                event::KeyCode::Right | event::KeyCode::Char('l') => Ok(Some(Action::MoveRight)),
                event::KeyCode::Char('i') => Ok(Some(Action::EnterMode(Mode::Insert))),
                _ => Ok(None),
            },
            _ => Ok(None),
        }
    }

    fn handle_insert_event(&mut self, ev: event::Event) -> Result<Option<Action>> {
        match ev {
            event::Event::Key(event) => match event.code {
                event::KeyCode::Esc => Ok(Some(Action::EnterMode(Mode::Normal))),
                event::KeyCode::Char(c) => Ok(Some(Action::AddChar(c))),
                event::KeyCode::Enter => Ok(Some(Action::NewLine)),
                _ => Ok(None),
            },
            _ => Ok(None),
        }
    }
}
