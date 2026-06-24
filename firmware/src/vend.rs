use log::info;

pub struct Vend;

impl Vend {
    pub fn init() -> anyhow::Result<Self> {
        info!("Vend: stub init (relay on GPIO 26)");
        Ok(Self)
    }

    pub fn dispense(&mut self, pulse_ms: u64) {
        info!("Vend: firing relay for {}ms", pulse_ms);
        std::thread::sleep(std::time::Duration::from_millis(pulse_ms));
        info!("Vend: relay off");
    }
}
