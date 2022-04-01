#[macro_export]
macro_rules! log_status {
    ($($arg:tt)*) => (println!("{} {}", "[*]".blue().bold(), format_args!($($arg)*)));
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => (println!("{} {}", "[!]".red().bold(), format_args!($($arg)*)));
}

#[macro_export]
macro_rules! log_success {
    ($($arg:tt)*) => (println!("{} {}", "[+]".green().bold(), format_args!($($arg)*)));
}
