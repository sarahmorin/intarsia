
**WARNING: This is an old doc that is likely outdated. It shows a lot of the thought process used,b ut might not reflect current system.**

# Initial Design

The name Query Optimizer Framework is somewhat misleading. Ideally, we are designing a system for building query optimizers (or perhaps even more generic compilers), not a single query optimizer. We draw inspiration from traditional query optimizers, egg and equality saturation, and term rewriting systems. The purpose of this document is to work from our current example of a SQL->Hydro optimizer to outline the goals of the more abstract framework design. As the design evolves and concretizes this doc will likely just provide a high level overview and link to pages for each components specific design.

> [!tldr] TL;DR as of 2025-07-29
> - The framework has 3 "programmable" components (input processing, optimization, and code gen) and a static function (control flow) to stitch them all together.
> - Input processing, code generation, and control flow are necessary but uninteresting components; beyond a basic sketch of requirements their design is omitted for now.
> - The goal of the optimizer component is not to build a singular "
> best query optimizer" but to construct a framework for building optimizers for a variety of source and target languages. See [Generalizing Query Optimization](#generalizing-query-optimization)
> - **Search Space** -> To start, the QGM will be a cascades-style e-graph, i.e. logical and physical operators and just operators and properties are handled by rules
> 	- Is this the end goal? No, ideally we could do something even more generic re the different equivalence relations on logical vs. physical operators and properties (perhaps ISLE's type checking rules are a good jumping off point). This is a place where starting small and iterating is worthwhile because we get a working implementation to compare to when investigating alternate structures later on.
> 	- Does this mean we're just re-doing cascades? I don't think so. I think providing more control over the search is still a solid contribution even on a re-used structure.
> - **Rules** -> will be typical lhs -> rhs, plus phase and priority/promise annotations
> 	- The phase annotation allows us to group rules into phases indicating when a rule becomes available. During a phase we execute the search using only the currently available rules until we reach budget (and return) or a fixpoint (and proceed to the next phase).
> 	- The priority or promise annotation hints at how beneficial a rule might be and can be used to compare it to other rules in the current phase.
> 	- Phase annotation imposes a strict limit on available rules to the search functions (i.e. the exploration APIs require the current phase and will only return active rules) while the promise allows the search algorithm to reason about and re-order application of rules.
> 	- Want to look into potential of the ISLE style rules, but that depends more on underlying search space
> - **Programmable search** -> Plan here is still quite open ended and warrants exploration
> 	- To start, we know how to build a top-down and bottom-up search on an e-graph, let's carefully examine what functions those implementations require, how they behave, how they affect global state, and ultimately get the venn diagram comparing them.
> 	- We can also define the trait requirements of the task list and task functions (i.e. tasks on the list).
> 	- From there we can try to outline a scheduler and determine what portions of it, if any, can be static vs. what we want to be programmable.
> - Cost functions can map to more interesting domains than just a numeric cost as long as it is a lattice.
> - The search budget, analysis and extraction are all fairly straightforward.
> 	

>[!question] Open Questions and Problems as of 2025-07-29
>## High Level Design
> - [ ] Search graph structure -> start with cascadey-e-graph and then work on a mutli-relation e-graph? or work on that now?
> - [ ] Programable Search -> still very open ended, is this the right plan to move forward or do I have a blind spot? Is covering top-down and bottom-up enough to start?
> - [ ] How do we abstract that query optimizer treatment of logical vs. physical to a more generic model? Do we even need to?
> 	- Honestly, I think the logical-physical model is essentially the same as a compiler mapping from a high level language to a lower level language.
> 	- One thing to be conscious of is ensuring that writing an optimizer within one language isn't a nightmare (square peg round hole)
> 	- [ ] Assuming we can only compute cost and extract a plan with all physical operators, what are the best minimum requirements needed to ensure we always have *some* physical plan? Does every logical operator need a default physical operator? Should pre-processing guarantee the e-graph includes one? Most query optimizers have a solution to this, so a) what's the best one, and b) what's the most general one?
> - [ ] Rules
> 	- [ ] Should we have distinct types? e.g. Logical transformation vs Physical implementation
> 	- [ ] Presumably, phases will be monotonically growing subsets to start. Is it worth investigating disjoint phases? Or more granular control over phase structuring?
> - [ ] Can we use ISLE style rules? I've only seen ISLE applied to an acyclic e-graph and not allowing cycles feels like a big loss.
> 	- [ ] Does the ISLE approach affect how much we can vary search strategies (it feels very top-down, but maybe I haven't given it a full think through)
> ## Implementation Details
> - [ ] Writing rules -> DSL vs. macros vs. straight up Rust?
> - [ ]  How custom should the input data processing be? See [Cost Function](#cost-function)
><mark style="background: #ABF7F7A6;">TODO</mark>
# Project Overview

Largely, our project has 2 outputs:
1. An abstract system framework one can use to implement and tune optimization/compilation from a source to a target language.
2. Instances of the system such as a SQL-Hydro query optimizer and a generic algebraic expression optimizer

> [!Example] SQL to Hydro
> First, let's consider what the SQL to Hydro optimizer would look like. Note, this example assumes we have already done the work of implementing and tuning the optimizer component, later on we will discuss how that work is done and what features the abstract framework must provide to facilitate that. This example outlines how the hydro optimizer processes a single query. 
> 
> **Input**
> - SQL Query
> - Database (or Catalog and Schema)
> 
> **Process**
> 1. Parse SQL query to AST
> 	- Basic syntax checks (likely taken care of by SQL parsing library)
> 	-  Validate query given schema
> 	- Convert to initial Plan AST format
> 2. Customize optimizer to database
> 	- Incorporate stats from catalog into relevant cost functions
> 	- (Optional) Using catalog, reorder/prioritize/disable rules accordingly
> 3. Build initial e-graph-esque data structures from starting query
> 4. Run optimization search using given optimizer "settings", such as:
> 	- Search strategy (e.g. bottom up, top down, pseudo-random walks, etc.)
> 	- Rule priorities (default and "custom")
> 	- Properties/Enforcers
> 	- Time and space limits
> 5. Extract best search strategy in Implementation AST format -> output
> 6. Apply transformation/interpretation from Plan AST to Hydro IR -> output
> 
> **Output**
> - Optimized query plan
> - Hydro implementation of that plan

In this example "optimization" is a bit of a black box. This is because, by the time the optimizer is running, the "interesting" work of building a custom optimization is done.

An instance of the optimizer has 3 main components:
1. Input ingestion -> Something capable of taking a statement in the source language to a common (or at least optimizer compatible) AST and gathering relevant input-specific cost information into a standard (optimizer compatible) format.
2. Optimization -> A customized optimization search that uses
3. Code Generation -> Something capable of translating the optimized plan from above to the target language. (Potentially also hooking into infra like hydro project structure for each of running)
## Generalizing Query Optimization
We want the system to build more than just query optimizers. So it should be able to build *good query optimizers* without loss of generality, i.e. it provides the necessary tools to construct a query optimizer without forcing other problems (such as algebraic expressions or equality saturation) into an overly specific domain. Additionally, since this is a framework and not a specific DBMS, some areas of query optimization are outside the scope of the project (at least for now). 

Common topics of query optimization and how they relate to the project:
* Search strategy -> How will we explore equivalent plans? 
	* Bottom-up vs. top-down vs. edgier alternatives, see [Programmable Searching](#programmable-searching) section
	* Core function of the optimizer component and programmable per-instance
* Search Space -> How will we represent and store potential plans?
	* See [Search Space](#search-space)
* Logical vs. Physical operators and properties -> Distinguishing between logical equivalence and physical implementation equivalence
	* Query optimizers typically have a set of logical operators and a set of physical operators and rules to equate logical to logical and logical to physical. Logical expressions are equivalent their logical properties are the same; however, physical expressions are only equivalent if their logical *and* physical properties match. 
	* Typically we only compute cost on physical expressions and assume logical expression have an infinite/undeterminable cost
	* This is a key challenge: we need a way to provide the necessary distinction between logical and physical operators without forcing the notion of logical and physical into the core optimizer. A natural abstraction is source vs. target language; however, we still need to ensure we can optimize within one language. ***We need to find balance.***
	* **This is a key issue in the design of the optimizer core.**
* Transformation Rules -> What transformations can we apply to queries to improve efficiency?
	* What are the key transformations that all optimizer employ?
	* What can we express with rules? How can we control the application of rules?
	* The structure of the transformation rules is paramount to the core design, while choices of specific transformations are relevant to specific optimizers.
* Cost estimation -> How do we estimate and compare cost of plans?
	* How do we ensure the catalog stats are accurate before beginning optimization?
	* How do we handle a stat that doesn't exist? Synchronously? Default to a constant guess?
	* Does the cost function make an accurate estimate given accurate data?
* Execution Feedback -> Can we use information from previous query or subquery executions to inform future optimizations
* Plan caching -> Cache previous results to reduce duplicate optimization work.

We divide these topics into relevance to the current project:
- Relevant to core optimizer component:
	- Search strategy
	- Search Space
	- Ability to phase and prioritize transformation rules
	  Access to a working cost function
- Relevant to specific instances built with the optimizer, but not core design
	- Cost model accuracy 
	- Specific transformation rules
- Outside Scope (or far future developments)
	- Execution feedback (Isn't really the focus, although its easy enough to see how one could construct a system capable of passing some data back through the input data if they build the infrastructure to do so.)
	- Plan Caching
## Relevant Background Resources
- [[Bottom-Up and System R Optimizers]] 
- [[Top-Down and Cascades Optimizers]]
- [[Alternative Approaches]]
- [[ISLE  Instruction Selection Lowering Expressions]]
- [[egg]]

From the resources listed above, the relevant points:
- Value in different search strategies -> we need a way to provide top-down, bottom-up, and potentially more experimental options like random walks
- Cascades seem like the best starting point
	- memo table QGM similar to e-graphs
	- task list and scheduling can be easily adapted to more programmable and abstract search strategies
	- macro-rules feel like ISLE rewrites
	- enforcer rules for physical properties might need to be relaxed or adapted to something else
- Cascades memo table and egg's e-graph are VERY similar and seem like the top contender for a QGM over which to search
	- I want to lean into e-graph more because its makes more abstract (not necessarily query optimizer) problems possible
	- Unlike e-graphs, query optimizers handle 2 kinds of operators: logical and physical, the issue being that we only compute a cost on physical operators, so we need some way to handle this
	- Cascades guidance is a good way to avoid redundant work (and the "explosion" of a rule)
- Most query optimizer have specific phases -> I think we can provide phases by gradually enabling sets of rules
	- With that in mind, every phase must be capable of producing a final output, so every logical operator needs a "default" physical operator mapping included in the earliest phase
	- "Simplification phases" can either be baked in to the initial translation from source language to implementation AST, or (since you might not "unlock" a simplification rule without some rewrites) we can provide a mechanism to make a rule a replacement in the e-graph rather than an addition
- ISLE - rewrites in Rust are virtual and fast
	- Integrating ISLE with the underlying QGM e-graph potentially makes searching and rewriting REALLY fast
	- The issue is that the only integration of ISLE with an e-graph that I've seen requires acyclic e-graphs, and generalizing that is a lot of overlap with egg2
	- Type safety could be a nifty way to handle properties, but that might be a stretch


# High Level Design

A fully-functioning instance of the optimizer has 3 main components: input processing, optimization, and code generation.  In order to communicate, all three components need to work with the same "optimizer compatible AST". Additionally, since the optimizer is "programmable" and its execution can be informed by input specific costs, we wrap the entire instance with a control flow component.

## Control Flow
To build an instance of the optimizer, the control flow needs:
- Optimizer configuration
	- Cost Functions
	- Search Strategy Config
	- Transformation Rules
	- Scheduling/Rule Priority
	- Search budget
- Parsing trait functions to map from source language to AST
- Input data processing functions to map additional input data to costs
- Code gen trait functions to map from AST to target language
To execute this instance on an expression, the control flow takes:
- Input Expression
- (Optional) Specific Cost Data
performs the following work:
1. Parse the input expression and process cost data
2. Build the optimizer using the given configuration and any specific information gleaned from cost/input expression
3. Run the optimizer on the input expression
4. Extract the optimized expression
5. Use code gen functions to translate from AST to target language
and outputs an optimized expression in the target language.
## Input Processing
Input processing has 3 main functions:
- Parse input query (or expression) and validate syntax
- Do an initial transformation from source language to implementation plan AST
- (Optionally) Process information relevant to cost estimation. For example, the query optimizer will take in a DB schema and catalog. The schema can be used in query validation and the catalog can inform the cost functions.
## Optimization
Most query optimizers (even the more extensible ones) are very domain specific. We aim for generality. Instead of focusing solely on relational algebra query optimization, what if we abstracted even further to "generic expression optimization". Rather than building a single optimizer, let's provide a framework capable of building a myriad of optimizers.

We provide a graph-like structure to represent implementation plans and a suite of functions to interact with that structure safely (even in a multi-threaded environment). The programmer provides transformation rules, cost functions, and search strategy parameters (e.g. rule priorities, scheduling, etc.) according to a structure we define (likely through Rust `trait`s). Together, these elements comprise a single instance of an optimizer component.
## Code Gen
Code generation maps the optimized plan into the target language. For now, code gen is very open ended and the only "required" function is the mapping from AST to *some* language that can be invoked by the control flow. The specific output of that mapping is entirely up to the programmer.

For now, code gen is left open ended and will be fleshed out  later.

## AST

<mark style="background: #ABF7F7A6;">TODO:</mark> Flesh out
- [ ] translation from input to AST and AST to output should be deterministic, idempotent, even though optimizer might not necessarily produce the same plan for an input given changes in parameters
- [ ] Basic abstract operators and arguments structure 
	- Op -> Rust enum of operators and terminal types
	- args -> arbitrary sized vector of ops
- [ ] Logical vs. Physical Operators (or source vs. target)
	- Need a way to distinguish between operators that correspond to an implementation in the target and those that are still too high level
	- Also need a way to do optimization one a single language
- [ ] How do we include properties?
	- To start, properties will be managed within the rules (ala cascades)
	- Eventually, it could be cool to annotate operators with input requirements and output affects (maybe this is just syntactic sugar maybe its something more)

# Input Processing

<mark style="background: #ABF7F7A6;">TODO</mark>: Not the *most* interesting part, will be fleshed out later.
# Optimizer

The big idea is to decompose optimization into a collection of independent operations. We examine current query optimizer approaches and draw inspiration from other areas, such as equality saturation, to construct an abstract framework of an "optimizer", each instantiation of which can behave differently so long as its operators satisfy a few properties. 
## Basic Workflow
Assuming we have a query/expression parsed into our AST and our cost estimation information loaded into the appropriate places, what will an optimizer instance do? The basis of the optimization is transformation rules and a graph-like search space holding an equivalence relation on query plans. The optimizer will search the current plan tree looking for unexplored transformations. When it finds the left-hand side of a transformation rule it will apply the right hand side, which will potentially induce more searching. Eventually, we will reach a stopping point. If we compute all possible substitutions and no new information can be learned, we have performed a full equality saturation; however, optimizers typically employ pruning methods to avoid searching expensive plans since we only care about finding the "best" plan. During the search/transformation and after, we can apply a cost function to the plan tree to get the estimated cost of each query/subquery. We can use this cost to help tailor our search and avoid exploring expensive subplans. When we complete the search, we apply the cost function and some analysis to compare costs/properties to find the lowest cost plan.
### Static vs. Dynamic Components
How much of the optimization can we make "programmable"? When breaking down the optimizer into independent components we can then stitch together, where do we draw the line between what's provided by default and what's provided by the programmer?

If everything is dynamic, then we have built nothing and simply asked programmers to write an optimizer -- so something has to be static, namely: data structures.
-  Search tree -> We provide an e-graph (like?) data structure over which search functions can operate and the corresponding interfaces to explore, insert, and prune the graph. The functions we use to interact with the search space must be consistent to guarantee correctness (and in the future provide thread safety), but the search algorithms built from those methods may vary. 
- Rules -> Although the rules themselves (and even the language they operate on) will vary, the syntax of rules and the functionality to search and apply rules are all fixed.
- Cost -> Cost functions (and even the codomain of those functions) are programmable, but they must satisfy type safety requirements and the codomain must be a semi-lattice. The domain is fixed (i.e. subtrees of the search space) and all cost functions must map into the same codomain, but that codomain can be a complex type so long as we can perform analysis on it.
- Task Queue -> Regardless of search strategy, there will be a set of search tasks. Now, the specifics of the queue (LIFO, FIFO, random pop, etc.) are variable, but all search strategies will employ the queue.
### Asynchrony
Distributing optimization search work is a good goal future goal. So, what do we need to make sure of now to allow us to introduce and play with distribution in the future:
- Thread safety on search graph -> As long as the search only interacts with the search tree using functions we provide, we can ensure those functions are thread safe or provide thread safe variants.
- Thread safety on task list -> Same thing. Managing the task list must only occur through explicitly provided methods.
- Decision making functions are deterministic and idempotent -> e.g. cost function
- Careful control of progression between phases -> The current rule phase must always agree. Moving on from one phase to another requires reaching a fixpoint and having remaining optimization budget; if there is disagreement about either something went wrong.

## Search Space
The graph-like data structure over which we search is, perhaps, one of the most important design choices, given that it is not programmable. The data structure must provide functions to search and modify the graph, but how those operations care composed to implement a specific search algorithm may vary so long as we can verify necessary invariants are maintained. For example, the cascades memo table in conjunction with the LIFO task list performs a top down search but we could implement a bottom-up (if you squint) search on that same memo table with a carefully managed FIFO + depth priority task list. Thus, as long as the structure doesn't impose strict restrictions on the search strategy, it is viable. 

A natural choice for this structure is an e-graph; however, we need to tackle the logical-physical and property based equivalence problem. Cascades handles this by unifying logical and physical operators under one abstract operator type and carefully managing the application of 3 types of rules:
- Transformation Rules -> Logical to logical
- Implementation Rules -> Logical to Physical
- Enforcer Rules -> Application of a Physical Property
This approach is easy to implement in an e-graph; however, I fear the enforcer rule approach to properties is limiting. In this model the management of properties is embedded in the rules and the programmer is responsible for ensuring properties are maintained correctly. I want to explore properties as more closely tied to operators (i.e. an operator requires certain properties of its inputs and applies/maintains/removes properties to its output) and the management of inputs/outputs satisfying properties handled "automatically" rather than explicitly in enforcer rules.

The key issue is that we need to manage two relations: logical equivalence and physical equivalence. Moreover, we might actually want to relax physical equivalence to physical close-enough-ness. So can we devise a multi-relational e-graph?

> [!important] Temporary Solution
> For now, I think option #1, the cascades style e-graph with enforcer rules, is a decent starting point. There is still room to generalize in search strategy, scheduling, etc. and generic cost functions. It also gives a point of comparison later on when experimenting with alternate e-graph implementations.
> 
> Ideally, the initial implementation will still leave room for future development on funky e-graphs with a more abstract concept of properties.

So how do we reconcile? Some options:
1. **We don't -- Cascades in an e-graph** -> Simple, easy to implement, but is it enough?
	- Cascades just treats logical physical operators as a single abstract class of operators and physical properties are managed by special rules. Essentially, we either generate logically equivalent expressions with transformation rules, or given a single logical expression we generate physically equivalent expressions with implementation and enforcer rules.
	- Limits the capabilities of physical properties, i.e. they are less abstract and customizable when they must be implemented as rules. Also implies that there is a mechanism to apply any physical property, which is true in most cases, but again makes the properties more specific.
2. **Nested E-classes** -> "Outer" e-classes maintain logical equivalence. "Inner" e-classes maintain physical equivalence within a set of logically equivalent operations. *This one is kind of crazy and needs much more thought.*
	- Keep every physical equivalence, rather than just the best so far (is there value in that?)
	- 2 equivalence relations:
		- Logical equivalence is defined by transformation rules
		- Physical equivalence is defined by transformation rules and property restrictions
		- "Outer" e-classes are a typical e-class (collection of e-nodes which are logical expressions) defined by a canonical element plus a collection of "inner" e-classes
		- "Inner" e-classes are equivalent to all logical e-nodes of their "outer" e-class and are defined (or distinguished from one another) by a set of properties according to the equivalence rules (if any) on properties.
	- Property based equivalence is tricky:
		- For simple properties like a bool flag, its easy to imagine how we might either distinguish expressions by the flag value or write a rule to ignore the property
		- For more complex properties like a real number value, it gets trickier
		- Furthermore, it's likely that properties will be important in relation to cost, so rules about property equivalence make more sense based on cost as opposed to expressions/operators
	- Maintaining invariants needs to be reworked, but I don't think its that far off:
		- logical-logical rules merge "outer" eclasses
		- logical-physical rules populate "inner" e-classes i.e. construct a physical expression
	- <mark style="background: #ABF7F7A6;">What happens if a logical-physical rule constructs a physical expression that already exists in another "outer" e-class?</mark>
		- Need to think this through more but my intuition is that this shouldn't be possible, or this is only possible if the outer e-classes are, in fact, equivalent but we're simply seeing the logical-physical rule before the logical-logical rule holding that equivalence?
3. ~~**E-nodes keep the "best so far" physical equivalence of the logical**~~  -> This doesn't work unless we do bottom-up (cost estimation can only be compared if we know arguments are static).
	- Very similar to memo tables/explore group/explore expression flow, essentially:
		- Explore group = logical-logical rules search in an e-class
		- Explore expression = logical-physical rule search in an e-node 
		- Enodes maintain the best physical equivalence seen so far
		- E-classes and e-nodes maintain a bitmap (or something) to track what rules have been tested so far.
		- When all rules have been applied to an e-class and all e-nodes in the e-class (and all children/args), we can compare all the physical implementations in that e-class and fold to the best one.
	- Impossible to determine "best so far" without evaluating all children to completion -> makes random walks style search less possible
	- Also very similar to the current egg e-class analysis work
	- Maintaining invariants doesn't change
	- Precludes programmer from writing physical-physical rules, but do we even want that? There's also the workaround of writing a "logical" wrapper for all physical operators to allow you o do this but then you inflate the e-graph and the rule set in a gross way
4. Something else entirely...?
## Rules
At a very high level, these are just rewrite rules which say the left-hand pattern of operators and variables is equivalent to (or can be replaced by) the right-hand side pattern of operators and variables. In addition to the substitution information, we also include information to inform scheduling and rule application.

Rules will include the following:
- Name -> optional string for helpful error messages
- LHS -> Search pattern
- RHS -> Substitution pattern
- Phase -> Number indicating the phase the rule becomes active (optional, defaults to the last phase)
- Priority -> Number indicating likelihood the rule will be beneficial (optional, defaults to "worst")
- (Optional) Type -> enum(?) indicating what kind of transformation this rule encodes. See discussion below.

Patterns can consist of:
- Logical operators -> Abstract operator, no computable cost, needs to be lowered to a physical operator
- Physical operators -> Specific implementation operator with a computable cost
- "Enforcer" operators  -> Operator to apply some property (such as sorted-ness)
- Variables
- Constants

The core implementation will provide everything but the rules themselves: the `Rule` struct, a mechanism (macro vs. rust vs. DSL tbd) to write the rule set in an optimizer instance, and functions to apply (search and insert according to rules) in the search graph.
### Rule Types
Some systems differentiate between transformation rules (logical-logical) and implementation rules (logical-physical). For example, cascades applies transformation rules during the "explore group/expression" operation and implementation rules during the "optimization group/expression" operation.

Additionally, most query optimization systems take a phased approach where an early pre-optimization phase applies some transformations, aka simplification rules, that are (more or less) guaranteed to be improvements (like pushing filters down). Although our rules include a phase annotation, we don't yet have a mechanism for strictly replacing a pattern rather than adding a pattern.

<mark style="background: #CACFD9A6;">OQ: Do we want some notion of rule types? </mark> Simplification rules are contradictory to e-graphs in that we lose information, but we could provide a version that rather than indiscriminately replacing info trigger some e-class folding operation? But is differentiating between rules by type even necessary?
### Properties and Enforcers
Cascades represents enforcers as rules that insert physical operators to change the physical properties of a plan. While it is convenient that they are baked into the existing rule system, this puts some burden on a programmer when writing rules to ensure the enforcers are used correctly. 

For now I think this is ok, but in the future it might be worth exploring properties as an attribute of operations which are then automatically enforced in rules. Even if this is just through a pre-processing on the rules where we essentially insert enforcer operators having the guarantee that you didn't miss an enforcer when writing a rule could be nice.
### Rule Scheduling and Priority
Most query optimizers provide one or both of the following mechanisms:
- Phases -> Group the rules into phases (either disjoint or increasing supersets) and execute one group at a time in order, moving on to the next group when you reach a fixpoint or your budget. Typically there is some simplification (e.g. push all filters down) before general optimization. Within optimization cascades gradually phase in rules in an attempt to find better plans earlier.
- Priority/promise -> Assign rules a priority to indicate how beneficial they might be

Both of these are natural additions to our optimization system. They also create a natural key prefix (`phase.priority.[...]`) for an index storing all rules (B+ tree?). Giving every rule a phase and priority (default to last of either if not provided) covers a lot of ground; however, it doesn't provide a strictly simplification phase where rewrites are replacements rather than adding equivalences. In order to construct such a phase, we need the simplification rule type (see discussion on that above).

The scheduler will also need to manage invariants in the search graph. In [[egg]], the loop over the e-graph has a read phase and write phase to get the benefits of deferred invariant maintenance. More on this discussed in the [search strategies](#programmable-searching) section.
## Programmable Searching
The juiciest part of the optimizer framework is the programmable search strategy. Rather than allowing for custom rules but enforcing a top-down or bottom-up, the search can be customized as well. Since the search space is fixed, we need to demonstrate how we can vary the search strategies on the underlying e-graph. 
### Search Strategies
First, let's consider the 3 big camps:
- Bottom-up -> System R 
- Top-Down -> Cascades
- Randomized

Since our search space is an e-graph, how could we model these search strategies?
- Top-down -> The top down search aligns well with the current work on e-graphs and e-matching, so no need to dive deep.
- Bottom-up -> A [bottom-up e-graph](https://www.philipzucker.com/bottom_up/) search is less common, but still very possible. 
	- The basic idea for matching is to take a rule, guess the variables, and then construct the lhs using those assignments and check if the term exists in the e-graph already.
	- The "guess the variables" means test all possible assignments of e-classes to variables -> for rules with many variables this can be VERY expensive, but for small rules it might be beneficial
	- As with top down we can introduce mechanisms to limit redundant work by only tracking what rules have been applied to what e-classes, ordering rules, etc.
	- We can still use a task list to queue up the execution of rules by priority, but since the e-matching isn't recursive, we need the search function to manage exploration explicitly through rule application.
- Random -> There are many different randomized algorithms and I won't outline any specific one here, just point out places where it would be easy to introduce randomness:
	- In a top down search, when exploring an e-class select e-nodes randomly, when exploring the e-node select rules to apply (among rules with the same priority) randomly, etc.
- <mark style="background: #ABF7F7A6;">TODO:</mark> Model more searches

Whatever search strategy we choose, the search and scheduler will interact via the task list. The "search threads" (just one synchronous function to start) are dispatched by the scheduler and handle the next task in the list. Meanwhile, the scheduler manages ordering and pruning the task list. Note, both the search and scheduler can modify the task list. In a top-down optimizer the search recursively insert tasks to the LIFO stack and pop them off to execute, while the scheduler manages budget. In a bottom-up optimizer the scheduler is likely responsible for queueing search tasks and the search merely executes tasks one at a time.

> [!important] Initial Search Implementation
> For now, I think we define traits for search, scheduler, task functions, and the task list. The first pass at implementation will include a top-down search function and a bottom up search function. From there we can work on ways to make the search programmable without fully rewriting the search logic for each instance of an optimizer.

Cost Functions
The cost function(s) map from a plan AST to a cost. The specific implementation of a cost function is rather open ended, so long as it meets the following requirements:
- **Deterministic and Idempotent** -> A given plan always produces the same
- **Underlying Input Data** -> The underlying input data should be passed in to the cost function by reference and provide a reasonably efficient lookup operation.
	- <mark style="background: #CACFD9A6;">OQ: Is there value in making this more strict?</mark> E.g. The underlying data must be parsed into a HashMap during input processing, or whatever data structure we pick to get some space/time guarantees? Or is it better to leave it open ended for really specific cases where another format might be faster?
- **Cost Value Lattice** -> The cost values can be complex so long as the analysis function can induce some ordering on them and cost values are monotonically increasing as more operators are applied. i.e. cost of `g(f(a))` cannot be less than the cost of `f(a)`
### Search Budget
<mark style="background: #ABF7F7A6;">TODO</mark>: Elaborate
- [ ] Pretty straight forward: unless we do full equality saturation or an exhaustive search, we want a hard limit on time spent optimizing. I lean towards the approach of a task limit or transformations applied limit as opposed to a real time spent limit. 
- [ ] Search budget provides a mechanism to control the random search, "try to optimize this subplan using a randomly chosen but if it takes longer than X tries pull the rip cord"
## Analysis and Extraction
After we must extract the best plan found, i.e. the plan with the lowest cost. Cost functions map from a plan sub tree to a cost codomain. Analysis functions compare costs. For the simple case of a single numeric cost value, the analysis function is trivial; however, more complex, multi-dimensional costs might require custom implementations. 

Given a working analysis function, we can extract the optimized expression from the graph as normally would in an e-graph.
# Code Generation

<mark style="background: #ABF7F7A6;">TODO:</mark> Again, not the most interesting component, leaving out for now.