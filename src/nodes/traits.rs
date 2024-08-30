use crate::watchers::WatcherType;
use crate::model::sedaroml::Model;
use log::debug;

pub trait Exchangeable {
  fn identifier(&self) -> String;
  fn watcher(&self) -> &WatcherType;
  fn watcher_mut(&mut self) -> &mut WatcherType;
  fn representation(&self) -> &Model;
  fn representation_mut(&mut self) -> &mut Model;
  fn read(&mut self);
  fn write(&self);
  fn done(&self) { debug!("Done: {}", self.identifier()) }
}