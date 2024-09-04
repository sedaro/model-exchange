/*
TODO:
Next step is to put together a compelling demo and show it off!
000. Update Sedaro watcher to use ModelDiff instead of metadata for change detection? Slower?
0. Put things in the exchange that have dependencies but that aren't connected to other things in the exchange.  Do we allow for this?
  - i.e. two unconnected sub-graphs
1. Docs (initially release as collision detection and resolution coming soon?)
3. Implement exchange lock (locks the entire exchange while a translation is in progress and is awaitable from things like tests and conflict resolution)
4. Handle conflict resolutions:
  - Watcher triggers while translation is in progress
  - Exchange wants to write a file that has changed since it was read
4. Add Cameo and AFSIM Nodes
5. Satisfy remainder of Mosaic Warfare requirements: Web gui? REST API?
7. Get rid of metadata file and use ModelDiff for change detection
8. Very likely communicating by writing to and from disk is a mistake and we should just be passing hte Models around between Nodes and the Exchange but need to think through this more and see if there is good reason to write to disk.  Maybe fault recovery?
*/

// Nodes impl an interface to check if the rep exists and if it doesn't, provides a cli capability for creating it
//   - This isnt possible for all nodes (like SedaroMl) because their rep isn't derived from anything.  In this case, we 
// need to communicate to the CLI interface to block until the file is added and then try again or quit. Quit for now.
// Nodes impl an interface for checking whether the rep has changed since the last time the exchange was active
// Can we assume that only the exchange touches the SedaroML files for the nodes?  For now, yes

// Excel needs to have one thing writing to the produce cells and another reading from the consume cells.  We need this to be disjoint models.
// How can we have this without bifurcating all models in the exchange.  Can the conflict bifurcation exist only in excel?  
// Solution here is to only respond to the ModelDiff in the sedaroml_to_excel

// Issue: To locally reconcile a cosim model, we need to read the other models to get the produce side and then consume the sim to get the consume side.  This is awkward.
// Solution:
// Can first run the intra-node change detection and reconciliation
// Then can run this in the exchange
// Once both are complete, there is a source of truth for the produce side of cosim and the cosim exchange can start up and run normally
// Alternatively, an initializer can be provided to define the whole or partial state of a node and then the rest of the nodes update to reflect this change

// ~~Exchange comes up, consumes from the sim to get the consume side (should it produce first?  Yes)~~
// Exchange comes up, looks at other models to figure out what produce side should be, produces, then consumes
// This makes the rep complete and the exchange can start up from here normally
// On consume, if there is a diff, the exchange is triggered, else noop
// On external change, node produces values from the model
// If something external to the node wrote to the consume interface, this would result in a conflict on the next consume and be handled the normal way.

// What does mutating the model look like instead of constructing a new?  What are the issues here?
// - How to remove blocks that were deleted?
// - How to add blocks that were added?
// - Essentially how do you know what you can mutate?  I sort of think you can't and the better thing is to have two models.  
//     - But if we do this, need to pass the other model to the translations so they can read it to do their job
//     - Update: the diff is the way and we stick with one model which is cleaner and provides a compelte view of the state of the interface

// Model diff
// Need deterministic approach to diffing out models but this is very valueable to have anyway
// translations act on the whole model and the reconciliation acts on the diff
// perhaps instead of the translations returning changed/unchanged the nodes figure this out syncronously and report back to the exchange
// - Or rather the exchange handles it before triggering `Changed`
// Keep current node behavior maybe and add in the option for diff based reconciliation


// Docs content
// ModEx handles the all the annoyances of interfacing and interoperability: concurrency, conflict resolution, race conditions, initialization, change detection (initial and in flight), etc.
// Also good because of the easy of writing interface models once SedaroQL is supported.
// Also good because of the portable, composable approach to building up the exchanges.  Can reuse Nodes, Translations, etc. without starting over each time you want to connect two things.
// Its a framework??
// Because the IR for ModEx is SedaroML, it integrates seamlessly into the Sedaro open source ecosystem (Blueprint, cosimulation, SedaroQL, etc.)
// Ideally the exchange would write/read to the local sedaroml file and then a parallel thread would reconcile the "foreign" model
// This approach is key for cosimulation where models are changing frequently.  Need to be mindful of conflicts though.
// Idea it to build stable, maintainable, revisitable software that can perform mission critical model interoperability 
// One method of conflict resolution is to bifurcate the model into two - one consume and one produce.  This allows for 
// uninterrupted translations but is restrictive in that a model can only write their half.  This is what the first version of cosimulation uses.
  // - This is now out of date (replaced by ModelDiff approach)
// No rework required to add new translation nodes/branches into the exchange.  If you already have an interface model between AFSIM and sedaro, just 
// plug it in to one of the compatible existing nodes (i.e. Sedaro or AFSIM)

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
use modex::translations::{Operation, Translation};
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
      Ok(())
    },
    reverse: |from: &Model, to: &mut Model| {
      // Spacecraft.root.dryMass as Mass.kg -> get_first_block_where!(name='spacecraft_dry_mass').value
      let block = from.block_by_id("NT0USZZSc9cZAmWJbClN-").expect("Block not found");
      let esr = block.get("esr").unwrap().as_f64().unwrap();
      
      let filter = HashMap::from([("name".to_string(), Value::String("battery_esr".into()))]);
      let battery_esr_name = to.get_first_block_where_mut(&filter).expect("Block matching filter expression not found.");
      battery_esr_name.insert("value".to_string(), esr.into());
      Ok(())
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
      Ok(())
    },
    reverse: |from: &Model, to: &mut Model| {
      let vector = from.root.get("consumed_value").unwrap().get(0).unwrap().get("ndarray").unwrap().as_array().unwrap();
      let x = vector[0].as_f64().unwrap();
      let y = vector[1].as_f64().unwrap();
      let z = vector[2].as_f64().unwrap();

      let filter = HashMap::from([("name".to_string(), Value::String("position_eci_x".into()))]);
      let block = to.get_first_block_where_mut(&filter).expect("Block matching filter expression not found.");
      block.insert("value".to_string(), x.into());
      let filter = HashMap::from([("name".to_string(), Value::String("position_eci_y".into()))]);
      let block = to.get_first_block_where_mut(&filter).expect("Block matching filter expression not found.");
      block.insert("value".to_string(), y.into());
      let filter = HashMap::from([("name".to_string(), Value::String("position_eci_z".into()))]);
      let block = to.get_first_block_where_mut(&filter).expect("Block matching filter expression not found.");
      block.insert("value".to_string(), z.into());
      Ok(())
    },
  };

  let other = Operation {
    name: Some("other".into()),
    forward: |_, _| {
      Ok(())
    },
    reverse: |_, _| {
      Ok(())
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

  // let exchange = Exchange::new(vec![translation_cosim]);
  let exchange = Exchange::new(vec![translation_cosim, t, tt]);
  exchange.wait();
}
