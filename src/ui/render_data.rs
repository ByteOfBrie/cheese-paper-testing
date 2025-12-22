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

#[derive(Debug, Default)]
pub struct FileObjectRDStore(HashMap<FileID, Rc<RefCell<dyn Any>>>);

impl FileObjectRDStore {
    pub fn get<V: Default + 'static>(&mut self, id: &FileID) -> Rc<RefCell<dyn Any>> {
        let v = self.0.get(id);
        if let Some(val) = v {
            return val.clone();
        }
        let new_val = Rc::new(RefCell::new(V::default()));
        let new_key = id.clone();

        self.0.insert(new_key, new_val.clone());
        new_val
    }
}

#[macro_export]
macro_rules! ford_get {
    ($v:ty, $var:ident, $store:expr, $id:expr) => {
        let ford_get_data = $store.get::<$v>($id);
        let mut ford_get_data_borrowed = ford_get_data.borrow_mut();
        let $var: &mut $v = ford_get_data_borrowed.downcast_mut::<$v>().unwrap();
    };
}
