mod map;
mod reduce;
mod saver;
mod testing;
pub use map::{run_map_task, run_map_task_default};
pub use reduce::{run_reduce_task, run_reduce_task_default};
pub use testing::{get_test_word_count_from_result, test_map, test_reduce, test_result};

#[derive(Copy, Clone)]
struct TasksConfig {
    initial_data_path: &'static str,
    map_data_path: &'static str,
    reduce_initial_data_path: &'static str,
    result_path: &'static str,
    folders_to_delete: [&'static str; 1],
    map_tasks_amount: usize,
    reduce_tasks_amount: usize,
}

#[cfg(feature = "prod")]
const CONFIG: TasksConfig = TasksConfig {
    initial_data_path: "/cal/commoncrawl/",
    map_data_path: "/tmp/4sl07_grp3/map_data/",
    reduce_initial_data_path: "/tmp/4sl07_grp3/to_reduce/",
    result_path: "./4sl07/deploy/result/",
    folders_to_delete: ["/tmp/4sl07_grp3/"],
    map_tasks_amount: 30,
    reduce_tasks_amount: 6,
};

#[cfg(not(feature = "prod"))]
const CONFIG: TasksConfig = TasksConfig {
    initial_data_path: "../data/",
    map_data_path: "./map_data/",
    reduce_initial_data_path: "./to_reduce/",
    result_path: "../result/",
    folders_to_delete: ["./map_data/"],
    map_tasks_amount: 2,
    reduce_tasks_amount: 6,
};

pub const R: usize = 10;
pub const INITIAL_DATA_PATH: &str = CONFIG.initial_data_path;
pub const MAP_DATA_PATH: &str = CONFIG.map_data_path;
pub const REDUCE_INITIAL_DATA_PATH: &str = CONFIG.reduce_initial_data_path;
pub const RESULT_PATH: &str = CONFIG.result_path;
pub const FOLDERS_TO_DELETE: [&str; 1] = CONFIG.folders_to_delete;
pub const MAP_TASKS_AMOUNT: usize = CONFIG.map_tasks_amount;
pub const REDUCE_TASKS_AMOUNT: usize = CONFIG.reduce_tasks_amount;
