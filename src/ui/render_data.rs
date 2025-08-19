use crate::ui::prelude::*;

use std::hash::Hash;

#[derive(Debug)]
pub struct RenderDataStore<K, V>(HashMap<K, Rc<RefCell<V>>>);

impl<K, V> Default for RenderDataStore<K, V> {
    fn default() -> Self {
        Self(HashMap::new())
    }
}

impl<K, V> RenderDataStore<K, V>
where
    K: Eq + Hash + Clone,
    V: Default,
{
    pub fn get(&mut self, k: &K) -> Rc<RefCell<V>> {
        let v = self.0.get(k);
        if let Some(val) = v {
            return val.clone();
        }
        let new_val = Rc::new(RefCell::new(V::default()));
        let new_key = k.clone();

        self.0.insert(new_key, new_val.clone());
        new_val
    }
}
