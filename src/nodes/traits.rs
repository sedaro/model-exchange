use crate::watchers::WatcherType;
use crate::model::sedaroml::Model;
use crate::commands::NodeCommands;
use crate::commands::NodeResponses;
use std::sync::mpsc::{SendError, Receiver, Sender};
use std::sync::{Arc, Mutex};

pub trait Exchangeable {
  fn identifier(&self) -> String;
  fn sedaroml_filename(&self) -> String;
  fn representation(&self) -> &Model;
  fn representation_mut(&mut self) -> &mut Model;
  fn tx(&self) -> &Sender<NodeCommands>;
  fn rx(&self) -> &Arc<Mutex<Receiver<NodeResponses>>>;
  fn tx_to_node(&self, command: NodeCommands) { 
    self.tx().send(command).unwrap_or_else(
      |e| panic!("Failed to communicated with nodes: {:?}", e)
    );
  }
  fn tx_to_node_blocking(&self, command: NodeCommands) -> NodeResponses {
    self.tx().send(command).unwrap();
    self.rx_from_node()
  }
  fn rx_from_node(&self) -> NodeResponses { self.rx().lock().unwrap().recv().unwrap() }
  fn refresh_representation(&mut self);
}