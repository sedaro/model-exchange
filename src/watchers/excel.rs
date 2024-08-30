use crate::change_queue::ChangeQueue;
use crate::watchers::traits::Watcher;

#[derive(Debug, Clone)]
pub struct ExcelWatcher {
  pub identifier: String,
  pub excel_filename: String,
  pub sedaroml_filename: String,
  change_queue_binding: Option<ChangeQueue>,
}
impl ExcelWatcher {
  pub fn new(identifier: String, excel_filename: String, sedaroml_filename: String) -> ExcelWatcher {
    ExcelWatcher { identifier, excel_filename, sedaroml_filename, change_queue_binding: None, }
  }
}
impl Watcher for ExcelWatcher {
  fn identifier(&self) -> String { self.identifier.clone() }
  fn change_queue(&self) -> Option<ChangeQueue> { self.change_queue_binding.clone() }
  fn bind(&mut self, change_queue: ChangeQueue) { self.change_queue_binding = Some(change_queue) }
}