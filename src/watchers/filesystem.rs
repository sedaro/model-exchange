use crate::change_queue::ChangeQueue;
use crate::watchers::traits::Watcher;
use std::fs::File;

#[derive(Debug, Clone)]
pub struct FileWatcher {
  pub identifier: String,
  pub filename: String,
  change_queue_binding: Option<ChangeQueue>,
}

impl FileWatcher {
  pub fn new(identifier: String, filename: String) -> FileWatcher {
    FileWatcher { identifier, filename, change_queue_binding: None, }
  }
  pub fn new_with_create(identifier: String, filename: String) -> FileWatcher {
    if File::open(&filename).is_err() {
      File::create(&filename).unwrap_or_else(|e| panic!("Failed to create file: {}: {}", filename, e));
    }
    FileWatcher { identifier, filename, change_queue_binding: None, }
  }
}

impl Watcher for FileWatcher {
  fn identifier(&self) -> String { self.identifier.clone() }
  fn change_queue(&self) -> Option<ChangeQueue> { self.change_queue_binding.clone() }
  fn bind(&mut self, change_queue: ChangeQueue) { self.change_queue_binding = Some(change_queue) }
}