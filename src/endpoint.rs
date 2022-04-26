//! This module contains the endpoint trait used to implemented api endpoints.

use crate::{LIVE_ENDPOINT, SANDBOX_ENDPOINT};
use serde::{de::DeserializeOwned, Serialize};
use std::borrow::Cow;

/// A trait implemented by api endpoints.
pub trait Endpoint {
    /// The serializable query type.
    type Query: Serialize;
    /// The serializable body type.
    type Body: Serialize;
    /// The deserializable response type.
    type Response: DeserializeOwned;

    /// The endpoint relative path. Must start with a `/`
    fn relative_path(&self) -> Cow<str>;

    /// The request method of this endpoint.
    fn method(&self) -> reqwest::Method;

    /// The query to be used when calling this endpoint.
    fn query(&self) -> Option<&Self::Query> {
        None
    }

    /// The body to be used when calling this endpoint.
    fn body(&self) -> Option<&Self::Body> {
        None
    }

    /// The full path of this endpoint.
    ///
    /// Automatically implemented.
    fn full_path(&self, is_sandbox: bool) -> String {
        if is_sandbox {
            format!("{}{}", SANDBOX_ENDPOINT, self.relative_path())
        } else {
            format!("{}{}", LIVE_ENDPOINT, self.relative_path())
        }
    }
}