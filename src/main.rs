use ferrischat_webserver::entrypoint;

#[actix_web::main]
async fn main() {
    #[cfg(target_arch = "x86_64")]
    if !is_x86_feature_detected!("pclmulqdq") {
        eprintln!("Your CPU doesn't support `pclmulqdq`. Exiting.");
        std::process::abort()
    }
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    entrypoint().await;
}
