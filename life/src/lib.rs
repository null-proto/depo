#![allow(unused)]

pub mod async_;
pub mod ret;
pub mod prelud;

fn cmp<'a_life, 'b_life>(a: &'a_life str, b: &'b_life str) -> &'a_life str
where
  'b_life: 'a_life,
{
  b
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn it_works() {
    let result = cmp("2", "6");
    assert_eq!(result, "6");
  }
}
