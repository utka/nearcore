#[macro_use] extern crate lazy_static;
#[macro_use] extern crate prometheus;
use prometheus::{self, IntCounter};

lazy_static! {
    pub static ref STORE_WRITE_COUNT: IntCounter =
        register_int_counter!("near_store_write_count",
            "Total number of writes to DB").unwrap();
    pub static ref STORE_READ_COUNT: IntCounter =
        register_int_counter!("near_store_read_count",
            "Total number of reads from DB");
    pub static ref STORE_EXIST_COUNT: IntCounter =
        register_int_counter!("near_store_read_count",
            "Total number of checks for if key is exists in DB")

    pub static ref STORE_WRITE_BYTES: IntCounter =
        register_int_counter!("near_store_write_bytes",
            "Total bytes written to the store")
    pub static ref STORE_READ_BYTES: IntCounter =
        register_int_counter!("near_store_read_bytes",
            "Total bytes read from the store")
}

