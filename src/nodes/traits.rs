use crate::model::sedaroml::Model;
use crate::commands::NodeCommands;
use crate::commands::NodeResponses;
use std::sync::mpsc::{Receiver, Sender, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub trait Exchangeable {
  fn identifier(&self) -> String;
  fn sedaroml_filename(&self) -> String;
  fn rep(&self) -> &Model;
  fn rep_mut(&mut self) -> &mut Model;
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
  fn rx_from_node_timeout(&self, timeout: Duration) -> Result<NodeResponses, RecvTimeoutError> { self.rx().lock().unwrap().recv_timeout(timeout) }
  fn refresh_rep(&mut self);
}