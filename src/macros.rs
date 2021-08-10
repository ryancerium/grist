#[macro_export]
macro_rules! CHECK_BOOL {
    ($bool_expr:expr) => {{
        let result = $bool_expr;
        if !result.as_bool() {
            println!("Failure: '{}' {}:{}", stringify!($bool_expr), file!(), line!());
        }
        result
    }};
}

#[macro_export]
macro_rules! CHECK_HRESULT {
    ($hresult_expr:expr) => {{
        let result = $hresult_expr;
        if result != S_OK {
            println!("Failure: '{}' {}:{}", stringify!(hresult_expr), file!(), line!());
        }
        result
    }};
}

#[macro_export]
macro_rules! CHECK_HWND {
    ($hwnd_expr:expr) => {
        let hwnd = $hwnd_expr;
        if $hwnd.is_null() {
            println!("HWND is null: {}:{}", file!(), line!());
        }
        hwnd
    };
}

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
        println!("{}: {:?}", stringify!($e), $e);
    };
}

#[macro_export]
macro_rules! P {
    ($e:expr) => {
        println!("{}: {}", stringify!($e), $e);
    };
}
