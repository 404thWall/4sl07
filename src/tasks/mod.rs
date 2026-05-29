mod map;
mod reduce;
mod saver;
mod testing;
pub use map::{run_map_task, run_map_task_default};
pub use reduce::{run_reduce_task, run_reduce_task_default};
pub use testing::{test_map, test_reduce};

pub static R: usize = 10; 
pub static MAP_DATA_PATH: &str = "./map_data/";
pub static REDUCE_INITIAL_DATA_PATH: &str = "./to_reduce/";
pub static RESULT_PATH: &str = "./result/";
