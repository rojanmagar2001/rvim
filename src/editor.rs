#[allow(dead_code, unreachable_patterns)]
use std::io::{stdout, Write};

use anyhow::Result;
use crossterm::{
    cursor,
    event::{self, read},
    style::{self, Color, Stylize},
    terminal, ExecutableCommand, QueueableCommand,
};

use crate::{buffer::Buffer, log};

pub enum Action {
    Quit,
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    PageDown,
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
    buffer: Buffer,
    vtop: u16,
    vleft: u16,
    cx: u16,
    cy: u16,
    mode: Mode,
    size: (u16, u16),
    stdout: std::io::Stdout,
}

impl Editor {
    pub fn new(buffer: Buffer) -> anyhow::Result<Self> {
        let mut stdout = stdout();
        terminal::enable_raw_mode()?;
        stdout
            .execute(terminal::EnterAlternateScreen)? // Switch to alternate screen
            .execute(terminal::Clear(terminal::ClearType::All))?; // Clear the screen

        let size = terminal::size()?;

        Ok(Self {
            buffer,
            cx: 0,
            cy: 0,
            vtop: 0,
            vleft: 0,
            mode: Mode::Normal,
            size,
            stdout,
        })
    }

    pub fn draw(&mut self) -> anyhow::Result<()> {
        self.draw_viewport()?;
        self.draw_statusline()?;
        self.stdout.queue(cursor::MoveTo(self.cx, self.cy))?; // Move cursor to (cx, cy)
        self.stdout.flush()?;

        Ok(())
    }

    fn vwidth(&self) -> u16 {
        self.size.0
    }

    fn vheight(&self) -> u16 {
        self.size.1 - 2
    }

    fn line_length(&self) -> u16 {
        log!("cx: {} -> {:?}", self.cx, self.vtop);
        if let Some(line) = self.viewport_line(self.cy) {
            line.len() as u16
        } else {
            0
        }
    }

    fn viewport_line(&self, n: u16) -> Option<String> {
        let buffer_line = self.vtop + n;
        self.buffer.get_line(buffer_line as usize)
    }

    pub fn draw_viewport(&mut self) -> anyhow::Result<()> {
        let vwidth = self.vwidth() as usize;
        for i in 0..self.vheight() {
            self.stdout.queue(cursor::MoveTo(0, i))?;
            let line = match self.viewport_line(i) {
                None => String::new(),
                Some(s) => s,
            };

            self.stdout
                .queue(cursor::MoveTo(0, i))?
                .queue(style::Print(format!("{line:<width$}", width = vwidth,)))?;
        }

        Ok(())
    }

    pub fn draw_statusline(&mut self) -> anyhow::Result<()> {
        let mode = format!(" {:?}  ", self.mode).to_uppercase();
        let file = format!(" {}", self.buffer.file.as_deref().unwrap_or("Untitled"));
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

    // TODO: in neovim, when you are at an x position and you move to a shorter line, the cursor
    // TODO: goes to the max x but returns to the previous line. This is not implemented here.
    fn check_bounds(&mut self) {
        let line_length = self.line_length();

        if self.cx >= line_length {
            if line_length > 0 {
                self.cx = self.line_length() - 1;
            } else {
                self.cx = 0;
            }
        }

        if self.cx >= self.vwidth() {
            self.cx = self.vwidth() - 1;
        }

        let line_on_buffer = self.vtop + self.cy;
        if line_on_buffer > self.buffer.len() as u16 - 1 {
            self.cy = self.buffer.len() as u16 - self.vtop;
        }
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        loop {
            self.check_bounds();
            self.draw()?;
            self.stdout.queue(cursor::MoveTo(self.cx, self.cy))?;
            self.stdout.flush()?;

            if let Some(action) = self.handle_event(read()?)? {
                match action {
                    Action::Quit => break,
                    Action::MoveUp => {
                        if self.cy == 0 {
                            // scroll up
                            if self.vtop > 0 {
                                self.vtop -= 1;
                            }
                        } else {
                            self.cy = self.cy.saturating_sub(1);
                        }
                    }
                    Action::MoveDown => {
                        self.cy += 1;
                        if self.cy >= self.vheight() {
                            // scroll if possible
                            self.vtop += 1;
                            self.cy -= 1;
                        }
                    }
                    Action::MoveLeft => {
                        self.cx = self.cx.saturating_sub(1);
                        if self.cx < self.vleft {
                            self.cx = self.vleft;
                        }
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

impl Drop for Editor {
    fn drop(&mut self) {
        _ = self.stdout.flush();
        _ = self.stdout.execute(terminal::LeaveAlternateScreen); // Switch back to main screen
        _ = terminal::disable_raw_mode();
    }
}
