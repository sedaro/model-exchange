### Model Exchange (ModEx)

An open-source model translation tool powered by SedaroML and SedaroQL and written in Rust.  Model Exchange enables flexible, multi-point exchange of model information between disparate tools that use disparate ontologies to represent – in part or in whole – the same information.

As a result of initial ModEx development efforts, the team has elected to pursue Sedaro Modeling Language (SedaroML) as the intermediate representation (IR) for models within ModEx.  SysMLv2 was originally baselined for this purpose.  The decision to use SedaroML was made due to the greater approachability, flexibility, and tooling that exists for the open-source language.  

SedaroML defines system properties and structure as normalized, interrelated, and hierarchical blocks of attributes. SedaroML is JSON-based and is designed to be easily human and machine readable/writeable. This includes model interpretation, traversal, etc.  Sedaro is editable as JSON in any text editor or within Sedaro Blueprint.

SedaroML is also queryable via the open-source Sedaro Query Language (SedaroQL) which is particularly powerful in the ModEx use-case as it enables the definition of flexible, reversible, and easily maintained translational mappings between input and output models.  This is something that would need to be developed in SysMLv2 but is already available for SedaroML.

The change from using SedaroML in place of SysMLv2 does not change the overall functionality or use of ModEx as proposed.  As mentioned, Sedaro provides a mature bi-directional translation capability between SysMLv2 and SedaroML which will be leveraged here to achieve SysMLv2 compatibility in the ModEx technology.

#### Dev Notes
- Proposal: https://sedarocorp-my.sharepoint.us/personal/robbie_robertson_sedaro_com/_layouts/15/onedrive.aspx?isAscending=false&sortField=Modified&id=%2Fpersonal%2Frobbie%5Frobertson%5Fsedaro%5Fcom%2FDocuments%2F00%20Sedaro%20Drive%2F01%20BD%20and%20Sales%2F02%20Proposals%2F71%20233%20C%20Phase%20I%20SBIRs%2FKill%20Webs%2FSF233%200016%20Phase%20I%20Tech%20Volume%20Mosaic%20v2%2Epdf&parent=%2Fpersonal%2Frobbie%5Frobertson%5Fsedaro%5Fcom%2FDocuments%2F00%20Sedaro%20Drive%2F01%20BD%20and%20Sales%2F02%20Proposals%2F71%20233%20C%20Phase%20I%20SBIRs%2FKill%20Webs
- Read this: https://link.springer.com/article/10.1007/s10270-021-00881-2
- There is a lot of compatibility with our current work in
  - Hierarchical models, model dependencies
  - SedaroQLv2
  - Studies
- Idea is input (A), translation, and output (B) models are all SedaroML.  Eventually.  Initially the translation model will be code
  - Via SedaroQL maps A<>B
- And then also use same thing for cosimulation which would allow for modex use in simulation or out of simulation.
  - Essentially, ModEx could be configured to run such that it translates the input model to the output model and then makes cosimulation calls to the simulation that is propagating the output model
    - Excel connected in as a cosimulator..... really cool
    - Same for SysML I guess?

#### Open Questions
- How to handle when things are deleted from/added to a model?
  - Does the .lock file storing the prior version help detect this?  I think so.
- How to handle translations like `get_first_block_where!(name='spacecraft_dry_mass').value as Mass.g <-> Spacecraft.dryMass` where there are two `Spacecraft` blocks?
  - Treat like vector math?  scalar + vector (i.e. scalar query result -> vector query result) and vector + vector (i.e. vector query result -> vector query result).  Second requires that vectors are the same length.
  - But what happens when you do vector + scalar (i.e. vector query result -> scalar query result)?  Should be an error?
- How to make py03 faster to compile?
- How to fix import issues re: xlwings and any 3rd party dep

#### Requirements
- [X] Multi-point/drop
- Will need to handle inter-step translation dependencies such that dependent translations are conducted after their dependencies are translated
- There should be a confirmation option so it doesn't make changed until the diff is approved
- Should handle units/QuantityKinds
  - Critical to translation of model and cosim, etc. because we don't know what the correct units should be ever and unit conversions between models is challenging without a standardization in the SedaroML intermediate representation (IR)
- Cycle detection


#### TODO
- `modex_python.excel`
  - Check for range intersection as this would be illegal


#### Watcher

Lock everything before a pipeline starts? This doesn't really work.  Hash the file?  How does the file system know that the content actually changed?  Does it?  It may not...

Before a translation "step" is performed, the output file is checked to make sure it hasn't changed since the lock was taken
To avoid races here, also check before writing to the output file.  Or maybe just only check before writing
This can be done by just comparing the contents of the lock file to the contents of the main file
Later we could actually compare model equivalence in case the changes were cosmetic or inconsequential to the model semantics




