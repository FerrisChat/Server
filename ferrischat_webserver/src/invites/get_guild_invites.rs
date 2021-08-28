use actix_web::{HttpRequest, HttpResponse, Responder};
use ferrischat_common::types::{InternalServerErrorJson, Invite, NotFoundJson};
use ferrischat_macros::get_db_or_fail;

/// GET api/v0/guilds/{guild_id}/invites
pub async fn get_guild_invites(req: HttpRequest, auth: crate::Authorization) -> impl Responder {
    let db = get_db_or_fail!();
    let guild_id = get_item_id!(req, "guild_id");
    let bigint_guild_id = u128_to_bigdecimal!(guild_id);

    let member_query = sqlx::query!(
        "SELECT user_id FROM members WHERE user_id = $1, guild_id = $2",
        auth.0,
        bigint_guild_id
    )
    .fetch_optional(db)
    .await;

    match member_query {
        Ok(member_id) => {
            if member_id.is_none() {
                return HttpResponse::Forbidden().finish();
            }
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                reason: format!("DB returned an error: {}", e),
            })
        }
    }

    let invites = {
        let resp = sqlx::query!("SELECT * FROM invites WHERE guild_id = $1", guild_id)
            .fetch_all(db)
            .await;

        match resp {
            Ok(resp) => resp
                .iter()
                .filter_map(|invite| {
                    Some(Invite {
                        code: invite.code.clone(),
                        owner_id: invite
                            .owner_id
                            .with_scale(0)
                            .into_bigint_and_exponent()
                            .0
                            .to_u128()?,
                        guild_id: invite
                            .guild_id
                            .with_scale(0)
                            .into_bigint_and_exponent()
                            .0
                            .to_u128()?,
                        created_at: invite.created_at,
                        uses: invite.uses,
                        max_uses: invite.max_uses,
                        max_age: invite.max_age,
                    })
                })
                .collect(),
            Err(e) => {
                return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                    reason: format!("DB returned an error: {}", e),
                })
            }
        }
    };

    HttpResponse::Ok().json(invites)
}
