pub struct Report(mlua::Error);
impl std::error::Error for Report {}
impl From<mlua::Error> for Report {
    fn from(value: mlua::Error) -> Self {
        Self(value)
    }
}
impl std::fmt::Debug for Report {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::fmt::Display for Report {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
pub type Result<T> = std::result::Result<T, Report>;
