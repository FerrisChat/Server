use actix_web::web::Json;
use actix_web::{HttpRequest, HttpResponse, Responder};
use ferrischat_common::request_json::InviteCreateJson;
use ferrischat_common::types::{InternalServerErrorJson, Invite};
use sqlx::types::time::OffsetDateTime;

const FERRIS_EPOCH: i64 = 1_577_836_800;

/// POST /api/v0/guilds/{guild_id}/invites
pub async fn create_invite(
    auth: crate::Authorization,
    invite_info: Json<InviteCreateJson>,
    req: HttpRequest,
) -> impl Responder {
    let db = get_db_or_fail!();
    let InviteCreateJson { max_age, max_uses } = invite_info.0;

    let guild_id = get_item_id!(req, "guild_id");
    let bigint_guild_id = u128_to_bigdecimal!(guild_id);

    let owner_id = auth.0;
    let bigint_owner_id = u128_to_bigdecimal!(owner_id);

    let now = OffsetDateTime::now_utc().unix_timestamp() - FERRIS_EPOCH;
    {
        let resp = sqlx::query!(
            "SELECT user_id FROM members WHERE user_id = $1 AND guild_id = $2",
            bigint_owner_id,
            bigint_guild_id
        )
        .fetch_optional(db)
        .await;

        match resp {
            Ok(resp) => match resp {
                Some(_) => (),
                None => {
                    return HttpResponse::Forbidden().finish();
                }
            },
            Err(e) => {
                return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                    reason: format!("DB returned an error: {}", e),
                });
            }
        }
    }
    let resp = sqlx::query!(
        "INSERT INTO invites VALUES ((SELECT array_to_string( \
            ARRAY(SELECT substr( \
                'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789', \
                ((random()*(36-1)+1)::integer),1) FROM generate_series(1,10)),'') \
            ), $1, $2, $3, $4, $5, $6) RETURNING code",
        bigint_owner_id,
        bigint_guild_id,
        now,
        0,
        max_uses,
        max_age
    )
    .fetch_one(db)
    .await;

    match resp {
        Ok(code) => HttpResponse::Created().json(Invite {
            code: code.code,
            owner_id: owner_id,
            guild_id: guild_id,
            created_at: now,
            uses: 0,
            max_uses: max_uses,
            max_age: max_age,
        }),
        Err(e) => HttpResponse::InternalServerError().json(InternalServerErrorJson {
            reason: format!("DB returned an error: {}", e),
        }),
    }
}
