use anyhow::bail;

use jup_sdk::perps::{PerpsFetcher, PerpsPosition};

pub async fn get_preps_position(
    maybe_wallet_address: Option<String>,
) -> anyhow::Result<Option<Vec<PerpsPosition>>> {
    // JUP Perps
    let wallet_address = match maybe_wallet_address {
        Some(wallet_address) => wallet_address,
        None => return Ok(None),
    };

    let perps_fetcher = PerpsFetcher::default();

    println!("Fetching positions for wallet: {wallet_address:?}");
    match perps_fetcher.fetch_positions(&wallet_address).await {
        Ok(positions_result) => {
            let positions: Vec<PerpsPosition> = positions_result
                .data_list
                .into_iter()
                .map(|position| PerpsPosition::from(position.clone()))
                .collect();
            Ok(Some(positions))
        }
        Err(error) => {
            bail!(error);
        }
    }
}
