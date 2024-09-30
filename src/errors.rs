pub struct AdocrError {
    line: usize,
    message: String,
}

impl AdocrError {
    pub fn new(line: usize, message: String) -> Self {
        let e = AdocrError { line, message };
        e.report();
        e
    }
    fn report(&self) {
        eprintln!("[{}]: {}", self.line, self.message)
    }
}
