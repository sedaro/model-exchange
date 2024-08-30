use std::sync::{Arc, Mutex};
use crate::model::sedaroml::{Block, Model};
use crate::watchers::WatcherType;
use crate::watchers::custom::{CustomWatcher, CustomWatcherFn, CustomWatcherEvent};
use crate::model::sedaroml::write_model;
use crate::nodes::traits::Exchangeable;
use log::{debug, info};
use std::time::Duration;
use ureq;
use crate::metadata::{read_metadata, write_metadata};
use std::borrow::{Borrow, BorrowMut};
use crate::watchers::traits::Watcher;

#[derive(Clone)]
pub enum SedaroCredentials {
  ApiKey(String),
  AuthHandle(String),
}

#[derive(Clone)]
pub struct Sedaro {
  identifier: String,
  sedaroml_filename: String,
  metadata_filename: String,
  branch_id: String,
  host_url: String,
  model_url: String,
  watcher: WatcherType,
  representation: Model,
  auth_header: (String, String),
}

impl Sedaro {
  pub fn new(identifier: &str, host_url: &str, branch_id: &str, credentials: SedaroCredentials) -> Arc<Mutex<Sedaro>> {
    let url = format!("{}/models/branches/{}", host_url, branch_id);
    let iden = identifier.to_string();
    let header = match credentials {
      SedaroCredentials::ApiKey(api_key) => ("X_API_KEY".to_string(), api_key),
      SedaroCredentials::AuthHandle(auth_handle) => ("X_AUTH_HANDLE".to_string(), auth_handle),
    };
    let url_clone = url.clone();
    let auth_header = header.clone();
    let metadata_filename = format!("{}.metadata.json", identifier);
    let metadata_filename_clone = metadata_filename.clone();
    
    let watch_fn: Arc<CustomWatcherFn> = Arc::new(move || {
      debug!("{}: Checking for changes at: {}", iden, &url_clone);
      let (model, date_modified) = get_sedaro_model(&url_clone, &auth_header);

      let metadata = read_metadata(&metadata_filename_clone).unwrap_or_else(
        |e| panic!("{}: Failed to read metadata from file: {:?}", iden, e)
      );
      if metadata.date_modified != date_modified {
        info!("{}: Model has changed. Updating metadata...", iden);
        write_metadata(&metadata_filename_clone, &date_modified);
        return Ok(CustomWatcherEvent { model: Some(model), changed: true });
      }

      Ok(CustomWatcherEvent { model: None, changed: false })
    });

    let mut exchangeable: Sedaro = Sedaro {
      identifier: identifier.into(),
      sedaroml_filename: format!("{}.json", identifier),
      metadata_filename,
      branch_id: branch_id.into(),
      host_url: host_url.into(),
      model_url: url,
      watcher: WatcherType::CustomWatcher(CustomWatcher::new(identifier.into(), watch_fn, Duration::from_millis(100))),
      representation: Model::new(),
      auth_header: header,
    };
    exchangeable.read();
    Arc::new(Mutex::new(exchangeable))
  }
}

impl Exchangeable for Sedaro {
  fn identifier(&self) -> String { self.identifier.clone() }
  fn watcher(&self) -> &WatcherType { self.watcher.borrow() }
  fn watcher_mut(&mut self) -> &mut WatcherType { self.watcher.borrow_mut() }
  fn representation(&self) -> &Model { self.representation.borrow() }
  fn representation_mut(&mut self) -> &mut Model { self.representation.borrow_mut() }
  fn read(&mut self) {
    let (model, date_modified) = get_sedaro_model(&self.model_url, &self.auth_header);
    write_model(&self.sedaroml_filename, &model).unwrap_or_else(
      |e| panic!("{}: Failed to write SedaroML to file: {:?}", self.identifier, e)
    );
    write_metadata(&self.metadata_filename, &date_modified);
    self.representation = model;
  }
  fn write(&self) {
    let url = format!("{}/template", &self.model_url);
    let date_modified = put_sedaro_model(&url, &self.auth_header, &self.representation);
    write_model(&self.sedaroml_filename, &self.representation).unwrap_or_else(
      |e| panic!("{}: Failed to write SedaroML to file: {:?}", self.identifier, e)
    );
    write_metadata(&self.metadata_filename, &date_modified);
    match self.watcher() { // Manually trigger that watcher to checkin with the exchange, concluding translations relevant to this destination
      WatcherType::CustomWatcher(ref watcher) => {
        watcher.trigger();
      },
      _ => panic!("Invalid watcher type"),
    }
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