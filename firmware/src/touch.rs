use log::info;

pub struct Touch;

impl Touch {
    pub fn init() -> anyhow::Result<Self> {
        info!("Touch: stub init");
        Ok(Self)
    }

    pub fn poll(&self) -> Option<usize> {
        None
    }
}
