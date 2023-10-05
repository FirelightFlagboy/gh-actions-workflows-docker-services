use reqwest::{
    header::{HeaderMap, HeaderValue, ACCEPT, USER_AGENT},
    ClientBuilder,
};

pub fn prepare_http_client_json() -> ClientBuilder {
    ClientBuilder::default()
        .gzip(true)
        .default_headers(HeaderMap::from_iter([
            (USER_AGENT, HeaderValue::from_static("pkg-info-updater")),
            (ACCEPT, HeaderValue::from_static("application/json")),
        ]))
}
