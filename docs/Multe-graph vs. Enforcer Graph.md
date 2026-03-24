---
---

# Problem Setup

## Language Constraints

Suppose we have a language consisting of:
- 2 Operators: $A$ and $B$
- Variables: Terminal constants represented by strings like $x$ and $y$
- 1 Property: $p$ (and $\bot$ i.e. the absence of properties)
- Properties form a lattice with a least element $\bot$.
- Our simple property systems has the following partial order: $\bot\lt p$
- Denote the property value of a term $t$ with $\rho(t)$. Let $x^p$ be shorthand for a term $x$ with $\rho(x)=p$.

We obey the following constraints/assumptions:
- B requires all its arguments to have property $p$ and maintains property $p$ itself.
- A accepts arguments with any properties.
- The properties of an $A$ expression are computed from the join of the arguments' properties as follows: $\rho(A(w, z))=P(w) \land P(z)$. i.e. if both arguments have property $p$, then the $A$ term has property $p$, otherwise $\bot$.
- The equivalence relation we want to model is equivalence modulo properties. Properties affect two things:
- valid arguments - terms can require that their arguments have *at least* certain properties.
- cost - the properties of terms and arguments can make them more efficient to compute i.e. cost less.
- A and B are equivalent operators modulo properties, that is, A and B perform the same operation but their acceptable arguments and performance can differ depending on properties.

## Goals

We want to represent the following classes of equivalent terms:
- $A(x, y)$, $A(x^p, y)$, $A(x, y^p)$, $A^p(x^p, y^p)$, $B^p(x^p, y^p)$
- $x$, $x^p$
- $y$ , $y^p$

Our graph structure should maintain:
- The same terms with and without properties (e.g. $x$ and $x^p$) are distinct, but still equivalent.
- Operators should only accept valid arguments (i.e. arguments that satisfy property requirements)
- Property requirements and equivalence modulo properties leverage the property lattice. i.e. $x^p$ and $x$ are equivalent to any operators without property requirements, but *not* equivalent to an operator that requires property $p$.

# Traditional E-Graph Attempts

First, let's try to encode this in a traditional e-graph using egg. Without any notion of properties, we would have a simple language and ruleset:

``` rust
define_language!{
    pub enum ABLang {
        "A" = A([Id; 2]),
        "B" = B([Id; 2]),
        Var(Symbol)
    }
}

let rules = vec![
    rewrite!("a-to-b"; "A(?x ?y)" => "B(?x, ?y)"),
];
```

and running this with egg produces:
<img
src="Multe-graph%20vs.%20Enforcer%20Graph-media/82880562e0dac24f80490961762dfc18e5b81422.png"
class="wikilink" alt="basic-egraph.png" />

We have no property encoding, so there's no way to know if $x$ and $y$ are in fact valid arguments to $B$. We could, perhaps, determine that information at extraction time with a clever cost function; however, we cannot determine from the e-graph alone what properties any of these terms have or require.

## Naive Attempt 1

Let's attempt to encode the properties in our language and equivalence relation:

``` rust
define_language! {
    pub enum Level2 {
        // Basic A operator without property p
        "A" = A([Id; 2]),
        // A operator with property p
        "AP" = AP([Id; 2]),
        // B operator with property p
        "BP" = BP([Id; 2]),
        // operator to apply property p to terms
        "P" = P(Id),
        // typical variables
        Var(Symbol),
    }
}
```

Now, we have 3 "function" operators: $A$, $A^p$, and $B^p$ (notice we don't have plain $B$ because operator $B$ always has property $p$ by its definition) and an "enforcer" operator $P$ to manually apply properties to terms.

Adapting the previous e-graph to our new encoding, we might expect:
<img
src="Multe-graph%20vs.%20Enforcer%20Graph-media/72188e84013b5136ff9bcccd707685df850f26f4.png"
class="wikilink" alt="first-approx-egraph.png" />
Although this seems reasonable at first, the approach has a few major issues:
- **Equivalence Modulo Properties** -\> This represents the 9 terms we aimed to represent initially and does not include any invalid terms (like $B$ with non-$p$ arguments); however, we do not have the right equivalence relation: equivalence modulo properties. We cannot discern a relationship between the $A^p$, $B^p$ terms and the $A$ terms, even though to a parent operator without property requirements they ought to be considered equivalent. Similarly, we don't know of any relationship between $x$, $y$ and $x^p$, $y^p$.
- **EqSat via Rewrite Rules**-\> How would we construct the rules for this example? Given an initial expression $A(x, y)$, we have no way to generate either $A^p$ or $B^p$ without some notion of equivalence to $A$. Further, how would we wrap terms like $x$ with the $P$ enforcer to get $x^p$ without relying on negation?
- **Cost Based Extraction** -\> Suppose we can magically solve the rewrite issue and can generate this e-graph from an initial expression of $A(x, y)$ with a sound ruleset. Now, we need to extract the optimal term. Our only options are the three $A$ expressions in our root e-class...but what if $A^p$ or $B^p$ actually *costs less*? We have no way to know because we are prevented from considering expressions with *more* properties than what we strictly require.
Maybe we can refine our encoding of properties and produce something sensible.
\## Naive Attempt 2
Let's take our property enforcer e-graph example one step further and add a "forgetful" operator that simply ignores properties. With this operator, we set $A$ equivalent to $A^p$ by simply forgetting that $A^p$ has property $p$. Now, we can say $A^p$ is not totally equivalent to $A$, rather, it is only equivalent to $A$ if we ignore some information.

``` rust
define_language! {
    pub enum Level3 {
        // The first four operators are the same as our previous example
        "A" = A([Id; 2]),
        "AP" = AP([Id; 2]),
        "BP" = BP([Id; 2]),
        "P" = P(Id),
        // ignore_p is a "forgetful" operator to ignore P when needed
        "ignore_p" = Pn(Id),
        Var(Symbol),
        
        // QUESTION: Should we add B = ignore_p(BP(...))?
    }
}
```

Our `ignore_p` operator can be wrapped around $p$ operators to relate them to non-p operators. For example:

``` rust
let mut rules: Vec<Rewrite<Level2, ()>> = vec![
    // Inflate variables to property and non property versions of themselves
    rw!("inflate x to ignore p pp x"; 
        "?x" => "(ignore_p (P ?x))" 
        if is_variable("?x")),
    // Ignore p of P cancels
    rw!("ignore-p-to-x"; 
        "(ignore_p (P ?x))" => "?x"),
];

rules.extend(vec![
    // ignorep of AP is equivalent to A 
    rw!("ignore_p ap to a"; 
        "(ignore_p (AP ?x ?y))" <=> "(A (ignore_p ?x) (ignore_p ?y))"),
    // AP = BP
    rw!("ap eq bp"; 
        "(AP ?x ?y)" <=> "(BP ?x ?y)"),
].concat(),);
```

What are these rules doing?
- Our first rule "inflates" terminals by wrapping them in ignore_p of P. The `if_variable` condition checks that all nodes in the `?x` e-class are Variables. This prevents this rule from running on the same e-class twice (since it inserts an `ignore_p` operator that will immediately cause is_variable to fail) and prevents us from adding property wrappers willy-nilly and inflating the e-graph unnecessarily.
- The second rule is the simplification of `ignore_p` of `P`. We write it separately so that it runs unconditionally, not just on variables.
- Our next rule says that ignore_p of AP is equivalent to A with ignore_p applied to each argument. Why do we push the ignore_p down? By definition, $A$ only has property $p$ if both of its arguments have property $p$. Since we are dividing $A$ and $A^p$ into distinct operators, $A$ can *never* have property $p$, so it need not consider the property $p$ of its arguments.
- This aligns with our semantics but is not necessarily a general approach. If, for some reason, we wanted to consider the performance impact of giving $A$ two $p$ arguments and explicitly remove the $p$ property of $A$ this approach would not suffice. Our definition of the `ignore_p` operator is not one that *removes* properties, but one that temporarily *forgets* them.
- Our final rule says that $A^p$ and $B^p$ are totally equivalent - they are equivalent operations with equivalent properties.

Given just these 4 rules and an initial expression $A(x, y)$, we can run egg and produce the following e-graph:

<figure>
<img
src="Multe-graph%20vs.%20Enforcer%20Graph-media/e3c2e7f79979227fcf5e6c6f4fadc61624cb7028.png"
class="wikilink" alt="second-approx-egraph.png" />
<figcaption aria-hidden="true">second-approx-egraph.png</figcaption>
</figure>

This definitely looks better. It seems we have solved our three issue with the first approach, but something is still incomplete...what if our initial expression had been $B^p(x^p,^p)$? Everything so far has assumed we begin with $A(x,y)$, but this isn't a guarantee. In order to truly explore all options, we should be able to start from any valid expression and generate all equivalent expressions.
\## Naive Attempt 3
Suppose, for example, that manually applying the property $p$ is a a very expensive operation. Earlier, we asked "what if $A^p$ and $B^p$ are cheaper even though they have more properties than the initial expression?". This could be possible if $B$ is a very efficient operation *and* we already have $x^p$ and $y^p$ cached. But perhaps we don't have $x^p$ and $y^p$ computed and we want to minimize manually applying $p$ . In our initial example, we can just use our input expression $A(x, y)$, but if our initial expression *required* property $p$, then we would need a way to get from, say $B^p(x^p, y^p)$ to $P(A(x, y))$. There is no path from $B^p$ to $P(A)$ in our current structure, so we should add a rule like:

``` rust
rw!("p a to ap"; "(AP (P ?x) (P ?y))" <=> "(P (A ?x ?y))"),
```

and generate the final e-graph:

<figure>
<img
src="Multe-graph%20vs.%20Enforcer%20Graph-media/ed73b66ca7f40e7850e8c0da5322212ea769fcb0.png"
class="wikilink" alt="third-approx-egraph.png" />
<figcaption aria-hidden="true">third-approx-egraph.png</figcaption>
</figure>

At this point, we finally have all the property/non property representations of our expressions. Two issues are clear:
- **Size** - The graph is much larger than our no properties starting point. We have gone from 4 nodes, 3 classes, and 4 edges to 11 nodes, 6 classes, and 12 edges.
- **Cycles and Complexity** - Introducing operators to track the presence and lack of properties creates cycles. Of course, the property operations are idempotent (so these aren't incorrect) and we can employ heuristics to avoid taking such cycles indefinitely, but they add complexity to the graph that isn't intuitive or strictly necessary.

Although embedding property information in a traditional e-graph with a clever encoding is possible, it is not necessarily efficient. This is one of the simplest examples we can construct. Imagine a real language with a multitude of operators that support various complex properties (i.e. not just the binary lattice). Not only does the graph explode, manually maintaining properties in the rules is challenging!

We propose a variant on the e-graph, a multe-graph, as a solution.
\# Multe-Graph
*For now, we don't formally define the multe-graph, but demonstrate its structure using our working example.*

To represent these terms in a multe-graph, we can use exactly the language of terms and operators provided. Just as in an e-graph, an e-node or e-class represents not just a single term but a collection of equivalent terms. Additionally, every node/class stores the set of properties *provided* by the terms it represents. The top level e-class might have substantial variety in the potential properties its terms have, so we also have virtual subclasses which restrict the e-class to the subset of nodes with a more specific property or set of properties.

The corresponding multe-graph for our example is:
<img
src="Multe-graph%20vs.%20Enforcer%20Graph-media/ef6139014167a28463444ae500f963f7af9547f1.png"
class="wikilink" alt="mutlegraph1.png" />

The graph has been color coded by properties - blue for terms with $p$, red for terms with $\bot$, and purple for terms with both.
- All the top level e-classes have terms with and without $p$, so they are purple.
- The individual $x$ and $y$ nodes do not have $p$, so they are red; whereas their $x^p$ and $y^p$ counter parts are blue.
- Since $B$ requires $p$, it points to a virtual subclass for each argument restricting the argument subclass to terms that also have property $p$.
- Finally, since $A$ accepts $x^p$ and $y^p$ as arguments and produces $A^p$, its node is purple since it produces both $A$ and $A^p$.

As you can see, all the terms we are interested in are represented by the multe-graph. More importantly, note what *is not* represented by the multe-graph: $B(x, y)$, $B(x^p, y)$, $B(x, y^p)$ (which are all invalid terms). $B$ does not point to the top-level argument e-class but to subclasses that satisfy property requirements.

The multe-graph more closely resembles our initial, no-property e-graph. It has far fewer nodes/classes/edges than our final version of the enforcer e-graph and it contains no unnecessary cycles. We can see the obvious relationships between terms and their property based differences very easily.

Finally, we can revert to our initial, very simple rewrite ruleset that equates $A$ and $B$. Our language definition includes the notion of property requirements, so they need not be specified in the ruleset.[^1]
\## Size Comparison Table

|             | Multe-Graph | Enforcer E-Graph | % increase |
|:------------|-------------|------------------|------------|
| Terms       | 9           | 9 [^2]           |            |
| Nodes       | 6           | 11               | 83.3%      |
| Classes     | 3           | 6                | 100%       |
| Sub-classes | 2           | \-               | \-         |
| Edges       | 4           | 12               | 200%       |

# Footnotes

[^1]: Depending on the domain, generating the "property" versions of terminal inputs can either be specified in the ruleset (e.g. with a rule similar to our previous "inflation" rule) or by initializing the graph with some sort of catalog data of existing inputs.

[^2]: The enforcer e-graph technically represents infinitely many terms of form `ignore_p(P(ignore_p(P(...))))`, but we don't count them here.
