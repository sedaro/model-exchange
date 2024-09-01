/*
TODO:

00. For cosimulation, seems like there should maybe be two types of models?? (one for consume and one for produce)
0. Put things in the exchange that have dependencies but that aren't connected to other things in the exchange.  Do we allow for this?
1. Docs (initially release as collision detection and resolution coming soon?)
2. Try in cosimulation (via adaptation of the custom watcher for cosim I guess?)
3. Implement exchange lock (locks the entire exchange while a translation is in progress and is awaitable from things like tests and conflict resolution)
4. Handle conflict resolutions:
  - Watcher triggers while translation is in progress
  - Exchange wants to write a file that has changed since it was read
4. Add Cameo and AFSIM Nodes
5. Satisfy remainder of Mosaic Warfare requirements: Web gui? REST API?
6. Need to handle things like excel changing while the exchange isn't running, leaving a conflict between excel.xlsx and excel.json just that on start up, translations to excel.xlsx can't operate over the current state of excel.xlsx
- How to handle changes that occurred while exchange wasn't running?
*/

// Docs content
// Ideally the exchange would write/read to the local sedaroml file and then a parallel thread would reconcile the "foreign" model
// This approach is key for cosimulation where models are changing frequently.  Need to be mindful of conflicts though.
// Idea it to build stable, maintainable, revisitable software that can perform mission critical model interoperability 

// Can run virtually in a different dir maybe or perhaps with recovery we don't care 
// Potentially need to run the operation array in reverse when performing a reverse translation.  Need to think about this more.  Order of operations thing
// This doesn't handle recursive deps yet in the translations.  ie. t_a requires that t_b is run first.
// ^ This does not mean recursive model deps but within a model translation, requiring the result of a prior translation.  If this is needed
// should combine into a single translation.

// Things to add tests for SOON
// Check that model identifiers are unique
// Double triggers (trigger comes in while translation is in progress) for same model and differen model in change (causing a conflict in this case)
// user error in a translation function

// Each node should implement (optionally?) a lock that prevents races and/or collissions
// This lock file should also potentially enable detecting when things are deleted/added instead of just changed but 
// need to think through this usecase more
// Potentially lock files or something more intelligent, like a .git file for locking? provide recoverability.  This should
// be optional though so as not to slow down things like cosim where the recoverability doesn't make sense because the model 
// is dynamic

// How to identify things?  cdktf type names?

// How to integrate with static model and dynamic model (i.e., via cosim)?

// Good error handling to start too - enforce that all errors are handled

// How to handle multi-file translations?  What if the files don't all changes at the same time?

// Enable a sense of virtualization such that the actual files aren't changed but virtual copies somewhere else in the filesystem?
// Would help with unit testbed
// Make just keep in memory instead of writing to file.  Model could implement via abstract write/read interface.  Maybe just have a VirtualModel type?

// spacecraft.sml -> [test.xlsx.sml, sedaro platform]
// test.xlsx -> [test.xlsx.sml]
// test.xlsx.sml -> [spacecraft.sml, test.xlsx]

// sedaroplatform <-> spacecraft.sml <-> test.xlsx.sml <-> test.xlsx
//                         ^-> cameo.sml <-> cameo
//                                 ^-> sparxea.sml <-> sparxea


use std::collections::HashMap;
use modex::nodes::cosimulation::{Cosimulation, SimulationJobId};
use modex::nodes::sedaroml::SedaroML;
use serde_json::Value;
use modex::logging::init_logger;
use modex::model::sedaroml::Model;
use modex::nodes::sedaro::{Sedaro, SedaroCredentials};
use modex::nodes::excel::Excel;
use modex::exchange::Exchange;
use modex::translations::{Operation, Translation, TranslationResult};
use modex::utils::read_json;


#[tokio::main]
async fn main() {
  init_logger().expect("Failed to initialize logger.");
  
  let secrets = read_json("secrets.json").expect("Failed to read secrets.json");
  let api_key = secrets.get("ALPHA").unwrap().as_str().unwrap();
  
  let excel = Excel::new("test.xlsx".into(), "test.xlsx".into());
  let sedaro = Sedaro::new(
    "Wildfire".into(),
    "https://api.astage.sedaro.com".into(),
    "PNdldNPBmJ2qRcYlBFCZnJ".into(),
    SedaroCredentials::ApiKey(api_key.to_string()),
  );
  let api_key = secrets.get("PROD").unwrap().as_str().unwrap();
  let cosim = Cosimulation::new(
    "Wildfire Cosim".into(),
    "https://api.sedaro.com".into(),
    SimulationJobId::LatestForScenario("PNhrrFtnB5XYv2qJ8RcZzN".into()),
    "NSghFfVT8ieam0ydeZGX-".into(),
    "NZ2SHUkS95z1GtmMZ0CTk".into(),
    SedaroCredentials::ApiKey(api_key.to_string()),
  );
  let test = SedaroML::new("test.json".into(), "test.json".into());

  let excel_to_sedaroml = Operation {
    name: Some("-".into()),
    forward: |from: &Model, to: &mut Model| {
      // get_first_block_where!(name='spacecraft_dry_mass').value as Mass.g -> Spacecraft.dryMass
      let filter = HashMap::from([("name".to_string(), Value::String("battery_esr".into()))]);
      let battery_esr_name = from.get_first_block_where(&filter).expect("Block matching filter expression not found.");
      let esr = battery_esr_name.get("value").unwrap().as_f64().unwrap();
      to.block_by_id_mut("NT0USZZSc9cZAmWJbClN-").expect("Block not found").insert("esr".to_string(), esr.into());
      Ok(TranslationResult::Changed)
    },
    reverse: |from: &Model, to: &mut Model| {
      // Spacecraft.root.dryMass as Mass.kg -> get_first_block_where!(name='spacecraft_dry_mass').value
      let block = from.block_by_id("NT0USZZSc9cZAmWJbClN-").expect("Block not found");
      let esr = block.get("esr").unwrap().as_f64().unwrap();
      
      let filter = HashMap::from([("name".to_string(), Value::String("battery_esr".into()))]);
      let battery_esr_name = to.get_first_block_where_mut(&filter).expect("Block matching filter expression not found.");
      battery_esr_name.insert("value".to_string(), esr.into());
      Ok(TranslationResult::Changed)
    },
  };

  let excel_to_cosim = Operation {
    name: Some("cosim".into()),
    forward: |from: &Model, to: &mut Model| {
      let filter = HashMap::from([("name".to_string(), Value::String("attitude_x".into()))]);
      let block = from.get_first_block_where(&filter).expect("Block matching filter expression not found.");
      let x = block.get("value").unwrap().as_f64().unwrap();
      let filter = HashMap::from([("name".to_string(), Value::String("attitude_y".into()))]);
      let block = from.get_first_block_where(&filter).expect("Block matching filter expression not found.");
      let y = block.get("value").unwrap().as_f64().unwrap();
      let filter = HashMap::from([("name".to_string(), Value::String("attitude_z".into()))]);
      let block = from.get_first_block_where(&filter).expect("Block matching filter expression not found.");
      let z = block.get("value").unwrap().as_f64().unwrap();
      let filter = HashMap::from([("name".to_string(), Value::String("attitude_w".into()))]);
      let block = from.get_first_block_where(&filter).expect("Block matching filter expression not found.");
      let w = block.get("value").unwrap().as_f64().unwrap();
      to.root.insert("produced_value".to_string(), serde_json::json!([{"ndarray": vec![x, y, z, w]}]));
      Ok(TranslationResult::Changed)
    },
    reverse: |from: &Model, to: &mut Model| {
      // let vector = from.root.get("consumed_value").unwrap().get(0).unwrap().get("ndarray").unwrap().as_array().unwrap();
      // let x = vector[0].as_f64().unwrap();
      // let y = vector[1].as_f64().unwrap();
      // let z = vector[2].as_f64().unwrap();

      // let filter = HashMap::from([("name".to_string(), Value::String("position_eci_x".into()))]);
      // let block = to.get_first_block_where_mut(&filter).expect("Block matching filter expression not found.");
      // block.insert("value".to_string(), x.into());
      // let filter = HashMap::from([("name".to_string(), Value::String("position_eci_y".into()))]);
      // let block = to.get_first_block_where_mut(&filter).expect("Block matching filter expression not found.");
      // block.insert("value".to_string(), y.into());
      // let filter = HashMap::from([("name".to_string(), Value::String("position_eci_z".into()))]);
      // let block = to.get_first_block_where_mut(&filter).expect("Block matching filter expression not found.");
      // block.insert("value".to_string(), z.into());
      Ok(TranslationResult::Unchanged)
    },
  };

  let other = Operation {
    name: Some("other".into()),
    forward: |_, _| {
      Ok(TranslationResult::Unchanged)
    },
    reverse: |_, _| {
      Ok(TranslationResult::Changed)
    },
  };

  let t = Translation {
    from: excel.clone(),
    to: sedaro.clone(),
    operations: vec![excel_to_sedaroml],
  };
  let tt = Translation {
    from: excel.clone(),
    to: test.clone(),
    operations: vec![other],
  };
  // let exchange = Exchange::new(vec![t, tt]);

  let translation_cosim = Translation {
    from: excel.clone(),
    to: cosim.clone(),
    operations: vec![excel_to_cosim],
  };

  let exchange = Exchange::new(vec![translation_cosim]);
  exchange.wait();
}
