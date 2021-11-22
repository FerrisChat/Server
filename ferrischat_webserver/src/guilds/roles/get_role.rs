use actix_web::{HttpRequest, HttpResponse, Responder};
use ferrischat_common::perms::Permissions;
use ferrischat_common::types::{InternalServerErrorJson, NotFoundJson, Role};

/// GET `/api/v0/guilds/{guild_id/roles/{role_id}`
pub async fn get_role(req: HttpRequest, _: crate::Authorization) -> impl Responder {
    let db = get_db_or_fail!();
    let role_id = u128_to_bigdecimal!(get_item_id!(req, "role_id"));
    let resp = sqlx::query!("SELECT * FROM roles WHERE id = $1", role_id)
        .fetch_optional(db)
        .await;

    match resp {
        Ok(resp) => match resp {
            Some(role) => HttpResponse::Ok().json(Role {
                id: bigdecimal_to_u128!(role.id),
                name: role.name,
                color: role.color,
                position: role.position,
                guild_id: bigdecimal_to_u128!(role.parent_guild),
                permissions: Permissions::from_bits_truncate(role.permissions),
            }),
            None => HttpResponse::NotFound().json(NotFoundJson {
                message: format!("Unknown role with id {}", role_id),
            }),
        },
        Err(e) => HttpResponse::InternalServerError().json(InternalServerErrorJson {
            reason: format!("database returned a error: {}", e),
            is_bug: false,
            link: None,
        }),
    }
}
