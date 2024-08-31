use std::path::Path;
use std::sync::{Arc, Mutex};
use crate::model::sedaroml::{Block, Model};
use crate::model::sedaroml::{write_model, read_model};
use crate::nodes::traits::Exchangeable;
use log::{debug, info};
use std::time::Duration;
use ureq;
use crate::metadata::{read_metadata, write_metadata};
use std::borrow::{Borrow, BorrowMut};
use std::sync::mpsc;
use std::thread;
use crate::commands::{NodeCommands, NodeResponses};

#[derive(Clone)]
pub enum SedaroCredentials {
  ApiKey(String),
  AuthHandle(String),
}

#[derive(Clone)]
pub struct Sedaro {
  identifier: String,
  sedaroml_filename: String,
  representation: Option<Model>,
  tx: mpsc::Sender<NodeCommands>,
  rx: Arc<Mutex<mpsc::Receiver<NodeResponses>>>,
}

impl Sedaro {
  pub fn new(identifier: String, host_url: String, branch_id: String, credentials: SedaroCredentials) -> Arc<Mutex<Sedaro>> {

    let sedaroml_filename = format!("{}.json", branch_id);
    let sedaroml_filename_clone = sedaroml_filename.clone();
    let identifier_clone = identifier.to_string();

    let (tx_to_node, rx_in_node) = mpsc::channel::<NodeCommands>();
    let (tx_to_exchange, rx_in_exchange) = mpsc::channel::<NodeResponses>();
    thread::spawn(move || {
      // Setup
      let url = format!("{}/models/branches/{}", host_url, branch_id);
      let auth_header = match credentials {
        SedaroCredentials::ApiKey(api_key) => ("X_API_KEY".to_string(), api_key),
        SedaroCredentials::AuthHandle(auth_handle) => ("X_AUTH_HANDLE".to_string(), auth_handle),
      };
      let metadata_filename = format!("{}.metadata.json", sedaroml_filename_clone.strip_suffix(".json").unwrap());
      let mut running = false;

      loop {
        match rx_in_node.recv_timeout(Duration::from_millis(100)) {
          Ok(command) => {
            match command {
              NodeCommands::Start => { 
                if !Path::exists(Path::new(&sedaroml_filename_clone)) || !Path::exists(Path::new(&metadata_filename)) {
                  debug!("{}: SedaroML file doesn't exist.  Fetching from: {}", identifier_clone, &url);
                  let (model, date_modified) = get_sedaro_model(&url, &auth_header);
                  write_model(&sedaroml_filename_clone, &model).unwrap_or_else(
                    |e| panic!("{}: Failed to write SedaroML to file: {:?}", identifier_clone, e)
                  );
                  write_metadata(&metadata_filename, &date_modified).unwrap_or_else(
                    |e| panic!("{}: Failed to write metadata to file: {:?}", identifier_clone, e)
                  );
                }
                running = true;
                tx_to_exchange.send(NodeResponses::Started).unwrap() 
              },
              NodeCommands::Stop => {
                running = false;
                tx_to_exchange.send(NodeResponses::Stopped).unwrap();
              },
              NodeCommands::Changed => {
                let url = format!("{}/template", &url);
                let model = read_model(&sedaroml_filename_clone).unwrap_or_else(
                  |e| panic!("{}: Failed to read SedaroML from file: {:?}", identifier_clone, e)
                );
                let date_modified = put_sedaro_model(&url, &auth_header, &model);
                write_metadata(&metadata_filename, &date_modified).unwrap_or_else(
                  |e| panic!("{}: Failed to write metadata to file: {:?}", identifier_clone, e)
                );
              },
              NodeCommands::Done => { debug!("{}: Done", identifier_clone) },
            }
          },
          Err(_) => {},
        }
        if running {
          debug!("{}: Checking for changes at: {}", identifier_clone, &url);
          let (model, date_modified) = get_sedaro_model(&url, &auth_header);

          let metadata = read_metadata(&metadata_filename).unwrap_or_else(
            |e| panic!("{}: Failed to read metadata from file: {:?}", identifier_clone, e)
          );
          if metadata.date_modified != date_modified {
            info!("{}: Model has changed. Updating metadata...", identifier_clone);
            write_metadata(&metadata_filename, &date_modified).unwrap_or_else(
              |e| panic!("{}: Failed to write metadata to file: {:?}", identifier_clone, e)
            );
            write_model(&sedaroml_filename_clone, &model).unwrap_or_else(
              |e| panic!("{}: Failed to write SedaroML to file: {:?}", identifier_clone, e)
            );
          }
        }
      }
    });

    let exchangeable: Sedaro = Sedaro {
      identifier: identifier.into(),
      sedaroml_filename,
      representation: None,
      tx: tx_to_node,
      rx: Arc::new(Mutex::new(rx_in_exchange)),
    };
    Arc::new(Mutex::new(exchangeable))
  }
}

impl Exchangeable for Sedaro {
  fn identifier(&self) -> String { self.identifier.clone() }
  fn sedaroml_filename(&self) -> String { self.sedaroml_filename.clone() }
  fn representation(&self) -> &Model { 
    match self.representation.borrow() {
      Some(representation) => representation,
      None => panic!("{}: Representation not initialized", self.identifier()),
    }
  }
  fn representation_mut(&mut self) -> &mut Model {
    let iden = self.identifier();
    match self.representation.borrow_mut() {
      Some(representation) => representation,
      None => panic!("{}: Representation not initialized", iden),
    }
  }
  fn tx(&self) -> &mpsc::Sender<NodeCommands> { &self.tx }
  fn rx(&self) -> &Arc<Mutex<mpsc::Receiver<NodeResponses>>> { &self.rx }
  fn refresh_representation(&mut self) {
    self.representation = Some(read_model(&self.sedaroml_filename()).unwrap_or_else(
      |e| panic!("{}: Failed to read SedaroML: {:?}", self.identifier(), e)
    ));
  }
}

fn get_sedaro_model(url: &str, auth_header: &(String, String)) -> (Model, String) {
  let response = match ureq::get(&url.to_string())
    .set("User-Agent", "modex/0.0")
    .set(&auth_header.0, &auth_header.1)
    .call() {
      Ok(response) => response.into_json::<serde_json::Value>().expect("Failed to deserialize response"),
      Err(e) => {
        let response: serde_json::Value = e.into_response().unwrap().into_json().expect("Failed to deserialize response");
        panic!("Failed to update model: {}", response.get("error").unwrap().get("message").unwrap().as_str().unwrap());
      },
  };
  let model_data = response.get("data").unwrap().clone();
  let model: Model = serde_json::from_value(model_data).unwrap();
  (model, response.get("dateModified").unwrap().as_str().unwrap().to_string())
}

fn put_sedaro_model(url: &str, auth_header: &(String, String), model: &Model) -> String {
  let response = match ureq::patch(&url)
    .set("User-Agent", "modex/0.0")
    .set(&auth_header.0, &auth_header.1)
    .send_json(ureq::json!({
      "root": model.root.clone(),
      "blocks": model.blocks.values().cloned().collect::<Vec<Block>>(),
      // TODO: Handle deletes (ideally we would have a model service route for just accepting a full model and updating it in place)
    })) {
    Ok(response) => response.into_json::<serde_json::Value>().expect("Failed to deserialize response"),
    Err(e) => {
      let response: serde_json::Value = e.into_response().unwrap().into_json().expect("Failed to deserialize response");
      panic!("Failed to update model: {}", response.get("error").unwrap().get("message").unwrap().as_str().unwrap());
    },
  };
  response.get("branch").unwrap().get("dateModified").unwrap().to_string()
}