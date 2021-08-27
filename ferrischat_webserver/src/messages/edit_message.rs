use actix_web::web::Json;

use actix_web::{HttpRequest, HttpResponse, Responder};
use ferrischat_common::types::{InternalServerErrorJson, NotFoundJson, Channel};
use ferrischat_common::request_json::GuildUpdateJson;

use num_traits::ToPrimitive;