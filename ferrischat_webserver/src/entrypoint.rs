use crate::channels::*;
use crate::guilds::*;
use crate::members::*;
use crate::messages::*;
use crate::not_implemented::not_implemented;
use crate::users::*;
use actix_web::{web, App, HttpResponse, HttpServer};
use ferrischat_db::load_db;
use ferrischat_macros::expand_version;

pub async fn entrypoint() {
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
            // POST   /guilds/{guild_id}/channels
            .route(
                expand_version!("guilds/{guild_id}/channels"),
                web::post().to(create_channel),
            )
            // GET    /channels/{channel_id}
            .route(
                expand_version!("channels/{channel_id}"),
                web::get().to(get_channel),
            )
            // PATCH  /channels/{channel_id}
            .route(
                expand_version!("channels/{channel_id}"),
                web::patch().to(not_implemented),
            )
            // DELETE /channels/{channel_id}
            .route(
                expand_version!("channels/{channel_id}"),
                web::delete().to(delete_channel),
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
            // DELETE /users/{channel_id}
            .route(
                expand_version!("users/{user_id}"),
                web::delete().to(delete_user),
            )
            .default_service(web::route().to(|| HttpResponse::NotFound()))
        // TODO: member and message endpoints
    })
    .bind("0.0.0.0:8080")
    .expect("failed to bind to 0.0.0.0:8080")
    .run()
    .await
    .expect("failed to run server")
}
