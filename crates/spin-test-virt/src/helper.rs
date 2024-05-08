#[macro_export]
macro_rules! println {
    ($($tt:tt)*) => {
        let stdout = $crate::bindings::wasi::cli::stdout::get_stdout();
        stdout
            .blocking_write_and_flush(format!("{}\n", format_args!($($tt)*)).as_bytes())
            .unwrap();
    };
}
