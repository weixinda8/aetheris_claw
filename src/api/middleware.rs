use crate::api::auth::{AuthManager, Claims, Permission};
use crate::security::rate_limit::RateLimiter;
use axum::{
    Extension,
    extract::Request,
    http::{HeaderName, StatusCode, header},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;

pub async fn jwt_auth_middleware(
    Extension(auth_manager): Extension<Arc<AuthManager>>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !auth_header.starts_with("Bearer ") {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = &auth_header[7..];

    let claims = auth_manager
        .validate_token(token)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    req.extensions_mut().insert(claims);

    Ok(next.run(req).await)
}

pub struct RequirePermission(pub Permission);

pub async fn require_permission_middleware(
    Extension(claims): Extension<Claims>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let extensions = req.extensions();
    let required_permission = extensions
        .get::<RequirePermission>()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    if claims.role.has_permission(&required_permission.0) {
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}

pub async fn extract_claims(
    Extension(auth_manager): Extension<Arc<AuthManager>>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok());

    if let Some(auth_header) = auth_header {
        if let Some(token) = auth_header.strip_prefix("Bearer ") {
            if let Ok(claims) = auth_manager.validate_token(token) {
                req.extensions_mut().insert(claims);
            }
        }
    }

    Ok(next.run(req).await)
}

pub async fn rate_limit_middleware(
    Extension(rate_limiter): Extension<Arc<RateLimiter>>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let client_ip = extract_client_ip(&req);

    let rate_result = if let Some(claims) = req.extensions().get::<Claims>() {
        rate_limiter.check_user(&claims.user_id.to_string())
    } else {
        rate_limiter.check_ip(&client_ip)
    };

    let mut response = if rate_result.allowed {
        next.run(req).await
    } else {
        Response::builder()
            .status(StatusCode::TOO_MANY_REQUESTS)
            .header(header::CONTENT_TYPE, "application/json")
            .body(axum::body::Body::from(
                serde_json::json!({
                    "error": "Too many requests",
                    "message": "Rate limit exceeded",
                    "retry_after": rate_result.reset_in_seconds
                })
                .to_string(),
            ))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    };

    let headers = response.headers_mut();
    headers.insert(
        HeaderName::from_static("x-ratelimit-limit"),
        rate_result.limit.to_string().parse().unwrap(),
    );
    headers.insert(
        HeaderName::from_static("x-ratelimit-remaining"),
        rate_result.remaining.to_string().parse().unwrap(),
    );
    headers.insert(
        HeaderName::from_static("x-ratelimit-reset"),
        rate_result.reset_in_seconds.to_string().parse().unwrap(),
    );

    Ok(response)
}

fn extract_client_ip(req: &Request) -> String {
    if let Some(forwarded) = req.headers().get("x-forwarded-for") {
        if let Ok(value) = forwarded.to_str() {
            if let Some(ip) = value.split(',').next() {
                return ip.trim().to_string();
            }
        }
    }

    if let Some(real_ip) = req.headers().get("x-real-ip") {
        if let Ok(value) = real_ip.to_str() {
            return value.to_string();
        }
    }

    "unknown".to_string()
}
