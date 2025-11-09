use std::time::Duration;

use tokio::time::Instant;

pub struct CacheData<T, const VALID_FOR_SECONDS: u64> {
    data: T,
    time_set: Instant
}

impl<T, const VALID_FOR: u64> CacheData<T, VALID_FOR> {
    pub fn new(data: T) -> Self {
        Self {
            data,
            time_set: Instant::now()
        }
    }

    pub fn get(&self) -> &T {
        &self.data
    }

    pub fn is_valid(&self) -> bool {
        self.time_set + Duration::from_secs(VALID_FOR) <= Instant::now()
    }

    pub fn update_data(&mut self, new_data: T) {
        self.data = new_data;
        self.time_set = Instant::now();
    }
}