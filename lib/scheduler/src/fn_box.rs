
pub trait FnBox<Args, Result = ()> {
  extern "rust-call" fn call_box(self: Box<Self>, args: Args) -> Result;
}

impl<'a> FnOnce() for Box<FnBox() + 'a> {
  extern "rust-call" fn call_once(self, args: ()) {
    self.call_box(args)
  }
}

#[cfg(when_coherence_is_fixed)]
impl<'a, Args, Result> FnOnce<Args, Result> for Box<FnBox<Args, Result> + 'a> {
  extern "rust-call" fn call_once(self, args: Args) -> Result {
    self.call_box(args)
  }
}

impl<F, Args, Result> FnBox<Args, Result> for F where F: FnOnce<Args, Result> {
  extern "rust-call" fn call_box(self: Box<F>, args: Args) -> Result {
    (*self).call_once(args)
  }
}

#[test]
fn can_be_boxed() {
  let f = move |:| {};
  let _: Box<FnBox()> = Box::new(f);
}

#[test]
fn can_be_sent() {
  use core::kinds::Send;
  let f = move |:| {};
  let _: Box<FnBox<()> + Send> = Box::new(f);
}

#[test]
fn can_be_called() {
  let mut called = false;

  {
    let f: Box<FnBox()> = Box::new(|:| { called = true });

    f();
  }

  assert_eq!(called, true);
}

#[test]
fn can_be_called_with_args() {
  let f: Box<FnBox(usize)> = Box::new(move |:x: usize| { assert_eq!(x, 3) });
  f.call_box((3,));
}

#[test]
fn can_return_values() {
  let f: Box<FnBox() -> usize> = Box::new(move |:| 3);
  assert_eq!(f.call_box(()), 3);
}

#[test]
fn everything() {
  let f: Box<FnBox(_) -> _> = Box::new(move |:x: usize| x + 1);
  assert_eq!(f.call_box((3,)), 4);
}