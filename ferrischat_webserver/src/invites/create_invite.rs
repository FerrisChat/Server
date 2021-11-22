use crate::ws::{fire_event, WsEventError};
use actix_web::web::Json;
use actix_web::{HttpRequest, HttpResponse, Responder};
use ferrischat_common::request_json::InviteCreateJson;
use ferrischat_common::types::{InternalServerErrorJson, Invite};
use ferrischat_common::ws::WsOutboundEvent;
use ferrischat_snowflake_generator::FERRIS_EPOCH;
use sqlx::types::time::OffsetDateTime;

/// POST `/api/v0/guilds/{guild_id}/invites`
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

    let now = OffsetDateTime::now_utc().unix_timestamp()
        - (i64::try_from(FERRIS_EPOCH).expect("failed to cast the Ferris Epoch to i64"));
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
                    is_bug: false,
                    link: None,
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

    let invite_obj = match resp {
        Ok(code) => Invite {
            code: code.code,
            owner_id,
            guild_id,
            created_at: now,
            uses: 0,
            max_uses,
            max_age,
        },
        Err(e) => {
            return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                reason: format!("DB returned an error: {}", e),
                is_bug: false,
                link: None,
            })
        }
    };

    let event = WsOutboundEvent::InviteCreate {
        invite: invite_obj.clone(),
    };

    if let Err(e) = fire_event(format!("invite_{}", guild_id), &event).await {
        let reason = match e {
            WsEventError::MissingRedis => "Redis pool missing".to_string(),
            WsEventError::RedisError(e) => format!("Redis returned an error: {}", e),
            WsEventError::JsonError(e) => {
                format!("Failed to serialize message to JSON format: {}", e)
            }
        };
        return HttpResponse::InternalServerError().json(InternalServerErrorJson {
            reason,
            is_bug: true,
            link: Some(
                "https://github.com/FerrisChat/Server/issues/new?assignees=tazz4843&\
                labels=bug&template=api_bug_report.yml&title=%5B500%5D%3A+failed+to+fire+event"
                    .to_string(),
            ),
        });
    }

    HttpResponse::Created().json(invite_obj)
}
