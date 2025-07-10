use crate::runner::RunnerError;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RunMode {
    Poll,
    ReDraw,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum WindowError {
    RunnerError(RunnerError),
    WindowNotFound,
}
