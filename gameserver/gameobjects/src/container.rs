use common::guid::{self, Guid};

pub struct ContainerFields {
    pub num_slots: u32,
    pub _padding: u32,
    pub items: [Option<Guid<guid::Item>>; 28],
}
