use super::*;

#[derive(Debug, Clone, PartialEq)]
pub enum RequestError {
    /// Something went wrong parsing or decoding the message. String describes error.
    BadMessage(String),
    /// The object is invalid or has been destroyed
    BadObject(ObjectId),
    /// An ID was destroyed/invalid
    BadId(GenericId),
    /// The entity is null or has been destroyed, may be the entity the request is on or may be one
    /// that appears in the arguments
    BadEntity(EntityKey),
    /// The entity doesn't have a member with this name
    BadName(EntityKey, String),
    /// When the request is invalid for some other reason, such as an out-of-range value, a value
    /// of the wrong type, a method that's not allowed the member, etc
    BadRequest(String),
    /// Returned when there is an internal server error. The connection logs this as an error as
    /// well as sending it to the client.
    InternalError(String),
}

pub type RequestResult<T> = Result<T, RequestError>;

impl std::fmt::Display for RequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::BadMessage(msg) => write!(f, "bad protocol message: {}", msg),
            Self::BadObject(o) => write!(f, "object #{} is invalid or destroyed", o),
            Self::BadId(id) => write!(f, "{:?} is invalid or destroyed", id),
            Self::BadEntity(e) => write!(f, "{:?} is invalid or destroyed", e),
            Self::BadName(e, n) => write!(f, "{:?} has no member {:?}", e, n),
            Self::BadRequest(msg) => write!(f, "{}", msg),
            Self::InternalError(e) => write!(f, "{}", e),
        }
    }
}

impl Error for RequestError {}
