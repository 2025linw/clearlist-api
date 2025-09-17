/// Logs error with `tracing` then exits process
///
/// **ONLY USED WITHIN WEB SERVER APPLICATION**
/// Crate libraries all utilize crate `Error`
#[macro_export]
macro_rules! log_error_and_exit {
    ($($arg:tt)*) => {{
        error!($($arg)*);

        std::process::exit(1)
    }};
}
