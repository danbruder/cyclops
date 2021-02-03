use shiplift::{ContainerOptions, Docker};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use warp::filters::path::FullPath;
use warp::host::Authority;
use warp::{
    http::{HeaderMap, Response},
    hyper::body::Bytes,
    Filter, Rejection, Reply,
};

use warp_reverse_proxy::{
    proxy_to_and_forward_response, query_params_filter, Method, QueryParameters,
};

async fn log_response(response: Response<Bytes>) -> Result<impl Reply, Rejection> {
    println!("{:?}", response);
    Ok(response)
}

type State = Arc<RwLock<HashMap<String, usize>>>;

#[tokio::main]
async fn main() {
    let hello = |name| warp::path!("hello").map(move || format!("Hello port foo, {}!", name));

    // // spawn base server
    tokio::spawn(warp::serve(hello("8080")).run(([0, 0, 0, 0], 8080)));
    tokio::spawn(warp::serve(hello("8081")).run(([0, 0, 0, 0], 8081)));
    tokio::spawn(warp::serve(hello("8082")).run(([0, 0, 0, 0], 8082)));

    let mut inner = HashMap::new();
    inner.insert("foo.danbruder.com".into(), 8080);
    inner.insert("bar.danbruder.com".into(), 8081);
    let state: State = Arc::new(RwLock::new(inner));
    let state = warp::any().map(move || state.clone());

    let app = warp::host::optional()
        .and(state)
        .and(warp::path::full())
        .and(query_params_filter())
        .and(warp::method())
        .and(warp::header::headers_cloned())
        .and(warp::body::bytes())
        // build proxy address and base path data from current filter
        .and_then(
            |authority: Option<Authority>,
             state: State,
             uri: FullPath,
             params: QueryParameters,
             method: Method,
             headers: HeaderMap,
             body: Bytes| async move {
                let docker = Docker::new();
                let _ = docker
                    .containers()
                    .create(
                        &ContainerOptions::builder("nginxdemos/hello")
                            .expose(80, "http", 4545)
                            .build(),
                    )
                    .await
                    .unwrap();

                let port = 4545;

                proxy_to_and_forward_response(
                    format!("http://127.0.0.1:{}/", port),
                    "".into(),
                    uri,
                    params,
                    method,
                    headers,
                    body,
                )
                .await
            },
        )
        .and_then(log_response);

    // spawn proxy server
    warp::serve(app).run(([0, 0, 0, 0], 3030)).await;
}
