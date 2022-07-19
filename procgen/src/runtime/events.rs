use std::{
    any::{Any, TypeId},
    collections::{hash_map::Entry, HashMap},
};

struct Ctx;

trait DynHandler {
    fn dyn_call(&self, ctx: &Ctx, event: &dyn Any) -> bool;
}

impl<H: Handler> DynHandler for H {
    fn dyn_call(&self, ctx: &Ctx, event: &dyn Any) -> bool {
        self.call(ctx, event.downcast_ref::<H::Event>().unwrap())
    }
}

trait Handler: DynHandler {
    type Event: 'static;
    fn call(&self, ctx: &Ctx, event: &Self::Event) -> bool;
}

impl<T> Handler for BasicHandler<T> {
    type Event = T;

    fn call(&self, ctx: &Ctx, event: &T) -> bool {
        (*self.function)(ctx, event)
    }
}

type HandlerFunction<T> = fn(&Ctx, &T) -> bool;

struct BasicHandler<T: 'static> {
    function: Box<HandlerFunction<T>>,
}

#[derive(Default)]
pub struct EventDispatcher {
    events: HashMap<TypeId, Vec<Box<dyn DynHandler>>>,
}

impl EventDispatcher {
    fn new() -> Self {
        Default::default()
    }

    fn add_handler<E: 'static, H: Handler<Event = E> + 'static>(
        &mut self,
        handler: H,
    ) -> &mut Self {
        let type_id = TypeId::of::<E>();

        if let Entry::Vacant(e) = self.events.entry(type_id) {
            e.insert(vec![Box::new(handler)]);
        } else {
            self.events
                .get_mut(&type_id)
                .unwrap()
                .push(Box::new(handler));
        }

        self
    }
}
