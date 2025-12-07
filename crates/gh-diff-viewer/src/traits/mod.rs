//! Extension traits for customizing diff viewer behavior.

mod comment_handler;
mod context_provider;
mod theme_provider;

pub use comment_handler::{CommentError, CommentHandler, CommentId};
pub use context_provider::{ContextError, ContextProvider};
pub use theme_provider::{DefaultTheme, ThemeProvider};
