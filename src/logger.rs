use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::Mutex;

pub struct Logger {
    file: Mutex<File>,
}

impl Logger {
    pub fn new(file: &str) -> Self {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(file)
            .expect("Unable to open log file");

        Self {
            file: Mutex::new(file),
        }
    }

    pub fn log(&self, message: &str) {
        let mut file = self.file.lock().unwrap();
        writeln!(file, "{}", message).expect("write to file works");
    }
}
