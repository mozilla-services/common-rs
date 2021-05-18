#[macro_use]
extern crate lazy_static;
extern crate hostname;
extern crate reqwest;

pub mod aws;

pub use hostname::get_hostname;
