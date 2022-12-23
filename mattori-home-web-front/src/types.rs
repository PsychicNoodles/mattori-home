#[derive(Debug, PartialEq)]
pub enum Mode {
    Heat,
    Cool,
    Dry,
}

#[derive(Debug, PartialEq)]
pub struct AcState {
    mode: Mode,
    temperature: u32,
    is_on: bool,
}
