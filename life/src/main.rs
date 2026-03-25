#![allow(unused)]

fn cmp<'a_life, 'b_life, T>(a: &'a_life str, b: &'b_life T) -> &'a_life T
where
  'b_life: 'a_life,
  T: ?Sized + 'a_life,
{
  b
}

macro_rules! readln {
  () => {
    let mut string = String::new();
    std::io::stdin().read_line(&mut string);
    string
  };

  ($($inp: tt)* ) => {{
    print!($($inp)*);
    use std::io::prelude::Write;
    std::io::stdout().flush();
    let mut string = String::new();
    std::io::stdin().read_line(&mut string);
    string.trim().to_owned()
  }};
}

fn test<'stable>() {
  let mut map: std::collections::HashMap<&str, &str> = Default::default();

  let runtime_var = readln!("> ");

  let runt = readln!("{}: > ", runtime_var);

  map.insert(&runtime_var, &runt);

  let result = cmp("Failed", &runtime_var);

  println!("---");
  println!("result: {map:#?}");
}

fn main() {
  test();
}
