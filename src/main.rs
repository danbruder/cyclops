use warp::{http::Response, hyper::body::Bytes, Filter, Rejection, Reply};
use warp_reverse_proxy::reverse_proxy_filter;

async fn log_response(response: Response<Bytes>) -> Result<impl Reply, Rejection> {
    println!("{:?}", response);
    Ok(response)
}

#[tokio::main]
async fn main() {
    let hello = warp::path!("hello" / String).map(|name| format!("Hello, {}!", name));

    // // spawn base server
    tokio::spawn(warp::serve(hello).run(([0, 0, 0, 0], 8080)));

    // Forward request to localhost in other port
    let app = warp::path!("hello" / ..).and(
        reverse_proxy_filter("".to_string(), "http://127.0.0.1:8080/".to_string())
            .and_then(log_response),
    );

    // spawn proxy server
    warp::serve(app).run(([0, 0, 0, 0], 3030)).await;
}
