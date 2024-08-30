use std::sync::{Arc, Mutex};
use crate::model::sedaroml::Model;
use crate::watchers::WatcherType;
use crate::watchers::filesystem::FileWatcher;
use super::traits::Exchangeable;
use crate::model::sedaroml::{read_model, write_model};
use std::borrow::{Borrow, BorrowMut};

#[derive(Clone)]
pub struct SedaroML {
  identifier: String,
  filename: String,
  watcher: WatcherType,
  representation: Model,
}

impl SedaroML {
  pub fn new(identifier: &str, filename: &str) -> Arc<Mutex<SedaroML>> {
    let mut exchangeable: SedaroML = SedaroML {
      identifier: identifier.into(),
      filename: filename.into(),
      watcher: WatcherType::FileWatcher(FileWatcher::new_with_create(identifier.into(), filename.into())),
      representation: Model::new(),
    };
    exchangeable.read();
    Arc::new(Mutex::new(exchangeable))
  }
}

impl Exchangeable for SedaroML {
  fn identifier(&self) -> String { self.identifier.clone() }
  fn watcher(&self) -> &WatcherType { self.watcher.borrow() }
  fn watcher_mut(&mut self) -> &mut WatcherType { self.watcher.borrow_mut() }
  fn representation(&self) -> &Model { self.representation.borrow() }
  fn representation_mut(&mut self) -> &mut Model { self.representation.borrow_mut() }
  fn read(&mut self) {
    self.representation = read_model(&self.filename).unwrap_or_else(
      |e| panic!("{}: Failed to read SedaroML from file: {:?}", self.identifier, e)
    );
  }
  fn write(&self) {
    write_model(&self.filename, &self.representation).unwrap_or_else(
      |_| panic!("{}: Failed to write SedaroML to file.", self.identifier)
    );
  }
}