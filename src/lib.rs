mod library;

pub use library::*;

pub mod prelude {
    pub use crate::library::fs::{self, Directory, File, Object};
}
