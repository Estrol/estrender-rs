#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        println!("[LOG]: {}", format!($($arg)*));
    };
}

#[macro_export]
macro_rules! dbg_log {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        println!("[DEBUG]: {}", format!($($arg)*));
    };
}

#[macro_export]
macro_rules! error_log {
    ($($arg:tt)*) => {
        eprintln!("[ERROR]: {}", format!($($arg)*));
    };
}

#[macro_export]
macro_rules! warn_log {
    ($($arg:tt)*) => {
        eprintln!("[WARNING]: {}", format!($($arg)*));
    };
}
