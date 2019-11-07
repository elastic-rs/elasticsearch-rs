/*!
Client-side error types.

The main `Error` type combines the various kinds of errors that can occur when interacting with Elasticsearch.

# Examples

Any method defined in `elastic` that could fail will return a `Result<T, Error>` that can be matched on.
The below example sends a request and then checks the response for an `Error::Api`:

```no_run
# #[macro_use] extern crate serde_json;
# use serde_json::Value;
# use elastic::prelude::*;
# use elastic::Error;
# fn main() { run().unwrap() }
# fn run() -> Result<(), Box<dyn ::std::error::Error>> {
# let client = SyncClientBuilder::new().build()?;
// Send a request.
// The returned error may be a REST API error from Elasticsearch or an internal error
let response = client.search::<Value>().send();

match response {
    Ok(response) => {
        // do something with the response
    },
    Err(Error::Api(e)) => {
        // handle a REST API error
    },
    Err(e) => {
        // handle a client error
    }
}
# Ok(())
# }
```
*/

use std::{
    error::Error as StdError,
    fmt,
    io,
};

use crate::http::{
    receiver::ResponseError,
    StatusCode,
};
#[cfg(any(feature="async_sender", feature="sync_sender"))]
use reqwest::Error as ReqwestError;
use serde_json;

pub use crate::http::receiver::ApiError;

quick_error! {
    /**
    An error encountered while interacting with Elasticsearch.

    API errors can be easily matched and destructured whereas client errors
    can be formatted, but not destructured.

    If the `RUST_BACKTRACE` environment variable is `1` then client errors will
    also contain a backtrace.
    */
    #[derive(Debug)]
    pub enum Error {
        /** An API error from Elasticsearch. */
        Api(err: ApiError) {
            cause(err)
            description("API error returned from Elasticsearch")
            display("API error returned from Elasticsearch. Caused by: {}", err)
        }
        /** Any other kind of error. */
        Client(err: ClientError) {
            cause(err)
            description("error sending a request or receiving a response")
            display("error sending a request or receiving a response. Caused by: {}", err)
        }
    }
}

pub(crate) mod string_error {
    quick_error! {
        #[derive(Debug)]
        pub enum Error {
            Message(err: String) {
                description("An error occurred")
                display("{}", err)
            }
        }
    }
}

/** Wraps a boxed stdlib error */
pub struct WrappedError(Box<dyn StdError + Send + Sync>);

impl fmt::Display for WrappedError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Debug for WrappedError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl StdError for WrappedError {
    fn description(&self) -> &str {
        self.0.description()
    }

    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.0.source()
    }
}

/** An error building a client, sending a request or receiving a response. */
#[derive(Debug)]
pub struct ClientError {
    inner: inner::Error,
}

impl StdError for ClientError {
    fn description(&self) -> &str {
        self.inner.description()
    }

    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.inner.source()
    }
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.inner.fmt(f)
    }
}

/** Creates a client error by wrapping an existing error */
pub fn build<E>(err: E) -> Error
where
    E: StdError + Send + 'static,
{
    Error::Client(ClientError {
        inner: inner::Error::with_chain(err, inner::ErrorKind::Build),
    })
}

/** Wraps a boxed stdlib error into a `WrappedError` */
pub fn wrapped(err: Box<dyn StdError + Send + Sync>) -> WrappedError {
    WrappedError(err)
}

/** Creates an error for when a request could not be generated or sent */
pub fn request<E>(err: E) -> Error
where
    E: StdError + Send + 'static,
{
    Error::Client(ClientError {
        inner: inner::Error::with_chain(err, inner::ErrorKind::Request),
    })
}

/** Creates an error from a failure response */
pub fn response<E>(status: StatusCode, err: E) -> Error
where
    E: Into<MaybeApiError<E>> + StdError + Send + 'static,
{
    match err.into() {
        MaybeApiError::Api(err) => Error::Api(err),
        MaybeApiError::Other(err) => Error::Client(ClientError {
            inner: inner::Error::with_chain(err, inner::ErrorKind::Response(status)),
        }),
    }
}

pub(crate) fn message<E>(err: E) -> string_error::Error
where
    E: Into<String>,
{
    string_error::Error::Message(err.into())
}

/** A convenient method to generate errors in tests. */
#[cfg(test)]
pub(crate) fn test() -> Error {
    Error::Client(ClientError {
        inner: inner::Error::from("a test error"),
    })
}

/** Error that can be either an ApiError or some other error */
pub enum MaybeApiError<E> {
    /** An API error */
    Api(ApiError),
    /** A different error */
    Other(E),
}

impl Into<MaybeApiError<ResponseError>> for ResponseError {
    fn into(self) -> MaybeApiError<Self> {
        match self {
            ResponseError::Api(err) => MaybeApiError::Api(err),
            err => MaybeApiError::Other(err),
        }
    }
}

impl Into<MaybeApiError<io::Error>> for io::Error {
    fn into(self) -> MaybeApiError<Self> {
        MaybeApiError::Other(self)
    }
}

#[cfg(any(feature="async_sender", feature="sync_sender"))]
impl Into<MaybeApiError<ReqwestError>> for ReqwestError {
    fn into(self) -> MaybeApiError<Self> {
        MaybeApiError::Other(self)
    }
}

impl Into<MaybeApiError<serde_json::Error>> for serde_json::Error {
    fn into(self) -> MaybeApiError<Self> {
        MaybeApiError::Other(self)
    }
}

mod inner {
    #![allow(missing_docs)]

    use http::StatusCode;

    error_chain! {
        errors {
            Build {
                description("error attempting to build a client")
                display("error attempting to build a client")
            }
            Request {
                description("error sending a request")
                display("error sending a request")
            }
            Response(status: StatusCode) {
                description("error receiving a response")
                display("error receiving a response. Status code: {}", status)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::*;

    #[test]
    fn error_is_send_sync() {
        assert_send::<Error>();
    }
}
