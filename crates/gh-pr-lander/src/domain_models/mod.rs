//! Domain models
//!
//! Core domain types used throughout the application.
//! These are pure domain concepts, separate from UI state.

pub mod auto_merge;
pub mod operation_monitor;
pub mod pr_filter;
pub mod pr_number;
pub mod pull_request;
pub mod repository;
pub mod task_status;

// Re-export commonly used types (allow unused - these are for external crate use)
#[allow(unused_imports)]
pub use auto_merge::AutoMergePr;
#[allow(unused_imports)]
pub use operation_monitor::{OperationMonitor, OperationType};
#[allow(unused_imports)]
pub use pr_filter::PrFilter;
#[allow(unused_imports)]
pub use pr_number::PrNumber;
pub use pull_request::{LoadingState, MergeableStatus, Pr};
pub use repository::Repository;
#[allow(unused_imports)]
pub use task_status::{TaskStatus, TaskStatusType};
