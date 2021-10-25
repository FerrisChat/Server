use clap::{crate_authors, crate_description, crate_name, crate_version, App, Arg};
use ferrischat_webserver::entrypoint;

#[actix_web::main]
async fn main() {
    #[cfg(target_arch = "x86_64")]
    if !is_x86_feature_detected!("pclmulqdq") {
        eprintln!("Your CPU doesn't support `pclmulqdq`. Exiting.");
        std::process::abort()
    }

    let app = clap::app_from_crate!().arg(
        Arg::with_name("config")
            .validator(|val| {
                std::path::PathBuf::from(val).exists().then(|| ()).map_or(
                    Err("config file path provided is nonexistent".to_string()),
                    |_| Ok(()),
                )
            })
            .takes_value(true)
            .value_name("FILE")
            .help("Path to configuration file")
            .default_value("./config.toml"),
    );
    let matches = app.get_matches();
    let cfg_path = matches
        .value_of("config")
        .expect("unexpected missing config file path")
        .into();
    ferrischat_config::load_config(cfg_path);
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    entrypoint().await;
}
