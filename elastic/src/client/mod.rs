/*!
HTTP client, requests and responses.

This module contains the HTTP client, as well as request and response types.

# Sync and async clients

`elastic` provides two clients:

- [`SyncClient`]() for making synchronous requests
- [`AsyncClient`]() for making asynchronous requests using  the [`futures`]() crate.

To be precise, `elastic` actually offers only one client, `Client<TSender>`, that is generic over the kind of requests it can send.
This makes it possible to share methods on request builders, but can make the method signatures more difficult to follow.

## Building a synchronous client

Use a `SyncClientBuilder` to configure a synchronous client.

> TODO: Fill this out

## Building an asynchronous client

Use an `AsyncClientbuilder` to configure an asynchronous client.

> TODO: Fill this out

# Request builders

Some commonly used endpoints have high-level builder methods you can use to configure requests easily.
They're exposed as methods on the `Client`:

Client method                               | Elasticsearch API                  | Raw request type                                        | Response type
------------------------------------------- | ---------------------------------- | ------------------------------------------------------- | ------------------------------------
[`search`][Client.search]                   | [Search][docs-search]              | [`SearchRequest`][SearchRequest]                        | [`SearchResponse`][SearchResponse]
[`get_document`][Client.get_document]       | [Get Document][docs-get]           | [`GetRequest`][GetRequest]                              | [`GetResponse`][GetResponse]
[`index_document`][Client.index_document]   | [Index Document][docs-index]       | [`IndexRequest`][IndexRequest]                          | [`IndexResponse`][IndexResponse]
[`put_mapping`][Client.put_mapping]         | [Put Mapping][docs-mapping]        | [`IndicesPutMappingRequest`][IndicesPutMappingRequest]  | [`CommandResponse`][CommandResponse]
[`create_index`][Client.create_index]       | [Create Index][docs-create-index]  | [`IndicesCreateRequest`][IndicesCreateRequest]          | [`CommandResponse`][CommandResponse]

All builders follow a standard pattern:

- The `Client` method that takes all required parameters without type inference
- Optional or inferred parameters can be overridden in builder methods with type inference
- `send` will return a specific response type

A search request for a value, where the response is matched for an `ApiError`:

```no_run
# #[macro_use] extern crate json_str;
# extern crate serde_json;
# extern crate elastic;
# use serde_json::Value;
# use elastic::prelude::*;
# use elastic::error::*;
# fn main() {
# let client = SyncClientBuilder::new().build().unwrap();
let response = client.search::<Value>()
                     .index("myindex")
                     .ty(Some("myty"))
                     .body(json_str!({
                         query: {
                             query_string: {
                                 query: "*"
                             }
                         }
                     }))
                     .send();

match response {
    Ok(response) => {
        // Iterate through the response hits
        for hit in response.hits() {
            println!("{:?}", hit);
        }
    },
    Err(e) => {
        match *e.kind() {
            ErrorKind::Api(ref e) => {
                // handle a REST API error
            },
            ref e => {
                // handle a HTTP or JSON error
            }
        }
    }
}
# }
```

The request builders are wrappers around the [`Client.request`][Client.request] method, taking a [raw request type][endpoints-mod].
A `get` request for a value:

```no_run
# extern crate serde_json;
# extern crate elastic;
# use serde_json::Value;
# use elastic::prelude::*;
# fn main() {
# let client = SyncClientBuilder::new().build().unwrap();
let response = client.get_document::<Value>(index("values"), id(1)).send();
# }
```

Is equivalent to:

```no_run
# extern crate serde_json;
# extern crate elastic;
# use serde_json::Value;
# use elastic::prelude::*;
# fn main() {
# let client = SyncClientBuilder::new().build().unwrap();
let response = client.request(GetRequest::for_index_ty_id("values", "value", 1))
                     .send()
                     .and_then(into_response::<GetResponse<Value>>);
# }
```

# Raw request types

Not all endpoints have strongly-typed builders, but all Elasticsearch API endpoints have a specific [raw request type][endpoints-mod] that can be used to build a request manually and send with the [`Client.request`][Client.Request] method.
The builders described above are just wrappers around these request types, but that doesn't mean raw requests are a second-class API.
You have more control over how requests are serialised, sent and deserialised using the raw requests API.
All request endpoints live in the [`endpoints`][endpoints-mod] module.

The process of sending raw requests is described in more detail below.

## The raw request process

The pieces involved in sending an Elasticsearch API request and parsing the response are modular.
Each one exposes Rust traits you can implement to support your own logic but if you just want to send a search/get request and parse a search/get response then you won't need to worry about this so much.

The basic flow from request to response is:

**1)** Turn a concrete [request type][endpoints-mod] into a [`RequestBuilder`][RequestBuilder]:

```text
[RequestType] ---> [Client.request()] ---> [RequestBuilder]
```

**2)** Send the [`RequestBuilder`][RequestBuilder] and get a [`ResponseBuilder`][ResponseBuilder]:

```text
[RequestBuilder.send()] ---> [ResponseBuilder]
```

**3)** Parse the [`ResponseBuilder`][ResponseBuilder] to a [response type][response-types]:

```text
[ResponseBuilder.into_response()] ---> [ResponseType]
```

The example below shows how these pieces fit together in code  by sending a simple `SearchRequest`, 
with the steps in the above process labelled:

```no_run
# extern crate elastic;
# #[macro_use]
# extern crate json_str;
# extern crate serde_json;
# use elastic::prelude::*;
# use elastic::error::*;
# use serde_json::Value;
# fn main() {
# let client = SyncClientBuilder::new().build().unwrap();
let req = SearchRequest::for_index("_all", empty_body());

let response = client.request(req) // 1
                     .send() // 2
                     .and_then(into_response::<SearchResponse<Value>>); // 3
# }
```

### 1. Building raw requests

The [`endpoints`][endpoints-mod] module contains code-generated request types for the Elasticsearch REST API.
Each request type expects its parameters upfront and is generic over the request body.

A raw search request:

```no_run
# #[macro_use] extern crate json_str;
# extern crate elastic;
# use elastic::prelude::*;
# fn main() {
let req = {
    let body = json_str!({
        query: {
            query_string: {
                query: "*"
            }
        }
    });

    SearchRequest::for_index_ty("myindex", "myty", body)
};
# }
```

A raw request to index a document:

```no_run
# #[macro_use] extern crate serde_derive;
# extern crate serde;
# extern crate serde_json;
# extern crate elastic;
# use elastic::prelude::*;
# #[derive(Serialize)]
# struct MyType;
# fn main() {
# let doc = MyType;
let req = {
    let body = serde_json::to_string(&doc).unwrap();

    IndexRequest::for_index_ty_id("myindex", "myty", 1, body)
};
# }
```

### 2. Sending requests

Both high-level request builders and raw requests have some common builder methods:

- [`params`][RequestBuilder.params] for setting url query parameters
- a `send` method for sending the request.
For high-level requests this returns a strongly-typed response.
For raw requests this returns a [`ResponseBuilder`][ResponseBuilder].
If the request was sent synchronously, the response is returned as a `Result`.
If the request was sent asynchronously, the response is returned as a `Future`.

```no_run
# use elastic::prelude::*;
# let client = SyncClientBuilder::new().build().unwrap();
# let req = PingRequest::new();
let request_builder = client.request(req);

// Set additional url parameters
let request_builder = request_builder.params(|p| p
    .url_param("pretty", true)
    .url_param("refresh", true)
);

// Send the request
let response = request_builder.send();
```

### 3. Parsing responses synchronously

Call [`SyncResponseBuilder.into_response`][SyncResponseBuilder.into_response] on a sent request to get a [strongly typed response][response-types]:

```no_run
# extern crate serde;
# extern crate serde_json;
# #[macro_use] extern crate serde_derive;
# #[macro_use] extern crate elastic_derive;
# extern crate elastic;
# use serde_json::Value;
# use elastic::prelude::*;
# use elastic::error::*;
# fn main() {
# #[derive(Serialize, Deserialize, ElasticType)]
# struct MyType {
#     pub id: i32,
#     pub title: String,
#     pub timestamp: Date<DefaultDateFormat>
# }
# let params = RequestParams::new("http://es_host:9200");
# let client = Client::new(params).unwrap();
# let req = PingRequest::new();
let response = client.request(req)
                     .send()?
                     .into_response::<SearchResponse<Value>>()?;

match response {
    Ok(response) => {
        // Iterate through the response hits
        for hit in response.hits() {
            println!("{:?}", hit);
        }
    },
    Err(e) => {
        match *e.kind() {
            ErrorKind::Api(ref e) => {
                // handle a REST API error
            },
            ref e => {
                // handle a HTTP or JSON error
            }
        }
    }
}
# }
```

Alternatively, call [`SyncResponseBuilder.into_raw`][SyncResponseBuilder.into_raw] on a sent request to get a raw [`HttpResponse`][HttpResponse]:

```no_run
# extern crate serde;
# #[macro_use] extern crate serde_derive;
# #[macro_use] extern crate elastic_derive;
# extern crate elastic;
# use std::io::Read;
# use elastic::prelude::*;
# fn main() {
# let params = RequestParams::new("http://es_host:9200");
# let client = Client::new(params).unwrap();
# let req = PingRequest::new();
let mut response = client.request(req)
                         .send()?
                         .into_raw()?;

let mut body = String::new();
response.read_to_string(&mut body).unwrap();

println!("{}", body);
# }
```

`SyncHttpResponse` implements the standard `Read` trait so you can buffer out the raw response data.
For more details see the [`responses`][responses-mod] module.

### 3. Parsing responses asynchronously

Call [`AsyncResponseBuilder.into_response`][AsyncResponseBuilder.into_response] on a sent request to get a [strongly typed response][response-types]:

```no_run
# extern crate serde;
# extern crate serde_json;
# #[macro_use] extern crate serde_derive;
# #[macro_use] extern crate elastic_derive;
# extern crate elastic;
# use serde_json::Value;
# use elastic::prelude::*;
# use elastic::error::*;
# fn main() {
# #[derive(Serialize, Deserialize, ElasticType)]
# struct MyType {
#     pub id: i32,
#     pub title: String,
#     pub timestamp: Date<DefaultDateFormat>
# }
# let params = RequestParams::new("http://es_host:9200");
# let client = Client::new(params).unwrap();
# let req = PingRequest::new();
let future = client.request(req)
                   .send()
                   .and_then(|response| res.into_response::<SearchResponse<Value>>);

future.and_then(|response| {
    // Iterate through the response hits
    for hit in response.hits() {
        println!("{:?}", hit);
    }

    Ok(())
};
# }
```

Alternatively, call [`AsyncResponseBuilder.into_raw`][AsyncResponseBuilder.into_raw] on a sent request to get a raw [`HttpResponse`][HttpResponse]:

```no_run
# extern crate serde;
# #[macro_use] extern crate serde_derive;
# #[macro_use] extern crate elastic_derive;
# extern crate elastic;
# use std::io::Read;
# use elastic::prelude::*;
# fn main() {
# let params = RequestParams::new("http://es_host:9200");
# let client = Client::new(params).unwrap();
# let req = PingRequest::new();
let mut future = client.request(req)
                       .send()
                       .and_then(|response| response.into_raw());

future.and_then(|response| {
    tokio_io::read_to_end(response, Vec::new())
})
.and_then(|(response, body)| {
    let body = String::from_utf8(body).unwrap();

    println!("{}", body);

    Ok(())
});
# }
```

`AsyncHttpResponse` implements `tokio_io`s `AsyncRead` trait so you can buffer out the raw response data.
For more details see the [`responses`][responses-mod] module.

[docs-search]: http://www.elastic.co/guide/en/elasticsearch/reference/current/search-search.html
[docs-get]: http://www.elastic.co/guide/en/elasticsearch/reference/current/docs-get.html
[docs-index]: https://www.elastic.co/guide/en/elasticsearch/reference/current/docs-index_.html
[docs-mapping]: https://www.elastic.co/guide/en/elasticsearch/reference/current/mapping.html
[docs-create-index]: https://www.elastic.co/guide/en/elasticsearch/reference/current/indices-create-index.html

[endpoints-mod]: requests/endpoints/index.html
[RequestParams]: struct.RequestParams.html
[Client.request]: struct.Client.html#method.request
[Client.search]: struct.Client.html#method.search
[Client.get_document]: struct.Client.html#method.get_document
[Client.index_document]: struct.Client.html#method.index_document
[Client.put_mapping]: struct.Client.html#method.put_mapping
[Client.create_index]: struct.Client.html#method.create_index

[RequestBuilder]: requests/struct.RequestBuilder.html
[RequestBuilder.params]: requests/struct.RequestBuilder.html#method.params
[SearchRequest]: requests/endpoints/struct.SearchRequest.html
[GetRequest]: requests/endpoints/struct.GetRequest.html
[IndexRequest]: requests/endpoints/struct.IndexRequest.html
[IndicesPutMappingRequest]: requests/endpoints/struct.IndicesPutMappingRequest.html
[IndicesCreateRequest]: requests/endpoints/struct.IndicesCreateRequest.html

[responses-mod]: responses/index.html
[SyncResponseBuilder]: responses/struct.SyncResponseBuilder.html
[SyncResponseBuilder.into_response]: responses/struct.SyncResponseBuilder.html#method.into_response
[SyncResponseBuilder.into_raw]: responses/struct.SyncResponseBuilder.html#method.into_raw
[AsyncResponseBuilder]: responses/struct.AsyncResponseBuilder.html
[AsyncResponseBuilder.into_response]: responses/struct.AsyncResponseBuilder.html#method.into_response
[AsyncResponseBuilder.into_raw]: responses/struct.AsyncResponseBuilder.html#method.into_raw
[SearchResponse]: responses/type.SearchResponse.html
[GetResponse]: responses/type.GetResponse.html
[IndexResponse]: responses/struct.IndexResponse.html
[CommandResponse]: responses/struct.CommandResponse.html
[HttpResponse]: responses/struct.HttpResponse.html
[response-types]: responses/parse/trait.IsOk.html#implementors
*/

pub mod requests;
pub mod responses;

use serde::de::DeserializeOwned;
use futures_cpupool::CpuPool;
use tokio_core::reactor::Handle;
use elastic_reqwest::{SyncBody, AsyncBody};
use reqwest::{Client as SyncHttpClient, Response as SyncRawResponse, Error as ClientError};
use reqwest::unstable::async::{Client as AsyncHttpClient};

use error::*;
use self::responses::parse::IsOk;

pub use elastic_reqwest::RequestParams;

pub trait Sender: Clone {
    #[doc(hidden)]
    type Body;
}

#[derive(Clone)]
pub struct SyncSender {
    http: SyncHttpClient
}

impl SyncSender {
    fn new() -> Result<Self> {
        let http = SyncHttpClient::new()?;

        Ok(SyncSender {
            http: http
        })
    }
}

impl Sender for SyncSender {
    type Body = SyncBody;
}

/**
A builder for a client.
*/
pub struct SyncClientBuilder {
    http: Option<SyncHttpClient>,
    params: RequestParams
}

impl SyncClientBuilder {
    /**
    Create a new client builder.

    By default, a client constructed by this builder will:

    - Send requests to `localhost:9200`
    - Not use any authentication
    - Not use TLS
    */
    pub fn new() -> Self {
        SyncClientBuilder {
            http: None,
            params: RequestParams::default()
        }
    }

    /**
    Set the base url. 

    The url must be fully qualified.
    This method is a convenient alternative to using `params` to specify the `base_url`.

    # Examples

    Specify a base url for the client to send requests to.
    In this case, the base url is HTTPS, and not on the root path:

    ```
    # use elastic::prelude::*;
    let builder = SyncClientBuilder::new()
        .base_url("https://my_es_cluster/some_path");
    ```
    */
    pub fn base_url<I>(mut self, base_url: I) -> Self 
        where I: Into<String>
    {
        self.params = self.params.base_url(base_url);

        self
    }

    /**
    Specify default request parameters.
    
    # Examples
    
    Require all responses use pretty-printing:
    
    ```
    # use elastic::prelude::*;
    let builder = SyncClientBuilder::new()
        .params(|params| params.url_param("pretty", true));
    ```

    Add an authorization header:

    ```
    # use elastic::prelude::*;
    use elastic::http::header::Authorization;

    let builder = SyncClientBuilder::new()
        .params(|params| params.header(Authorization("let me in".to_owned())));
    ```

    Specify a base url (prefer the [`base_url`][SyncClientBuilder.base_url] method on `SyncClientBuilder` instead):

    ```
    # use elastic::prelude::*;
    let builder = SyncClientBuilder::new()
        .params(|params| params.base_url("https://my_es_cluster/some_path"));
    ```

    [SyncClientBuilder.base_url]: #method.base_url
    */
    pub fn params<F>(mut self, builder: F) -> Self
        where F: Fn(RequestParams) -> RequestParams
    {
        self.params = builder(self.params);

        self
    }

    /** Use the given `reqwest::Client` for sending requests. */
    pub fn http_client(mut self, client: SyncHttpClient) -> Self {
        self.http = Some(client);

        self
    }

    /** 
    Construct a [`Client`][Client] from this builder. 

    [Client]: struct.Client.html
    */
    pub fn build(self) -> Result<SyncClient> {
        if let Some(http) = self.http {
            Ok(SyncClient {
                sender: SyncSender {
                    http: http
                },
                params: self.params
            })
        } else {
            SyncClient::new(self.params)
        }
    }
}

#[derive(Clone)]
pub struct AsyncSender {
    http: AsyncHttpClient,
    de_pool: Option<CpuPool>
}

impl AsyncSender {
    fn new(handle: &Handle) -> Result<Self> {
        let http = AsyncHttpClient::new(handle)?;

        Ok(AsyncSender {
            http: http,
            de_pool: None
        })
    }
}

impl Sender for AsyncSender {
    type Body = AsyncBody;
}

pub struct AsyncClientBuilder {
    http: Option<AsyncHttpClient>,
    de_pool: Option<CpuPool>,
    params: RequestParams
}

impl AsyncClientBuilder {
    pub fn new() -> Self {
        let de_pool = CpuPool::new(4);

        AsyncClientBuilder {
            http: None,
            de_pool: Some(de_pool),
            params: RequestParams::default()
        }
    }

    pub fn base_url<I>(mut self, base_url: I) -> Self 
        where I: Into<String>
    {
        self.params = self.params.base_url(base_url);

        self
    }

    pub fn params<F>(mut self, builder: F) -> Self
        where F: Fn(RequestParams) -> RequestParams
    {
        self.params = builder(self.params);

        self
    }

    pub fn de_pool(mut self, de_pool: Option<CpuPool>) -> Self {
        self.de_pool = de_pool;

        self
    }

    pub fn http_client(mut self, client: AsyncHttpClient) -> Self {
        self.http = Some(client);

        self
    }

    pub fn build(self, handle: &Handle) -> Result<AsyncClient> {
        let http = self.http.map(|http| Ok(http)).unwrap_or(AsyncHttpClient::new(handle))?;

        Ok(AsyncClient {
            sender: AsyncSender {
                http: http,
                de_pool: self.de_pool,
            },
            params: self.params,
        })
    }
}

/**
A HTTP client for the Elasticsearch REST API.

The `Client` is a structure that lets you create and send [`RequestBuilder`][RequestBuilder]s.
It's mostly a thin wrapper over a `reqwest::Client` and is re-usable.

# Examples

Create a `Client` for an Elasticsearch node at `es_host:9200`:

```no_run
# use elastic::prelude::*;
let params = RequestParams::new("http://es_host:9200").url_param("pretty", true);

let client = Client::new(params).unwrap();

[RequestBuilder]: requests/index.html
```
*/
#[derive(Clone)]
pub struct Client<TSender> {
    sender: TSender,
    params: RequestParams,
}

impl Client<SyncSender> {
    /**
    Create a new synchronous client for the given parameters.
    
    The parameters given here are used as the defaults for any
    request made by this client, but can be overriden on a
    per-request basis.
    This method can return a `HttpError` if the underlying `reqwest::Client`
    fails to create.
    
    # Examples
    
    Create a `Client` with default parameters:
    
    ```
    # use elastic::prelude::*;
    let client = SyncClientBuilder::new().build().unwrap();
    ```
    
    Create a `Client` for a specific node:
    
    ```
    # use elastic::prelude::*;
    let client = Client::new(RequestParams::new("http://eshost:9200")).unwrap();
    ```
    
    See [`RequestParams`][RequestParams] for more configuration options.

    [RequestParams]: struct.RequestParams.html
    */
    fn new(params: RequestParams) -> Result<Self> {
        let http = SyncSender::new()?;

        Ok(Client {
               sender: http,
               params: params,
           })
    }
}

impl Client<AsyncSender> {
    /**
    Create a new asynchronous client for the given parameters.
    
    The parameters given here are used as the defaults for any
    request made by this client, but can be overriden on a
    per-request basis.
    This method can return a `HttpError` if the underlying `reqwest::Client`
    fails to create.
    
    # Examples
    
    Create a `Client` with default parameters:
    
    ```
    # use elastic::prelude::*;
    let client = SyncClientBuilder::new().build().unwrap();
    ```
    
    Create a `Client` for a specific node:
    
    ```
    # use elastic::prelude::*;
    let client = Client::new(RequestParams::new("http://eshost:9200")).unwrap();
    ```
    
    See [`RequestParams`][RequestParams] for more configuration options.

    [RequestParams]: struct.RequestParams.html
    */
    fn new(handle: &Handle, params: RequestParams) -> Result<Self> {
        let http = AsyncHttpClient::new(handle)?;

        Ok(Client {
               sender: AsyncSender {
                   http: http,
                   de_pool: None,   
               },
               params: params,
           })
    }
}

pub type SyncClient = Client<SyncSender>;
pub type AsyncClient = Client<AsyncSender>;
