mod map;
mod reduce;
mod saver;
mod testing;
pub use map::{run_map_task, run_map_task_default};
pub use reduce::{run_reduce_task, run_reduce_task_default};
pub use testing::{test_map, test_reduce};

pub const R: usize = 10;
pub const INITIAL_DATA_PATH: &str = "/cal/commoncrawl/";
pub const MAP_DATA_PATH: &str = "/tmp/4sl07_grp3/map_data/";
pub const REDUCE_INITIAL_DATA_PATH: &str = "/tmp/4sl07_grp3/to_reduce/";
pub const RESULT_PATH: &str = "./result/";
pub const FOLDERS_TO_DELETE: [&str; 1] = ["/tmp/4sl07_grp3/"];
pub const MAP_TASKS_AMOUNT: usize = 8;
pub const REDUCE_TASKS_AMOUNT: usize = 8;
