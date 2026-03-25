use crate::prelud::Signel;

#[derive(Debug, Clone)]
pub enum Return {
  Success,
  Failure,

  Event1,
  Event2,
  Event3,

  Info(String),

  Unbound,
}

unsafe impl Send for Return {}
unsafe impl Sync for Return {}

impl Return {
  pub fn info<T>(i: T) -> Signel
  where
    T: AsRef<str>,
  {
    let s: &str = i.as_ref();
    Ok(Self::Info(s.to_owned()))
  }

  pub fn ok() -> Signel {
    Ok(Self::Success)
  }

  pub fn fail() -> Signel {
    Ok(Self::Failure)
  }

  pub fn event(id: u8) -> Signel {
    Ok(match id {
      1 => Self::Event1,
      2 => Self::Event2,
      3 => Self::Event3,
      _ => Self::Unbound,
    })
  }
}
