mod headers;
mod method;
pub(crate) mod parser;
mod version;

pub use headers::Headers;
pub use method::Method;
pub use parser::BuildError;
pub use version::Version;

pub(crate) mod header {
    pub const CONNECTION_HEADER: &str = "Connection";
    pub const CLOSE_CONNECTION_HEADER: &str = "close";
}
