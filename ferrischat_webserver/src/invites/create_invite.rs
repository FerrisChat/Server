use actix_web::web::Json;
use actix_web::{HttpRequest, HttpResponse, Responder};
use ferrischat_macros::get_db_or_fail;
use ferrischat_snowflake_generator::generate_snowflake;

// POST /api/v0/guilds/{guild_id}/invites
pub async fn create_invite(

) -> impl Responder {
  let db = get_db_or_fail!();
  
}
