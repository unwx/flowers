use std::ops::Deref;
use std::rc::Rc;

#[derive(Clone)]
pub struct MyRc<T> {
    delegate: Rc<T>,
}

impl<T> MyRc<T> {
    pub fn new(value: T) -> Self {
        Self {
            delegate: Rc::new(value),
        }
    }
}

impl<T> Deref for MyRc<T> {
    type Target = Rc<T>;

    fn deref(&self) -> &Self::Target {
        &self.delegate
    }
}
