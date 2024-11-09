#### Open Questions
- How to handle when things are deleted from/added to a model?
  - Does the .lock file storing the prior version help detect this?  I think so.
- How to handle translations like `get_first_block_where!(name='spacecraft_dry_mass').value as Mass.g <-> Spacecraft.dryMass` where there are two `Spacecraft` blocks?
  - Treat like vector math?  scalar + vector (i.e. scalar query result -> vector query result) and vector + vector (i.e. vector query result -> vector query result).  Second requires that vectors are the same length.
  - But what happens when you do vector + scalar (i.e. vector query result -> scalar query result)?  Should be an error?
- How to make py03 faster to compile?
- How to fix import issues re: xlwings and any 3rd party dep

#### TODO
- Add [examples](./examples)
- Update Sedaro watcher to use ModelDiff instead of metadata for change detection? Slower? (Get rid of metadata file and use ModelDiff for change detection)
- Add Cameo and AFSIM Nodes
- Put things in the exchange that have dependencies but that aren't connected to other things in the exchange.  Do we allow for this?
  - i.e. two unconnected sub-graphs
- Implement exchange lock (locks the entire exchange while a translation is in progress and is awaitable from things like tests and conflict resolution)
- Handle runtime conflict resolutions:
  - Watcher triggers while translation is in progress
  - Exchange wants to write a file that has changed since it was read
- Very likely communicating things by writing to and from disk and we should just be passing the Models around between Nodes and the Exchange but need to think through this more and see if there is good reason to write to disk.  Maybe fault recovery?
  - Is there any advantage to writing to disk with regards to cascading multiple exchanges, in different processes, off an another?
- `modex_python.excel`
  - Check for range intersection as this would be illegal
- Need to fix the the Exchange watcher trigger as a result of the Sedaro Node writing the model to disk when `temp-0`, etc. relationship references are resolved by the `put`.

#### Other
- Will need to handle inter-step translation dependencies such that dependent translations are conducted after their dependencies are translated
- There should be a confirmation option so it doesn't make changed until the diff is approved
- Should handle units/QuantityKinds
  - Critical to translation of model and cosim, etc. because we don't know what the correct units should be ever and unit conversions between models is challenging without a standardization in the SedaroML intermediate representation (IR)
- Cycle detection

#### Watcher

Lock everything before a pipeline starts? This doesn't really work.  Hash the file?  How does the file system know that the content actually changed?  Does it?  It may not...

Before a translation "step" is performed, the output file is checked to make sure it hasn't changed since the lock was taken
To avoid races here, also check before writing to the output file.  Or maybe just only check before writing
This can be done by just comparing the contents of the lock file to the contents of the main file
Later we could actually compare model equivalence in case the changes were cosmetic or inconsequential to the model semantics

#### Misc.

```
Nodes impl an interface to check if the rep exists and if it doesn't, provides a cli capability for creating it
  - This isnt possible for all nodes (like SedaroMl) because their rep isn't derived from anything.  In this case, we 
need to communicate to the CLI interface to block until the file is added and then try again or quit. Quit for now.
Nodes impl an interface for checking whether the rep has changed since the last time the exchange was active
Can we assume that only the exchange touches the SedaroML files for the nodes?  For now, yes

~~Exchange comes up, consumes from the sim to get the consume side (should it produce first?  Yes)~~
Exchange comes up, looks at other models to figure out what produce side should be, produces, then consumes
This makes the rep complete and the exchange can start up from here normally
On consume, if there is a diff, the exchange is triggered, else noop
On external change, node produces values from the model
If something external to the node wrote to the consume interface, this would result in a conflict on the next consume and be handled the normal way.

What does mutating the model look like instead of constructing a new?  What are the issues here?
- How to remove blocks that were deleted?
- How to add blocks that were added?
- Essentially how do you know what you can mutate?  I sort of think you can't and the better thing is to have two models.  
    - But if we do this, need to pass the other model to the translations so they can read it to do their job
    - Update: the diff is the way and we stick with one model which is cleaner and provides a compelte view of the state of the interface

Model diff
Need deterministic approach to diffing out models but this is very valueable to have anyway
translations act on the whole model and the reconciliation acts on the diff
perhaps instead of the translations returning changed/unchanged the nodes figure this out synchronously and report back to the exchange
- Or rather the exchange handles it before triggering `Changed`
Keep current node behavior maybe and add in the option for diff based reconciliation

Can run virtually in a different dir maybe or perhaps with recovery we don't care 
Potentially need to run the operation array in reverse when performing a reverse translation.  Need to think about this more.  Order of operations thing
This doesn't handle recursive deps yet in the translations.  ie. t_a requires that t_b is run first.
^ This does not mean recursive model deps but within a model translation, requiring the result of a prior translation.  If this is needed
should combine into a single translation.

Each node should implement (optionally?) a lock that prevents races and/or collissions
This lock file should also potentially enable detecting when things are deleted/added instead of just changed but 
need to think through this usecase more
Potentially lock files or something more intelligent, like a .git file for locking? provide recoverability.  This should
be optional though so as not to slow down things like cosim where the recoverability doesn't make sense because the model 
is dynamic

How to integrate with static model and dynamic model (i.e., via cosim)?

Enable a sense of virtualization such that the actual files aren't changed but virtual copies somewhere else in the filesystem?
Would help with unit testbed
Maybe just keep in memory instead of writing to file.  Model could implement via abstract write/read interface.  Maybe just have a VirtualModel type?
```