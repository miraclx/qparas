<!-- markdownlint-disable MD014 -->

# QParas

Query the [Paras](https://paras.id) API, returning JSON data.

See <https://parashq.github.io> for a broader list of queries and their respective parameters.

## Installation

First, install Rust and Cargo. See <https://rustup.rs/>.

```bash
cargo install qparas
```

## Usage

```text
$ qparas [query] [params...]
```

## Examples

See <https://parashq.github.io> for a broader list of queries and their respective parameters.

- List all tokens for a particular collection that are for sale.

  ```console
  $ qparas token-series collection_id=mint.havendao.near min_price=0 __sort=metadata.score::-1
  ```

  - `__sort=metadata.score::-1`: Sort by rarity score in descending order.

- Get the two most recent price updates for a single token.

  ```console
  $ qparas activities contract_id=mint.havendao.near token_id=253 type=add_market_data __limit=2 __min=2
  ```

  - `__limit=2`: Ask the server to return two results per page.
  - `__min=2`: Return at least 2 results.

- Get the 50 most recent activities for a collection.

  ```console
  $ qparas collection-activities collection_id=mint.havendao.near __limit=25 __min=50
  ```

  - `__limit=50`: Ask the server to return 25 results per page.
  - `__min=50`: Return at least 50 results.
