use crate::change_queue::ChangeQueue;
use crate::watchers::traits::Watcher;
use crate::model::sedaroml::Model;
use std::sync::Arc;
use std::time::Duration;
use std::io::Error;

pub struct CustomWatcherEvent {
  pub model: Option<Model>,
  pub changed: bool,
}
pub type CustomWatcherResult = Result<CustomWatcherEvent, Error>;
pub type CustomWatcherFn = dyn Fn() -> CustomWatcherResult + Send + Sync;


#[derive(Clone)]
pub struct CustomWatcher {
  pub identifier: String,
  pub change_queue_binding: Option<ChangeQueue>,
  pub watch_fn: Arc<CustomWatcherFn>,
  pub interval: Duration,
}
impl CustomWatcher {
  pub fn new(identifier: String, watch_fn: Arc<CustomWatcherFn>, interval: Duration) -> CustomWatcher {
    CustomWatcher { identifier, change_queue_binding: None, watch_fn, interval, }
  }
}
impl Watcher for CustomWatcher {
  fn identifier(&self) -> String { self.identifier.clone() }
  fn change_queue(&self) -> Option<ChangeQueue> { self.change_queue_binding.clone() }
  fn bind(&mut self, change_queue: ChangeQueue) { self.change_queue_binding = Some(change_queue) }
}