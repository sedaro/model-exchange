use std::sync::{Arc, Mutex};
use crate::model::sedaroml::Model;
use crate::model::sedaroml::{write_model, read_model};
use crate::nodes::traits::Exchangeable;
use log::{debug, info, warn};
use std::time::{Duration, Instant};
use ureq;
use std::borrow::{Borrow, BorrowMut};
use std::sync::mpsc;
use std::thread;
use crate::commands::{NodeCommands, NodeResponses};
use crate::nodes::sedaro::SedaroCredentials;

#[derive(Clone)]
pub struct Cosimulation {
  identifier: String,
  sedaroml_filename: String,
  rep: Option<Model>,
  tx: mpsc::Sender<NodeCommands>,
  rx: Arc<Mutex<mpsc::Receiver<NodeResponses>>>,
}

#[derive(Debug, Clone)]
pub enum SimulationJobId {
  Id(String),
  LatestForScenario(String),
}

impl Cosimulation {
  pub fn new(identifier: String, host_url: String, id: SimulationJobId, agent_id: String, external_state_id: String, credentials: SedaroCredentials) -> Arc<Mutex<Cosimulation>> {

    let job_iden = match id {
      SimulationJobId::Id(ref id) => id.clone(),
      SimulationJobId::LatestForScenario(ref scenario_id) => scenario_id.clone(),
    };
    let sedaroml_filename = format!("{job_iden}_{agent_id}_{external_state_id}.json");
    let sedaroml_filename_clone = sedaroml_filename.clone();
    let identifier_clone = identifier.to_string();

    let (tx_to_node, rx_in_node) = mpsc::channel::<NodeCommands>();
    let (tx_to_exchange, rx_in_exchange) = mpsc::channel::<NodeResponses>();
    thread::spawn(move || {
      // Setup
      let url = |job_id: String| -> String { format!("{host_url}/simulations/jobs/{job_id}/externals/{agent_id}/{external_state_id}") };
      let auth_header = match credentials {
        SedaroCredentials::ApiKey(api_key) => ("X_API_KEY".to_string(), api_key),
        SedaroCredentials::AuthHandle(auth_handle) => ("X_AUTH_HANDLE".to_string(), auth_handle),
      };
      let mut running_job_id = None;
      let mut prev_consumed_value = serde_json::json!(null);

      loop {
        match rx_in_node.recv_timeout(Duration::from_millis(1000)) {
          Ok(command) => {
            debug!("{}: Received command: {:?}", identifier_clone, command);
            match command {
              NodeCommands::Start => { 
                // When a cosimulator starts up, it doesn't have prior state so there is no need to check for the 
                // existence of the SedaroML file nor conflicts between it and the simulator.
                
                // Local cosim model reconciliation currently behaves as follows:
                // 1. Node attaches to running simulation
                // 2. Node consumes (get) from the simulation once, to populate the consume side of the model `consumed_value`
                // so that it is available to the rest of the exchange.
                // 3. Exchange starts with partially initialized rep which is safe because the other side of the rel (`produced_value`) 
                // will be set, by definition, by a translation into this Node prior to a `put` call.
                //   NOTE: Translations that write to this Node's rep shall be capable of handling a prior `produced_value` of None
                //   LATER: Can add an initializer here which can be used to set the initial value of `produced_value`.
                //   WARN: It is possible that the first consume (get) will not return until the first produce (put). If 
                //     this is the case, additional work will be needed to generically deconflict.

                // Attach to running simulation
                let job_id = is_job_running_blocking(identifier_clone.clone(), host_url.clone(), id.clone(), &auth_header);
                running_job_id = Some(job_id.clone());

                // Consume to initialize rep
                let value = get_from_simulator(&url(job_id), &auth_header);
                let model: Model = serde_json::from_value(serde_json::json!({
                  "index": {},
                  "blocks": {},
                  "consumed_value": value,
                  "produced_value": null, // This will be set by the first producer, before the first `put` call
                })).unwrap();
                write_model(&sedaroml_filename_clone, &model).unwrap_or_else(
                  |e| panic!("{}: Failed to write SedaroML to file: {:?}", identifier_clone, e)
                );
                prev_consumed_value = value;
                tx_to_exchange.send(NodeResponses::Started).unwrap() 
              },
              NodeCommands::ResolveConflict(_) => {},
              NodeCommands::Stop => {
                running_job_id = None;
                tx_to_exchange.send(NodeResponses::Stopped).unwrap();
              },
              NodeCommands::Changed(diff) => {
                let t = Instant::now();
                match diff.root.updated_fields.get("produced_value") {
                  Some(value) => {
                    put_to_simulator(&url(running_job_id.clone().unwrap()), &auth_header, &value.new_value);
                  },
                  None => warn!("{}: `root.produced_value` not in ModelDiff.", identifier_clone),
                }
                tx_to_exchange.send(NodeResponses::Done(t.elapsed())).unwrap();
              },
              NodeCommands::Done => {},
            }
          },
          Err(_) => {},
        }
        if running_job_id.is_some() {
          let job_id = running_job_id.clone().unwrap();
          let value = get_from_simulator(&url(job_id), &auth_header);
          if prev_consumed_value != value {
            debug!("{}: Model in simulation has changed. Updating...", identifier_clone);
            let mut model = read_model(&sedaroml_filename_clone).unwrap_or_else(
              |e| panic!("{}: Failed to read SedaroML from file: {:?}", identifier_clone, e)
            );
            model.root.insert("consumed_value".to_string(), value.clone());
            write_model(&sedaroml_filename_clone, &model).unwrap_or_else(
              |e| panic!("{}: Failed to write SedaroML to file: {:?}", identifier_clone, e)
            );
            prev_consumed_value = value;
          }
        }
      }
    });

    let exchangeable = Cosimulation {
      identifier: identifier.into(),
      sedaroml_filename,
      rep: None,
      tx: tx_to_node,
      rx: Arc::new(Mutex::new(rx_in_exchange)),
    };
    Arc::new(Mutex::new(exchangeable))
  }
}

impl Exchangeable for Cosimulation {
  fn identifier(&self) -> String { self.identifier.clone() }
  fn sedaroml_filename(&self) -> String { self.sedaroml_filename.clone() }
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

fn get_from_simulator(url: &str, auth_header: &(String, String)) -> serde_json::Value {
  match ureq::get(&url.to_string())
    .set("User-Agent", "modex/0.0")
    .set(&auth_header.0, &auth_header.1)
    .call() {
      Ok(response) => response.into_json::<serde_json::Value>().expect("Failed to deserialize response"),
      Err(e) => {
        let response: serde_json::Value = e.into_response().unwrap().into_json().expect("Failed to deserialize response");
        panic!("Failed to update model: {}", response.get("error").unwrap().get("message").unwrap().as_str().unwrap());
      },
  }
}

fn put_to_simulator(url: &str, auth_header: &(String, String), value: &serde_json::Value) {
  match ureq::patch(&url)
    .set("User-Agent", "modex/0.0")
    .set(&auth_header.0, &auth_header.1)
    .send_json(ureq::json!({
      "values": value,
      // "timestamp": // TODO
    })) {
    Ok(response) => response.into_json::<serde_json::Value>().expect("Failed to deserialize response"),
    Err(e) => {
      let response: serde_json::Value = e.into_response().unwrap().into_json().expect("Failed to deserialize response");
      panic!("Failed to put to cosimulator: {}", response.get("error").unwrap().get("message").unwrap().as_str().unwrap());
    },
  };
}

/// Returns the ID of the running job, blocks otherwise
fn is_job_running_blocking(identifier: String, host_url: String, id: SimulationJobId, auth_header: &(String, String)) -> String {
  let url = match id {
    SimulationJobId::Id(id) => format!("{host_url}/simulations/jobs/{id}"),
    SimulationJobId::LatestForScenario(scenario_id) => format!("{host_url}/simulations/branches/{scenario_id}/control?latest"),
  };
  let job_id: Option<String>;
  loop {
    let response = match ureq::get(&url.to_string())
      .set("User-Agent", "modex/0.0")
      .set(&auth_header.0, &auth_header.1)
      .call() {
        Ok(response) => response.into_json::<serde_json::Value>().expect("Failed to deserialize response"),
        Err(e) => {
          let response: serde_json::Value = e.into_response().unwrap().into_json().expect("Failed to deserialize response");
          panic!("Failed to check if job is running: {}", response.get("error").unwrap().get("message").unwrap().as_str().unwrap());
        },
    };
    let job = response.as_array().unwrap().get(0).unwrap();
    let status = job.get("status").unwrap().as_str().unwrap();
    if status == "RUNNING" {
      job_id = Some(job.get("id").unwrap().as_str().unwrap().to_string());
      break;
    };
    thread::sleep(Duration::from_secs(1));
    info!("{identifier}: Waiting for simulation job to enter `RUNNING` status.  Current status: `{}`", status);
  }
  job_id.unwrap().to_string()
}

// #[cfg(test)]
// mod test {
//   use super::*;

//   #[test]
//   fn test_cosim_response_to_model() {
//     let v = serde_json::json!([60000.0, [{"ndarray": [12, 13, 14]}, "yes"]]);
//     let truth = serde_json::json!({
//       "index": {
//         "StateVariableGroup": ["_", "1"],
//         "StateVariable": ["0", "1.0", "1.1"]
//       },
//       "blocks": {
//         "_": {
//           "type": "StateVariableGroup",
//           "variables": ["0", "1"],
//         },
//         "0": {
//           "type": "StateVariable",
//           "remote_type": "float",
//           "value": 60000,
//         },
//         "1": {
//           "type": "StateVariableGroup",
//           "value": ["ndarray", [12, 13, 14]],
//         },
//         "1.0": {
//           "type": "StateVariable",
//           "remote_type": "ndarray",
//           "value": [12, 13, 14],
//         },
//         "1.1": {
//           "type": "StateVariable",
//           "remote_type": "string",
//           "value": "yes",
//         },
//       },
//     });
//     let model = cosim_response_to_model(v);
//     assert_eq!(model.to_pretty_string(), truth);
//   }
// }