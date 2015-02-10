
pub trait Future: Send {
    type Item: Send;

    fn then<F>(self, f: F) where F: FnOnce(Self::Item) + Send;
}

pub trait FutureExt: Future + Sized {
    fn map<F, B>(self, f: F) -> Map<Self, F> where 
        F: FnOnce(Self::Item) -> B + Send,
    {
        Map{future: self, f: f}
    }
}

impl<Fut> FutureExt for Fut where Fut: Future {}

#[must_use = "future adaptors are lazy and do nothing unless consumed"]
pub struct Map<Fut, F> {
    future: Fut,
    f: F,
}

impl<Fut: Future, F, B: Send> Future for Map<Fut, F> where F: FnOnce(Fut::Item) -> B + Send {
    type Item = B;

    fn then<G>(self, g: G) where G: FnOnce(<Self as Future>::Item) + Send {
        let Map{future, f} = self;
        future.then(move |b| g(f(b)))
    }
}
