#![allow(unused)]

use std::{
  ffi::c_short,
  fmt::{Debug, Display},
  sync::Mutex,
};

trait Trait {
  fn display(&self)
  where
    Self: Display,
  {
    println!("trait: {}", self);
  }
}

struct Refs<'a, T>(&'a T);

impl<T> Trait for Refs<'_, T> {}

impl<T> Display for Refs<'_, T>
where
  T: Debug,
{
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "debug {:?}", self.0)
  }
}

struct Struct<'s, T>(&'s mut [T]);

impl<'a, T> Struct<'a, T> {
  fn new<'new: 'a>(n: &'new mut [T]) -> Self {
    Self(n)
  }
}

impl<'real, T> Struct<'real, T> {
  fn get_ontime_mut<'once>(mut self) -> &'once mut T
  where
    'real: 'once,
  {
    let r = &mut self.0[0];
    // self.0 = &mut self.0[1..];

    r
  }
}

#[derive(Debug)]
struct Mtx {
  inner: Mutex<String>,
}

impl Mtx {
  fn get_string(&mut self) -> &mut str {
    self.inner.get_mut().unwrap()
  }

  fn modify(&self, new: &str) {
    let len = self.inner.lock().unwrap().len();
    self.inner.lock().unwrap().insert_str(len, new);
  }

  fn mutation(&self) {
    *(self.inner.lock().unwrap()) = "mutated".to_owned();
  }
}

fn mutext() {
  let mut m = Mtx {
    inner: Mutex::new(String::from("data A ")),
  };

  println!("before : {:#?}", m);

  m.modify("data b");

  println!("after : {:#?}", m);

  let mut ostr: &mut str = m.get_string();

  unsafe {
    *(ostr.as_mut_ptr().add(2) ) = 55u8;
  }

  let _ = drop(ostr);
  println!("after : {:#?}", m);

  m.mutation();
  println!("after : {:#?}", m);
}

fn main() {
  let a = 44i32;
  let refs = Refs(&a);
  refs.display();

  let mut data: Vec<_> = (1..10).into_iter().collect();
  let mut s = Struct::new(data.as_mut_slice());

  let mut once = s.get_ontime_mut();
  *once = 33;

  println!("muted data: {:?}", data);

  mutext();
}

#[cfg(test)]
mod test {

  struct Has<'lifetime> {
    lifetime: &'lifetime str,
  }

  #[test]
  fn has() {
    let long = String::from("long");
    let mut has = Has { lifetime: &long };
    assert_eq!(has.lifetime, "long");

    {
      let short = Box::new(String::from("short"));
      let short = short.leak();
      // "switch" to short lifetime
      // has.lifetime = &short; // short cant live long
      assert_eq!(has.lifetime, "short");

      // "switch back" to long lifetime (but not really)
      has.lifetime = &long;
      assert_eq!(has.lifetime, "long");
      // `short` dropped here
    }

    assert_eq!(has.lifetime, "long");
  }
}
