use gtfs_rt::FeedEntity;
use gtfs_rt::VehiclePosition;
use kactus::insert::insert_gtfs_rt;
use redis::Commands;
use reqwest::Client as ReqwestClient;
use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use std::time::SystemTime;
#[derive(Debug, Deserialize)]
struct Meta {
    total: u32,
    timeHorizon: u32,
}

#[derive(Debug, Deserialize)]
struct ResponseItem {
    bId: String,
    d: String,
    h: u32,
    hN: String,
    dId: Option<String>,
    vId: String,
    dH: bool,
    uT: String,
    la: f64,
    lo: f64,
    mId: String,
    mD: String,
    mLn: String,
    mSn: String,
    s: u32,
    sD: String,
    sLn: String,
    sSn: String,
    oEId: Option<String>,
    oEP: Option<String>,
    oLa: Option<f64>,
    oLo: Option<f64>,
    lId: u32,
    templates: Option<String>,
}

#[derive(Debug, Deserialize)]
struct JsonResponse {
    status: String,
    meta: Meta,
    response: Vec<ResponseItem>,
}
