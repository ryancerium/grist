#[macro_export]
macro_rules! PRINT_STYLE {
    ($style:expr, $winstyle:ident) => {
        if ($style) & $winstyle == $winstyle {
            println!("  {}", stringify!($winstyle));
        }
    };
}

#[macro_export]
macro_rules! D {
    ($e:expr) => {
        println!("{}: '{:?}'", stringify!($e), $e);
    };
}

#[macro_export]
macro_rules! P {
    ($e:expr) => {
        println!("{}: '{}'", stringify!($e), $e);
    };
}
