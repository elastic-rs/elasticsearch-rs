//! This sample is a more fleshed out application using `elastic`.
//!
//! It expects you have an Elasticsearch node running on `localhost:9200`.

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate elastic_derive;
#[macro_use]
extern crate quick_error;

extern crate serde;
#[macro_use]
extern crate serde_json;
extern crate elastic;

pub mod model;
pub mod ops;

use std::error::Error;
use elastic::client::RequestParams;
use ops::Client;
use ops::commands::{EnsureBankIndexExists, PutBulkAccounts};
use ops::queries::SimpleSearchQuery;

fn run() -> Result<(), Box<Error>> {
    let client = Client::new(RequestParams::default())?;

    println!("checking index");

    client.ensure_bank_index_exists()?;

    println!("updating docs");

    client.put_bulk_accounts("data/accounts.json")?;

    let accounts = client.simple_search_query("Bruce Coffey")?;

    for account in accounts.hits() {
        println!("{:?}", account);
    }

    Ok(())
} 

fn main() {
    run().unwrap()
}
