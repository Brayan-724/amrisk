use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::mem;

#[derive(Debug, Default)]
pub struct StoresContainer {
    stores: HashMap<TypeId, Box<dyn Any>>,
    shared_stores: HashMap<TypeId, Box<dyn Any>>,
}

impl StoreContainer for StoresContainer {
    fn container(&mut self) -> &mut StoresContainer {
        self
    }

    fn shared_store<T: SharedStore>(&mut self) -> &mut T::Store {
        let type_id = TypeId::of::<T>();

        self.shared_stores
            .entry(type_id)
            .or_insert_with(|| Box::from(T::Store::default()))
            .downcast_mut()
            .expect("`TypeId` ensures `Any` type safety")
    }
}

impl StoresContainer {
    pub fn clear_local(&mut self) {
        self.stores = HashMap::default();
    }

    fn store<C: StoreContainer, T: SharedStore<C>>(container: &mut C) -> &mut T::Store {
        let type_id = TypeId::of::<T>();

        container
            .container()
            .stores
            .entry(type_id)
            .or_insert_with(|| Box::from(T::Store::default()))
            .downcast_mut()
            .expect("`TypeId` ensures `Any` type safety")
    }
}

pub trait SharedStore<C: StoreContainer = StoresContainer>: 'static + Sized {
    type Store: Default;

    fn store(container: &mut C) -> &mut Self::Store {
        container.store::<Self>()
    }
}

pub trait StoreContainer: 'static + Sized {
    fn container(&mut self) -> &mut StoresContainer;

    fn clear_store<S: SharedStore<Self>>(&mut self) {
        mem::swap(S::store(self), &mut S::Store::default());
    }

    fn store<T: SharedStore<Self>>(&mut self) -> &mut T::Store {
        StoresContainer::store::<_, T>(self)
    }

    fn shared_store<T: SharedStore>(&mut self) -> &mut T::Store {
        self.container().shared_store::<T>()
    }
}
