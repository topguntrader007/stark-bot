use actix_web::{web, HttpResponse, Responder};

/// Version from Cargo.toml, available at compile time
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/api/health").route(web::get().to(health_check)));
    cfg.service(web::resource("/api/version").route(web::get().to(get_version)));
}

async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "version": VERSION
    }))
}

async fn get_version() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "version": VERSION
    }))
}
