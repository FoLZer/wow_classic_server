use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    ops::{Deref, DerefMut},
};

pub trait DirtyTrackable {
    fn is_dirty(&self) -> bool;
}

pub struct DirtyTrackingMap<K, V>
where
    K: Eq + Hash + Clone,
    V: DirtyTrackable,
{
    values: HashMap<K, V>,
    dirty_keys: HashSet<K>,
}

impl<K, V> Default for DirtyTrackingMap<K, V>
where
    K: Eq + Hash + Clone,
    V: DirtyTrackable,
{
    fn default() -> Self {
        Self {
            values: HashMap::new(),
            dirty_keys: HashSet::new(),
        }
    }
}

impl<K, V> DirtyTrackingMap<K, V>
where
    K: Eq + Hash + Clone,
    V: DirtyTrackable,
{
    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.values.get(key)
    }

    pub fn get_mut(&mut self, key: &K) -> Option<DirtyTrackingMut<'_, K, V>> {
        let value = self.values.get_mut(key)?;
        Some(DirtyTrackingMut {
            key: key.clone(),
            value,
            dirty_keys: &mut self.dirty_keys,
        })
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.values.iter()
    }

    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.values.keys()
    }

    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.values.values()
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        if value.is_dirty() {
            self.dirty_keys.insert(key.clone());
        } else {
            self.dirty_keys.remove(&key);
        }
        self.values.insert(key, value)
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.dirty_keys.remove(key);
        self.values.remove(key)
    }

    pub fn extract_if<F>(
        &mut self,
        mut predicate: F,
    ) -> impl Iterator<Item = (K, V)> + use<'_, K, V, F>
    where
        F: FnMut(&K, &mut V) -> bool,
    {
        let dirty_keys = &mut self.dirty_keys;
        self.values.extract_if(move |key, value| {
            let extract = predicate(key, value);

            if extract {
                dirty_keys.remove(key);
            } else if value.is_dirty() {
                dirty_keys.insert(key.clone());
            }

            extract
        })
    }

    pub fn take_dirty(&mut self) -> HashSet<K> {
        std::mem::take(&mut self.dirty_keys)
    }
}

impl<'a, K, V> IntoIterator for &'a DirtyTrackingMap<K, V>
where
    K: Eq + Hash + Clone,
    V: DirtyTrackable,
{
    type Item = (&'a K, &'a V);
    type IntoIter = std::collections::hash_map::Iter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.values.iter()
    }
}

pub struct DirtyTrackingMut<'a, K, V>
where
    K: Eq + Hash + Clone,
    V: DirtyTrackable,
{
    key: K,
    value: &'a mut V,
    dirty_keys: &'a mut HashSet<K>,
}

impl<K, V> Deref for DirtyTrackingMut<'_, K, V>
where
    K: Eq + Hash + Clone,
    V: DirtyTrackable,
{
    type Target = V;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<K, V> DerefMut for DirtyTrackingMut<'_, K, V>
where
    K: Eq + Hash + Clone,
    V: DirtyTrackable,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.value
    }
}

impl<K, V> Drop for DirtyTrackingMut<'_, K, V>
where
    K: Eq + Hash + Clone,
    V: DirtyTrackable,
{
    fn drop(&mut self) {
        if self.value.is_dirty() {
            self.dirty_keys.insert(self.key.clone());
        }
    }
}
