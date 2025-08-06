use std::any::Any;

struct NoData();

pub struct RenderData(Box<dyn Any + 'static>);

impl Default for RenderData {
    fn default() -> Self {
        Self(Box::new(NoData()))
    }
}

impl std::fmt::Debug for RenderData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.is::<NoData>() {
            write!(f, "[no data]")
        } else {
            write!(f, "[data]")
        }
    }
}

impl RenderData {
    pub fn obtain<T: Default + 'static>(&mut self) -> &mut T {
        if !self.0.is::<T>() {
            assert!(
                self.0.is::<NoData>(),
                "RenderData must always be accessed with the same type"
            );
            self.0 = Box::new(T::default());
        }

        self.0.downcast_mut::<T>().unwrap()
    }
}
