pub async fn init_auth() {
    let config = argon2_async::Config::new_insecure();

    argon2_async::set_config(config).await;
}
