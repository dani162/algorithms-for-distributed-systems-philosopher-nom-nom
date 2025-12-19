use philosopher_nom_nom_ring::Transceiver;

pub struct Thinker {
    transceiver: Transceiver,
}
impl Thinker {
    pub fn new(transceiver: Transceiver) -> Self {
        Self { transceiver }
    }

    pub fn tick(&mut self, buffer: &mut [u8]) {
        todo!()
    }
}
