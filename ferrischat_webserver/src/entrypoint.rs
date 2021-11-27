#![allow(clippy::wildcard_imports)]

use axum::routing::{delete, get, patch, post};
use axum::Router;
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

    let router = Router::new()
        // POST   /guilds
        .route(expand_version!("guilds"), post(create_guild))
        // GET    /guilds/:guild_id
        .route(expand_version!("guilds/:guild_id"), get(get_guild))
        // PATCH  /guilds/:guild_id
        .route(expand_version!("guilds/:guild_id"), patch(edit_guild))
        // DELETE /guilds/:guild_id
        .route(expand_version!("guilds/:guild_id"), delete(delete_guild))
        // POST   guilds/:guild_id/channels
        .route(
            expand_version!("guilds/:guild_id/channels"),
            post(create_channel),
        )
        // GET    channels/:channel_id
        .route(expand_version!("channels/:channel_id"), get(get_channel))
        // PATCH  channels/:channel_id
        .route(expand_version!("channels/:channel_id"), patch(edit_channel))
        // DELETE channels/:channel_id
        .route(
            expand_version!("channels/:channel_id"),
            delete(delete_channel),
        )
        // POST   channels/:channel_id/messages
        .route(
            expand_version!("channels/:channel_id/messages"),
            post(create_message),
        )
        // GET    channels/:channel_id/messages
        .route(
            expand_version!("channels/:channel_id/messages"),
            get(get_message_history),
        )
        // GET     channels/:channel_id/messages/:message_id
        .route(
            expand_version!("channels/:channel_id/messages/:message_id"),
            get(get_message),
        )
        // PATCH  channels/:channel_id/messages/:message_id
        .route(
            expand_version!("channels/:channel_id/messages/:message_id"),
            patch(edit_message),
        )
        // DELETE channels/:channel_id/messages/:message_id
        .route(
            expand_version!("channels/:channel_id/messages/:message_id"),
            delete(delete_message),
        )
        // GET    guilds/:guild_id/members/:member_id
        .route(
            expand_version!("guilds/:guild_id/members/:member_id"),
            get(get_member),
        )
        // PATCH  guilds/:guild_id/members/:member_id
        .route(
            expand_version!("guilds/:guild_id/members/:member_id"),
            patch(not_implemented),
        )
        // DELETE guilds/:guild_id/members/:member_id
        .route(
            expand_version!("guilds/:guild_id/members/:member_id"),
            delete(delete_member),
        )
        // POST guilds/:guild_id/invites
        .route(
            expand_version!("guilds/:guild_id/invites"),
            post(create_invite),
        )
        // GET guilds/:guild_id/invites
        .route(
            expand_version!("guilds/:guild_id/invites"),
            get(get_guild_invites),
        )
        // GET /invites/:code
        .route(expand_version!("invites/:code"), get(get_invite))
        // POST /invites/:code
        .route(expand_version!("invites/:code"), post(use_invite))
        // POST   /users/
        .route(expand_version!("users"), post(create_user))
        // GET    /users/:user_id
        .route(expand_version!("users/:user_id"), get(get_user))
        // PATCH  /users/:user_id
        .route(expand_version!("users/:user_id"), patch(edit_user))
        // DELETE /users/:user_id
        .route(expand_version!("users/:user_id"), delete(delete_user))
        // POST /users/:user_id/bots
        .route(expand_version!("users/:user_id/bots"), post(create_bot))
        // GET /users/:user_id/bots
        .route(
            expand_version!("users/:user_id/bots"),
            get(get_bots_by_user),
        )
        // PATCH /users/:user_id/bots/:bot_id
        .route(
            expand_version!("users/:user_id/bots/:bot_id"),
            patch(edit_bot),
        )
        // DELETE /users/:user_id/bots/:bot_id
        .route(
            expand_version!("users/:user_id/bots/:bot_id"),
            delete(delete_bot),
        )
        // POST     /users/:user_id/bots/:bot_id/auth
        .route(
            expand_version!("users/:user_id/bots/:bot_id/auth"),
            post(get_bot_token),
        )
        // POST    /auth
        .route(expand_version!("auth"), post(get_token))
        // GET     /ws/info
        .route(expand_version!("ws/info"), get(ws_info))
        .route(
            expand_version!("guilds/:guild_id/roles"),
            post(roles::create_role),
        )
        .route(
            expand_version!("guilds/:guild_id/roles/:role_id"),
            delete(roles::delete_role),
        )
        .route(
            expand_version!("guilds/:guild_id/roles/:role_id"),
            patch(roles::edit_role),
        )
        .route(
            expand_version!("guilds/:guild_id/roles/:role_id"),
            get(roles::get_role),
        )
        .route(
            expand_version!("guilds/:guild_id/members/:user_id/role/:role_id"),
            post(roles::add_member_role),
        )
        .route(
            expand_version!("guilds/:guild_id/members/:user_id/role/:role_id"),
            delete(roles::remove_member_role),
        )
        .route(expand_version!("verify"), post(send_verification_email))
        .route(expand_version!("verify/{token}"), get(verify_email))
        .route(
            expand_version!("teapot"),
            get(async || HttpResponse::new(StatusCode::IM_A_TEAPOT)),
        )
        .route(
            expand_version!("ping"),
            get(async || HttpResponse::new(StatusCode::OK)),
        );

    let stream = tokio::net::UnixListener::bind(format!(
        "{}/webserver.sock",
        std::env::var("FERRISCHAT_HOME").unwrap_or_else(|_| "/etc/ferrischat/".to_string())
    ))
    .expect("failed to bind to unix socket");
    axum::Server::builder(stream)
        .serve(router.into_make_service())
        .await
        .expect("failed to start HTTP server");
}
