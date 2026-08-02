mod config;
mod db;
mod findings;
mod traffic;
mod web;

use argon2::password_hash::{PasswordHasher, SaltString};
use argon2::Argon2;
use db::Database;
use std::net::SocketAddr;
use std::sync::Arc;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

fn random_hex<const N: usize>() -> String {
    let mut bytes = [0u8; N];
    getrandom::getrandom(&mut bytes).expect("operating-system RNG");
    parade_common::hex_encode(&bytes)
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).is_some_and(|value| value == "--version") {
        println!("parade-hub {VERSION}");
        return;
    }
    if args.get(1).is_some_and(|value| value == "hash-password") {
        if args.get(2).is_some() {
            eprintln!("usage: printf '%s\\n' 'password' | parade-hub hash-password");
            std::process::exit(2)
        }
        use std::io::Read as _;
        let mut password = String::new();
        std::io::stdin()
            .read_to_string(&mut password)
            .expect("read password from standard input");
        let password = password.trim_end_matches(['\r', '\n']);
        if password.len() < 12 {
            eprintln!("password must contain at least 12 characters");
            std::process::exit(2)
        }
        let salt =
            SaltString::encode_b64(&parade_common::hex_decode(&random_hex::<16>()).expect("hex"))
                .expect("salt");
        println!(
            "{}",
            Argon2::default()
                .hash_password(password.as_bytes(), &salt)
                .expect("hash password")
        );
        return;
    }
    let path = args
        .get(1)
        .map(String::as_str)
        .unwrap_or("/etc/parade/hub.toml");
    let cfg = Arc::new(config::load(path).unwrap_or_else(|error| {
        eprintln!("parade-hub: {error}");
        std::process::exit(1)
    }));
    let db = Database::open(&cfg.hub.database_path).unwrap_or_else(|error| {
        eprintln!("parade-hub: {error}");
        std::process::exit(1)
    });
    let maintenance_db = db.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            let now = chrono::Utc::now().timestamp();
            if let Err(error) = maintenance_db.rollover_due_cycles(now) {
                eprintln!("parade-hub: traffic rollover maintenance failed: {error}");
            }
            if let Err(error) = maintenance_db.prune_operational_history(now) {
                eprintln!("parade-hub: retention maintenance failed: {error}");
            }
        }
    });
    let app = web::App::new(db, cfg.clone());
    let addr: SocketAddr = cfg.hub.listen.parse().unwrap_or_else(|_| {
        eprintln!("parade-hub: invalid listen address {}", cfg.hub.listen);
        std::process::exit(1)
    });
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .unwrap_or_else(|error| {
            eprintln!("parade-hub: cannot bind {addr}: {error}");
            std::process::exit(1)
        });
    eprintln!("parade-hub {VERSION} · read-only monitoring · http://{addr}");
    axum::serve(
        listener,
        web::router(app).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .expect("server error");
}
