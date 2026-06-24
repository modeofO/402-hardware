use log::info;

pub struct Display;

impl Display {
    pub fn init() -> anyhow::Result<Self> {
        info!("Display: stub init");
        Ok(Self)
    }

    pub fn show_message(&mut self, msg: &str) {
        info!("Display: {}", msg);
    }

    pub fn show_menu(&mut self, items: &[crate::types::MenuItem]) {
        info!("Display: showing {} menu items", items.len());
        for item in items {
            info!("  {} — {} USDC", item.name, item.price_usdc);
        }
    }

    pub fn show_qr(&mut self, data: &str) {
        info!("Display: QR code for {}", data);
    }
}
