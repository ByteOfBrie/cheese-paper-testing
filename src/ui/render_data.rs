use std::{
    any::Any,
    cell::{OnceCell, RefCell},
    rc::Rc,
};

pub struct RenderData(OnceCell<Box<dyn Any + 'static>>);

impl Default for RenderData {
    fn default() -> Self {
        Self(OnceCell::new())
    }
}

impl std::fmt::Debug for RenderData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[render data]")
    }
}

impl RenderData {
    pub fn obtain<T: Default + 'static>(&self) -> Rc<RefCell<T>> {
        let content = self.0.get_or_init(|| {
            let data: Rc<RefCell<T>> = Rc::new(RefCell::new(T::default()));
            Box::new(data)
        });

        let rc: &Rc<RefCell<T>> = content.downcast_ref::<Rc<RefCell<T>>>().unwrap();

        rc.clone()
    }
}
