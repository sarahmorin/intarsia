# MultEGraph Optimizer (MultEOpt until I can come up with something better)

The MulteOpt repo holds a first pass at a MulteGraph implementation that will eventually be extended to a full query optimization framework. Since this is very much a work on progress, this README doesn't have useful setup instructions.

The idea behind the multe-graph is to support more nuanced reasoning in a single, compact e-graph structure. An e-graph is capable of compactly representing a set of equivalent expressions; however, it can only maintain a single notion of equivalence.
The multe-graph extends this capability by supporting a single, coarse-grained notion of equivalence and *N* sub-equivalences.
For example, a NestedLoops Join and a SortMerge Join with the same input tables/predicates are logically equivalent since they produce the same set of tuples, but they are not equivalence with respect to sortedness.
We could also consider funkier properties like "number of tables touched" to produce some output. 

Once we have the multe-graph, we use it as the foundation for an optimization framework.
An e-graph is very similar to the memo-table used in cascades style, top-down query optimizers (e.g. Microsoft SQL).
We decompose the traditional e-matching process into a set of search tasks that can then be composed into a variety of search strategies.
*More details on this later*.

## Background Information

- **E-Graphs**: [egg](https://www.mwillsey.com/papers/egg) is an e-graph library written in Rust
  - egg is a really good place to start to understand the basics of an e-graph and it offers more support for getting started and playing around than the current multe-graph repo.
- **Query Optimization**: If you want more background on traditional query optimization techniques, these are some good starting points:
  - System R (bottom-up) - IBMs OG DBMS, [Access path selection in a relational database management system](https://dl.acm.org/doi/10.1145/582095.582099) describes part of the system R optimizer
  - Cascades (top-down) - The original cascades paper is inscrutable, these [CMU lecture notes](https://15799.courses.cs.cmu.edu/spring2025/notes/05-cascades.pdf) are much more digestible
  - [Randomized algorithms for optimizing large join queries](https://dl.acm.org/doi/10.1145/93597.98740)
  - Worst-case optimal Join optimizations
