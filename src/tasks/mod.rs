mod map;
mod reduce;
mod saver;
mod testing;
pub use map::{run_map_task, run_map_task_default};
pub use reduce::{run_reduce_task, run_reduce_task_default};
pub use testing::{test_map, test_reduce};

pub const R: usize = 10;
pub const MAP_DATA_PATH: &str = "./map_data/";
pub const REDUCE_INITIAL_DATA_PATH: &str = "./to_reduce/";
pub const RESULT_PATH: &str = "./result/";
pub const MAP_TASKS_AMOUNT: usize = 2;
pub const REDUCE_TASKS_AMOUNT: usize = 8;
