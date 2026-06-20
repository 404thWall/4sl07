mod map;
mod reduce;
mod saver;
mod testing;
pub mod versions;
pub use map::{run_map_task, run_map_task_version};
pub use reduce::{run_reduce_task, run_reduce_task_version};
pub use testing::{run_all, test_all, test_result};

use crate::tasks::MapReduceVersion::DefaultWithLanguageSplit;

pub use versions::MapReduceVersion;

#[derive(Copy, Clone)]
struct TasksConfig {
    wet_paths_url: &'static str,
    initial_data_path: &'static str,
    map_data_path: &'static str,
    reduce_initial_data_path: &'static str,
    result_path: &'static str,
    tmp_dir: &'static str,
    timing_analysis_file_path: &'static str,
    folders_to_delete: &'static [&'static str],
    map_tasks_amount: usize,
    reduce_tasks_amount: usize,
    default_version: MapReduceVersion,
}

#[cfg(feature = "prod")]
const CONFIG: TasksConfig = TasksConfig {
    wet_paths_url: "https://data.commoncrawl.org/crawl-data/CC-MAIN-2023-14/wet.paths.gz",
    initial_data_path: "/cal/commoncrawl/",
    map_data_path: "/tmp/4sl07g3/map_data/",
    reduce_initial_data_path: "/tmp/4sl07g3/to_reduce/",
    result_path: "/tmp/4sl07g3/result/",
    tmp_dir: "/tmp/4sl07g3/tmp/",
    timing_analysis_file_path: "/tmp/4sl07g3/timing_analysis.json",
    folders_to_delete: &[
        "/tmp/4sl07g3/result",
        "/tmp/4sl07g3/map_data/",
        "/tmp/4sl07g3/to_reduce/",
        "/tmp/4sl07g3/tmp/",
    ],
    map_tasks_amount: 1000,
    reduce_tasks_amount: 250,
    default_version: DefaultWithLanguageSplit,
};

#[cfg(not(feature = "prod"))]
const CONFIG: TasksConfig = TasksConfig {
    wet_paths_url: "https://data.commoncrawl.org/crawl-data/CC-MAIN-2023-14/wet.paths.gz",
    initial_data_path: "../data/",
    map_data_path: "./map_data/",
    reduce_initial_data_path: "./to_reduce/",
    result_path: "../result/",
    tmp_dir: "./tmp/",
    timing_analysis_file_path: "./timing_analysis.json",
    folders_to_delete: &["./map_data/", "./tmp/"],
    map_tasks_amount: 4,
    reduce_tasks_amount: 6,
    default_version: DefaultWithLanguageSplit,
};

pub const WET_PATHS_URL: &str = CONFIG.wet_paths_url;
pub const INITIAL_DATA_PATH: &str = CONFIG.initial_data_path;
pub const MAP_DATA_PATH: &str = CONFIG.map_data_path;
pub const REDUCE_INITIAL_DATA_PATH: &str = CONFIG.reduce_initial_data_path;
pub const RESULT_PATH: &str = CONFIG.result_path;
pub const TMP_DIR: &str = CONFIG.tmp_dir;
pub const TIMING_ANALYSIS_FILE_PATH: &str = CONFIG.timing_analysis_file_path;
pub const FOLDERS_TO_DELETE: &[&str] = CONFIG.folders_to_delete;
pub const MAP_TASKS_AMOUNT: usize = CONFIG.map_tasks_amount;
pub const REDUCE_TASKS_AMOUNT: usize = CONFIG.reduce_tasks_amount;
pub const DEFAULT_VERSION: MapReduceVersion = CONFIG.default_version;
