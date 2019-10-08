//! Have you ever needed to pass a mutable reference to yourself to a struct member?
//! Now you can! 100% safe rust code.
//!
//! # Examples
//!
//! ## You can pass mutable references to self into member structs:
//!
//! ```
//! #[macro_use]
//! extern crate detach;
//! use detach::prelude::*;
//!
//! fn main() {
//!     struct Sub;
//!     impl Sub {
//!         pub fn write(&mut self, t: &mut Top) {
//!             t.0.push_str("test");
//!         }
//!     }
//!     struct Top(pub String, pub Detach<Sub>);
//!     impl Top {
//!         pub fn run(&mut self) {
//!             detach_run!(self.1, |s| s.write(self));
//!         }
//!     }
//!     let mut t = Top("".to_string(), Detach::new(Sub));
//!     t.run();
//!     assert_eq!("test", &t.0);
//! }
//! ```
//!
//! ## You can invoke `detach_run!` with an exper:
//!
//! ```
//! #[macro_use]
//! extern crate detach;
//! use detach::prelude::*;
//!
//! fn main() {
//!     let mut i = Detach::new(42_i8);
//!     detach_run!(i, |z| z = 3);
//!     assert_eq!(3, *i);
//! }
//! ```
//!
//! ## You can invoke `detach_run!` with a block:
//!
//! ```
//! #[macro_use]
//! extern crate detach;
//! use detach::prelude::*;
//!
//! fn main() {
//!     let mut i = Detach::new(42_i8);
//!     detach_run!(i, |z| {
//!         z = 3;
//!     });
//!     assert_eq!(3, *i);
//! }
//! ```
//!
//! ## You can return info from the macro:
//!
//! ```
//! #[macro_use]
//! extern crate detach;
//! use detach::prelude::*;
//!
//! fn main() {
//!     let mut i = Detach::new(42_i8);
//!     let z = detach_run!(i, |z| {
//!         z = 3;
//!         return z;
//!     });
//!     assert_eq!(3, *i);
//!     assert_eq!(3, z);
//! }
//! ```
//!
//! ## You cannot use ? directly (we wouldn't re-attach the member):
//!
//! ```compile_fail
//! #[macro_use]
//! extern crate detach;
//! use detach::prelude::*;
//!
//! fn main() {
//!     let mut i = Detach::new(42_i8);
//!     detach_run!(i, |z| {
//!         z = 3;
//!         let r: Result<(), ()> = Ok(());
//!         // THIS WON'T COMPILE:
//!         r?;
//!     });
//!     assert_eq!(3, *i);
//! }
//! ```
//!
//! ## But you can return results, and ? after the macro:
//!
//! ```
//! #[macro_use]
//! extern crate detach;
//! use detach::prelude::*;
//!
//! fn test() -> Result<(), ()> {
//!     let mut i = Detach::new(42_i8);
//!     detach_run!(i, |z| {
//!         z = 3;
//!         let r: Result<(), ()> = Ok(());
//!         return r;
//!     })?;
//!     assert_eq!(3, *i);
//!     Ok(())
//! }
//!
//! fn main() {
//!     test().unwrap();
//! }
//! ```

pub mod prelude {
    pub use crate::Detach;
}

#[macro_export]
macro_rules! detach_run {
    ($item:expr, |$id:ident| $code:block) => {{
        let mut $id = $item.take();
        let out = (|| $code)();
        $item.put($id);
        out
    }};
    ($item:expr, |$id:ident| $code:expr) => {{
        let mut $id = $item.take();
        let out = (|| $code)();
        $item.put($id);
        out
    }};
}

/// Allows sub-struct to be easily detached and re-attached
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Detach<T>(Option<T>);

impl<T> Detach<T> {
    /// create a new Detach instance
    pub fn new(inner: T) -> Self {
        Self(Some(inner))
    }

    /// extract the owned inner instance
    pub fn to_inner(self) -> T {
        self.0.expect("detach exists-to_inner")
    }

    /// returns true if this instance contains `Some` value.
    pub fn is_attached(&self) -> bool {
        self.0.is_some()
    }

    /// take the owned inner instance (without droping the container)
    pub fn take(&mut self) -> T {
        std::mem::replace(&mut self.0, None).expect("detach exists-take")
    }

    /// replace the owned inner instance
    pub fn put(&mut self, t: T) {
        std::mem::replace(&mut self.0, Some(t));
    }
}

impl<T> std::ops::Deref for Detach<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref().expect("detach exists-deref")
    }
}

impl<T> std::ops::DerefMut for Detach<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut().expect("detach exists-deref-mut")
    }
}

impl<T> std::borrow::Borrow<T> for Detach<T> {
    fn borrow(&self) -> &T {
        self.0.as_ref().expect("detach exists-borrow")
    }
}

impl<T> std::borrow::BorrowMut<T> for Detach<T> {
    fn borrow_mut(&mut self) -> &mut T {
        self.0.as_mut().expect("detach exists-borrow-mut")
    }
}

impl<T> std::convert::AsRef<T> for Detach<T> {
    fn as_ref(&self) -> &T {
        self.0.as_ref().expect("detach exists-as-ref")
    }
}

impl<T> std::convert::AsMut<T> for Detach<T> {
    fn as_mut(&mut self) -> &mut T {
        self.0.as_mut().expect("detach exists-as-mut")
    }
}

impl<T> std::convert::From<T> for Detach<T> {
    fn from(t: T) -> Detach<T> {
        Self(Some(t))
    }
}
