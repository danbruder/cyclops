use warp::host::Authority;
use warp::{http::Response, hyper::body::Bytes, Filter, Rejection, Reply};

use warp_reverse_proxy::{extract_request_data_filter, proxy_to_and_forward_response};

async fn log_response(response: Response<Bytes>) -> Result<impl Reply, Rejection> {
    println!("{:?}", response);
    Ok(response)
}

#[tokio::main]
async fn main() {
    let hello = |name| warp::path!("hello").map(move || format!("Hello port foo, {}!", name));

    // // spawn base server
    tokio::spawn(warp::serve(hello("8080")).run(([0, 0, 0, 0], 8080)));
    tokio::spawn(warp::serve(hello("8081")).run(([0, 0, 0, 0], 8081)));
    tokio::spawn(warp::serve(hello("8082")).run(([0, 0, 0, 0], 8082)));

    let request_filter = extract_request_data_filter();
    let app = warp::host::optional()
        // build proxy address and base path data from current filter
        .map(|authority: Option<Authority>| {
            let port = match authority.as_ref().map(|a| a.host()) {
                Some("foo.danbruder.com") => 8080,
                Some("bar.danbruder.com") => 8081,
                Some(_) => 8082,
                None => 8082,
            };
            (format!("http://127.0.0.1:{}/", port), "".to_string())
        })
        .untuple_one()
        // build the request with data from previous filters
        .and(request_filter)
        .and_then(proxy_to_and_forward_response)
        .and_then(log_response);

    // spawn proxy server
    warp::serve(app).run(([0, 0, 0, 0], 3030)).await;
}
