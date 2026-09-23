//! Global corridor nodes for the investor demonstration.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum Region {
    SouthAmerica,
    NorthAmerica,
    Europe,
    Asia,
    MiddleEast,
    Africa,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CountryNode {
    pub id: String,
    pub name: String,
    pub region: Region,
    pub central_bank: String,
    pub fiat_currency: String,
    pub cbdc_name: String,
    pub is_sanctioned: bool,
    pub swift_member: bool,
    pub flag: String,
    pub map_x: f64,
    pub map_y: f64,
}

pub fn country_catalog() -> Vec<CountryNode> {
    vec![
        node(
            "br",
            "Brazil",
            Region::SouthAmerica,
            "Banco Central do Brasil",
            "BRL",
            "Drex",
            false,
            true,
            "🇧🇷",
            32.0,
            72.0,
        ),
        node(
            "us",
            "United States",
            Region::NorthAmerica,
            "Federal Reserve",
            "USD",
            "Digital USD (pilot)",
            false,
            true,
            "🇺🇸",
            22.0,
            42.0,
        ),
        node(
            "de",
            "Germany",
            Region::Europe,
            "Deutsche Bundesbank / ECB",
            "EUR",
            "Digital Euro",
            false,
            true,
            "🇩🇪",
            50.0,
            34.0,
        ),
        node(
            "cn",
            "China",
            Region::Asia,
            "People's Bank of China",
            "CNY",
            "e-CNY",
            false,
            true,
            "🇨🇳",
            78.0,
            40.0,
        ),
        node(
            "ru",
            "Russia",
            Region::Europe,
            "Bank of Russia",
            "RUB",
            "Digital Ruble",
            true,
            false,
            "🇷🇺",
            62.0,
            28.0,
        ),
        node(
            "ir",
            "Iran",
            Region::MiddleEast,
            "Central Bank of Iran",
            "IRR",
            "Digital Rial",
            true,
            false,
            "🇮🇷",
            58.0,
            46.0,
        ),
        node(
            "ae",
            "United Arab Emirates",
            Region::MiddleEast,
            "Central Bank of the UAE",
            "AED",
            "Digital Dirham",
            false,
            true,
            "🇦🇪",
            60.0,
            50.0,
        ),
        node(
            "za",
            "South Africa",
            Region::Africa,
            "South African Reserve Bank",
            "ZAR",
            "Project Khokha",
            false,
            true,
            "🇿🇦",
            52.0,
            78.0,
        ),
        node(
            "in",
            "India",
            Region::Asia,
            "Reserve Bank of India",
            "INR",
            "Digital Rupee",
            false,
            true,
            "🇮🇳",
            68.0,
            52.0,
        ),
        node(
            "ng",
            "Nigeria",
            Region::Africa,
            "Central Bank of Nigeria",
            "NGN",
            "eNaira",
            false,
            true,
            "🇳🇬",
            48.0,
            58.0,
        ),
    ]
}

fn node(
    id: &str,
    name: &str,
    region: Region,
    central_bank: &str,
    fiat: &str,
    cbdc: &str,
    sanctioned: bool,
    swift: bool,
    flag: &str,
    x: f64,
    y: f64,
) -> CountryNode {
    CountryNode {
        id: id.into(),
        name: name.into(),
        region,
        central_bank: central_bank.into(),
        fiat_currency: fiat.into(),
        cbdc_name: cbdc.into(),
        is_sanctioned: sanctioned,
        swift_member: swift,
        flag: flag.into(),
        map_x: x,
        map_y: y,
    }
}

pub fn find_country(id: &str) -> Option<CountryNode> {
    country_catalog()
        .into_iter()
        .find(|item| item.id == id)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegionActivity {
    pub region: Region,
    pub label: String,
    pub active_nodes: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CorridorVolume {
    pub pair: String,
    pub volume_usd: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalNetworkSnapshot {
    pub regions: Vec<RegionActivity>,
    pub corridors: Vec<CorridorVolume>,
}

pub fn global_network_snapshot() -> GlobalNetworkSnapshot {
    GlobalNetworkSnapshot {
        regions: vec![
            region_row(Region::SouthAmerica, "América do Sul", 3),
            region_row(Region::NorthAmerica, "América do Norte", 2),
            region_row(Region::Europe, "Europa", 4),
            region_row(Region::Asia, "Ásia", 5),
            region_row(Region::MiddleEast, "Oriente Médio", 3),
            region_row(Region::Africa, "África", 2),
        ],
        corridors: vec![
            CorridorVolume {
                pair: "CNY ↔ BRL".into(),
                volume_usd: "$450M hoje".into(),
            },
            CorridorVolume {
                pair: "AED ↔ EUR".into(),
                volume_usd: "$210M hoje".into(),
            },
            CorridorVolume {
                pair: "INR ↔ ZAR".into(),
                volume_usd: "$95M hoje".into(),
            },
            CorridorVolume {
                pair: "USD ↔ BRL".into(),
                volume_usd: "$380M hoje".into(),
            },
        ],
    }
}

fn region_row(region: Region, label: &str, active: u32) -> RegionActivity {
    RegionActivity {
        region,
        label: label.into(),
        active_nodes: active,
    }
}

pub struct CorridorContext {
    pub origin: CountryNode,
    pub dest: CountryNode,
    pub sanctioned_touch: bool,
    pub usd_corridor: bool,
    pub high_swift_friction: bool,
}

pub fn corridor_context(origin_id: &str, dest_id: &str) -> Result<CorridorContext, String> {
    let origin = find_country(origin_id).ok_or_else(|| "unknown origin country".to_string())?;
    let dest = find_country(dest_id).ok_or_else(|| "unknown destination country".to_string())?;
    let sanctioned_touch = origin.is_sanctioned || dest.is_sanctioned;
    let usd_corridor = origin.fiat_currency == "USD" || dest.fiat_currency == "USD";
    let high_swift_friction = sanctioned_touch || usd_corridor || !origin.swift_member || !dest.swift_member;
    Ok(CorridorContext {
        origin,
        dest,
        sanctioned_touch,
        usd_corridor,
        high_swift_friction,
    })
}

#[tauri::command]
pub fn get_available_countries() -> Vec<CountryNode> {
    country_catalog()
}

#[tauri::command]
pub fn get_global_network_snapshot_cmd() -> GlobalNetworkSnapshot {
    global_network_snapshot()
}
