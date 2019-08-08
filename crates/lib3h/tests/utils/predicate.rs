/// Check if Protocol is of type $p
#[allow(unused_macros)]
macro_rules! one_is {
    ($p:pat) => {
        |d| {
            if let $p = d {
                return true;
            }
            return false;
        }
    };
}

/// Check if Protocol is of type $p and meets conditions set in $code
#[allow(unused_macros)]
macro_rules! one_is_where {
    ($p:pat, $code:tt) => {
        move |d| return if let $p = d { $code } else { false }
    };
}

#[allow(unused_macros)]
macro_rules! one_let {
    ($p:pat = $enum:ident $code:tt) => {
        if let $p = $enum {
            $code
        } else {
            panic!("Received unexpected Protocol message type: {:?}", $enum);
        }
    };
}
