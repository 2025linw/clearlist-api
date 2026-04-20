//! # Constants Module
//!
//! This module contains all constants used throughout the code base

/// Default limit for all `query` functions
pub const DEFAULT_LIMIT: i64 = 50;

/// Maximum limit for all `query` functions
pub const MAX_LIMIT: i64 = 200;

/// User not found error message
pub const USER_NOT_FOUND: &str = "User does not exist";

/// Task not found error message
pub const TASK_NOT_FOUND: &str = "Task not found";

/// Tag not found error message
pub const TAG_NOT_FOUND: &str = "Task not found";
