# MultEGraph

Can we design an e-graph like data structure capable of storing multiple relations compactly?

>[!info] Open Questions
>- [ ] What does a property set look like? What does the property set -> id map look like?
>- [ ] Whats the right way to represent membership in a property set sat class?
>- [ ] What's the right way to identify "extractable" operators? 

## Traditional E-Graph Review
A typical e-graph stores an equivalence relation over terms of some language. It consists of:
- e-nodes -> representation of a single term as an operator and a list of arguments
- e-class -> A group of equivalent e-nodes
- argument edges -> an e-node takes as its arguments e-classes (all edges come *from* e-nodes and go *to* e-classes)
- union find -> stores the e-class equivalence relation over e-class IDs
- Hashcons -> maps terms to their e-class IDs
- map -> some mapping from ID to actual e-class/e-node structs

When using an e-graph for equality saturation or optimization problems, we insert an initial term into the e-graph and then repeatedly apply rewrite rules as follows:
- e-match -> given the left-hand side of a rewrite rule, search the e-graph for e-classes with a matching e-node term and a substitution mapping
- merge -> if we find a match, insert the right-hand side of the rule using the substitution map and insert it into the e-graph (either lookup an existing e-class containing it or create a new single e-class). Merge the e-classes and add to the repair list. 
- repair -> (either immediately or defer) Maintain the congruence invariant by checking and merging (if needed) all parents of the classes in the repair list

Eventually, we will stop (timeout) or the e-graph will be saturated. At this point, we can apply a cost function to extract the best term (optimization) or search for another term (equality comparison) in the graph.

## Limitations of the traditional E-graph and motivations
The traditional e-graph has two major limitations:
1. It is only capable of storing a single equivalence relation. 
2. The equivalence relation is entirely induced by the rewrite rules. 

What if we wanted to store something more complex? For example, in query optimization, we often distinguish between logical and physical equivalence. Logical equivalence says that two queries are equivalent if they produce the same set of tuples. On the other hand, queries are physically equivalent if they are both logically equivalent and have matching physical properties (such as sortedness). 

The cascades approach is to unify the logical operators, physical operators, and enforcers (special operators that apply physical properties) into a single class of abstract operators. Implementing this in an e-graph tends toward producing a graph with many very similar e-classes that are all logically equivalent but distinguished by physical properties. While the approach works, introducing the zero-cost, non-operators of enforcers and ballooning the e-graph out seem suboptimal. I think we can do better. 

## First Pass at a Multe-graph

I propose a new e-graph like structure capable of holding a single equivalence relation and N additional sub-equivalences efficiently. The basic idea is this: the e-graph will hold the logical equivalence relation and since physical equivalence requires logical equivalence, each logical e-class will also contain N physical equivalence classes representing equivalence under some set of property constraints.

### Definitions and Terminology

> [!info] A note on logical vs. physical
> In query optimizer land, we often use logical and physical to distinguish between relational algebra operators (e.g. join) and implementations of those operators (NL Join vs. HashMerge join vs. etc.). We even categorize rules as logical-logical vs. logical-physical.
> 
> I would argue that we need not distinguish between logical and physical operators. In fact, once we define an expression as an operator and set of properties, these are physical operators and logical operators are simply those with an empty property set.

#### Language
Suppose we have a language $L=(O, P)$ consisting of a set of operators $O$ (terminals are operators with 0 arguments) and a set of properties $P$. 

##### Operators and Properties
Operators name functions while properties present a set of potential attributes for expressions to have. In traditional e-graphs, operators are simply a named function with a fixed arity. Now, we expand this definition to include properties as well. For each argument, operators may (or may not) specify property requirements. Additionally, each operator may assign properties to its outputs.

For example, a merge join might have been represented by `MergeJoin(table1, table2, index)`. Now, we can include the sortedness requirements on table1 and 2 and maintain the knowledge that our output will also be sorted on this index: `MergeJoin(table1 [sorted: [index]], table2 [sorted: [index]], index) [sorted: [index]]`. As long as all property names are defined in the set of properties of the language and the values of those properties are bound in the definition expression (or constants), we can check this at compile time.

In operator definitions, properties only refer to static requirements of arguments and what we know about results. That is, properties here expand our knowledge about the operator without needing to introduce enforcer operators and encode these requirements in the rules. It is possible that at runtime, an instance of that operator and/or its arguments will have more properties than are strictly required/applied by definition. 

##### Expressions
An expression in $L$ is recursively defined as an operator, list of arguments, and set of known properties where the list of arguments is either empty of a list of other expressions and the set of properties associated with executing that operator over those arguments (essentially, it is what we *have* not what we *want/require*). An expression $x=o(\{(e_i, p_i) | i=1\ldots n\}, P')$:
- Operator $o$ 
- Known Properties $P'$ -> set of properties belonging to the "output" of the expression
- Arguments -> A set of $(e_i, p_i)$ tuples where $e_i$ is an expression in the language and $p_i$ is a set of property requirements on the $i$th argument.

*Note: The known properties and argument properties are derived from and equivalent to the definition of $o$, this representation just helps keep them in mind rather than buried within the operator.*

The logical representation of an expression $x$ is given by $x=o(e_1, e_2, \ldots, e_n)$.
*Note: An expression is always "logical" if $\emptyset =p_i=P$  $\forall i$*.

##### Rules
A rule is an equivalence mapping between logical representations of expressions. ***Rules do not consider properties beyond what is defined by operators***. That is, rules cannot specify conditions related to potential properties or modify an operators properties.

#### Equivalence
> [!warn] Renaming
> Right now I use the terms logical and physical equivalence but what I really mean is a parent equivalence relation and a child

##### Logical Equivalence / Operational Equivalence
Two expressions are logically equivalent if we have a rewrite rule mapping them to each other, or they have matching operators and all arguments are logically equivalent.

##### Physical Equivalence / Property Equivalence
Two rules are physically equivalent modulo property set $P'$ if they are logically equivalent and their known property sets both contain $P'$.

#### E-graph
The new variant on an e-graph: (new info in bold)
- e-nodes -> an expression in the language $L$
	- **Property Sat Store** -> (spit balling here) A bitmap indicating which property requirements this enode/expression satisfies
- **e-classes**
	- **parent e-class** -> A group of logically equivalent e-nodes (which belong to possible many overlapping child e-classes) defined by a single ID
	- **child e-class** -> a group of logically and physically equivalent e-nodes contained in a parent e-class defined by an ID tuple: (parent ID, property equivalence ID). The id (parent ID, 0) refers to the entire logical parent e-class.
	- *Note: this is a conceptual understanding of e-classes an does not reflect how the actual e-class struct is stored. In fact, we still only store the logical e-class and derive the physical e-classes within it using the property sat map in its enodes.*
- **argument edges** -> an e-node takes as its arguments a child e-class id tuple
- union find -> stores the parent e-class equivalence relation
- Hashcons -> maps terms to their parent e-class IDs
- eclass-node map -> some mapping from ID to actual e-class/e-node structs
- **Property Map** -> Map from property requirement sets to IDs
	- Ideally we want this to be 2-way

Essentially, all e-classes are now identified by a `(logical ID, physical ID)` tuple. The "over-arching" logical class is the class referred to by `(logical, 0)`. 

##### E-matching
To understand how e-matching works, we need to address how we construct terms from expressions. In a traditional e-graph, terms are either terminal operators or expressions with an operator where each argument expression has been replaced with an the e-class ID of that expression. In a multe-graph terms are constructed similarly, but now we replace arguments with an ID tuple corresponding to the logical e-class of the argument and the property set class of the arguments requirements. Naturally, if an argument has no property requirements we use (logical, 0).

Given an expression, we use its logical representation to do a lookup for logical ID in the hashcons and its property set to lookup its property ID in the property map.

Now, given an expression and an e-class ID `x` to start searching in, we:
1. **Match on operator** -> Begin searching all the e-nodes in `(x, 0)` for a match to our expressions operator and arity (regular e-matching).
2. **Match arguments** -> If we find a matching e-node, e-match on all of its arguments to get a substitution map
3. **Match on properties** -> If all arguments have matched successfully, construct the property requirements for our operator definition using the substitution for bindings that might depend on arguments. 
	1. Lookup the property set ID -> if it already exists, then we can lookup if the e-node belongs to it since bitmaps should always be up-to-date before the reading phase
	2. If it does not exist, insert it into the property map (this does not affect any nodes/classes). Add a task to the worklist to indicate we have a new property class. Use the containment operator to determine if our e-node satisfies the property requirements (this is still a read operation so it is safe).

If all three stages of matching succeed, then we have found a match and a set of substitutions. 

##### Merging
Merging is actually quite simple. Rules provide logical equivalences and tell us how to merge logical classes, so we handle merging exactly as we would in a traditional e-graph and marge only logical classes/IDs. Once we have merged two logical/parent e-classes, recanonicalization proceeds as normal, updating the logical ID in any potentially affected ID tuples.

##### Repair
In the multe-graph, we must repair invariants as with the regular e-graph. In addition to repairing logical e-classes as normal, we can also use the repair process to update the physical property classes.