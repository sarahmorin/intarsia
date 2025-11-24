# MulteOpt (until I come up with a better name) Notes

For now, this readme just contains my dev notes on the repo. In the future I'll put these elsewhere and make a pretty readme that explains the crate.

## On Generic-ness

In meeting on 8/27 we highlighted that I was using too many traits and generics. 

### Changes I've made since that meeting:
  - `AST` renamed to `OpLang` for readability
  - `Property` renamed to `PropertySet` for understanding
  - Eliminated `PropLang` -> unnecessary
  - Eliminated `Analysis` (for now) -> making use of Egraph analysis might be interesting in the future, but for now I'm going to omit it to simplify things.
  - Moved the original, basic egraph implementation to its own repo so now `types` doesn't need to support both implementations and can be designed solely with the multegraph in mind
  - Made 3 distinct Id type structs:
    - `Id` for "logical Ids" of enodes and the larger e-classes
    - `PropSetId` for property set Ids (helps avoid mistakenly using a `PropSetId` as an `Id` or vice versa)
    - `MulteId` which is a (`Id`, `PropSetId`) tuple
    - ~~Unified `Id` and `MulteId` into a single `Id` enum and included `PropId` to make clear distinction between `Id`s that only correspond to a Property Set (i.e. outside the context of eclasses)~~
    - Updated unionfind to take `usize` only since `Id` is no longer just a type alias (otherwise things get wonky) and the egraph handles unwrapping `Id`s correctly to use in the unionfind
  - No more `Term` and `MulteTerm`, now `Term` refers to what `MulteTerm` was since this is only a multegraph
  - Made `Subst` less generic, now it is always just `Var` to `MulteId`
  - Eliminated `OpInfo` entirely -> How do we get this info now?
    - arity comes from the `OpLang` trait (each operator should have a specific, fixed arity (at least for now))
    - We provide functions using the function above to specifically get the `PropertySet` for output or a given input
  - Added `PropInfo` to hold the set of functions we need to map from expr/term/pattern to propertyset
    - defines relationship between a property set and an oplang
    - details on computing property sets below
  - `PropertyMap` Simplified to wrapper struct on `BiMap`
    - [ ] Eventually I would like PropertyMap to be a data structure that maps property sets to a congruent, but easier to represent partially ordered set 

### Places where I think the trait/generic is worthwhile:
  - `OpLang` -> The language of operators you want to use, I think this is a nonnegotiable generic when used in things like `Expr<L>` but I can see the argument that the trait is too much.
    - I like using a trait to define functionality that we might need to require down the line, e.g. `is_extractable()` allows us to unify source and target languages into one set of operators but keep a map to distinguish between later on. Maybe that's enough to justify the trait though.
  - `PropertySet` (formerly `Property`) -> The set of properties associated with a term in the language. We don't know what properties will need to look like. The example we use now is a simple bitmap, but its not a huge leap to see that you might need a more complex struct. We need the trait to make sure a `bottom` element is provided.
  - *There is an argument to be made here that `OpLang` and `PropertySet` should be unified into a single trait. I kept them separate so as not to disrupt my existing basic egraph implementation (which I will need later for comparisons) but it's worth considering making that code more separate.*
  - `PropInfo<L, P>` is a struct which holds the functions needed to map expressions and patterns to property set information. A programmer must define these functions to instantiate the struct and pass that struct to the multegraph constructor. It defines a relationship between an lang and a property set and tells us how they are related without being a confusing trait to carry around.
    - Current functions are expr to prop set (props it has), expr to props of arg (props it wants for arg at index x), and those two for patterns
    - expression functions must return something (they also need to be deterministic)
    - pattern functions are fallible, since we might depend on arguments that are variables to generate property requirements
      - **What do we do during matching when this happens?**
    - Do we also need something that tells us what argument indices feed into property set computation? could be useful during search/replace
  - `Parseable` trait -> If you implement this single function trait on your `OpLang`, I can build a parser (passable right now, but could be improved) that allows you to use the rules macro.
    - [ ] Should this just be baked into the `OpLang` trait? Maybe, but if you don't want to write a parser there's also nothing stopping you from writing rules directly in rust and forgoing the macro

## Enode Insertions wrt Properties

In the mutle-graph we cannot use the same method of enode insertion as the traditional e-graph; we need to handle properties. 

#### Traditional Insertion of Expression
```python
insert(expr) -> id:
  arg_ids = [
    for arg in expr:
      arg_id = insert(arg)
      arg_id
  ]
  term = Term(expr.op, [arg_ids])
  canonicalize(term)
  if hashcons[term] = id:
    return id
  else:
    new_id = unionfind.add_set()
    enode = ENode(new_id, term)
    eclass = EClass(new_id)
    eclass.add_enode(enode)
    enode_map.insert(enode)
    eclass_map.insert(eclass)
    hashcons[term] = new_id
    for child in term.args:
      eclass_map[child].add_parent(new_id)
    return new_id
``` 

#### Adding an expression in the mutle-graph
```python
insert(expr) -> id:
  arg_ids = [
    for idx, arg in expr:
      arg_id = insert(arg)
      # Get the property requirements and convert to id
      prop_req = get_arg_req_props(expr, idx)
      prop_req_id = prop_map.get_or_insert(prop_req)
      # Return multe id
      (arg_id, prop_req_id)
  ]
  term = Term(expr.op, [arg_ids])
  canonicalize(term)
  if hashcons[term] = id:
    return id
  else:
    new_id = unionfind.add_set()
    # Compute the output properties of the expression
    props = get_output_props(expr)
    props_id = prop_map.get_or_insert(props)
    # Include properties in the enode metadata
    enode = ENode(new_id, term, props)
    eclass = EClass(new_id)
    eclass.add_enode(enode)
    enode_map.insert(enode)
    eclass_map.insert(eclass)
    hashcons[term] = new_id
    for child in term.args:
      eclass_map[child].add_parent(new_id)
    return new_id
```

#### Handling Properties when inserting pattern matches

Given a pattern and substitution we need to compute the property requirements of the pattern's argument. We typically follow the same recursive insertion model with one change: ***if an argument is a variable, we simply use the eclass Id from the substitution map rather than doing the recursive call.***

In the mutlegraph, we need to handle the case where one of the property-deriving arguments is a variable in the pattern.
If the pattern has no variables, or none of the property-deriving arguments at any level are variables, then we can call the get property functions above and treat the pattern exactly like an expression because those functions will not touch a variable. 
Thus, we can proceed with the usual algorithm described above.
Now, since the property functions respect the logical equivalence, anytime a property-deriving argument is a variable, we can simply substitute the canonical element of that eclass.
The question now is when to do this substitution? I think the issue boils down to how much information the property functions require. 
Since patterns/expressions/terms are recursive, the prop functions will "recurse" through the expr to get the property deriving arguments. 
Suppose this recursion is unbounded, i.e. the prop function for expresions with operator `g` not only depends on argument 0, but also argument 0's argument 0, e.g. `g(f(a(...)))` and `g(f(b(...)))` have different properties where `f(a(...))` and `f(b(...))` are not equivalent. Now, the property-deriving arguments are no longer a list of indices, but a recursive structure itself.
Keep in mind, this still does not mean the property functions are recursive, they cannot depend on themselves, they just might recurse through an expression (without computing any properties) to compute the property set from that argument. 
The most sensible starting point is to say 



Options:
1. Introduce new get_props function that take in pattern, substitutions, and the mutlegraph and do the necessary substitutions here
   - requires another set of property deriving functions (potential inconsistency)
   - might do redundant work
   - supports property functions that require reading multiple levels of an argument (for example, distinguishes between f(a) and f(b))
2. Before the first call to insert match, recurse through the entire pattern and perform the necessary substitutions.
  - pattern is stable during insert match, we only do substitutions once
  - if prop functions recurse through multiple pattern/expression levels, we need to carry these argument indices down somehow 
3. 

## Property Sets and Correctness Requirements

On it's own, a `PropertySet` (from here on out referred to as PS for short) is just a structure that satisfies a partial order and provides a bottom element. Pretty straightforward.
Now, when we relate a PS to an `OpLang` (aka language or operator in the language) we need a set of functions that provide the mapping from expression in the language to PS and those functions need to satisfy some specific properties.
I don't believe I've clearly outlined these anywhere yet.

>  ### TL;DR after chat w/ Max
> 
> The arguments of any operator can be partitioned into 2 sets:
>   - **Property Factors** (idk I want a name for these groups) -> those that we use to derive PSs (and have no PS requirements themselves)
>   - **Property Dependents** ->  those that do not inform PS values and might have PS requirements
>
> For example: the arguments of `MergeJoin(A, B, x)` can be divided into {`x`} (the arguments that inform PSs) and {`A`, `B`} (the set of arguments that do not, and in this case do in fact have PS requirements)
>
> Now, since we are working with recursive expressions and arguments are just more expressions themselves, it is entirely possible that a Property Factor will belong to an eclass with multiple nodes. To handle these cases, we propose that the functions mapping expressions/terms/patterns (or more specifically operators and their property factors) to PSs must respect the logical (coarsest grained) equivalence relation. 
>
> For example, suppose the expression above has already been inserted to the egraph and we now have a term with hashconsed Id's as arguments: `MergeJoin(10, 20, 30)`. Then, the PS of the expression and PS requirements on any of the property dependents are determined by the property factors which now point to an eclass rather than a single expression. It should hold that we can derive these PSs from any element of eclass 30.
>
> #### Is this approach sound?
> For now, I think yes. The "logical" equivalence classes represent sets of expressions that correspond to the same information (even if that information might differ in properties like sortedness or source); it feels natural (and inline with e-graphs) that a function would respect that equivalence and produce the same output for all elements of an equivalent set (all else fixed.)
>
> #### What does this get us?
> - **No need to "bubble up" changes to property sets during repair/rebuild.** Suppose we add an enode to a class that serves a property factor for one of its parents. Since the PS computation respects equivalence, this addition does not affect the PS of any parents.
> - **Handling Patterns is almost trivial.** Suppose we have matched a pattern and are inserting the replacement, but one or more of the property factor arguments are just variables pointing to entire eclasses. Now we can just pick any element in that class to compute the PS of the inserted term.
>
> #### Open Questions:
> - [ ] Do we actually need to partition arguments like this, or can we infer that info?
>   - Not sure. This ties in to a larger question about how we will require programmers to define languages and PS computation functions.
> - [ ] Can property factor arguments have properties?
>   - I think yes...? They can't be used for anything in the PS computation, but they can exist.
> - [ ] Can property factor arguments have PS requirements?
>   - I think no. This can create a weird chain of dependencies when computing PSs.
> - [ ] Does this requirement on property factors limit us in any detrimental way? Is there any reasonable case where the property set computation might not want to respect the logical equivalence relation?

### MulteGraph Formalism

In the context of a language, a property either represents the concrete set of properties an expression *has* or the minimum set of properties an expression *needs to satisfy*.

#### Expressions and Properties
- Let $O$ be a set of function symbols and constants, or *operators* that define some language.
- Let an expression $x$ be given by $f\in O$ or recursively defined $f(x_1, x_2, \ldots, x_n)$ and let $L$ be the infinite set of all such expressions $x$.
- Let $P_L$ represent the set of all possible property sets derived from expressions $x\in L$.
  - Let $\bot\in P_L$ denote the bottom element of the set of property sets. That is, $\forall p\in P_L$ we have $\bot\le p$.
- Let $V$ be a set of variables and let a pattern $r$ be given by $f\in O\cup V$ or recursively defined $f(r_1, r_2, \ldots, r_m)$.
  - Let $L_V$ be the infinite set of all such patterns.
- For an expression $x = f(a_1, a_2, \ldots, a_n)\in L$, let $p_x\in P$ denote the property set of expression $x$ and let $p_{x_i}\in P$ denote the property set requirements of argument $a_i$ in expression $x$.
  - Note: We are dealing with two uses of property set here. The *property set of expression x* refers to the properties that expression *has*. The *property set requirements of an argument in an expression* refer to the minimum properties that argument must have to be an argument at that index to the expression. In the case of PS requirements, it is possible that the properties an argument are greater than the properties required.
- We define functions to map from expressions to property sets. 
  - $F: L\to P$ maps expressions to the property set they have. For an expression $x\in L$, $F(x) = p_x$.
  - $G: (L\times \N)\to P$ maps expressions to the property set requirements of their arguments. Thus $G(x, i) = p_{x_i}$.
  - $\lambda: L\times\N \to \{0, 1\}$ Indicates if the argument at index $i$ is a property factor ($\lambda(x, i)=0$) or dependent ($\lambda(x, i)=1$) 
  - [ ] TODO: alternate version of these are functions that take an operator and set of property factors, is that better?
- Given an expression $x \in L$, a suitable argument $a_i$ is any expression $y\in L$ with $G(x, i) = p_{x_i} \le p_y = F(y)$, that is the properties $y$ has contain the properties required by $x$ or argument $i$. We say $y$ ***satisfies*** $p_{x_i}$.

#### MulteGraph
- The MulteGraph deals with 3 types of identifiers:
  - Let $N_L$ be a set of logical ids
  - Let $N_P$ be a set of property ids
  - Let $M$ be a set of *MulteIds* given by $N_L\times N_P$
- Terms and Enodes
  - A *term* is a representation of an expression in the graph where arguments are replaced with *MulteIds* corresponding to the class where the argument resides. 
    - Let $T$ be the infinite set of terms with operators in $O$ and arguments in $M$. 
  - An *enode* is a structure holding a *term* and some metadata (typically used for performance gains)
  - *A Note on the Distinction*: I like referring to terms (the representation of an expression in the egraph) and enodes (the struct that stores the term and perhaps some other information) separately, but generally they are treated as one idea.
- EClasses
  - A *logical eclass* (typically referred to as just an *eclass*) is a group of enodes. These eclasses are stored as physical structs.
  - A *property-based eclass* (aka a *sub-eclass*) is a subset of an eclass such that all enode members satisfy some property set requirement. These are represented virtually over the physical eclass.
- Both enodes and eclasses have logical ids. Enodes all have unique logical ids while and eclasses id is the id of its canonical enode.
- Let $U$ be the unionfind storing the logical equivalence relation over ids in $N_L$
- Let $H_L$ be a hashcons $H: T\to N_L$
- Let $E_c$ and $E_n$ be maps from $N_L$ to eclass and enode structs respectively.
- Let $H_P$ be a bimap for property set ids $H_P:P\leftrightarrow N_P$

The multegraph also maintains some common egraph operations/definitions/invariants (bulleted but not explained for brevity):
- Canonicalization 
- Representation of term
- Equivalence -> the typical notion of equivalence refers to equivalence of eclasses not subeclasses.
- Congruence
- Congruence and Hashcons invariants

#### Adding Terms and Computing Properties in the MulteGraph

Like a traditional egraph the multegraph does not store expressions, but terms within enodes. When we insert information into the multegraph, we do one of two operations:
- Insert an expression
- Insert a pattern match and a corresponding substitution map relating unknown variables to eclasses

We insert expressions recursively

**Base case**: An expression $x$ with no arguments
- Since the expression has no arguments, it is already in its term form.
- We use $H_L[x]$ to get the logical id of the term if it already exists in the hashcons. If it does, we simply return the Id.
- If not, we create a new singleton eclass in $U$ with id $i\in N_L$
- Use $F(x)$ to get the properties of the expression, $p_x$
- Use $H_P[p_x]$ to get the property id $j\in N_P$ of $x$
- Create a new enode in the new class with id $i$ and property id $j$ in its struct data (we can also add the property set directly if it is reasonably small to store, otherwise we use ids to save space)
- Add the enode and eclass to the struct maps $E_n$ and $E_c$ respectively.
- Return $i$

**Recursion:** An expression $x$ with one or more arguments.
- Call this function on each argument expression and collect their logical ids $\{l_1, l_2, \ldots, l_n\}$
- For every argument index $i$, construct a multeid as follows:
  - If $\lambda(x, i) == 0$, use $(l_i, 0)$
  - If $\lambda(x, i) == 1$, use $(l_i, G(x, i))$
- Construct the term representation of $x$ by replacing each argument with its corresponding multeid. For example, $f(a_1, a_2, \ldots, a_n)$ becomes $f((l_1, p_{x_1}|0), (l_2, p_{x_2}|0), \ldots, (l_n, p_{x_n}|0))$.
- At this point, we lookup the term with $H_L$ and return its id or create a new enode as above.

Inserting a pattern and substitution follows the same process with one notable exception: whenever an argument is a variable, we don't recurse. Instead, we lookup the variable in the substitution map to get the eclass id that would have been returned by a recursive call. 

With the introduction of properties, we also need to handle the case where a variable argument is used as a property factor by the parent term. In this case, we rely on the fact that $F$ and $G$ respect the logical equivalence relation maintained by $U$ (see detailed discussion below). After looking up the eclass in the substitution map, select any element from that eclass to use as the property factor expression. If there are multiple such arguments, do this for all of them. With a representative for each property factor, we can compute $G$ for all property dependent arguments and construct multeids of each argument as above.


#### Guarantees and Attributes

- [ ] TODO: I want to formart this section better, for now its a braindump

1. Logical eclasses are never empty.
2. Subclasses may be empty.
3. $F$ and $G$ are not recursive, nor do they depend on one another. (i.e. properties don't depend on properties)
4. $F$ and $G$ respect the logical equivalence relation maintained by $U$, particularly when it comes to property factors. Said another way, if $F(x)$ depends on an argument of $a_i$ of $x$ and the term representation of $x$ has $a_i$ pointing to an eclass $E$, $F(x)$ should produce the same result for any element of $E$.

# Development Plan (very tentative)

General outline:
1. **Finish the basic multe-graph implementation** -> This will more or less mimic a traditional e-graph, but simply allow the use of properties (to little benefit)
   1. Decide on and finish PR implementation
   2. Resolve recent changes in rules, parser, testlang, regular e-graph etc.
   3. Implement extraction and define cost function requirements (should be pretty basic) for both
   4. Get a simple test going in the testlang with rules, simple PS, and a hardcoded cost function.
2. **Define interfaces for a programmable search** -> Replace traditional e-matching with a collection of functions and structs to run a very fine grained search strategy
   1. Decide on what operations we can break e-matching into and how granular to go (also how much info to expose to caller)
   2. Decide on a queue/task structure for search operations (ala cascades operations, but also combine with e-graph rebuild/repair workload)
   3. Decide on guardrails (if any) to prevent absolute chaos from ensuing (e.g. enforcing read/write phase orders)
   4. Define the search/executor (single/multi thread) that will pop things off the worklist and do them and add more
3. **Implement Custom Search Interfaces** -> Build the damn thing.
   1. TBD -> once we know design, can chunk up implementation work
4. **Build Running Example of a Custom Search** -> Cascades, Bottom-Up, Random?
   1. Using everything built so far, build a working example of a searcher that runs the cascades style top-down search, but uses properties instead of enforcers
   2. Also, try to do a bottom up query optimizer search
   3. Also, maybe we build one of the random optimizers just for funsies
   4. Honestly, this might take way more time than I think so we might need to adjust timeline here
5. **Testing** -> At this point I might want to do some basic perf testing to see if this is moving in a useful direction
6. **Revisit system diagram and adjust if needed** -> After doing a good chunk of work, check back in about the high level design
   1. How do we feel about the difference between "programming and optimizer" and "running that optimizer on input"
   2. Do we need stronger or weaker requirements anywhere?
   3. Is there anything lacking in the pull information from outside source interface? Right now its pretty open ended, should it be more defined?
7. **Revisit Rules/Parsing/Language Definition** -> Right now language parsing and rules is pretty basic, think about making it more elegant and programmer friendly
   1. decide on syntax for rules, language, and property definitions
   2. perhaps provide a way to distinguish between a source and target language (and under the hood we unify to one set of operators with "extractable" flags)
   3. macro to generate parsers for custom languages
   4. Also seriously consider ISLE style rewrites
8. **Build Actual SQL parsing** -> Move on from dummy test lang to a usable SQL parser and IR
   1. (Optionally) also add a hydro lang representation
9.  **Work on Code Gen** -> Build up the "what happens after extraction" part

## Decisions I need to make

- [x] Keep regular e-graph here or move to its own repo?
  - its on its own now
- [x] PS requirements
  - See above



---

# Archive Notes

Things I think are likely out of date but I haven't sorted through them yet to see if theres info worth keeping. For now, my old reasoning lives on just in case.


#### Expressions

Mapping from an expression to a property set can be tricky depending on language complexity. Some examples:
- In the simplest case a property set might be a single boolean flag determined by the operator. Then, $F$ and $G$ need not consider the values or arguments in an expression, only the operator itself. Because variables can only replace entire expressions and not an operator within an expression, this also means $F'$ and $G'$ are simple.
- To make things slightly more complex, the property set might be determined by the operator and one of its arguments. In an expression the argument is well formed, so we can still implement $F$ and $G$ relatively easily; however, $F'$ and $G'$ are now more likely to fail and will likely map to $\^\bot$ whenever that argument is a variable in the pattern.
- Now, imagine the property set if a more complex structure and it depends on several arguments that vary from operator to operator. We can write $F$ and $G$ so long as we can reasonable determine the values of the arguments needed to determine properties (i.e. ideally we shouldn't depend on values that are buried a few levels below in the expression.)

So, how do we ensure that mapping from an expression to a property set is possible?

The safest conditions to guarantee we can efficiently and correctly compute property sets with $F$ and $G$ are:
- **Correctness** -> $F$ and $G$ are a) not recursive, and b) do not depend on one another; i.e. $F$ and $G$ can depend on the expressions operator and its arguments *values* but it cannot depend on the properties of the arguments.
  - Without these conditions it is possible to generate a cycle of dependencies, potentially making it impossible to derive a property set.
- **Performance** -> It should be reasonably efficient (minimally recursive) to retrieve the argument values $F$ and $G$ depend on. 
  - Since expressions are recursive, even constants are considered an expression which we then unwrap to get the value.
  - Ideally, $F$ and $G$ depend on argument values that are only 1 expression level away. Burrowing into many levels of expression is expensive and requires a good amount of checking on expression form.

> Now, there is an argument to relax the correctness condition on $G$ in the following way: $F$ cannot be recursive or depend on $G$, $G$ cannot be recursive but it *may depend on $F$*. 
>
> This still prevents dependency cycles, but it opens the floodgates for very poor performance.

#### Terms

Terms are expressions in the multe-graph where all arguments are other e-classes. In general, we generate terms from expressions and since property sets can be generated for expressions, we could simply convert the expression to a term and then map all the property sets to the term as they were in the expression with the MulteId. For example, suppose we have the expression `MergeJoin(A, B, x)` and we want to insert it to an empty graph. In the traditional e-graph world, we can recursively compute eclass Ids for arguments, so we might end up with something like:

| Expr                 | Id   |
| :------------------- | :--- |
| `A`                  | 1    |
| `B`                  | 2    |
| `x`                  | 3    |
| `MergeJoin(1, 2, 3)` | 4    |

Now supposed we also know the property requirement "sorted by `x`" (probably a bitmap) is referred to with property set Id `10`. Now, we can impose property requirements in the Term using a MutleId (logical, property) like so:

| Expr                                  | Id   |
| :------------------------------------ | :--- |
| `A`                                   | 1    |
| `B`                                   | 2    |
| `x`                                   | 3    |
| `MergeJoin((1, 10), (2, 10), (3, 0))` | 4    |

So we know that the arguments that were originally `A` and `B` must be sorted by `x` even after we have replaced the expression's arguments with eclass Ids.

For the most part, this seems reasonable, but what happens if we insert something equivalent to `x` to eclass 3? 

To me, there are 2 sensible options:
1. We assume (and impose) that $F$ and $G$ do not violate congruence of elements of e-classes used to derive properties. For example, consider the `MergeJoin(A, B, x)` example. $G$ depends on the 3rd argument, in this case `x`, to compute the property requirements of arguments 1 and 2. We could impose that `G(MergeJoin(A, B, x), 1) = G(MergeJoin(A, B, y) 1)` for all `x` and `y` in the same logical e-class.
   - This is kind of wild. I need to think a lot more on this. It almost feels right...
   - Essentially, can we compute a property set from an e-class as opposed to a single node? Or, can we make that computation easier by selecting any element from the class and relying on an equivalence with respect to property set derivation?
   - This makes handling variables in patterns much easier.
2. Every time we add an element to an e-class used to derive a property set, we handle the case that its parents might need to be duplicated with alternate property set requirements in rebuild/repairing.
   - This feels very "e-graphy" 
   - Could result in ballooning e-classes. 
   - Much less restrictive, but doesn't handle the pattern variable issue.

#### Patterns

Ok...patterns. First things first, we only handle patterns during a search. Everything we store in the mutle-graph and anything we might cost or extract is an expression. So the issues we run into with computing PSs from patterns need only be considered in the context of searching and replacing. In fact, we only need to consider replacement since the search part matches over existing terms which have property set requirements baked into argument Ids. 

Suppose we have a rule specifying a search pattern and replacement pattern and a single substitution map from variables to classes for a successful search. Let's consider replacement patterns (assuming we're using the safer set of requirements on $F$ and $G$) by cases.
1. Case 1: The pattern is either just an expression, or none of the arguments that $F$ or $G$ (for any index) depend on are variables. We treat this exactly like an expression.
2. Case 2: $F$ depends on an argument $a_i$ which is a variable in the pattern. Since we don't actually need the result of $F$ to do matching or construct a term, this is fine. This case is handled entirely by whatever term strategy we pick above.
3. Case 3: $G$ depends an argument $a_i$ which is a variable in the pattern. We need the output of $G$ for every argument to construct the MulteIds used in the term. Now we have to make some decisions about behavior.

The question here is similar to the one above: Can we compute a property set from an e-class? If not, how do we proceed with inserting the replacement?

Some options, in no particular order:
- Default to PS $\bot$ (*note: not the failure element $\^\bot$*). 
  - This is problematic because it can result in "invalid" expressions which claim to have lower property requirements than they should.
- Prevent rules like this at compile time. We can do a static analysis to determine if any rules result in patterns where PS computations depend on variables and we can simply not allow those rules.
- Term strategy 1 -> Pick an element from the eclass and use it to compute the property. 
  - This uses the property sets are equivalent modulo eclass of the elements used to compute them idea from above.
- Term strategy 2 -> Assuming we know the requirements on $a_i$, go into the class containing any elements that could be $a_i$ and for every possible value of $a_i$ make a substitution set using that value.
  - Again, makes a lot of stuff that might be unnecessary, TBD.


### Bad Ideas, do we need to explicitly prevent them?
- [ ] Recursive map functions, ideally properties won't depend on other properties. The most recursion we can reasonably allow is determining the concrete value of an argument and passing it back up, not computing properties at every level.

