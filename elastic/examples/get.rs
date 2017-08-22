//! Get a document from an index.
//!
//! NOTE: This sample expects you have a node running on `localhost:9200`.
//!
//! This sample demonstrates how to get a previously indexed document.
//! There are a few error cases to consider, depending on whether or not the index
//! exists, and the document is indexed.
//! Also see the `typed` sample for a more complete implementation.

extern crate serde_json;
extern crate elastic;

use serde_json::Value;
use elastic::error::*;
use elastic::prelude::*;

fn main() {
    // A reqwest HTTP client and default parameters.
    // The `params` includes the base node url (http://localhost:9200).
    let client = SyncClientBuilder::new().build()?;

    let res = client
        .get_document::<Value>(index("typed_sample_index"), id("1"))
        .ty("mytype")
        .send();

    // Responses can be unpacked in various ways. In this case we check if:
    // - The call succeeded and the document was found
    // - The call succeeded but the document wasn't found
    // - The call failed because the index doesn't exist
    // - The call failed for some other reason
    match res.map(|res| res.into_document()) {
        // The doc was found
        Ok(Some(doc)) => {
            println!("document found: {:?}", doc);
        }
        // The index exists, but the doc wasn't found
        Ok(None) => {
            println!("document not found, but index exists");
        }
        // No index
        Err(Error::Api(ApiError::IndexNotFound { .. })) => {
            println!("index not found");
        },
        // Some other error: panic
        Err(e) => panic!("{:?}", e)
    }
}
