pub struct Buffer {
    pub file: Option<String>,
    pub lines: Vec<String>,
}

impl Buffer {
    pub fn from_file(file: Option<String>) -> Self {
        let lines = match &file {
            Some(file) => std::fs::read_to_string(file)
                .unwrap()
                .lines()
                .map(|s| s.to_string())
                .collect(),
            None => vec![],
        };

        Self { file, lines }
    }

    pub fn get_line(&self, line: usize) -> Option<String> {
        if self.lines.len() > line {
            Some(self.lines[line].clone())
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }
}
