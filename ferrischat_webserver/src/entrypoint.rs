use crate::auth::get_token;
use crate::channels::*;
use crate::guilds::*;
use crate::members::*;
use crate::messages::*;
use crate::not_implemented::not_implemented;
use crate::users::*;
use crate::ws::*;
use actix_web::{web, App, HttpResponse, HttpServer};
use ferrischat_db::load_db;
use ferrischat_macros::expand_version;
use ferrischat_redis::load_redis;
use ring::rand::{SecureRandom, SystemRandom};
use tokio::sync::mpsc::channel;

#[allow(clippy::expect_used)]
pub async fn entrypoint() {
    // the very, very first thing we should do is load the RNG
    // we expect here, since without it we literally cannot generate tokens whatsoever
    crate::RNG_CORE
        .set(SystemRandom::new())
        .expect("failed to set RNG");
    {
        let mut v = Vec::with_capacity(32);
        // we call fill here to be sure that the RNG will block if required here instead of
        // in the webserver loop
        crate::RNG_CORE
            .get()
            .expect("RNG was already set but unloaded?")
            .fill(&mut v)
            .expect("failed to generate RNG");
    }

    {
        let (tx, mut rx) = channel::<(
            String,
            tokio::sync::oneshot::Sender<Result<String, argonautica::Error>>,
        )>(250);
        let mut hasher = argonautica::Hasher::new();
        hasher
            .opt_out_of_secret_key(true) // we don't need secret keys atm
            .configure_password_clearing(true) // clear passwords from memory after hashing
            .configure_memory_size(8_192); // use 8MiB memory to hash

        std::thread::spawn(move || {
            while let Some(d) = rx.blocking_recv() {
                let (password, sender) = d;

                let r = hasher.with_password(password).hash();
                let _ = sender.send(r);
            }
        });

        crate::GLOBAL_HASHER
            .set(tx)
            .expect("couldn't set global hasher for some reason");
    }
    {
        let (tx, mut rx) = channel::<(
            (String, String),
            tokio::sync::oneshot::Sender<Result<bool, argonautica::Error>>,
        )>(250);
        let mut verifier = argonautica::Verifier::new();
        verifier
            .configure_password_clearing(true)
            .configure_secret_key_clearing(true);

        std::thread::spawn(move || {
            while let Some(d) = rx.blocking_recv() {
                let (password, sender) = d;

                let r = verifier
                    .with_password(password.0)
                    .with_hash(password.1)
                    .verify();
                let _ = sender.send(r);
            }
        });

        crate::GLOBAL_VERIFIER
            .set(tx)
            .expect("failed to set password verifier");
    }

    load_redis().await;
    load_db().await;

    HttpServer::new(|| {
        App::new()
            // POST   /guilds
            .route(expand_version!("guilds"), web::post().to(create_guild))
            // GET    /guilds/{guild_id}
            .route(
                expand_version!("guilds/{guild_id}"),
                web::get().to(get_guild),
            )
            // PATCH  /guilds/{guild_id}
            .route(
                expand_version!("guilds/{guild_id}"),
                web::patch().to(not_implemented),
            )
            // DELETE /guilds/{guild_id}
            .route(
                expand_version!("guilds/{guild_id}"),
                web::delete().to(delete_guild),
            )
            // POST   guilds/{guild_id}/channels
            .route(
                expand_version!("guilds/{guild_id}/channels"),
                web::post().to(create_channel),
            )
            // GET    channels/{channel_id}
            .route(
                expand_version!("channels/{channel_id}"),
                web::get().to(get_channel),
            )
            // PATCH  channels/{channel_id}
            .route(
                expand_version!("channels/{channel_id}"),
                web::patch().to(not_implemented),
            )
            // DELETE channels/{channel_id}
            .route(
                expand_version!("channels/{channel_id}"),
                web::delete().to(delete_channel),
            )
            // POST   channels/{channel_id}/messages
            .route(
                expand_version!("channels/{channel_id}/messages"),
                web::post().to(create_message),
            )
            // GET    channels/{channel_id}/messages
            .route(
                expand_version!("channels/{channel_id}/messages"),
                web::get().to(get_message_history),
            )
            // GET     channels/{channel_id}/messages/{message_id}
            .route(
                expand_version!("channels/{channel_id}/messages/{message_id}"),
                web::get().to(get_message),
            )
            // PATCH  channels/{channel_id}/messages/{message_id}
            .route(
                expand_version!("channels/{message_id}"),
                web::patch().to(not_implemented),
            )
            // DELETE channels/{channel_id}/messages/{message_id}
            .route(
                expand_version!("channels/{channel_id}/messages/{message_id}"),
                web::delete().to(delete_message),
            )
            // POST   guilds/{guild_id}/members
            .route(
                expand_version!("guilds/{guild_id}/members"),
                web::post().to(not_implemented),
            )
            // GET    guilds/{guild_id}/members/{member_id}
            .route(
                expand_version!("guilds/{guild_id}/members/{member_id}"),
                web::get().to(not_implemented),
            )
            // PATCH  guilds/{guild_id}/members/{member_id}
            .route(
                expand_version!("guilds/{guild_id}/members/{member_id}"),
                web::patch().to(not_implemented),
            )
            // DELETE guilds/{guild_id}/members/{member_id}
            .route(
                expand_version!("guilds/{guild_id}/members/{member_id}"),
                web::delete().to(not_implemented),
            )
            // POST   /users/
            .route(expand_version!("users"), web::post().to(create_user))
            // GET    /users/{user_id}
            .route(expand_version!("users/{user_id}"), web::get().to(get_user))
            // PATCH  /users/{user_id}
            .route(
                expand_version!("users/{user_id}"),
                web::patch().to(not_implemented),
            )
            // DELETE /users/{user_id}
            .route(
                expand_version!("users/{user_id}"),
                web::delete().to(not_implemented),
            )
            // POST    /auth/{user_id}
            .route(expand_version!("auth/{user_id}"), web::post().to(get_token))
            // GET     /ws/info
            .route(expand_version!("ws/info"), web::get().to(ws_info))
            // GET     /ws/connect
            .route(expand_version!("ws/connect"), web::get().to(ws_connect))
            .default_service(web::route().to(HttpResponse::NotFound))
        // TODO: member and message endpoints
    })
    .bind("0.0.0.0:8080")
    .expect("failed to bind to 0.0.0.0:8080")
    .run()
    .await
    .expect("failed to run server")
}
