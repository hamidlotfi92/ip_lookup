use axum::{ extract::State, response::Json as JSONResponse };

use serde::Deserialize;
use axum_macros::debug_handler;
use std::{ net::Ipv4Addr, sync::Arc };
use tokio::sync::RwLock;
use futures::StreamExt;
use crate::{ hashmap::IPRangeHashMap, is_valid_ip, BulkIpParam, Error, IpInfo, Result };

#[derive(Clone)]
pub struct AppState {
    pub hashmap: Arc<RwLock<IPRangeHashMap>>,
}

#[derive(Deserialize, Debug)]
pub struct SingleIpParam {
    ip: String,
}

#[debug_handler]
pub async fn handler(
    params: axum::extract::Query<SingleIpParam>,
    State(state): axum::extract::State<AppState>
) -> Result<axum::response::Json<IpInfo>> {
    match is_valid_ip(&params.ip) {
        Some("IPv4") => {
            let ip = params.ip.parse::<std::net::Ipv4Addr>().unwrap();
            // Acquire a read lock on the hashmap.
            let hashmap = state.hashmap.read().await;
            if let Some(res) = hashmap.search(u32::from(ip)) {
                Ok(
                    axum::response::Json(IpInfo {
                        ip: params.ip.to_string(),
                        range: Some(res.cidr_range.to_string()),
                        asn: Some(res.asn.to_string()),
                        isp: Some(res.isp.to_string()),
                        error: None,
                    })
                )
            } else {
                Err(Error::NotFound)
            }
        }
        _ => Err(Error::InvalidDate),
    }
}

#[debug_handler]
pub async fn bulk_handler(
    State(state): axum::extract::State<AppState>,
    axum::Json(payload): axum::Json<BulkIpParam>
) -> JSONResponse<Vec<IpInfo>> {
    let results: Vec<IpInfo> = futures::stream
        ::iter(payload.ips.into_iter())
        .then(|ip_str| {
            let state = state.clone();
            async move {
                match is_valid_ip(&ip_str) {
                    Some("IPv4") => {
                        let ip = match ip_str.parse::<Ipv4Addr>() {
                            Ok(ip) => ip,
                            Err(_) => {
                                return IpInfo {
                                    ip: ip_str.clone(),
                                    range: None,
                                    asn: None,
                                    isp: None,
                                    error: Some("Invalid IPv4 format".to_string()),
                                };
                            }
                        };

                        let hashmap = state.hashmap.read().await;
                        if let Some(info) = hashmap.search(u32::from(ip)) {
                            IpInfo {
                                ip: ip_str.clone(),
                                range: Some(info.cidr_range.to_string()),
                                asn: Some(info.asn.to_string()),
                                isp: Some(info.isp.to_string()),
                                error: None,
                            }
                        } else {
                            IpInfo {
                                ip: ip_str.clone(),
                                range: None,
                                asn: None,
                                isp: None,
                                error: Some("IP not found".to_string()),
                            }
                        }
                    }
                    Some("IPv6") =>
                        IpInfo {
                            ip: ip_str.clone(),
                            range: None,
                            asn: None,
                            isp: None,
                            error: Some("IPv6 lookup not supported".to_string()),
                        },
                    _ =>
                        IpInfo {
                            ip: ip_str.clone(),
                            range: None,
                            asn: None,
                            isp: None,
                            error: Some("Invalid IP address".to_string()),
                        },
                }
            }
        })
        .collect().await;

    JSONResponse(results)
}
