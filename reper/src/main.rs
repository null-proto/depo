fn main() {
  let mut num = 0u64;

  println!("0x{:.>16x?}", num);

  num |= 0xaau8 as u64;

  println!("0x{:.>16x?}", num);

  num |= (0xbbu8 as u64) << 8;
  println!("0x{:.>16x?}", num);

  num |= (0xccu8 as u64) << 16;
  println!("0x{:.>16x?}", num);

  num |= (0xddu8 as u64) << 24;
  println!("0x{:.>16x?}", num);

  num |= (0xeeu8 as u64) << 32;
  num |= (0xffu8 as u64) << 56;

  println!("0x{:.>16X?}", num);

  println!("0x{:.>2X?}" , (num >> 56) as u8);
}
