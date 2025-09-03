# Notes

For now, this readme just contains my dev notes on the repo. In the future I'll put these elsewhere and make a pretty readme that explains the crate.

## On Generics

In meeting on 8/27 we highlighted that I was using too many traits and generics. 

### Changes I've made since that meeting:
  - `AST` renamed to `OpLang` for readability
  - `Property` renamed to `PropertySet` for understanding
  - Eliminated `PropLang` -> unnecessary
  - Eliminated `Analysis` (for now) -> making use of Egraph analysis might be interesting in the future, but for now I'm going to omit it to simplify things.
  - Unified `Id` and `MulteId` into a single `Id` enum and included `PropId` to make clear distinction between `Id`s that only correspond to a Property Set (i.e. outside the context of eclasses)
    - Not entirely sure if I want to commit to this approach yet.
    - Updated unionfind to take `usize` only since `Id` is no longer just a type alias (otherwise things get wonky) and the egraph handles unwrapping `Id`s correctly to use in the unionfind
    - Also eliminates the need to `Term` and `MulteTerm`
  - Made `Subst` less generic, now it is always just `Var` to `Id`
  - Eliminated `OpInfo` entirely -> How do we get this info now?
    - arity comes from the `OpLang` trait (each operator should have a specific, fixed arity (at least for now))
    - We provide functions using the function above to specifically get the `PropertySet` for output or a given input
  - Added `PropInfo` to hold the set of functions we need to map from expr/term/pattern to propertyset
    - defines relationship between a property set and an oplang
  - `PropertyMap` Simplified to wrapper struct on `BiMap`
  - More details below

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

### Implications of adding propset to `Expr`
  - Adding `propset` to `Expr` means it will now be part of the hashconsed information mapping terms to Ids. 
  - The concern here is that two terms like `Op(x, y)` and `Op(x, y)` (which previously would have been a single expression mapping to a single Id in the hascons) might somehow have different property set Ids and suddenly get mapped to 2 Ids in the hashcons. 
  - Since `PropertySets` are derived from an expression, any two identical expressions must have the same property set, thus it is impossible to have two such concerning expressions
    - The function mapping expr -> propertyset must be deterministic

## Property Sets and Correctness Requirements

On it's own, a `PropertySet` (from here on out referred to as PS for short) is just a structure that satisfies a partial order and provides a bottom element. Pretty straightforward. Now, when we relate a PS to an `OpLang` (aka language or operator in the language) we need a set of functions that provide the mapping from expression in the language to PS and those functions need to satisfy some specific properties. I don't believe I've clearly outlined these anywhere yet.

In the context of a language, a property either represents the concrete set of properties an expression *has* or the minimum set of properties an expression *needs to satisfy*. Some terminology:
- Let $L$ be the set of all possible expressions in the language
- Let $P(L)$ represent the set of all possible property sets of form $P$ in the language $L$.
  - Let $\bot\in P(L)$ denote the bottom element of the set of property sets. That is, $\forall p\in P(L)$ we have $\bot\le p$.
- An *expression* is a fully formed element $x\in L$. Typically, expressions are recursively defined using operators from the language and arguments that are either: a) expressions themselves, or b) terminal elements in the language.
- A *pattern* is an element $v\in L\cup V$ where $V$ is the set of all possible expressions in $L$ where at least one term in the expression is an unknown variable. Patterns, like expressions, are recursively defined using operators in the language and, in addition to expressions and terminals, permit variables as arguments.
- For an expression $x = f(a_1, a_2, \ldots, a_n)\in L$, let $p_x\in P$ denote the property set of expression $x$ and let $p_{x_i}\in P$ denote the property set requirements of argument $a_i$ in expression $x$.
  - $F: L\to P$ maps expressions to the property set they have. For an expression $x\in L$, $F(x) = p_x$.
  - $G: (L\times \N)\to P$ maps expressions $x\in L$ to the property set requirements of their argument at index $i\in\N$. Then $G(x, i) = p_{x_i}$.
- Given an expression $x = f(a_1, a_2, \ldots, a_n)\in L$, we say a suitable argument $a_i$ is any expression $y$ with $p_{x_i} \le p_y$, that is the properties $y$ has contain the properties required by $x$ or argument $i$. We say $y$ ***satisfies*** $p_{x_i}$.
- For a pattern $v\in L\cup V$, we will also define mapping functions as above; however, it is possible that variables in the pattern may obfuscate the information needed to determine the specific property set. Since $\bot$ refers to the minimal element of a property set, we use $\^\bot$ to represent this "failure" case.
  - $F': L\cup V\to P\cup\{\^\bot\}$ maps patterns to the property set they have.
  - $G': ((L\cup V)\times \N)\to P\cup\{\^\bot\}$ maps patterns to the property set requirements of their argument at index $i\in\N$.
  - If we do not include $\^\bot$ and simply map to $P$ then $F'$ and $G'$ are nondeterminstic (even if we try our hardest to handle failure cases cleverly.)
  - More discussion on this and potential other approaches in the following section.

### Mapping from Expression/Term to PropertySet

TL;DR the core questions I want to dig into are:
- [ ] How do we compute a property set from an e-class?
  - To be clear, the question is not "how do we compute the property *of* the e-class?". Instead "if we have a term whose property set is derived from one of its arguments and that argument is an e-class with multiple e-nodes, how do we use the e-class in the derivation?". 
  - We see this issue when inserting expressions to the e-graph (either fully formed or replacement patterns with variables).
- [ ] If we cannot use an entire e-class in a property set computation, how do we handle variables in patterns?
  - This question assumes that in the "fully formed expression" case, we duplicate expressions with differing property set requirements during repair (I explain this better below).

> **In this section it is *really* important to distinguish between the PS of an expression/term and a PS derived from an expression/term that is associated with another (likely parent) expression/term.**

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

|Expr | Id|
|:--|:--|
|`A`| 1|
|`B`| 2 |
|`x`| 3|
|`MergeJoin(1, 2, 3)`|4|

Now supposed we also know the property requirement "sorted by `x`" (probably a bitmap) is referred to with property set Id `10`. Now, we can impose property requirements in the Term using a MutleId (logical, property) like so:

|Expr | Id|
|:--|:--|
|`A`| 1|
|`B`| 2 |
|`x`| 3|
|`MergeJoin((1, 10), (2, 10), (3, 0))`|4|

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