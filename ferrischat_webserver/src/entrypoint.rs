#![allow(clippy::wildcard_imports)]

use actix_web::http::StatusCode;
use actix_web::{web, App, HttpResponse, HttpServer};
use ring::rand::{SecureRandom, SystemRandom};

use ferrischat_auth::init_auth;
use ferrischat_db::load_db;
use ferrischat_macros::expand_version;
use ferrischat_redis::load_redis;
use ferrischat_ws::{init_ws, init_ws_server};

use crate::auth::*;
use crate::channels::*;
use crate::guilds::*;
use crate::invites::*;
use crate::members::*;
use crate::messages::*;
use crate::not_implemented::not_implemented;
use crate::users::*;
use crate::ws::*;

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

    init_auth();
    load_redis().await;
    load_db().await;
    init_ws().await;
    init_ws_server().await;

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
                web::patch().to(edit_guild),
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
                web::patch().to(edit_channel),
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
                expand_version!("channels/{channel_id}/messages/{message_id}"),
                web::patch().to(edit_message),
            )
            // DELETE channels/{channel_id}/messages/{message_id}
            .route(
                expand_version!("channels/{channel_id}/messages/{message_id}"),
                web::delete().to(delete_message),
            )
            // GET    guilds/{guild_id}/members/{member_id}
            .route(
                expand_version!("guilds/{guild_id}/members/{member_id}"),
                web::get().to(get_member),
            )
            // PATCH  guilds/{guild_id}/members/{member_id}
            .route(
                expand_version!("guilds/{guild_id}/members/{member_id}"),
                web::patch().to(not_implemented),
            )
            // DELETE guilds/{guild_id}/members/{member_id}
            .route(
                expand_version!("guilds/{guild_id}/members/{member_id}"),
                web::delete().to(delete_member),
            )
            // POST guilds/{guild_id}/invites
            .route(
                expand_version!("guilds/{guild_id}/invites"),
                web::post().to(create_invite),
            )
            // GET guilds/{guild_id}/invites
            .route(
                expand_version!("guilds/{guild_id}/invites"),
                web::get().to(get_guild_invites),
            )
            // GET /invites/{code}
            .route(expand_version!("invites/{code}"), web::get().to(get_invite))
            // POST /invites/{code}
            .route(
                expand_version!("invites/{code}"),
                web::post().to(use_invite),
            )
            // POST   /users/
            .route(expand_version!("users"), web::post().to(create_user))
            // GET    /users/{user_id}
            .route(expand_version!("users/{user_id}"), web::get().to(get_user))
            // PATCH  /users/{user_id}
            .route(
                expand_version!("users/{user_id}"),
                web::patch().to(edit_user),
            )
            // DELETE /users/{user_id}
            .route(
                expand_version!("users/{user_id}"),
                web::delete().to(delete_user),
            )
            // POST /users/{user_id}/bots
            .route(
                expand_version!("users/{user_id}/bots"),
                web::post().to(create_bot),
            )
            // GET /users/{user_id}/bots
            .route(
                expand_version!("users/{user_id}/bots"),
                web::get().to(get_bots_by_user),
            )
            // PATCH /users/{user_id}/bots/{bot_id}
            .route(
                expand_version!("users/{user_id}/bots/{bot_id}"),
                web::patch().to(edit_bot),
            )
            // DELETE /users/{user_id}/bots/{bot_id}
            .route(
                expand_version!("users/{user_id}/bots/{bot_id}"),
                web::delete().to(delete_bot),
            )
            // POST     /users/{user_id}/bots/{bot_id}/auth
            .route(
                expand_version!("users/{user_id}/bots/{bot_id}/auth"),
                web::post().to(get_bot_token),
            )
            // POST    /auth
            .route(expand_version!("auth"), web::post().to(get_token))
            // GET     /ws/info
            .route(expand_version!("ws/info"), web::get().to(ws_info))
            .route(
                expand_version!("guilds/{guild_id}/roles"),
                web::post().to(roles::create_role),
            )
            .route(
                expand_version!("guilds/{guild_id}/roles/{role_id}"),
                web::delete().to(roles::delete_role),
            )
            .route(
                expand_version!("guilds/{guild_id}/roles/{role_id}"),
                web::patch().to(roles::edit_role),
            )
            .route(
                expand_version!("guilds/{guild_id}/roles/{role_id}"),
                web::get().to(roles::get_role),
            )
            .route(
                expand_version!("guilds/{guild_id}/members/{user_id}/role/{role_id}"),
                web::post().to(roles::add_member_role),
            )
            .route(
                expand_version!("guilds/{guild_id}/members/{user_id}/role/{role_id}"),
                web::delete().to(roles::remove_member_role),
            )
            .route(
                expand_version!("verify"),
                web::post().to(send_verification_email),
            )
            .route(
                expand_version!("verify/{token}"),
                web::get().to(verify_email),
            )
            .route(
                expand_version!("teapot"),
                web::get().to(async || HttpResponse::new(StatusCode::IM_A_TEAPOT)),
            )
            .route(
                expand_version!("ping"),
                web::get().to(async || HttpResponse::new(StatusCode::OK)),
            )
            .default_service(web::route().to(HttpResponse::NotFound))
    })
    .max_connections(250_000)
    .max_connection_rate(8192)
    .bind_uds("/etc/ferrischat/knit.sock")
    .expect("failed to bind to knit sock!")
    .run()
    .await
    .expect("failed to run server");
}
