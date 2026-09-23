mod commands;
mod countries;
mod simulate;

/// Desktop shell for the investor demonstration.
/// Commands in `commands` are mocks with the same shapes the gateway will use later.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            commands::start_live_feed(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::initiate_payment,
            commands::get_network_stats,
            commands::get_wallet_balances,
            commands::get_transactions,
            commands::simulate_live_transaction,
            simulate::simulate_payment,
            countries::get_available_countries,
            countries::get_global_network_snapshot_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running the sovereign desk");
}
