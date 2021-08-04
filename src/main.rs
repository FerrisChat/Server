use ferrischat_webserver::entrypoint;

#[actix_web::main]
async fn main() {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    entrypoint().await;
}
