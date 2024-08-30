/*
TODO:
xlwings ctrl + x things is really annoying.  It has stopped working and I can't explain why breaking out the exchange test caused things to start failing again.
0.5. Interaction between inputs and outputs of the exchange as well as individual stages of the exchange are messy and unintuitive.  
Need to clean this up and document it. Ideally we would use this same interface for customization by end users.
0. Slow to query the API during translation.  Could parallelize but also, we do things like read to refresh the model before a translation
which seems unnecessary.  Why are we doing this?  Can this be handled differently when no a local file?
Ideally the exchange would write/read to the local sedaroml file and then a parallel thread would reconcile the "foreign" model
This approach is key for cosimulation where models are changing frequently.  Need to be mindful of conflicts though.
2. Try in cosimulation (via adaptation of the custom watcher for cosim I guess?)
3. Implement exchange lock (locks the entire exchange while a translation is in progress and is awaitable from things like tests)
4. Handle conflict resolutions:
  - Watcher triggers while translation is in progress
  - Exchange wants to write a file that has changed since it was read
4. Add Sedaro and Cameo "exchangeables"
5. Satisfy remainder of Mosaic Warfare requirements: Web gui? REST API?
6. How to handle startup chaos where the watchers get mounted and then things start creating/changing files because the SedaroML IR doesn't exist yet
- I think the solution here is to read/create the rep before starting the watcher
- Need to handle things like excel changing while the exchange isn't running, leaving a conflict between excel.xlsx and excel.json just that on start up, translations to excel.xlsx can't operate over the current state of excel.xlsx
0. Colors to printing interface
0. How to handle changes that occurred while exchange wasn't running?
*/

// Can run virtually in a different dir maybe or perhaps with recovery we don't care 
// Potentially need to run the operation array in reverse when performing a reverse translation.  Need to think about this more.  Order of operations thing
// This doesn't handle recursive deps yet in the translations.  ie. t_a requires that t_b is run first.
// ^ This does not mean recursive model deps but within a model translation, requiring the result of a prior translation.  If this is needed
// should combine into a single translation.

// Things to add tests for SOON
// Check that model identifiers are unique
// Double triggers (trigger comes in while translation is in progress) for same model and differen model in change (causing a conflict in this case)
// user error in a translation function

// A need to periodically poll (http or something local, shouldn't matter) for changes via async thread

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
use modex::nodes::sedaroml::SedaroML;
use serde_json::Value;
use modex::logging::init_logger;
use modex::model::sedaroml::Model;
use modex::nodes::sedaro::{Sedaro, SedaroCredentials};
use modex::nodes::excel::Excel;
use modex::exchange::Exchange;
use modex::translations::{Operation, Translation, TranslationResult};
use modex::utils::python_signal_handler;
use std::thread::sleep;
use std::time::Duration;


#[tokio::main]
async fn main() {
  init_logger().expect("Failed to initialize logger.");
  
  let excel = Excel::new("test.xlsx", "test.xlsx");
  let sedaro = Sedaro::new(
    "spacecraft.json", 
    "https://api.astage.sedaro.com", 
    "PNdldNPBmJ2qRcYlBFCZnJ",
    SedaroCredentials::ApiKey("NXLFwnrS7LRHX36oIsFW-.uR_2HGUyO9YVrKnTsT1m_9qV5728TxpTFNna22dchdEnS2ZWD5WgzcLBnj4ufosex6ovG1BNfrZIldHrI68pWg".into()), // alpha
    // SedaroCredentials::ApiKey("NTH7518Ucq11BW7mEdxAF.1dSD4JZltUZROLtkxUS_nSnXxZ_NrrkR_kole1rswPG3A6gZW64iVSXfZQQg1c6nG0HwbS1dxXlovfxvdV5wOQ".into()), // prod
  );
  let test = SedaroML::new("test.json", "test.json");

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

  let other = Operation {
    name: Some("other".into()),
    forward: |from: &Model, to: &mut Model| {
      // get_first_block_where!(name='spacecraft_dry_mass').value as Mass.g -> Spacecraft.dryMass
      let filter = HashMap::from([("name".to_string(), Value::String("battery_esr".into()))]);
      let battery_esr_name = from.get_first_block_where(&filter).expect("Block matching filter expression not found.");
      let esr = battery_esr_name.get("value").unwrap().as_f64().unwrap();
      println!("OTHER: {}", esr);
      Ok(TranslationResult::Unchanged)
    },
    reverse: |from: &Model, to: &mut Model| {
      Ok(TranslationResult::Unchanged)
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

  let exchange = Exchange::new(vec![t, tt]);
  // loop { // This is a hack to get the xlwings process to terminate on ctrl+c
  //   python_signal_handler().unwrap();
  //   sleep(Duration::from_millis(100));
  // }
  exchange.wait();
}
