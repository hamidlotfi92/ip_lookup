use axum::http::StatusCode;
use axum::response::{ IntoResponse, Response };
use axum_response_cache::CacheLayer;
use configs::Config;
use rand::Rng;
use hashmap::IPRangeDirectLookup;
use routes::{ bulk_handler, handler, AppState };
use utils::read_ip_ranges_from_file;
use std::net::{ Ipv4Addr, Ipv6Addr };
use std::time::{ Duration, Instant };
use tokio::time;
use std::sync::Arc;
use std::fs;
use tokio::sync::RwLock;
use config::Config as ConfigLoader;
use serde::{ Deserialize, Serialize };
mod hashmap;
mod utils;
mod routes;
mod configs;
pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Clone, Serialize, strum_macros::AsRefStr)]
pub enum Error {
    NotFound,
    InvalidDate,
}
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match self {
            Error::InvalidDate => (StatusCode::BAD_REQUEST, "Invalid IP address").into_response(),
            Error::NotFound => (StatusCode::NOT_FOUND, "Invalid IP address").into_response(),
        }
    }
}
async fn monitor_file_changes(state: AppState, file_path: String) {
    let mut last_mod_time = fs
        ::metadata(&file_path)
        .and_then(|meta| meta.modified())
        .ok();

    loop {
        time::sleep(Duration::from_secs(10)).await; // check every 5 minutes

        let metadata = match fs::metadata(&file_path) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("Error getting metadata for {}: {}", file_path, e);
                continue;
            }
        };

        let modified_time = match metadata.modified() {
            Ok(time) => time,
            Err(e) => {
                eprintln!("Error getting modified time for {}: {}", file_path, e);
                continue;
            }
        };

        if let Some(last) = last_mod_time {
            if modified_time > last {
                println!("File {} changed; updating hashmap...", file_path);

                let mut new_hashmap = IPRangeDirectLookup::new(30);
                if let Err(e) = read_ip_ranges_from_file(&file_path, &mut new_hashmap) {
                    eprintln!("Error reloading file {}: {}", file_path, e);
                    continue;
                }

                {
                    let mut hashmap_guard = state.hashmap.write().await;
                    *hashmap_guard = new_hashmap;
                }
                println!("hashmap successfully updated.");

                last_mod_time = Some(modified_time);
            }
        } else {
            last_mod_time = Some(modified_time);
        }
    }
}

fn is_valid_ip(ip_str: &str) -> Option<&'static str> {
    if ip_str.parse::<Ipv4Addr>().is_ok() {
        Some("IPv4")
    } else if ip_str.parse::<Ipv6Addr>().is_ok() {
        Some("IPv6")
    } else {
        None
    }
}

#[derive(Deserialize, Debug)]
struct BulkIpParam {
    ips: Vec<String>,
}

#[derive(Serialize)]
struct IpInfo {
    ip: String,
    range: Option<String>,
    asn: Option<String>,
    isp: Option<String>,
    error: Option<String>,
}
fn generate_random_ips(count: usize) -> Vec<Ipv4Addr> {
    let mut rng = rand::thread_rng();
    (0..count)
        .map(|_|
            Ipv4Addr::new(
                rng.gen_range(0..=255),
                rng.gen_range(0..=255),
                rng.gen_range(0..=255),
                rng.gen_range(0..=255)
            )
        )
        .collect()
}

#[tokio::main]
async fn main() {
    let settings: ConfigLoader = ConfigLoader::builder()
        .add_source(config::File::with_name("config"))
        .build()
        .unwrap();

    let config: Config = settings.try_deserialize().unwrap();

    let mut hashmap = IPRangeDirectLookup::new(20);
    let file_path = config.server.file_path;

    read_ip_ranges_from_file(&file_path, &mut hashmap).expect("Failed to read ita.cfg");

    let binding_address = config.server.binding_address;

    // Wrap the hashmap in an Arc and RwLock.
    let state = AppState {
        hashmap: Arc::new(RwLock::new(hashmap.clone())),
    };
    let ip_count = 1;
    let ips = generate_random_ips(ip_count);
    println!("random ips generated, testing now ...");
    let start = Instant::now();
    for ip in ips.iter() {
        hashmap.search(u32::from(*ip));
    }

    println!("{}", start.elapsed().as_nanos());
    // Spawn the file monitor task.
    let monitor_state = state.clone();
    let monitor_file_path = file_path.clone();
    tokio::spawn(async move {
        monitor_file_changes(monitor_state, monitor_file_path).await;
    });

    let app = axum::Router
        ::new()
        .route("/single", axum::routing::get(handler))
        .route("/bulk", axum::routing::post(bulk_handler))
        .layer(CacheLayer::with_lifespan(20))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(binding_address).await.unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
