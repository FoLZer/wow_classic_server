use std::{
    collections::BTreeMap,
    marker::PhantomData,
    num::NonZeroU32,
    ops::Bound::{Excluded, Unbounded},
};

use common::guid::{Guid, GuidType};

pub enum GuidAllocatorError {
    AlreadyFree,
    NotAllocated,
}

pub struct GuidAllocator<T: GuidType> {
    next_id: u64,
    free_ranges: BTreeMap<u32, u32>,
    marker: PhantomData<T>,
}

impl<T: GuidType> GuidAllocator<T> {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            free_ranges: BTreeMap::new(),
            marker: PhantomData,
        }
    }

    pub fn allocate(&mut self) -> Option<Guid<T>> {
        if let Some((&start, &end)) = self.free_ranges.first_key_value() {
            self.free_ranges.remove(&start);
            if start < end {
                self.free_ranges.insert(start + 1, end);
            }

            return Some(Guid::from_u32(NonZeroU32::new(start).unwrap()));
        }

        if self.next_id > u32::MAX as u64 {
            return None;
        }

        let id = self.next_id as u32;
        self.next_id += 1;
        Some(Guid::from_u32(NonZeroU32::new(id).unwrap()))
    }

    pub fn free(&mut self, guid: Guid<T>) -> Result<(), GuidAllocatorError> {
        let id = guid.get_u32().get();
        if id as u64 >= self.next_id {
            return Err(GuidAllocatorError::NotAllocated);
        }

        let predecessor = self
            .free_ranges
            .range(..=id)
            .next_back()
            .map(|(&start, &end)| (start, end));
        if predecessor.is_some_and(|(_, end)| end >= id) {
            return Err(GuidAllocatorError::AlreadyFree);
        }

        let successor = self
            .free_ranges
            .range((Excluded(id), Unbounded))
            .next()
            .map(|(&start, &end)| (start, end));

        let mut range_start = id;
        let mut range_end = id;

        if let Some((start, end)) = predecessor
            && end.checked_add(1) == Some(id)
        {
            self.free_ranges.remove(&start);
            range_start = start;
        }

        if let Some((start, end)) = successor
            && id.checked_add(1) == Some(start)
        {
            self.free_ranges.remove(&start);
            range_end = end;
        }

        self.free_ranges.insert(range_start, range_end);
        if range_end as u64 + 1 == self.next_id {
            self.free_ranges.remove(&range_start);
            self.next_id = range_start as u64;
        }

        Ok(())
    }
}
