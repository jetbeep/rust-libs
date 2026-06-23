#![allow(warnings)]

#[cfg(not(test))]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
