//! # QParas
//!
//! Query the Paras.id API, returning JSON data.
//!
//! ## Usage
//!
//! See <https://parashq.github.io/>
//!
//! ```text
//! $ qparas [query] [params...]
//! ```
//!
//! ## Example query: `token-series`
//!
//! ```console
//! $ qparas token-series [params...]
//! ```
//!
//! - `collection_id`=`mint.havendao.near`
//! - `exclude_total_burn`=`true`
//! - `lookup_token`=`true`
//! - `contract_id`=`x.paras.near`
//! - `title`=`Dino Kid`
//! - `token_series_id`=`1`
//! - `min_price`=`1100000000000000000000001`
//! - `max_price`=`1100000000000000000000001`
//! - `creator_id`=`hdriqi`
//! - `is_verified`=`true`
//! - `creator_id`=`afiqshofy.near`
//! - `category_id`=`card4card-nov-21`
//! - `collection_search`=`fiction`
//! - `owner_id`=`irfi.near`
//! - `search`=`key to paras`
//! - `attributes`[`kind]=Normies`
//! - `null`=`null`
//! - Qualifiers
//!   - `__sort`=`lowest_price::1`
//!   - `__skip`=`0`
//!   - `__limit`=`1`

use std::io::{self, Write};

use log::{debug, info};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ParasResponse {
    Paged {
        // status: u8,
        #[serde(rename = "data")]
        page: PagedData,
    },
    Value(Value),
}

#[derive(Debug, Deserialize)]
struct PagedData {
    results: Vec<Value>,
    // skip: u8,
    // limit: u8,
}

const PARAS_URL: &str = "https://api-v2-mainnet.paras.id";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args = std::env::args().skip(1).collect::<Vec<_>>();

    let mut result = json!([]);

    let (path, queries) = args.split_at(1);

    let queries = queries
        .iter()
        .map(|query| query.split_once("="))
        .collect::<Option<Vec<_>>>()
        .ok_or("invalid query arg")?;

    debug!("user queries: {:?}", queries);

    let url = format!(
        "{}/{}",
        std::env::var("PARAS_URL").as_deref().unwrap_or(PARAS_URL),
        path[0]
    );
    info!("base url: {}", url);

    let client = reqwest::Client::new();

    let mut request = client.get(url).build()?;

    request
        .url_mut()
        .query_pairs_mut()
        .append_pair("__limit", "30")
        .extend_pairs(queries.iter())
        .finish();

    let mut paged_offset = (0, None);

    let mut stderr = io::stderr().lock();
    let mut stdout = io::stdout().lock();
    loop {
        write!(
            stderr,
            "\x1b[K\x1b[38;5;249m(Page {}: {} entries)\x1b[0m\x1b[G",
            paged_offset.0,
            result.as_array().map_or_else(
                || result.as_object().map_or(0, |o| !o.is_empty() as usize),
                |c| c.len()
            )
        )?;

        let mut request = request.try_clone().ok_or("request clone")?;

        match paged_offset.1 {
            Some(Some(ref keys)) => {
                request
                    .url_mut()
                    .query_pairs_mut()
                    .extend_pairs(Vec::as_slice(&keys));
            }
            Some(None) => break,
            _ => {}
        }

        info!("send request: {}", request.url());

        let response = client.execute(request).await?;

        let (n_page, last) = &mut paged_offset;

        *n_page += 1;
        match response.json().await? {
            ParasResponse::Value(value) => {
                info!("received unpaged response");
                result = value;
                last.replace(None);
            }
            ParasResponse::Paged { page } => {
                let collection = result.as_array_mut().ok_or("unexpected")?;
                last.replace(page.results.last().and_then(|last| {
                    let mut res = vec![];
                    if let Some(id) = last.get("_id").and_then(|id| id.as_str()) {
                        res.push(("_id_next".to_string(), id.to_string()));
                    };
                    Some(res)
                }));
                collection.extend(page.results);
                info!(
                    "got page {}, total entries = {}, offset = {:?}",
                    n_page,
                    collection.len(),
                    last
                );
            }
        }
    }

    write!(stderr, "\x1b[K")?; // clean the previous progress bar
    match writeln!(stdout, "{:#}", result) {
        Err(err) if err.kind() == io::ErrorKind::BrokenPipe => {}
        any => any?,
    };
    writeln!(
        stderr,
        "(Pages: {}, Entries: {})",
        paged_offset.0,
        result.as_array().map_or_else(
            || result.as_object().map_or(0, |o| !o.is_empty() as usize),
            |c| c.len()
        )
    )?;

    Ok(())
}
