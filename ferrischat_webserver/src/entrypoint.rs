use crate::channels::*;
use crate::guilds::*;
use crate::members::*;
use crate::messages::*;
use crate::users::*;
use ferrischat_db::load_db;
use rocket::response::status;
use rocket::{Ignite, Rocket};

#[post("/")]
/// POST `/api/v1/guilds/`
async fn private_create_guild() -> status::Custom<&'static str> {
    create_guild().await
}

#[get("/<id>/get")]
/// GET `/api/v1/guilds/<id>/get`
async fn private_get_guild(id: u64) -> status::Custom<&'static str> {
    get_guild(id).await
}

#[delete("/<id>/delete")]
/// DELETE `/api/v1/guilds/<id>/delete`
async fn private_delete_guild(id: u64) -> status::Custom<&'static str> {
    delete_guild(id).await
}

#[post("/")]
/// POST `/api/v1/users/`
async fn private_create_user() -> status::Custom<&'static str> {
    create_user().await
}

#[get("/<id>/get")]
/// GET `/api/v1/users/<id>/get`
async fn private_get_user(id: u64) -> status::Custom<&'static str> {
    get_user(id).await
}

#[delete("/<id>/delete")]
/// DELETE `/api/v1/users/<id>/delete`
async fn private_delete_user(id: u64) -> status::Custom<&'static str> {
    delete_user(id).await
}

#[post("/")]
/// POST `/api/v1/channels/`
async fn private_create_channel() -> status::Custom<&'static str> {
    create_channel().await
}

#[get("/<id>/get")]
/// GET `/api/v1/channels/<id>/get`
async fn private_get_channel(id: u64) -> status::Custom<&'static str> {
    get_channel(id).await
}

#[delete("/<id>/delete")]
/// DELETE `/api/v1/channels/<id>/delete`
async fn private_delete_channel(id: u64) -> status::Custom<&'static str> {
    delete_channel(id).await
}

#[post("/<id>/send")]
/// POST `/api/v1/channel/<id>/send`
async fn private_send_message(id: u64) -> status::Custom<&'static str> {
    send_message(id).await
}

#[get("/<id>/get")]
/// GET `/api/v1/message/<id>/get`
async fn private_get_message(id: u64) -> status::Custom<&'static str> {
    get_message(id).await
}

#[get("/<id>/delete")]
/// DELETE `/api/v1/message/<id>/delete`
async fn private_delete_message(id: u64) -> status::Custom<&'static str> {
    delete_message(id).await
}

#[post("/<id>/join")]
/// POST `/api/v1/guilds/<id>/join`
async fn private_join_guild(id: u64) -> status::Custom<&'static str> {
    create_member(id).await
}

#[get("/<id>/get")]
/// GET `/api/v1/member/<id>/get`
async fn private_get_member(id: u64) -> status::Custom<&'static str> {
    get_member(id).await
}

#[delete("/<id>/delete")]
/// DELETE `/api/v1/member/<id>/delete`
async fn private_delete_member(id: u64) -> status::Custom<&'static str> {
    delete_member(id).await
}

pub async fn entrypoint() {
    load_db().await;

    rocket::build()
        .mount(
            "/api/v1/guilds",
            routes![
                private_create_guild,
                private_get_guild,
                private_delete_guild,
                private_join_guild
            ],
        )
        .mount(
            "/api/v1/user",
            routes![private_create_user, private_get_user, private_delete_user],
        )
        .mount(
            "/api/v1/channels",
            routes![
                private_create_channel,
                private_get_channel,
                private_delete_channel,
                private_send_message
            ],
        )
        .mount(
            "/api/v1/message",
            routes![private_get_message, private_delete_message],
        )
        .mount(
            "/api/v1/member",
            routes![private_get_member, private_delete_member],
        )
        .launch()
        .await
        .expect("failed to launch rocket!")
}
