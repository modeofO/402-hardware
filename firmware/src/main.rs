mod api;
mod display;
mod pins;
mod touch;
mod types;
mod vend;
mod wifi;

use log::info;
use types::TerminalState;

fn main() {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    info!("x402 vending terminal starting");
    info!("State: {:?}", TerminalState::Boot);

    // TODO: Initialize peripherals and enter state machine loop
    info!("Scaffold complete — modules loaded, waiting for implementation");

    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
