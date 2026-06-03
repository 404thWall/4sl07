pub mod main_client;
pub mod main_server;

pub fn clean_temporary_files() {
    for path in crate::tasks::FOLDERS_TO_DELETE {
        let temp_data_folder: &std::path::Path = std::path::Path::new(path);
        println!("Deleting {}...", temp_data_folder.display());
        if temp_data_folder.exists() {
            std::fs::remove_dir_all(temp_data_folder).ok();
        }
    }
}
