#![allow(non_snake_case)]
use chrono::Utc;
use dioxus::prelude::*;
use futures::future::join_all;
use serde::{Deserialize, Serialize};

const API_KEY: &str = "eKvfgR/g3LnmNbhfe4pfiQ==BxWkl56ZIGWtIt5K"; // 请替换为您的实际API密钥

fn main() {
    launch(App);
}

pub fn App() -> Element {
    use_context_provider(|| Signal::new(PreviewState::Unset));
    let crypto_list = use_signal(|| vec!["BTCUSDT", "ETHUSDT", "SOLUSDT", "DOGEUSDT", "BNBUSDT"]);

    rsx! {
        div {
            class: "container mx-auto p-4",
            h1 { class: "text-3xl font-bold mb-6 text-center text-blue-600", "Crypto Price Tracker" }
            div {
                class: "flex flex-col md:flex-row gap-4",
                div {
                    class: "w-full md:w-1/2 bg-white rounded-lg shadow-lg p-4",
                    h2 { class: "text-xl font-semibold mb-4 text-gray-800", "Cryptocurrency List" }
                    CryptoList { crypto_list: crypto_list.read().clone() }
                }
                div {
                    class: "w-full md:w-1/2 bg-white rounded-lg shadow-lg p-4",
                    h2 { class: "text-xl font-semibold mb-4 text-gray-800", "Detailed Preview" }
                    Preview {}
                }
            }
        }
    }
}

#[component]
fn CryptoList(crypto_list: Vec<&'static str>) -> Element {
    let cryptos = use_resource(move || get_crypto_prices(crypto_list.clone()));

    rsx! {
        div { class: "space-y-4",
            match &*cryptos.read_unchecked() {
                Some(Ok(list)) => rsx! {
                    for crypto in list {
                        CryptoListing { crypto: crypto.clone() }
                    }
                },
                Some(Err(err)) => rsx! {
                    div { class: "text-red-500", "An error occurred while fetching crypto prices: {err}" }
                },
                None => rsx! {
                    div { class: "text-gray-500", "Loading cryptocurrencies..." }
                },
            }
        }
    }
}

async fn resolve_crypto(
    mut full_crypto: Signal<Option<CryptoDetailData>>,
    mut preview_state: Signal<PreviewState>,
    symbol: String,
) {
    if let Some(cached) = full_crypto.as_ref() {
        *preview_state.write() = PreviewState::Loaded(cached.clone());
        return;
    }

    *preview_state.write() = PreviewState::Loading;
    if let Ok(crypto) = get_crypto_detail(&symbol).await {
        *preview_state.write() = PreviewState::Loaded(crypto.clone());
        *full_crypto.write() = Some(crypto);
    }
}

#[component]
fn CryptoListing(crypto: ReadOnlySignal<CryptoPrice>) -> Element {
    let preview_state = consume_context::<Signal<PreviewState>>();
    let CryptoPrice { symbol, price } = crypto();
    let full_crypto = use_signal(|| None);

    let formatted_price = format!("${}", price);
    let time = Utc::now().format("%D %l:%M %p");

    rsx! {
        div {
            class: "bg-gray-100 rounded-md p-4 hover:bg-gray-200 transition-colors duration-200 cursor-pointer",
            onmouseenter: move |_event| { resolve_crypto(full_crypto, preview_state, symbol.clone()) },
            div {
                class: "flex justify-between items-center",
                span { class: "text-lg font-semibold text-gray-800", "{symbol}" }
                span { class: "text-lg font-bold text-green-600", "{formatted_price}" }
            }
            div {
                class: "text-sm text-gray-600 mt-2",
                "Last updated: {time}"
            }
        }
    }
}

#[derive(Clone, Debug)]
enum PreviewState {
    Unset,
    Loading,
    Loaded(CryptoDetailData),
}

fn Preview() -> Element {
    let preview_state = consume_context::<Signal<PreviewState>>();

    rsx! {
        div { class: "bg-gray-100 rounded-md p-4 h-full",
            match preview_state() {
                PreviewState::Unset => rsx! {
                    div { class: "text-gray-600 text-center", "Hover over a cryptocurrency to see more details" }
                },
                PreviewState::Loading => rsx! {
                    div { class: "text-blue-600 text-center", "Loading detailed information..." }
                },
                PreviewState::Loaded(crypto) => {
                    rsx! {
                        div { class: "space-y-2",
                            h3 { class: "text-2xl font-bold text-gray-800 mb-4", "{crypto.symbol} Details" }
                            PreviewItem { label: "Price", value: format!("${}", crypto.price) }
                            PreviewItem { label: "24h High", value: format!("${}", crypto.high_24h) }
                            PreviewItem { label: "24h Low", value: format!("${}", crypto.low_24h) }
                            PreviewItem { label: "24h Volume", value: crypto.volume_24h.clone() }
                            PreviewItem { label: "Market Cap", value: format!("${}", crypto.market_cap) }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn PreviewItem(label: &'static str, value: String) -> Element {
    rsx! {
        div { class: "flex justify-between items-center",
            span { class: "text-gray-600", "{label}:" }
            span { class: "font-semibold text-gray-800", "{value}" }
        }
    }
}

// Define the Crypto API and types

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CryptoPrice {
    pub symbol: String,
    pub price: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CryptoDetailData {
    pub symbol: String,
    pub price: String,
    pub high_24h: String,
    pub low_24h: String,
    pub volume_24h: String,
    pub market_cap: String,
}

pub async fn get_crypto_prices(symbols: Vec<&str>) -> Result<Vec<CryptoPrice>, reqwest::Error> {
    let client = reqwest::Client::new();
    let futures = symbols.into_iter().map(|symbol| {
        let url = format!(
            "https://api.api-ninjas.com/v1/cryptoprice?symbol={}",
            symbol
        );
        let client = client.clone();
        async move {
            client
                .get(&url)
                .header("X-Api-Key", API_KEY)
                .send()
                .await?
                .json::<CryptoPrice>()
                .await
        }
    });

    Ok(join_all(futures)
        .await
        .into_iter()
        .filter_map(|crypto| crypto.ok())
        .collect())
}

pub async fn get_crypto_detail(symbol: &str) -> Result<CryptoDetailData, reqwest::Error> {
    // 注意：这是一个模拟函数，因为 API Ninjas 没有提供这样的详细信息
    // 在实际应用中，你需要使用一个提供这些信息的 API
    let price = reqwest::Client::new()
        .get(&format!(
            "https://api.api-ninjas.com/v1/cryptoprice?symbol={}",
            symbol
        ))
        .header("X-Api-Key", API_KEY)
        .send()
        .await?
        .json::<CryptoPrice>()
        .await?;

    let price_f64 = price.price.parse::<f64>().unwrap_or(0.0);

    Ok(CryptoDetailData {
        symbol: price.symbol,
        price: price.price.clone(),
        high_24h: format!("{:.2}", price_f64 * 1.05),
        low_24h: format!("{:.2}", price_f64 * 0.95),
        volume_24h: format!("{:.0}", price_f64 * 1000000.0),
        market_cap: format!("{:.0}", price_f64 * 1000000000.0),
    })
}
