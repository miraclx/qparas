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

    let mut indexed_ids = std::collections::HashSet::new();
    let mut result = json!([]);

    let (path, queries) = args.split_at(1);

    let queries = queries
        .iter()
        .map(|query| query.split_once("="))
        .collect::<Option<Vec<_>>>()
        .ok_or("invalid query arg")?;

    // "__sort=metadata.score::-1"
    // ["__sort", [["metadata", "score"], "-1"]]
    // returns: [["metadata", "score"], "-1"]
    let sort_spec = queries
        .iter()
        .find(|(k, _)| *k == "__sort")
        .and_then(|(_, spec)| spec.split_once("::"))
        .map(|(k, v)| (k.split(".").collect::<Vec<_>>(), v))
        .map(|(k, v)| if k.is_empty() { Err("") } else { Ok((k, v)) })
        .transpose()?;

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

        let (n_page, paged_discriminant) = &mut paged_offset;

        match paged_discriminant {
            Some(Some(paged_discriminant)) => {
                request
                    .url_mut()
                    .query_pairs_mut()
                    .extend_pairs(Vec::as_slice(paged_discriminant));
            }
            Some(None) => break,
            _ => {}
        }

        info!("send request: {}", request.url());

        let response = client.execute(request).await?;

        *n_page += 1;
        match response.json().await? {
            ParasResponse::Value(value) => {
                info!("received unpaged response");
                result = value;
                paged_discriminant.replace(None);
            }
            ParasResponse::Paged { page } => {
                let collection = result.as_array_mut().ok_or("unexpected")?;

                let last_entry = page.results.last();

                let new_paged_discriminant = last_entry.and_then(|last| {
                    let mut res = vec![];
                    let default_id_spec = Some((vec!["_id"], ""));
                    let mut specs = [&default_id_spec, &sort_spec].into_iter();
                    while let Some(Some((ref selectors, _))) = specs.next() {
                        let mut entry = Some(last);
                        let mut selectors = selectors.iter();
                        let mut last_used_selector = None;
                        while let (Some(parent), Some(selector)) = (entry, selectors.next()) {
                            last_used_selector.replace(selector);
                            entry = parent.get(selector);
                        }
                        if let (Some(val), Some(selector)) = (entry, last_used_selector) {
                            // __sort=metadata.score::-1 will add score_next=652.3842
                            if let Some(val) = val
                                .as_str()
                                .map(|s| s.to_string())
                                .or_else(|| val.as_f64().map(|s| s.to_string()))
                            {
                                res.push((format!("{}_next", selector), val));
                            }
                        }
                    }
                    Some(res)
                });

                let mut ids = vec![];
                collection.extend(page.results.into_iter().filter(|entry| {
                    entry
                        .get("_id")
                        .and_then(|entry| entry.as_str())
                        .map_or(true, |id| {
                            ids.push(id.to_string());
                            !indexed_ids.contains(id)
                        })
                }));
                let pre_length = indexed_ids.len();
                indexed_ids.extend(ids);
                if indexed_ids.len() == pre_length {
                    paged_discriminant.replace(None);
                } else {
                    paged_discriminant.replace(new_paged_discriminant);
                }

                info!(
                    "got page {}, total entries = {}, offset = {:?}",
                    n_page,
                    collection.len(),
                    paged_discriminant
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
