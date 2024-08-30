pub mod filesystem;
pub mod excel;
pub mod custom;
pub mod traits;

#[derive(Clone)]
pub enum WatcherType {
  FileWatcher(filesystem::FileWatcher),
  ExcelWatcher(excel::ExcelWatcher),
  CustomWatcher(custom::CustomWatcher),
}