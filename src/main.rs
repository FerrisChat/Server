#[tokio::main]
async fn main() {
    ferrischat_http::start()
        .await
        .expect("could not start HTTP server");
}
