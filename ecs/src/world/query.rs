use std::marker::PhantomData;


pub trait QueryParameters {
    
}

pub struct Query<P: QueryParameters> {
    _parameters: PhantomData<fn(P) -> P>,
}

impl<P: QueryParameters> Query<P> {
    
}
