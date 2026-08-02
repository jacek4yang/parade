use crate::config::Config;
use crate::db::{audit, Database, DbError};
use crate::traffic::CycleRuleInput;
use argon2::password_hash::{PasswordHash, PasswordVerifier};
use argon2::Argon2;
use axum::body::Bytes;
use axum::extract::{ConnectInfo, Path, Query, State};
use axum::http::{header, HeaderMap, HeaderValue, Request, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use parade_common::{
    EnrollmentRequest, EnrollmentResponse, ObservationLease, ObservationProfile, ReportAck,
    SignedReport, TrafficPolicy, MAX_REPORT_BYTES, PROTOCOL_VERSION,
};
use rusqlite::{params, OptionalExtension, TransactionBehavior};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tower_http::timeout::TimeoutLayer;

type LeaseStatusRow = (String, String, i64, i64, String, i64, i64, Option<i64>);
type NetworkRollupRow = (String, i64, i64, i64, String, String);

static INDEX_HTML: &str = include_str!("../assets/index.html");
static LOGIN_HTML: &str = include_str!("../assets/login.html");
static INSTALL_SH: &str = include_str!("../assets/install.sh");
const COOKIE: &str = "parade_session";

#[derive(Clone)]
pub struct App {
    pub db: Database,
    pub cfg: Arc<Config>,
    limits: Arc<Mutex<RateLimits>>,
}
impl App {
    pub fn new(db: Database, cfg: Arc<Config>) -> Self {
        Self {
            db,
            cfg,
            limits: Arc::new(Mutex::new(RateLimits::default())),
        }
    }
}

#[derive(Default)]
struct RateLimits {
    items: HashMap<String, (u32, Instant)>,
    calls: u64,
}
impl RateLimits {
    fn allow(&mut self, key: String, max: u32, window: Duration) -> bool {
        let now = Instant::now();
        self.calls = self.calls.wrapping_add(1);
        if self.calls.is_multiple_of(256) {
            self.items
                .retain(|_, (_, start)| now.duration_since(*start) < window);
        }
        if self.items.len() >= 10_000 && !self.items.contains_key(&key) {
            return false;
        }
        let item = self.items.entry(key).or_insert((0, now));
        if now.duration_since(item.1) >= window {
            *item = (0, now);
        }
        item.0 = item.0.saturating_add(1);
        item.0 <= max
    }
}

pub fn router(app: App) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/app.js", get(app_js))
        .route("/app.css", get(app_css))
        .route("/login.css", get(login_css))
        .route("/login.js", get(login_js))
        .route("/install.sh", get(install_script))
        .route("/dist/*path", get(dist))
        .route("/api/v1/login", post(login))
        .route("/api/v1/logout", post(logout))
        .route("/api/v1/session", get(session))
        .route("/api/v1/enroll", post(enroll))
        .route(
            "/api/v1/reports",
            post(report).layer(axum::extract::DefaultBodyLimit::max(MAX_REPORT_BYTES)),
        )
        .route("/api/v1/fleet", get(fleet))
        .route("/api/v1/findings", get(fleet_findings))
        .route("/api/v1/events", get(fleet_events))
        .route("/api/v1/traffic", get(fleet_traffic))
        .route("/api/v1/audit", get(audit_events))
        .route("/api/v1/servers", post(create_server))
        .route(
            "/api/v1/servers/:id",
            get(server_detail).delete(delete_server),
        )
        .route("/api/v1/servers/:id/enrollment", post(mint_enrollment))
        .route("/api/v1/servers/:id/resources", get(resources))
        .route("/api/v1/servers/:id/processes", get(processes))
        .route("/api/v1/servers/:id/network", get(network))
        .route("/api/v1/servers/:id/findings", get(findings))
        .route("/api/v1/servers/:id/events", get(events))
        .route("/api/v1/servers/:id/traffic", get(traffic))
        .route("/api/v1/servers/:id/traffic/rule", put(traffic_rule))
        .route("/api/v1/servers/:id/traffic/seed", post(traffic_seed))
        .route(
            "/api/v1/servers/:id/traffic/adjustments",
            post(traffic_adjustment),
        )
        .route(
            "/api/v1/servers/:id/leases",
            get(lease_status).post(create_lease),
        )
        .route(
            "/api/v1/servers/:id/leases/:lease_id",
            axum::routing::delete(cancel_lease),
        )
        .layer(middleware::from_fn(security_headers))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(15),
        ))
        .with_state(app)
}

async fn security_headers(request: Request<axum::body::Body>, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(
        "permissions-policy",
        HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
    );
    headers.insert(header::CONTENT_SECURITY_POLICY,HeaderValue::from_static("default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; connect-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'"));
    response
}

async fn index(State(app): State<App>, headers: HeaderMap) -> Response {
    if !authenticated(&app, &headers) {
        return Html(LOGIN_HTML.replace("{{TITLE}}", &app.cfg.dashboard.title)).into_response();
    }
    Html(INDEX_HTML.replace("{{TITLE}}", &app.cfg.dashboard.title)).into_response()
}
async fn app_js() -> Response {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        include_str!("../assets/app.js"),
    )
        .into_response()
}
async fn app_css() -> Response {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        include_str!("../assets/app.css"),
    )
        .into_response()
}
async fn login_css() -> Response {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        include_str!("../assets/login.css"),
    )
        .into_response()
}
async fn login_js() -> Response {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        include_str!("../assets/login.js"),
    )
        .into_response()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Login {
    password: String,
}
async fn login(
    State(app): State<App>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(input): Json<Login>,
) -> Response {
    let ip = client_ip(&app, &headers, peer);
    if !app.limits.lock().expect("rate limiter").allow(
        format!("login:{ip}"),
        8,
        Duration::from_secs(900),
    ) {
        return error(StatusCode::TOO_MANY_REQUESTS, "login rate limit exceeded");
    }
    let Ok(hash) = PasswordHash::new(&app.cfg.dashboard.password_hash) else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "administrator authentication is not configured safely",
        );
    };
    if Argon2::default()
        .verify_password(input.password.as_bytes(), &hash)
        .is_err()
    {
        return error(StatusCode::UNAUTHORIZED, "invalid credentials");
    }
    let token = random_hex::<32>();
    let csrf = random_hex::<32>();
    let now = chrono::Utc::now().timestamp();
    let expires = now + (app.cfg.hub.session_hours.min(168) as i64 * 3600);
    if let Err(e) = app
        .db
        .create_session(&token, &csrf, &ip.to_string(), now, expires)
    {
        return db_error(e);
    }
    let secure = if app.cfg.hub.secure_cookies {
        "; Secure"
    } else {
        ""
    };
    let session_cookie = format!(
        "{COOKIE}={token}; Path=/; HttpOnly; SameSite=Strict; Max-Age={}{secure}",
        expires - now
    );
    let csrf_cookie = format!(
        "parade_csrf={csrf}; Path=/; SameSite=Strict; Max-Age={}{secure}",
        expires - now
    );
    let mut response = Json(json!({"csrf_token":csrf,"expires_at":expires})).into_response();
    response.headers_mut().append(
        header::SET_COOKIE,
        HeaderValue::from_str(&session_cookie).expect("session cookie"),
    );
    response.headers_mut().append(
        header::SET_COOKIE,
        HeaderValue::from_str(&csrf_cookie).expect("CSRF cookie"),
    );
    response
}
async fn logout(State(app): State<App>, headers: HeaderMap) -> Response {
    let Some(token) = cookie(&headers) else {
        return StatusCode::NO_CONTENT.into_response();
    };
    if !csrf_ok(&app, &headers) {
        return error(StatusCode::FORBIDDEN, "CSRF validation failed");
    }
    if let Err(e) = app
        .db
        .revoke_session(&token, chrono::Utc::now().timestamp())
    {
        return db_error(e);
    }
    let secure = if app.cfg.hub.secure_cookies {
        "; Secure"
    } else {
        ""
    };
    let mut response = StatusCode::NO_CONTENT.into_response();
    response.headers_mut().append(
        header::SET_COOKIE,
        HeaderValue::from_str(&format!(
            "{COOKIE}=gone; Path=/; HttpOnly; SameSite=Strict; Max-Age=0{secure}"
        ))
        .expect("session cookie"),
    );
    response.headers_mut().append(
        header::SET_COOKIE,
        HeaderValue::from_str(&format!(
            "parade_csrf=gone; Path=/; SameSite=Strict; Max-Age=0{secure}"
        ))
        .expect("CSRF cookie"),
    );
    response
}
async fn session(State(app): State<App>, headers: HeaderMap) -> Response {
    if !authenticated(&app, &headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    Json(json!({"authenticated":true,"read_only_targets":true,"title":app.cfg.dashboard.title}))
        .into_response()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ServerInput {
    id: String,
    name: String,
    #[serde(default)]
    group: String,
}
async fn create_server(
    State(app): State<App>,
    headers: HeaderMap,
    Json(input): Json<ServerInput>,
) -> Response {
    if let Err(response) = mutation_auth(&app, &headers) {
        return *response;
    }
    match app
        .db
        .create_server(&input.id, &input.name, &input.group, now(), "admin")
    {
        Ok(()) => (StatusCode::CREATED, Json(json!({"id":input.id}))).into_response(),
        Err(e) => db_error(e),
    }
}
async fn delete_server(
    State(app): State<App>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<DeleteInput>,
) -> Response {
    if let Err(response) = mutation_auth(&app, &headers) {
        return *response;
    }
    match app.db.tombstone_server(&id, now(), "admin", &input.reason) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => db_error(e),
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DeleteInput {
    reason: String,
}

async fn fleet(
    State(app): State<App>,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
) -> Response {
    if !authenticated(&app, &headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let limit = query.limit.unwrap_or(100).clamp(1, 500);
    let offset = query.offset.unwrap_or(0).max(0);
    let conn = match app.db.connection() {
        Ok(c) => c,
        Err(e) => return db_error(e),
    };
    let mut stmt=match conn.prepare("SELECT id,name,group_name,status,last_seen,os,kernel,arch,agent_version,coverage_json FROM servers WHERE status!='deleted' ORDER BY name,id LIMIT ?1 OFFSET ?2"){Ok(s)=>s,Err(e)=>return db_error(e.into())};
    let rows=match stmt.query_map(params![limit,offset],|r|Ok(json!({"id":r.get::<_,String>(0)?,"name":r.get::<_,String>(1)?,"group":r.get::<_,String>(2)?,"status":display_status(r.get::<_,String>(3)?,r.get::<_,Option<i64>>(4)?,now(),app.cfg.hub.stale_after_secs,app.cfg.hub.offline_after_secs),"last_seen":r.get::<_,Option<i64>>(4)?,"os":r.get::<_,Option<String>>(5)?,"kernel":r.get::<_,Option<String>>(6)?,"arch":r.get::<_,Option<String>>(7)?,"agent_version":r.get::<_,Option<String>>(8)?,"coverage":serde_json::from_str::<Value>(&r.get::<_,String>(9)?).unwrap_or(json!([]))}))){Ok(r)=>r,Err(e)=>return db_error(e.into())};
    let servers = match rows.collect::<Result<Vec<_>, _>>() {
        Ok(v) => v,
        Err(e) => return db_error(e.into()),
    };
    let total: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM servers WHERE status!='deleted'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let mut summary = serde_json::Map::new();
    for status in ["online", "stale", "offline", "pending", "revoked"] {
        summary.insert(status.into(), json!(0));
    }
    if let Ok(mut all) =
        conn.prepare("SELECT status,last_seen FROM servers WHERE status!='deleted'")
    {
        if let Ok(rows) = all.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<i64>>(1)?))
        }) {
            for (stored, last_seen) in rows.flatten() {
                let status = display_status(
                    stored,
                    last_seen,
                    now(),
                    app.cfg.hub.stale_after_secs,
                    app.cfg.hub.offline_after_secs,
                );
                let current = summary.get(&status).and_then(Value::as_i64).unwrap_or(0);
                summary.insert(status, json!(current + 1));
            }
        }
    }
    Json(json!({"api_version":1,"total":total,"limit":limit,"offset":offset,"servers":servers,"summary":summary,"read_only_targets":true,"generated_at":now()})).into_response()
}

async fn audit_events(State(app): State<App>, headers: HeaderMap) -> Response {
    if !authenticated(&app, &headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let conn = match app.db.connection() {
        Ok(value) => value,
        Err(error) => return db_error(error),
    };
    let mut stmt = match conn.prepare("SELECT id,occurred_at,operator,action,server_id,detail_json FROM audit_events ORDER BY occurred_at DESC LIMIT 500") {
        Ok(value) => value,
        Err(error) => return db_error(error.into()),
    };
    let rows = match stmt.query_map([], |row| {
        let detail: String = row.get(5)?;
        Ok(json!({
            "id": row.get::<_,i64>(0)?,
            "occurred_at": row.get::<_,i64>(1)?,
            "operator": row.get::<_,String>(2)?,
            "action": row.get::<_,String>(3)?,
            "server_id": row.get::<_,Option<String>>(4)?,
            "detail": serde_json::from_str::<Value>(&detail).unwrap_or(json!({"unparsed":true})),
        }))
    }) {
        Ok(value) => value,
        Err(error) => return db_error(error.into()),
    };
    match rows.collect::<Result<Vec<_>, _>>() {
        Ok(items) => Json(json!({"items":items})).into_response(),
        Err(error) => db_error(error.into()),
    }
}

async fn fleet_findings(State(app): State<App>, headers: HeaderMap) -> Response {
    if !authenticated(&app, &headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let conn = match app.db.connection() {
        Ok(value) => value,
        Err(error) => return db_error(error),
    };
    let mut stmt = match conn.prepare("SELECT f.id,f.server_id,s.name,f.rule_id,f.rule_version,f.severity,f.confidence,f.state,f.first_seen,f.last_seen,f.occurrence_count,f.evidence,f.explanation,f.verification,f.coverage_caveat FROM security_findings f JOIN servers s ON s.id=f.server_id WHERE s.status!='deleted' ORDER BY CASE f.severity WHEN 'critical' THEN 0 WHEN 'review' THEN 1 ELSE 2 END,f.last_seen DESC LIMIT 500") { Ok(value) => value, Err(error) => return db_error(error.into()) };
    let rows = match stmt.query_map([], |r| Ok(json!({"id":r.get::<_,i64>(0)?,"server_id":r.get::<_,String>(1)?,"server_name":r.get::<_,String>(2)?,"rule_id":r.get::<_,String>(3)?,"rule_version":r.get::<_,i64>(4)?,"severity":r.get::<_,String>(5)?,"confidence":r.get::<_,String>(6)?,"state":r.get::<_,String>(7)?,"first_seen":r.get::<_,i64>(8)?,"last_seen":r.get::<_,i64>(9)?,"occurrences":r.get::<_,i64>(10)?,"evidence":r.get::<_,String>(11)?,"explanation":r.get::<_,String>(12)?,"verification":r.get::<_,String>(13)?,"coverage_caveat":r.get::<_,String>(14)?}))) { Ok(value) => value, Err(error) => return db_error(error.into()) };
    match rows.collect::<Result<Vec<_>, _>>() {
        Ok(items) => Json(json!({"items":items})).into_response(),
        Err(error) => db_error(error.into()),
    }
}

async fn fleet_events(State(app): State<App>, headers: HeaderMap) -> Response {
    if !authenticated(&app, &headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let conn = match app.db.connection() {
        Ok(value) => value,
        Err(error) => return db_error(error),
    };
    let mut stmt = match conn.prepare("SELECT e.id,e.server_id,COALESCE(s.name,'Hub'),e.occurred_at,e.category,e.severity,e.summary,e.evidence FROM events e LEFT JOIN servers s ON s.id=e.server_id ORDER BY e.occurred_at DESC LIMIT 500") { Ok(value) => value, Err(error) => return db_error(error.into()) };
    let rows = match stmt.query_map([], |r| Ok(json!({"id":r.get::<_,i64>(0)?,"server_id":r.get::<_,Option<String>>(1)?,"server_name":r.get::<_,String>(2)?,"occurred_at":r.get::<_,i64>(3)?,"category":r.get::<_,String>(4)?,"severity":r.get::<_,String>(5)?,"summary":r.get::<_,String>(6)?,"evidence":r.get::<_,String>(7)?}))) { Ok(value) => value, Err(error) => return db_error(error.into()) };
    match rows.collect::<Result<Vec<_>, _>>() {
        Ok(items) => Json(json!({"items":items})).into_response(),
        Err(error) => db_error(error.into()),
    }
}

async fn fleet_traffic(State(app): State<App>, headers: HeaderMap) -> Response {
    if !authenticated(&app, &headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let conn = match app.db.connection() {
        Ok(value) => value,
        Err(error) => return db_error(error),
    };
    let open: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM billing_cycle_instance WHERE state='open'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let seeded: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM traffic_seed WHERE active_primary=1",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let uncertain: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM billing_cycle_instance WHERE state='open' AND confidence!='high'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let mut groups = Vec::new();
    if let Ok(mut stmt) = conn.prepare("SELECT timezone,anchor_day,anchor_time,COUNT(*) FROM billing_cycle_rule WHERE enabled=1 GROUP BY timezone,anchor_day,anchor_time ORDER BY COUNT(*) DESC LIMIT 50") {
        if let Ok(rows) = stmt.query_map([],|r|Ok(json!({"timezone":r.get::<_,String>(0)?,"anchor_day":r.get::<_,i64>(1)?,"anchor_time":r.get::<_,String>(2)?,"servers":r.get::<_,i64>(3)?}))) {
            groups = rows.flatten().collect();
        }
    }
    Json(json!({"open_cycles":open,"seeded_cycles":seeded,"uncertain_cycles":uncertain,"boundary_groups":groups})).into_response()
}
#[derive(Deserialize)]
struct ListQuery {
    limit: Option<i64>,
    offset: Option<i64>,
}

async fn server_detail(
    State(app): State<App>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    if !authenticated(&app, &headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let conn = match app.db.connection() {
        Ok(c) => c,
        Err(e) => return db_error(e),
    };
    match conn.query_row("SELECT id,name,group_name,status,last_seen,os,kernel,arch,agent_version,coverage_json,inventory_hash FROM servers WHERE id=?1 AND status!='deleted'",[id],|r|{
        let stored = r.get::<_,String>(3)?;
        let last_seen = r.get::<_,Option<i64>>(4)?;
        Ok(json!({"id":r.get::<_,String>(0)?,"name":r.get::<_,String>(1)?,"group":r.get::<_,String>(2)?,"status":display_status(stored,last_seen,now(),app.cfg.hub.stale_after_secs,app.cfg.hub.offline_after_secs),"last_seen":last_seen,"os":r.get::<_,Option<String>>(5)?,"kernel":r.get::<_,Option<String>>(6)?,"arch":r.get::<_,Option<String>>(7)?,"agent_version":r.get::<_,Option<String>>(8)?,"coverage":serde_json::from_str::<Value>(&r.get::<_,String>(9)?).unwrap_or(json!([])),"inventory_hash":r.get::<_,Option<String>>(10)?,"read_only_target":true}))
    }).optional(){Ok(Some(v))=>Json(v).into_response(),Ok(None)=>error(StatusCode::NOT_FOUND,"server not found"),Err(e)=>db_error(e.into())}
}

async fn resources(State(app): State<App>, headers: HeaderMap, Path(id): Path<String>) -> Response {
    latest_json(
        &app,
        &headers,
        &id,
        "resource_rollups",
        "payload_json",
        "interval_end",
    )
}
async fn processes(State(app): State<App>, headers: HeaderMap, Path(id): Path<String>) -> Response {
    latest_json(
        &app,
        &headers,
        &id,
        "process_summaries",
        "payload_json",
        "observed_at",
    )
}
async fn network(State(app): State<App>, headers: HeaderMap, Path(id): Path<String>) -> Response {
    if !authenticated(&app, &headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let conn = match app.db.connection() {
        Ok(value) => value,
        Err(error) => return db_error(error),
    };
    let listeners: Option<(String, i64)> = match conn
        .query_row(
            "SELECT payload_json,observed_at FROM socket_summaries WHERE server_id=?1 ORDER BY observed_at DESC LIMIT 1",
            [&id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
    {
        Ok(value) => value,
        Err(error) => return db_error(error.into()),
    };
    let traffic: Option<NetworkRollupRow> = match conn
        .query_row(
            "SELECT interfaces_json,interval_end,observed_rx_delta,observed_tx_delta,confidence,anomaly_flags_json FROM traffic_rollup WHERE server_id=?1 ORDER BY interval_end DESC LIMIT 1",
            [&id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
        )
        .optional()
    {
        Ok(value) => value,
        Err(error) => return db_error(error.into()),
    };
    let observed_at = listeners
        .as_ref()
        .map(|value| value.1)
        .into_iter()
        .chain(traffic.as_ref().map(|value| value.1))
        .max();
    let listener_values = listeners
        .and_then(|value| serde_json::from_str::<Value>(&value.0).ok())
        .unwrap_or(json!([]));
    let (interfaces, rx_delta, tx_delta, confidence, anomalies) = traffic
        .map(|value| {
            (
                serde_json::from_str::<Value>(&value.0).unwrap_or(json!([])),
                value.2,
                value.3,
                value.4,
                serde_json::from_str::<Value>(&value.5).unwrap_or(json!([])),
            )
        })
        .unwrap_or((json!([]), 0, 0, "unsupported".into(), json!([])));
    Json(json!({
        "observed_at":observed_at,
        "data":{
            "listeners":listener_values,
            "interfaces":interfaces,
            "observed_rx_delta":rx_delta,
            "observed_tx_delta":tx_delta,
            "confidence":confidence,
            "anomaly_flags":anomalies
        }
    }))
    .into_response()
}
fn latest_json(
    app: &App,
    headers: &HeaderMap,
    id: &str,
    table: &str,
    column: &str,
    time: &str,
) -> Response {
    if !authenticated(app, headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let sql = format!(
        "SELECT {column},{time} FROM {table} WHERE server_id=?1 ORDER BY {time} DESC LIMIT 1"
    );
    let conn = match app.db.connection() {
        Ok(c) => c,
        Err(e) => return db_error(e),
    };
    match conn.query_row(&sql,[id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,i64>(1)?))).optional(){Ok(Some((value,at)))=>Json(json!({"observed_at":at,"data":serde_json::from_str::<Value>(&value).unwrap_or(json!(null))})).into_response(),Ok(None)=>Json(json!({"observed_at":null,"data":[]})).into_response(),Err(e)=>db_error(e.into())}
}

async fn findings(State(app): State<App>, headers: HeaderMap, Path(id): Path<String>) -> Response {
    list_records(&app,&headers,"SELECT id,rule_id,rule_version,severity,confidence,state,first_seen,last_seen,occurrence_count,evidence,explanation,verification,coverage_caveat FROM security_findings WHERE server_id=?1 ORDER BY CASE severity WHEN 'critical' THEN 0 WHEN 'review' THEN 1 ELSE 2 END,last_seen DESC LIMIT 500",&id,true)
}
async fn events(State(app): State<App>, headers: HeaderMap, Path(id): Path<String>) -> Response {
    list_records(&app,&headers,"SELECT id,category,severity,occurred_at,summary,evidence,'','','','','','' ,'' FROM events WHERE server_id=?1 ORDER BY occurred_at DESC LIMIT 500",&id,false)
}
fn list_records(app: &App, headers: &HeaderMap, sql: &str, id: &str, finding: bool) -> Response {
    if !authenticated(app, headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let conn = match app.db.connection() {
        Ok(c) => c,
        Err(e) => return db_error(e),
    };
    let mut stmt = match conn.prepare(sql) {
        Ok(s) => s,
        Err(e) => return db_error(e.into()),
    };
    let rows=stmt.query_map([id],|r|if finding{Ok(json!({"id":r.get::<_,i64>(0)?,"rule_id":r.get::<_,String>(1)?,"rule_version":r.get::<_,i64>(2)?,"severity":r.get::<_,String>(3)?,"confidence":r.get::<_,String>(4)?,"state":r.get::<_,String>(5)?,"first_seen":r.get::<_,i64>(6)?,"last_seen":r.get::<_,i64>(7)?,"occurrences":r.get::<_,i64>(8)?,"evidence":r.get::<_,String>(9)?,"explanation":r.get::<_,String>(10)?,"verification":r.get::<_,String>(11)?,"coverage_caveat":r.get::<_,String>(12)?}))}else{Ok(json!({"id":r.get::<_,i64>(0)?,"category":r.get::<_,String>(1)?,"severity":r.get::<_,String>(2)?,"occurred_at":r.get::<_,i64>(3)?,"summary":r.get::<_,String>(4)?,"evidence":r.get::<_,String>(5)?}))});
    match rows.and_then(|values| values.collect::<Result<Vec<_>, _>>()) {
        Ok(values) => Json(json!({"items":values})).into_response(),
        Err(e) => db_error(e.into()),
    }
}

async fn traffic(State(app): State<App>, headers: HeaderMap, Path(id): Path<String>) -> Response {
    if !authenticated(&app, &headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    match app.db.traffic_usage(&id) {
        Ok(v) => Json(json!(v)).into_response(),
        Err(DbError::NotFound) => Json(json!({"state":"awaiting_checkpoint"})).into_response(),
        Err(e) => db_error(e),
    }
}
async fn traffic_rule(
    State(app): State<App>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<CycleRuleInput>,
) -> Response {
    if let Err(r) = mutation_auth(&app, &headers) {
        return *r;
    }
    match app.db.set_cycle_rule(&id, &input, now(), "admin") {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => db_error(e),
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SeedInput {
    combined_bytes: u64,
    effective_at: i64,
    #[serde(default)]
    note: String,
}
async fn traffic_seed(
    State(app): State<App>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<SeedInput>,
) -> Response {
    if let Err(r) = mutation_auth(&app, &headers) {
        return *r;
    }
    match app.db.add_traffic_seed(
        &id,
        input.combined_bytes,
        input.effective_at,
        &input.note,
        now(),
        "admin",
    ) {
        Ok(v) => Json(json!(v)).into_response(),
        Err(e) => db_error(e),
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AdjustmentInput {
    signed_bytes: i64,
    effective_at: i64,
    reason: String,
}
async fn traffic_adjustment(
    State(app): State<App>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<AdjustmentInput>,
) -> Response {
    if let Err(r) = mutation_auth(&app, &headers) {
        return *r;
    }
    match app.db.add_traffic_adjustment(
        &id,
        input.signed_bytes,
        input.effective_at,
        &input.reason,
        now(),
        "admin",
    ) {
        Ok(v) => Json(json!(v)).into_response(),
        Err(e) => db_error(e),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LeaseInput {
    profile: ObservationProfile,
    duration_secs: i64,
}
async fn create_lease(
    State(app): State<App>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<LeaseInput>,
) -> Response {
    if let Err(r) = mutation_auth(&app, &headers) {
        return *r;
    }
    let now = now();
    let expiry = now + input.duration_secs;
    let profile = match input.profile {
        ObservationProfile::LiveDetail { .. } => {
            ObservationProfile::LiveDetail { expires_at: expiry }
        }
        other => other,
    };
    let lease = ObservationLease {
        lease_id: random_hex::<16>(),
        profile,
        issued_at: now,
        expires_at: expiry,
    };
    if lease.validate(now).is_err() {
        return error(
            StatusCode::BAD_REQUEST,
            "invalid or excessive observation lease",
        );
    }
    let mut conn = match app.db.connection() {
        Ok(c) => c,
        Err(e) => return db_error(e),
    };
    let profile_json = match serde_json::to_string(&lease.profile) {
        Ok(value) => value,
        Err(_) => return error(StatusCode::BAD_REQUEST, "invalid observation profile"),
    };
    let result = (|| -> Result<(), DbError> {
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute(
            "UPDATE observation_leases SET state='expired',ended_at=?2 WHERE server_id=?1 AND state='active' AND expires_at<=?2",
            params![id, now],
        )?;
        let active: i64 = tx.query_row(
            "SELECT COUNT(*) FROM observation_leases WHERE server_id=?1 AND state='active'",
            [&id],
            |row| row.get(0),
        )?;
        if active > 0 {
            return Err(DbError::Conflict(
                "an observation lease is already active for this server",
            ));
        }
        let server_active: i64 = tx.query_row(
            "SELECT COUNT(*) FROM servers WHERE id=?1 AND status='active'",
            [&id],
            |row| row.get(0),
        )?;
        if server_active != 1 {
            return Err(DbError::NotFound);
        }
        tx.execute("INSERT INTO observation_leases(lease_id,server_id,profile_json,issued_at,expires_at,state,issued_by) VALUES(?1,?2,?3,?4,?5,'active','admin')",params![lease.lease_id,id,profile_json,now,expiry])?;
        audit(
            &tx,
            now,
            "admin",
            "observation_lease.create",
            Some(&id),
            &json!({"lease_id":lease.lease_id,"profile":lease.profile,"expires_at":expiry})
                .to_string(),
        )?;
        tx.commit()?;
        Ok(())
    })();
    match result {
        Ok(()) => Json(json!({
            "lease_id":lease.lease_id,
            "profile":lease.profile,
            "issued_at":lease.issued_at,
            "expires_at":lease.expires_at,
            "state":"active",
            "response_count":0,
            "encoded_response_bytes":0
        }))
        .into_response(),
        Err(error) => db_error(error),
    }
}

async fn lease_status(
    State(app): State<App>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    if !authenticated(&app, &headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let now = now();
    let conn = match app.db.connection() {
        Ok(value) => value,
        Err(error) => return db_error(error),
    };
    if let Err(error) = conn.execute(
        "UPDATE observation_leases SET state='expired',ended_at=?2 WHERE server_id=?1 AND state='active' AND expires_at<=?2",
        params![id, now],
    ) {
        return db_error(error.into());
    }
    let row: Result<Option<LeaseStatusRow>, _> = conn
        .query_row(
            "SELECT lease_id,profile_json,issued_at,expires_at,state,response_count,encoded_response_bytes,last_response_at FROM observation_leases WHERE server_id=?1 ORDER BY issued_at DESC LIMIT 1",
            [&id],
            |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?,row.get(6)?,row.get(7)?)),
        )
        .optional();
    match row {
        Ok(Some((lease_id, profile, issued_at, expires_at, state, responses, bytes, last))) => {
            let profile: Value = serde_json::from_str(&profile).unwrap_or(json!("unknown"));
            Json(json!({"lease_id":lease_id,"profile":profile,"issued_at":issued_at,"expires_at":expires_at,"state":state,"response_count":responses,"encoded_response_bytes":bytes,"last_response_at":last})).into_response()
        }
        Ok(None) => Json(json!({"state":"inactive","response_count":0,"encoded_response_bytes":0}))
            .into_response(),
        Err(error) => db_error(error.into()),
    }
}

async fn cancel_lease(
    State(app): State<App>,
    headers: HeaderMap,
    Path((id, lease_id)): Path<(String, String)>,
) -> Response {
    if let Err(response) = mutation_auth(&app, &headers) {
        return *response;
    }
    let now = now();
    let mut conn = match app.db.connection() {
        Ok(value) => value,
        Err(error) => return db_error(error),
    };
    let result = (|| -> Result<(), DbError> {
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if tx.execute(
            "UPDATE observation_leases SET state='cancelled',ended_at=?3 WHERE lease_id=?1 AND server_id=?2 AND state='active'",
            params![lease_id, id, now],
        )? != 1
        {
            return Err(DbError::NotFound);
        }
        audit(
            &tx,
            now,
            "admin",
            "observation_lease.cancel",
            Some(&id),
            &json!({"lease_id":lease_id}).to_string(),
        )?;
        tx.commit()?;
        Ok(())
    })();
    match result {
        Ok(()) => {
            Json(json!({"lease_id":lease_id,"state":"cancelled","ended_at":now})).into_response()
        }
        Err(error) => db_error(error),
    }
}

async fn mint_enrollment(
    State(app): State<App>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    if let Err(r) = mutation_auth(&app, &headers) {
        return *r;
    }
    let manifest_path = std::path::Path::new(&app.cfg.hub.dist_dir).join("SHA256SUMS");
    let manifest = match tokio::fs::read(&manifest_path).await {
        Ok(value) if !value.is_empty() => value,
        _ => {
            return error(
                StatusCode::SERVICE_UNAVAILABLE,
                "release manifest is not staged; build Agent artifacts first",
            )
        }
    };
    let public_key = match tokio::fs::read(
        std::path::Path::new(&app.cfg.hub.dist_dir).join("release-public.pem"),
    )
    .await
    {
        Ok(value)
            if parade_common::sha256_hex(&value)
                == app.cfg.hub.release_public_key_sha256.to_ascii_lowercase() =>
        {
            value
        }
        _ => {
            return error(
                StatusCode::SERVICE_UNAVAILABLE,
                "release public key is missing or does not match its configured pin",
            )
        }
    };
    let signature =
        match tokio::fs::read(std::path::Path::new(&app.cfg.hub.dist_dir).join("SHA256SUMS.sig"))
            .await
        {
            Ok(value) if value.len() == 64 => value,
            _ => {
                return error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "detached release manifest signature is not staged",
                )
            }
        };
    let _ = (public_key, signature);
    let manifest_sha256 = parade_common::sha256_hex(&manifest);
    let token = random_hex::<32>();
    let expires = now() + 900;
    match app.db.mint_enrollment(&id,&token,expires,now(),"admin"){
        Ok(())=>Json(json!({
            "expires_at":expires,
            "manifest_sha256":manifest_sha256.clone(),
            "release_public_key_sha256":app.cfg.hub.release_public_key_sha256,
            "command":format!(
                "curl -fsSL {}/install.sh | sudo env PARADE_ENROLL_TOKEN={} PARADE_MANIFEST_SHA256={} PARADE_RELEASE_KEY_SHA256={} bash",
                app.cfg.hub.public_url.trim_end_matches('/'),token,manifest_sha256,app.cfg.hub.release_public_key_sha256
            )
        })).into_response(),
        Err(e)=>db_error(e)
    }
}
async fn enroll(
    State(app): State<App>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(input): Json<EnrollmentRequest>,
) -> Response {
    let source = client_ip(&app, &headers, peer);
    if !app.limits.lock().expect("rate limiter").allow(
        format!("enroll:{source}"),
        30,
        Duration::from_secs(900),
    ) {
        return error(
            StatusCode::TOO_MANY_REQUESTS,
            "enrollment rate limit exceeded",
        );
    }
    if input.protocol_version != PROTOCOL_VERSION || input.agent_nonce.len() != 64 {
        return error(
            StatusCode::BAD_REQUEST,
            "unsupported or malformed enrollment request",
        );
    }
    let agent_id = random_hex::<16>();
    let now = now();
    match app
        .db
        .enroll_agent(&input.token, &input.public_key_hex, &agent_id, now)
    {
        Ok(server_id) => Json(EnrollmentResponse {
            protocol_version: PROTOCOL_VERSION,
            server_id: server_id.clone(),
            agent_id,
            report_url: format!(
                "{}/api/v1/reports",
                app.cfg.hub.public_url.trim_end_matches('/')
            ),
            next_boundary: None,
            traffic_policy: traffic_policy(&app.db, &server_id).unwrap_or_default(),
        })
        .into_response(),
        Err(e) => db_error(e),
    }
}

async fn report(
    State(app): State<App>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let now = now();
    if !app.limits.lock().expect("rate limiter").allow(
        format!("report-peer:{}", peer.ip()),
        5_000,
        Duration::from_secs(60),
    ) {
        return error(StatusCode::TOO_MANY_REQUESTS, "report rate limit exceeded");
    }
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok());
    let signed = match decode_report_envelope(content_type, &body, now) {
        Ok(value) => value,
        Err((status, message)) => return error(status, message),
    };
    let public = match app.db.public_key_for(&signed.server_id, &signed.agent_id) {
        Ok(v) => v,
        Err(e) => return db_error(e),
    };
    if signed.verify(&public).is_err() {
        return error(StatusCode::UNAUTHORIZED, "invalid report signature");
    }
    if !app.limits.lock().expect("rate limiter").allow(
        format!("verified-agent:{}", signed.agent_id),
        120,
        Duration::from_secs(60),
    ) {
        return error(StatusCode::TOO_MANY_REQUESTS, "report rate limit exceeded");
    }
    let ip = client_ip(&app, &headers, peer).to_string();
    match app.db.ingest_verified(&signed, &ip, now, body.len()) {
        Ok(()) => {
            let lease = active_lease(&app.db, &signed.server_id, now).ok().flatten();
            let policy = traffic_policy(&app.db, &signed.server_id).unwrap_or_default();
            (
                StatusCode::ACCEPTED,
                Json(ReportAck {
                    accepted: true,
                    duplicate: false,
                    sequence: signed.sequence,
                    lease,
                    traffic_policy: policy,
                }),
            )
                .into_response()
        }
        Err(DbError::Duplicate) => Json(ReportAck {
            accepted: true,
            duplicate: true,
            sequence: signed.sequence,
            lease: active_lease(&app.db, &signed.server_id, now).ok().flatten(),
            traffic_policy: traffic_policy(&app.db, &signed.server_id).unwrap_or_default(),
        })
        .into_response(),
        Err(e) => db_error(e),
    }
}

fn traffic_policy(db: &Database, server: &str) -> Result<TrafficPolicy, DbError> {
    let row: Option<(i64, String)> = db
        .connection()?
        .query_row(
            "SELECT version,interface_policy_json FROM billing_cycle_rule WHERE server_id=?1 AND enabled=1",
            [server],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let Some((version, raw)) = row else {
        return Ok(TrafficPolicy::default());
    };
    let value: Value = serde_json::from_str(&raw)
        .map_err(|_| DbError::Invalid("invalid stored interface policy"))?;
    let strings = |key: &str| {
        value
            .get(key)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(ToOwned::to_owned)
            .collect()
    };
    Ok(TrafficPolicy {
        version: u32::try_from(version).unwrap_or(u32::MAX),
        selected_interfaces: strings("selected"),
        excluded_interfaces: strings("excluded"),
    })
}

fn decode_report_envelope(
    content_type: Option<&str>,
    body: &[u8],
    now: i64,
) -> Result<SignedReport, (StatusCode, &'static str)> {
    if body.len() > MAX_REPORT_BYTES {
        return Err((StatusCode::PAYLOAD_TOO_LARGE, "report exceeds size limit"));
    }
    if content_type != Some("application/x-parade") {
        return Err((
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "application/x-parade required",
        ));
    }
    let signed: SignedReport =
        postcard::from_bytes(body).map_err(|_| (StatusCode::BAD_REQUEST, "malformed report"))?;
    if signed.server_id.is_empty()
        || signed.server_id.len() > 64
        || signed.agent_id.is_empty()
        || signed.agent_id.len() > 64
        || signed.message_id.len() != 64
    {
        return Err((StatusCode::BAD_REQUEST, "invalid report identifiers"));
    }
    if signed.sent_at.abs_diff(now) > 600 {
        return Err((StatusCode::UNAUTHORIZED, "stale report"));
    }
    if signed.body.traffic.sampled_at.abs_diff(signed.sent_at) > 60
        || signed.body.resources.interval_end.abs_diff(signed.sent_at) > 60
        || signed.body.resources.interval_end != signed.body.traffic.sampled_at
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "report observation timestamps are inconsistent",
        ));
    }
    Ok(signed)
}

fn active_lease(
    db: &Database,
    server: &str,
    now: i64,
) -> Result<Option<ObservationLease>, DbError> {
    let conn = db.connection()?;
    let row:Option<(String,String,i64,i64)>=conn.query_row("SELECT lease_id,profile_json,issued_at,expires_at FROM observation_leases WHERE server_id=?1 AND state='active' AND expires_at>?2 ORDER BY issued_at DESC LIMIT 1",params![server,now],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional()?;
    Ok(row.and_then(|(id, p, issued, expires)| {
        serde_json::from_str(&p)
            .ok()
            .map(|profile| ObservationLease {
                lease_id: id,
                profile,
                issued_at: issued,
                expires_at: expires,
            })
    }))
}

async fn install_script(State(app): State<App>) -> Response {
    (
        [(header::CONTENT_TYPE, "text/x-shellscript; charset=utf-8")],
        INSTALL_SH.replace("{{HUB_BASE}}", app.cfg.hub.public_url.trim_end_matches('/')),
    )
        .into_response()
}
async fn dist(State(app): State<App>, Path(path): Path<String>) -> Response {
    if path.split('/').any(|segment| {
        segment.is_empty()
            || segment == "."
            || segment == ".."
            || !segment
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    }) {
        return error(StatusCode::BAD_REQUEST, "invalid artifact path");
    }
    let root = match tokio::fs::canonicalize(&app.cfg.hub.dist_dir).await {
        Ok(value) => value,
        Err(_) => return error(StatusCode::NOT_FOUND, "artifact directory not found"),
    };
    let full = match tokio::fs::canonicalize(root.join(&path)).await {
        Ok(value) if value.starts_with(&root) => value,
        _ => return error(StatusCode::NOT_FOUND, "artifact not found"),
    };
    match tokio::fs::read(full).await {
        Ok(bytes) => {
            let digest = parade_common::sha256_hex(&bytes);
            let content_type = if path == "SHA256SUMS" {
                "text/plain; charset=utf-8"
            } else {
                "application/octet-stream"
            };
            (
                [
                    ("content-type", content_type),
                    ("x-parade-sha256", digest.as_str()),
                ],
                bytes,
            )
                .into_response()
        }
        Err(_) => error(StatusCode::NOT_FOUND, "artifact not found"),
    }
}

fn authenticated(app: &App, headers: &HeaderMap) -> bool {
    cookie(headers).is_some_and(|token| app.db.verify_session(&token, now()).unwrap_or(false))
}
fn mutation_auth(app: &App, headers: &HeaderMap) -> Result<(), Box<Response>> {
    if !authenticated(app, headers) {
        Err(Box::new(StatusCode::UNAUTHORIZED.into_response()))
    } else if !csrf_ok(app, headers) {
        Err(Box::new(error(
            StatusCode::FORBIDDEN,
            "CSRF validation failed",
        )))
    } else {
        Ok(())
    }
}
fn csrf_ok(app: &App, headers: &HeaderMap) -> bool {
    let Some(token) = cookie(headers) else {
        return false;
    };
    let Some(csrf) = headers.get("x-parade-csrf").and_then(|v| v.to_str().ok()) else {
        return false;
    };
    app.db.verify_csrf(&token, csrf, now()).unwrap_or(false)
}
fn cookie(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .map(str::trim)
        .find_map(|part| {
            part.strip_prefix(&format!("{COOKIE}="))
                .map(ToOwned::to_owned)
        })
}
fn client_ip(app: &App, headers: &HeaderMap, peer: SocketAddr) -> IpAddr {
    if app.cfg.hub.trusted_proxies.contains(&peer.ip()) {
        if let Some(ip) = headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.rsplit(',').next())
            .and_then(|v| v.trim().parse().ok())
        {
            return ip;
        }
    }
    peer.ip()
}
fn random_hex<const N: usize>() -> String {
    let mut bytes = [0u8; N];
    getrandom::getrandom(&mut bytes).expect("operating-system RNG");
    parade_common::hex_encode(&bytes)
}
fn now() -> i64 {
    chrono::Utc::now().timestamp()
}
fn display_status(stored: String, last: Option<i64>, at: i64, stale: i64, offline: i64) -> String {
    if stored != "active" {
        return stored;
    }
    match last {
        None => "pending".into(),
        Some(seen) if at - seen > offline => "offline".into(),
        Some(seen) if at - seen > stale => "stale".into(),
        Some(_) => "online".into(),
    }
}
fn error(status: StatusCode, message: &str) -> Response {
    (status, Json(json!({"error":message}))).into_response()
}
fn db_error(error_value: DbError) -> Response {
    match error_value {
        DbError::Invalid(message) => error(StatusCode::BAD_REQUEST, message),
        DbError::Conflict(message) => error(StatusCode::CONFLICT, message),
        DbError::NotFound => error(StatusCode::NOT_FOUND, "not found"),
        DbError::Unauthorized => error(StatusCode::UNAUTHORIZED, "unauthorized"),
        DbError::Duplicate => error(StatusCode::CONFLICT, "duplicate report"),
        DbError::Replay => error(StatusCode::CONFLICT, "replayed or out-of-order report"),
        DbError::Sql(error_value) => {
            eprintln!("database error: {error_value}");
            error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "database operation failed",
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use parade_common::{ObservationProfile, ResourceRollup, TelemetryReport, TrafficCheckpoint};

    fn encoded(at: i64) -> Vec<u8> {
        let key = SigningKey::from_bytes(&[4; 32]);
        let value = SignedReport::new(
            "server".into(),
            "agent".into(),
            at,
            1,
            "a".repeat(64),
            TelemetryReport {
                agent_version: "test".into(),
                profile: ObservationProfile::Normal,
                uptime_secs: 1,
                os: "Linux".into(),
                kernel: "test".into(),
                arch: "x86_64".into(),
                inventory_hash: "b".repeat(64),
                lease_id: None,
                resources: ResourceRollup {
                    interval_start: at - 1,
                    interval_end: at,
                    ..Default::default()
                },
                traffic: TrafficCheckpoint {
                    sampled_at: at,
                    ..Default::default()
                },
                processes: None,
                listeners: None,
                coverage: vec![],
            },
            &key,
        )
        .unwrap();
        postcard::to_allocvec(&value).unwrap()
    }

    #[test]
    fn report_envelope_rejects_stale_oversized_malformed_and_wrong_content_type() {
        let at = 2_000_000_000;
        let bytes = encoded(at);
        assert!(decode_report_envelope(Some("application/x-parade"), &bytes, at).is_ok());
        assert_eq!(
            decode_report_envelope(Some("application/x-parade"), &bytes, at + 601)
                .unwrap_err()
                .0,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            decode_report_envelope(Some("application/json"), &bytes, at)
                .unwrap_err()
                .0,
            StatusCode::UNSUPPORTED_MEDIA_TYPE
        );
        assert_eq!(
            decode_report_envelope(Some("application/x-parade"), b"broken", at)
                .unwrap_err()
                .0,
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            decode_report_envelope(
                Some("application/x-parade"),
                &vec![0; MAX_REPORT_BYTES + 1],
                at,
            )
            .unwrap_err()
            .0,
            StatusCode::PAYLOAD_TOO_LARGE
        );
    }

    #[test]
    fn forwarded_client_ip_is_used_only_for_an_explicitly_trusted_peer() {
        let path =
            std::env::temp_dir().join(format!("parade-web-test-{}.sqlite", std::process::id()));
        let db = Database::open(&path).unwrap();
        let config = Arc::new(crate::config::Config {
            hub: crate::config::HubConfig {
                listen: "127.0.0.1:8008".into(),
                database_path: path.to_string_lossy().into_owned(),
                public_url: "http://127.0.0.1:8008".into(),
                dist_dir: "/nonexistent".into(),
                release_public_key_sha256: "0".repeat(64),
                trusted_proxies: vec!["127.0.0.1".parse().unwrap()],
                secure_cookies: false,
                session_hours: 1,
                stale_after_secs: 600,
                offline_after_secs: 1_800,
            },
            dashboard: crate::config::DashboardConfig {
                title: "test".into(),
                password_hash: "$argon2id$placeholder".into(),
            },
        });
        let app = App::new(db, config);
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "203.0.113.5".parse().unwrap());
        assert_eq!(
            client_ip(&app, &headers, "192.0.2.10:1234".parse().unwrap()),
            "192.0.2.10".parse::<IpAddr>().unwrap()
        );
        assert_eq!(
            client_ip(&app, &headers, "127.0.0.1:1234".parse().unwrap()),
            "203.0.113.5".parse::<IpAddr>().unwrap()
        );
        drop(app);
        let _ = std::fs::remove_file(path);
    }
}
