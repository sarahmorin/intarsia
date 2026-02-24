> Warning: OUTDATED - ignore me until further notice

# Implementing Cascades-style Search with ISLE over an e-graph

The basic flow of cascades is:

1. I want to optimize an expression group, so I need to optimize every expression in that group
2. Before I can do that, I need to:
   1. optimize it's children (go to 1 on the argument expressions)
   2. Explore its logical equivalences
3. To explore the logical equivalences of a group, I need to explore the logical equivalences of each expression in that group
3. To explore an expression I need to explore each of its argument groups, then I can explore rules wrt that expression
4. For every matching logical rule, I call apply rule to insert the equivalent term to the current group
   1. This might create an unexplored expression, so I make sure to handle that case by calling explore expression on any new expression I insert
5. Once I've explored the group, I can optimize the group by optimizing each expression in the group
6. To optimize an expression, I find relevant physical rules and call apply rule to insert the physical expression to the group
   1. Since physical expression never appear on the left-hand side of my rewrite rules, I don't need to do any additional bookkeeping

Since physical "optimize" rules won't affect which logical "explore" rules match, it is acceptable to begin exploring a parent while you are optimizing its children.
Another way to describe cascades is with dependencies:

1. A group is optimized iff the group is explored and every expression in the group has been optimized
2. A group is explored iff every expression in the group is explored
3. An expression is explored if all its children (i.e. argument groups) are explored and all matching logical rules have been applied to it
4. An expression is optimized if all its children have been optimized and for each child group, we have added the best argument that meets our property requirements to the "winner's circle"

So the question is, how do we implement a cascade style search with ISLE rewrite rules?

- A single entry point to the rewrite rules that does not recurse (we never call the set of rewrite rules from within a rewrite rule)
- A runner to managing merging expressions we generated as a result of applying rules and repairing the e-graph
- Extractors that initiate optimization of arguments
- Constructors that initiate optimization of the newly constructed "top-level/parent" terms
- A record of which classes and expressions have already been explored and optimized
- (Optional) if we want it to be "real" cascades two entry points - one for logical one for physical

The `.isle` code will look something like this:

```lisp
;; TYPES
;; Id type corresponds to E-Node and E-Class Ids
(type Id (primitive Id))

;; OPERATORS
;; For every operator <op> we declare a term like (decl multi op_name (<Id per argument>) Id)
;; Which says op_name(Id, Id, ...) -> Id (the usual e-graph term representation)
;; The `multi` keyword tells us that this term should have multi constructors and extractors
;; For our purposes, we really only need the multi extractor - which says that when we try to extract
;; an expression from an Id (which represents a *class* of expressions) we might get multiple results
;; For example, and addition operator would look like
(decl multi add (Id Id) Id)
;; We also define an external extractor and constructor
;; The extractor takes an Id and tries to unwrap it into an add(Id Id) term
;; The constructor wraps two argument Ids in an add operator and returns the Id of the add term
(extern extractor add ext_add)
(extern constructor add con_add)
...

;; CONTROL FLOW WRAPPERS
;; We have two kinds of rules: logical-logical and logical-physical
;; For each type of rule, we have a different entry point that allows us to call every rule of that type on a given term all at once.
;; Notice, we don't define extern extractors or constructors for these terms.
;; Instead these functions will be generated for us by ISLE using the rules we specify.

;; Explore -> Entry point for logical-logical rules
(decl multi explore (Id) Id)

;; Optimize -> Entry point for logical-physical rules
(decl multi optimize (Id) Id)

;; RULE DEFINITIONS
;; Every rule maps from one expression to another, using one of our two entrypoints above.
;; Importantly, the `explore` and `optimize` terms only ever appear at the outer-level of the head of a rule
;; They never appear in rule bodies or in the head multiple times
;; For example:
(rule 
    (explore (add x y))
    (add y x)
)

(rule 
    (optimize (add x 0))
    x
)

...
```

You might notice a few things that feel strange:

1. We never used run anywhere...?
2. It doesn't really seem like explore and optimize will go deeper than one level

And that's by design! Explore and optimize simple function as "apply rule" and the control flow that decides when to apply them exists entirely in run and the operators extractors and constructors.


Run applies logical rules then physical rules.
```
run(Id) -> Id:
    // Apply logical rules
    ids = explore(Id)
    merge_and_repair(ids)

    // Apply physical rules
    ids = optimize(Id)
    merge_and_repair(ids)

    return canonical(id)
```
*cascades doesn't maintain the congruence invariant so we could theoretically skip the merge and repair calls*

When we call into explore and optimize, we use our extractors to find term matches, these extractors get us one level deeper and allow us to kick off a dependency.
```
ext_op(Id) -> Option<Vec<Id>>:
    // Lookup all matching terms in the given e-class
    matches = egraph.lookup(Id).match(op)

    // NOTE: This is where we can prune and bound with a globally defined cost function

    // For every match, call run
    for mid in matches:
        run(mid)
```
Notice, we call run before returning from the extractor, so we will explore and optimize the arguments before we continue exploring or apply matches at the parent level. This creates the dependency we need between parents and arguments.

```
con_op(args: Vec<Id>) -> Id:
    // Construct the new term and insert it into the e-graph
    id = egraph.insert(op(args))

    // If we created a new Id, we call run on the new term
    if id.was_new():
        run(id)
    
    // Return the ID to be merged by the calling run()
    return id
```
This ensures we don't mistakenly consider a group explored or optimized without optimizing new terms that were added during the exploration. Essentially, this creates the dependencies we need between groups and their terms.

## Conclusions

1. Is this the end goal? No, obviously this is a strange way to use ISLE that, frankly, barely uses ISLE. The only time we use ISLE generated code is for rule matching, but efficiently checking every rule against one e-node is still a win and probably faster than boring old e-matching.
2. Is cascades a perfect search? Also no. This isn't meant to be "how you should do a search with ISLE over an e-graph" its meant to be "look you can do cascades". It also reveals how and where we might be able to change the search strategy without getting the search tangled in the rules, which should really just define the equivalences.
3. The pseudocode doesn't include any of the optimizations we will want to add to avoid redundant work. Only calling run on new ids is a start, but we can probably do better.
4. Where do the properties go? I'm not sure. I think one option is to do a manual implementation of 1 or 2 properties in the Rust code and (separate from both isle code and the e-graph) to see what the challenges are. This will be super slow, but might help inform the real implementation.