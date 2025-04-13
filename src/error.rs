use std::fmt;
#[derive(Debug, PartialEq)]
pub struct CapViCamError {
    msg: String,
}

impl std::convert::From<String> for CapViCamError {
    fn from(msg: String) -> Self {
        CapViCamError { msg }
    }
}
impl std::convert::From<&str> for CapViCamError {
    fn from(msg: &str) -> Self {
        CapViCamError {
            msg: msg.to_string(),
        }
    }
}
impl fmt::Display for CapViCamError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self.msg)
    }
}
