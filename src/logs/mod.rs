pub(crate) mod render;

use std::collections::VecDeque;

#[derive(Default)]
pub struct Logger {
    logs: VecDeque<Log>,
    last_error: Option<String>,
}

pub enum Log {
    Info(String),
    Error(String),
}

impl Logger {
    pub fn info(&mut self, log: impl Into<String>) {
        self.log(Log::Info(log.into()));
    }

    pub fn error(&mut self, log: impl Into<String>) {
        self.log(Log::Error(log.into()));
    }

    pub fn log(&mut self, log: Log) {
        const MAX_LOGS: usize = 100;

        if let Log::Error(e) = &log {
            self.last_error = Some(e.to_string());
        }

        self.logs.push_back(log);
        if self.logs.len() > MAX_LOGS {
            self.logs.pop_front();
        }
    }

    pub fn logs(&self) -> impl DoubleEndedIterator<Item = &Log> {
        let (a, b) = self.logs.as_slices();
        a.iter().chain(b.iter())
    }

    pub fn last_error(&self) -> Option<&String> {
        self.last_error.as_ref()
    }
}
