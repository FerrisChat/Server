use actix_web::{HttpRequest, HttpResponse, Responder};
use ferrischat_common::types::{Invite, InternalServerErrorJson, NotFoundJson}
use ferrischat_macros::get_db_or_fail;

/// GET api/v0/invites/{code}
pub async fn get_invite(req: HttpRequest, _: crate::Authorization) -> impl Responder {
    let db = get_db_or_fail!();
    let code = {
        use actix_web::HttpResponse;
        match req.match_info().get("code") {
          Some(c) => c,
          None => {
            return HttpResponse::InternalServerError().json(InternalServerErrorJson {
              reason: "\"code\" not found in match_info: this is a bug, please report it at \
              https://github.com/FerrisChat/Server/issues/new?assignees=tazz4843&labels=bug&\
              template=api_bug_report.yml&title=%5B500%5D%3A+code+not+found+in+match_info".to_string(),
            })
          }
        }
      };
    let resp = sqlx::query!("SELECT * FROM invites WHERE code = $1", code)
        .fetch_optional(db)
        .await;
    
    match resp {
        Ok(resp) => match resp {
            Some(invite) => HttpResponse::Ok().json(Invite {
                code: code,
                owner_id: bigdecimal_to_u128!(invite.owner_id),
                guild_id: bigdecimal_to_u128!(invite.guild_id),
                created_at: invite.created_at,
                uses: invite.uses
                max_uses: invite.max_uses,
                max_age: invite.max_age
            }),
            None => HttpResponse::NotFound().json(NotFoundJson {
                message: "Invite Not Found".to_string(),
            }),
        },
        Err(e) => HttpResponse::InternalServerError().json(InternalServerErrorJson {
            reason: format!("DB returned an error: {}", e),
        }),
    }
}