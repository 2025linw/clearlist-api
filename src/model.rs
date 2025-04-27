pub mod area;
pub mod auth;
pub mod project;
pub mod tag;
pub mod task;

/// Trait to convert from Database Model to Response Model
pub trait ToResponse {
    type Response;

    /// Converts type to Response
    fn to_response(&self) -> Self::Response;
}
