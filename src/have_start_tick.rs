pub trait HaveBaseStartTick {
    fn base_start_tick(&self) -> u32;
}

pub trait HaveStartTick {
    fn start_tick(&self) -> u32;
}
