use std::sync::{Arc, Mutex};
use std::time::Duration;
use crate::model::sedaroml::Model;
use super::traits::Exchangeable;
use crate::model::sedaroml::read_model;
use std::borrow::{Borrow, BorrowMut};
use std::sync::mpsc;
use crate::commands::{NodeCommands, NodeResponses};
use std::thread;
use log::debug;

#[derive(Clone)]
pub struct SedaroML {
  identifier: String,
  filename: String,
  rep: Option<Model>,
  tx: mpsc::Sender<NodeCommands>,
  rx: Arc<Mutex<mpsc::Receiver<NodeResponses>>>,
}

impl SedaroML {
  pub fn new(identifier: String, filename: String) -> Arc<Mutex<SedaroML>> {
    
    let identifier_clone = identifier.to_string().clone();
    let (tx_to_node, rx_in_node) = mpsc::channel::<NodeCommands>();
    let (tx_to_exchange, rx_in_exchange) = mpsc::channel::<NodeResponses>();
    thread::spawn(move || {
      loop {
        match rx_in_node.recv_timeout(Duration::from_millis(100)) {
          Ok(command) => {
            debug!("{}: Received command: {:?}", identifier_clone, command);
            match command {
              NodeCommands::Start => { tx_to_exchange.send(NodeResponses::Started).unwrap() },
              NodeCommands::Stop => { tx_to_exchange.send(NodeResponses::Stopped).unwrap() },
              NodeCommands::Changed => { tx_to_exchange.send(NodeResponses::Done(Duration::from_secs(0))).unwrap() },
              NodeCommands::Done => {},
            }
          },
          Err(_) => {},
        }
      }
    });

    let exchangeable: SedaroML = SedaroML {
      identifier: identifier.into(),
      filename: filename.into(),
      rep: None,
      tx: tx_to_node,
      rx: Arc::new(Mutex::new(rx_in_exchange)),
    };
    Arc::new(Mutex::new(exchangeable))
  }
}

impl Exchangeable for SedaroML {
  fn identifier(&self) -> String { self.identifier.clone() }
  fn sedaroml_filename(&self) -> String { self.filename.clone() }
  fn rep(&self) -> &Model { 
    match self.rep.borrow() {
      Some(rep) => rep,
      None => panic!("{}: Representation not initialized", self.identifier()),
    }
  }
  fn rep_mut(&mut self) -> &mut Model {
    let iden = self.identifier();
    match self.rep.borrow_mut() {
      Some(rep) => rep,
      None => panic!("{}: Representation not initialized", iden),
    }
  }
  fn tx(&self) -> &mpsc::Sender<NodeCommands> { &self.tx }
  fn rx(&self) -> &Arc<Mutex<mpsc::Receiver<NodeResponses>>> { &self.rx }
  fn refresh_rep(&mut self) {
    self.rep = Some(read_model(&self.sedaroml_filename()).unwrap_or_else(
      |e| panic!("{}: Failed to read SedaroML: {:?}", self.identifier(), e)
    ));
  }
}