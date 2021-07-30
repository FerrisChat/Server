use ferrischat_webserver::entrypoint;

#[tokio::main]
async fn main() {
    entrypoint().await;
}
