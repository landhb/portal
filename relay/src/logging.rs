// Disable warnings
#[allow(unused_macros)]
// The debug version
#[cfg(debug_assertions)]
#[macro_export]
macro_rules! log {
    ($( $args:expr ),*) => { println!( $( $args ),* ); }
}

// Non-debug version
#[macro_export]
#[cfg(not(debug_assertions))]
macro_rules! log {
    ($( $args:expr ),*) => {
        ()
    };
}
