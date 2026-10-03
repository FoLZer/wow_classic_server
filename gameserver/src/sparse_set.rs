pub struct SparseSet<T> {
    entries: Vec<Entry<T>>,
    active_indices: Vec<usize>,
}

impl<T> SparseSet<T> {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            active_indices: Vec::new(),
        }
    }

    pub fn from_vec(values: Vec<T>) -> Self {
        let active_indices = (0..values.len()).collect();
        let entries = values
            .into_iter()
            .enumerate()
            .map(|(active_index, value)| Entry {
                active_index: Some(active_index),
                value,
            })
            .collect();

        Self {
            entries,
            active_indices,
        }
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.entries.get(index).map(|v| &v.value)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.entries.get_mut(index).map(|v| &mut v.value)
    }

    pub fn activate(&mut self, index: usize) {
        if self.entries[index].active_index.is_some() {
            return;
        }

        self.entries[index].active_index = Some(self.active_indices.len());
        self.active_indices.push(index);
    }

    pub fn deactivate(&mut self, index: usize) {
        let Some(position) = self.entries[index].active_index.take() else {
            return;
        };

        self.active_indices.swap_remove(position);

        if let Some(&moved_index) = self.active_indices.get(position) {
            self.entries[moved_index].active_index = Some(position);
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.active_indices
            .iter()
            .map(|&index| &self.entries[index].value)
    }
}

struct Entry<T> {
    active_index: Option<usize>,
    value: T,
}
