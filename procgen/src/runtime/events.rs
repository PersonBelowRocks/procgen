use std::{
    any::{Any, TypeId},
    collections::{hash_map::Entry, HashMap},
};

trait DynHandler<Ctx> {
    fn dyn_call(&self, ctx: &Ctx, event: &dyn Any);
}

impl<Ctx, H: Handler<Ctx>> DynHandler<Ctx> for H {
    fn dyn_call(&self, ctx: &Ctx, event: &dyn Any) {
        self.call(ctx, event.downcast_ref::<H::Event>().unwrap())
    }
}

trait Handler<Ctx>: DynHandler<Ctx> {
    type Event: 'static;
    fn call(&self, ctx: &Ctx, event: &Self::Event);
}

impl<Ctx, T> Handler<Ctx> for BasicHandler<Ctx, T> {
    type Event = T;

    fn call(&self, ctx: &Ctx, event: &T) {
        (*self.function)(ctx, event)
    }
}

type HandlerFunction<Ctx, T> = fn(&Ctx, &T);
type HandlerStorage<Ctx> = Vec<Box<dyn DynHandler<Ctx>>>;

struct BasicHandler<Ctx, T: 'static> {
    function: Box<HandlerFunction<Ctx, T>>,
}

pub struct EventDispatcher<Ctx> {
    handlers: HashMap<TypeId, HandlerStorage<Ctx>>,
}

impl<Ctx> Default for EventDispatcher<Ctx> {
    fn default() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }
}

impl<Ctx> EventDispatcher<Ctx> {
    fn new() -> Self {
        Default::default()
    }

    fn add_handler<E: 'static, H: Handler<Ctx, Event = E> + 'static>(
        &mut self,
        handler: H,
    ) -> &mut Self {
        let type_id = TypeId::of::<E>();

        if let Entry::Vacant(e) = self.handlers.entry(type_id) {
            e.insert(vec![Box::new(handler)]);
        } else {
            self.handlers
                .get_mut(&type_id)
                .unwrap()
                .push(Box::new(handler));
        }

        self
    }

    fn fire_event<E: 'static>(&self, ctx: &Ctx, event: &E) {
        if let Some(handlers) = self.handlers.get(&event.type_id()) {
            handlers.iter().for_each(|h| h.dyn_call(ctx, event))
        }
    }
}
