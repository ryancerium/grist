#[macro_export]
macro_rules! CHECK_BOOL {
    ($bool_expr:expr) => {
        match $bool_expr {
            0 => {
                println!(
                    "Failure: '{}' {}:{}",
                    stringify!($bool_expr),
                    file!(),
                    line!()
                );
                0
            }
            value => value,
        }
    };
}

#[macro_export]
macro_rules! CHECK_HRESULT {
    ($hresult_expr:expr) => {
        match $hresult_expr {
            S_OK => S_OK,
            value => {
                println!(
                    "Failure: '{}' {}:{}",
                    stringify!($hresult_expr),
                    file!(),
                    line!()
                );
                value
            }
        }
    };
}
